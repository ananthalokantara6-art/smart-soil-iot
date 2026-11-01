//! Mesin keputusan preskriptif (Anggota 3): irigasi dan ameliorasi.

pub mod amendment;
pub mod irrigation;

use soil_contracts::{Parameter, ReadingWindow};

use crate::analytics::statistics;
use crate::error::{Result, SoilError};

pub use amendment::{recommend_soil_amendment, AmendmentAction, AmendmentConfig, SoilTexture};
pub use irrigation::{
    calculate_irrigation, calculate_water_deficit, IrrigationConfig, IrrigationRecommendation,
};

/// Konfigurasi gabungan DSS untuk satu petak.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DecisionConfig {
    pub irrigation: IrrigationConfig,
    pub amendment: AmendmentConfig,
}

/// Hasil DSS untuk satu jendela data: nilai yang dipakai + tindakan yang disarankan.
#[derive(Debug, Clone, PartialEq)]
pub struct Prescription {
    pub sample_count: usize,
    /// Kelembapan pembacaan usable TERAKHIR (dasar keputusan irigasi).
    pub current_moisture_pct: f32,
    /// Rata-rata pH jendela (pH berubah lambat, rata-rata meredam noise).
    pub mean_ph: f32,
    pub irrigation: Option<IrrigationRecommendation>,
    pub amendment: Option<AmendmentAction>,
}

/// Susun resep tindakan dari jendela pembacaan (hanya pembacaan `usable`, sama seperti pipeline M4).
///
/// * Irigasi memakai kelembapan pembacaan terakhir: menyiram adalah keputusan "sekarang",
///   rata-rata jendela akan menyembunyikan tanah yang sedang cepat mengering.
/// * Ameliorasi memakai rata-rata pH jendela.
pub fn prescribe(window: &ReadingWindow, cfg: &DecisionConfig) -> Result<Prescription> {
    let moisture_series = window.series(Parameter::Moisture);
    let current_moisture = moisture_series
        .iter()
        .max_by_key(|(ts, _)| *ts)
        .map(|(_, v)| *v)
        .ok_or(SoilError::EmptyDataset)?;

    let ph: Vec<f64> = window
        .series(Parameter::Ph)
        .iter()
        .map(|(_, v)| f64::from(*v))
        .collect();
    let mean_ph = statistics::mean(&ph).ok_or(SoilError::EmptyDataset)? as f32;

    Ok(Prescription {
        sample_count: moisture_series.len(),
        current_moisture_pct: current_moisture,
        mean_ph,
        irrigation: calculate_irrigation(current_moisture, &cfg.irrigation)?,
        amendment: recommend_soil_amendment(mean_ph, &cfg.amendment)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use soil_contracts::{DeviceId, PlotId, QualityFlag, SoilReading};

    fn window(rows: &[(f32, f32, QualityFlag)]) -> ReadingWindow {
        let (dev, plot) = (DeviceId::new(), PlotId::new());
        let t0 = Utc::now();
        ReadingWindow {
            plot_id: plot,
            from: t0,
            to: t0 + Duration::minutes(rows.len() as i64),
            readings: rows
                .iter()
                .enumerate()
                .map(|(i, &(m, ph, q))| SoilReading {
                    device_id: dev,
                    plot_id: plot,
                    ts: t0 + Duration::minutes(i as i64),
                    moisture_pct: m,
                    temperature_c: 27.0,
                    ph,
                    ec_ms_cm: 0.5,
                    quality: q,
                })
                .collect(),
        }
    }

    #[test]
    fn prescribes_both_actions_for_dry_acidic_soil() {
        let w = window(&[
            (50.0, 5.0, QualityFlag::Good),
            (38.0, 5.0, QualityFlag::Good),
        ]);
        let p = prescribe(&w, &DecisionConfig::default()).unwrap();
        // irigasi memakai pembacaan TERAKHIR (38%), bukan rata-rata (44%)
        assert!((p.current_moisture_pct - 38.0).abs() < 1e-3);
        assert_eq!(p.irrigation.unwrap().recommended_duration_minutes, 192);
        assert!(matches!(
            p.amendment,
            Some(AmendmentAction::ApplyAgriculturalLime { .. })
        ));
    }

    #[test]
    fn prescribes_nothing_when_everything_is_fine() {
        let w = window(&[(75.0, 6.4, QualityFlag::Good)]);
        let p = prescribe(&w, &DecisionConfig::default()).unwrap();
        assert!(p.irrigation.is_none());
        assert!(p.amendment.is_none());
    }

    #[test]
    fn ignores_bad_quality_readings() {
        // pembacaan Bad (outlier) tidak boleh ikut menurunkan rata-rata
        let w = window(&[(75.0, 6.4, QualityFlag::Good), (0.0, 1.0, QualityFlag::Bad)]);
        let p = prescribe(&w, &DecisionConfig::default()).unwrap();
        assert_eq!(p.sample_count, 1);
        assert!(p.irrigation.is_none());
    }

    #[test]
    fn irrigation_follows_latest_reading_not_the_mean() {
        // rata-rata 60% (tidak perlu siram) tetapi pembacaan terakhir 30% (kering)
        let w = window(&[
            (80.0, 5.0, QualityFlag::Good),
            (70.0, 5.0, QualityFlag::Good),
            (30.0, 5.0, QualityFlag::Good),
        ]);
        let p = prescribe(&w, &DecisionConfig::default()).unwrap();
        assert!(p.irrigation.is_some());
    }

    #[test]
    fn empty_usable_data_is_an_error() {
        let w = window(&[(50.0, 5.0, QualityFlag::Bad)]);
        assert_eq!(
            prescribe(&w, &DecisionConfig::default()),
            Err(SoilError::EmptyDataset)
        );
    }
}
