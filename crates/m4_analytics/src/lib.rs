//! M4 - Soil Analytics and Decision Support (TANPA deep learning).
//!
//! Implementasi awal: skor risiko waterlogging dan kekeringan dari rata-rata kelembapan.
//! TODO kelompok 4: deteksi anomali pH/EC, tren, prediksi, rekomendasi irigasi, ketidakpastian
//! yang lebih baik (interval kepercayaan).

pub mod analytics;

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use soil_contracts::{
    AnalysisResult, Analyzer, Confidence, ContractError, Parameter, Plugin, PluginHost, PluginKind,
    PluginMeta, Provenance, ReadingWindow, Result, RiskKind, RiskLevel, RiskScore, Trend,
    TrendDirection,
};

pub const ID: &str = "m4.threshold_risk";
pub const VERSION: &str = "0.1.0";

/// Ambang kelembapan (% VWC) untuk gambut. Nilai awal; kelompok 4 wajib mengkalibrasi
/// dan mencatat sumber literaturnya di `docs/research-notes.md`.
const WET_START: f32 = 75.0;
const WET_FULL: f32 = 95.0;
const DRY_START: f32 = 45.0;
const DRY_FULL: f32 = 20.0;
/// Jumlah sampel yang dianggap cukup untuk kepercayaan penuh.
const FULL_CONFIDENCE_SAMPLES: f32 = 30.0;

#[derive(Debug, Clone, Default)]
pub struct ThresholdAnalyzer;

fn level(score: f32) -> RiskLevel {
    match score {
        s if s >= 0.85 => RiskLevel::Critical,
        s if s >= 0.6 => RiskLevel::High,
        s if s >= 0.3 => RiskLevel::Medium,
        _ => RiskLevel::Low,
    }
}

/// Interpolasi linear 0..1 antara `from` (0.0) dan `to` (1.0), dipotong.
fn ramp(x: f32, from: f32, to: f32) -> f32 {
    ((x - from) / (to - from)).clamp(0.0, 1.0)
}

/// Kemiringan regresi linear sederhana (satuan per jam). `None` jika data kurang.
fn slope_per_hour(points: &[(chrono::DateTime<Utc>, f32)]) -> Option<f32> {
    if points.len() < 2 {
        return None;
    }
    let t0 = points[0].0;
    let xs: Vec<f32> = points
        .iter()
        .map(|(t, _)| (*t - t0).num_seconds() as f32 / 3600.0)
        .collect();
    let n = points.len() as f32;
    let mean_x = xs.iter().sum::<f32>() / n;
    let mean_y = points.iter().map(|(_, y)| *y).sum::<f32>() / n;
    let sxx: f32 = xs.iter().map(|x| (x - mean_x).powi(2)).sum();
    if sxx == 0.0 {
        return None;
    }
    let sxy: f32 = xs
        .iter()
        .zip(points)
        .map(|(x, (_, y))| (x - mean_x) * (y - mean_y))
        .sum();
    Some(sxy / sxx)
}

#[async_trait]
impl Analyzer for ThresholdAnalyzer {
    fn meta(&self) -> PluginMeta {
        PluginMeta {
            id: ID.into(),
            name: "Threshold risk analyzer".into(),
            version: VERSION.into(),
            kind: PluginKind::Analytics,
            description: "Skor risiko waterlogging dan kekeringan berbasis ambang kelembapan"
                .into(),
            requires_contracts_major: 0,
        }
    }

