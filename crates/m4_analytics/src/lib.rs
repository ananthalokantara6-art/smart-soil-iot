//! M4 - Soil Analytics and Decision Support.

pub mod analytics;
pub mod decision;
pub mod error;
pub mod pipeline;
pub mod presentation;

use std::sync::Arc;

use async_trait::async_trait;
use soil_contracts::{
    AnalysisResult, Analyzer, Plugin, PluginHost, PluginKind, PluginMeta, ReadingWindow, Result,
};

pub const ID: &str = "m4.threshold_risk";
pub const VERSION: &str = "0.1.0";

#[derive(Debug, Clone, Default)]
pub struct ThresholdAnalyzer;

#[async_trait]
impl Analyzer for ThresholdAnalyzer {
    fn meta(&self) -> PluginMeta {
        PluginMeta {
            id: ID.into(),
            name: "Threshold risk analyzer".into(),
            version: VERSION.into(),
            kind: PluginKind::Analytics,
            description: "Analisis tanah terpadu berbasis komputasi fungsional murni".into(),
            requires_contracts_major: 0,
        }
    }

    async fn analyze(&self, window: &ReadingWindow) -> Result<AnalysisResult> {
        pipeline::process_soil_pipeline(window).map_err(Into::into)
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
    use chrono::{Duration, Utc};
    use soil_contracts::{DeviceId, PlotId, QualityFlag, RiskKind, RiskLevel, SoilReading};

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

    #[tokio::test]
    async fn waterlogged_soil_is_high_risk() {
        let r = ThresholdAnalyzer
            .analyze(&window(92.0, 30, QualityFlag::Good))
            .await
            .unwrap();
        let wet = r
            .risks
            .iter()
            .find(|x| x.kind == RiskKind::Waterlogging)
            .unwrap();
        assert!(wet.level >= RiskLevel::High);
    }

    #[tokio::test]
    async fn dry_soil_is_high_risk() {
        let r = ThresholdAnalyzer
            .analyze(&window(25.0, 30, QualityFlag::Good))
            .await
            .unwrap();
        let dry = r
            .risks
            .iter()
            .find(|x| x.kind == RiskKind::Dryness)
            .unwrap();
        assert!(dry.level >= RiskLevel::High);
    }
}