    async fn analyze(&self, window: &ReadingWindow) -> Result<AnalysisResult> {
        let moisture = window.series(Parameter::Moisture);
        if moisture.is_empty() {
            return Err(ContractError::Validation(
                "tidak ada pembacaan yang dapat dipakai pada jendela ini".into(),
            ));
        }

        let n = moisture.len() as f32;
        let mean = moisture.iter().map(|(_, v)| *v).sum::<f32>() / n;
        let confidence = Confidence::new(n / FULL_CONFIDENCE_SAMPLES);

        let wet = ramp(mean, WET_START, WET_FULL);
        let dry = ramp(-mean, -DRY_START, -DRY_FULL);

        let risks = vec![
            RiskScore {
                kind: RiskKind::Waterlogging,
                score: wet,
                level: level(wet),
                confidence,
            },
            RiskScore {
                kind: RiskKind::Dryness,
                score: dry,
                level: level(dry),
                confidence,
            },
        ];

        let trends = slope_per_hour(&moisture)
            .map(|slope| {
                let direction = if slope > 0.5 {
                    TrendDirection::Rising
                } else if slope < -0.5 {
                    TrendDirection::Falling
                } else {
                    TrendDirection::Stable
                };
                vec![Trend {
                    parameter: Parameter::Moisture,
                    direction,
                    slope_per_hour: slope,
                    confidence,
                }]
            })
            .unwrap_or_default();

        Ok(AnalysisResult {
            plot_id: window.plot_id,
            window_from: window.from,
            window_to: window.to,
            risks,
            anomalies: vec![],
            trends,
            sample_count: moisture.len(),
            provenance: Provenance {
                source: ID.into(),
                version: VERSION.into(),
                method: "threshold_ramp_on_mean_moisture".into(),
                inputs: vec![
                    format!("samples={}", moisture.len()),
                    format!("mean_moisture_pct={mean:.2}"),
                    format!("wet_ramp={WET_START}..{WET_FULL}"),
                    format!("dry_ramp={DRY_START}..{DRY_FULL}"),
                ],
                generated_at: Utc::now(),
            },
        })
    }
}

impl Plugin for ThresholdAnalyzer {
    fn meta(&self) -> PluginMeta {
        Analyzer::meta(self)
    }

    fn register(&self, host: &mut dyn PluginHost) -> Result<()> {
        host.register_analyzer(Arc::new(self.clone()));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use soil_contracts::{DeviceId, PlotId, QualityFlag, SoilReading};

    fn window(moisture: f32, count: i64, quality: QualityFlag) -> ReadingWindow {
        let plot = PlotId::new();
        let t0 = Utc::now();
        let readings = (0..count)
            .map(|i| SoilReading {
                device_id: DeviceId::new(),
                plot_id: plot,
                ts: t0 + Duration::minutes(i),
                moisture_pct: moisture,
                temperature_c: 27.0,
                ph: 4.0,
                ec_ms_cm: 0.5,
                quality,
            })
            .collect();
        ReadingWindow {
            plot_id: plot,
            from: t0,
            to: t0 + Duration::minutes(count),
            readings,
        }
    }

    fn risk(result: &AnalysisResult, kind: RiskKind) -> RiskScore {
        result
            .risks
            .iter()
            .find(|r| r.kind == kind)
            .cloned()
            .expect("risk ada")
    }

    #[tokio::test]
    async fn waterlogged_soil_is_high_risk() {
        let r = ThresholdAnalyzer
            .analyze(&window(92.0, 30, QualityFlag::Good))
            .await
            .unwrap();
        assert!(risk(&r, RiskKind::Waterlogging).level >= RiskLevel::High);
        assert_eq!(risk(&r, RiskKind::Dryness).level, RiskLevel::Low);
    }

    #[tokio::test]
    async fn dry_soil_is_high_risk() {
        let r = ThresholdAnalyzer
            .analyze(&window(25.0, 30, QualityFlag::Good))
            .await
            .unwrap();
        assert!(risk(&r, RiskKind::Dryness).level >= RiskLevel::High);
    }

    #[tokio::test]
    async fn result_has_provenance_and_confidence() {
        let r = ThresholdAnalyzer
            .analyze(&window(60.0, 15, QualityFlag::Good))
            .await
            .unwrap();
        assert_eq!(r.provenance.source, ID);
        assert!(!r.provenance.inputs.is_empty());
        let c = risk(&r, RiskKind::Waterlogging).confidence.value();
        assert!((c - 0.5).abs() < 1e-6);
    }

    #[tokio::test]
    async fn bad_readings_are_ignored() {
        let err = ThresholdAnalyzer
            .analyze(&window(60.0, 10, QualityFlag::Bad))
            .await
            .unwrap_err();
        assert!(matches!(err, ContractError::Validation(_)));
    }
}
