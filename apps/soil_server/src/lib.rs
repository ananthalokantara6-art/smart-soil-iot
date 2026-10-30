//! Orchestrator: satu-satunya tempat modul M1-M6 saling disambungkan.
//!
//! Modul tidak saling mengenal; semuanya bertemu di sini lewat trait `soil_contracts`.
//! Jika sebuah modul diganti implementasinya, cukup ubah `App::new`.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use soil_contracts::{
    Alert, AlertLevel, AlertSink, AnalysisResult, ContractError, DeviceId, GrowthStage, PlantId,
    PlotId, RawReading, ReadingSource, Recommendation, Result, RiskKind, RiskLevel, SoilReading,
    UserId,
};

use m1_sensing::RangeValidator;
use m2_gateway::{Gateway, StaticDirectory};
use m3_platform::MemoryPlatform;
use m4_analytics::ThresholdAnalyzer;
use m5_expert::BasicRuleEvaluator;
use m6_plugin::PluginRegistry;

pub struct App {
    pub platform: Arc<MemoryPlatform>,
    pub registry: PluginRegistry,
    gateway: Gateway,
    plot_id: PlotId,
}

impl App {
    /// Membangun aplikasi dengan satu perangkat, satu pemilik, dan satu petak.
    pub fn new(device: DeviceId, owner: UserId, plot_id: PlotId) -> Result<Self> {
        let platform = Arc::new(MemoryPlatform::new());

        let mut directory = StaticDirectory::new();
        directory.register(device, owner, plot_id);

        let gateway = Gateway::new(
            Arc::new(RangeValidator::default()),
            Arc::new(directory),
            platform.clone(),
        );

        // M4 dan M5 masuk lewat M6 sebagai plugin.
        let mut registry = PluginRegistry::new();
        registry.install(&ThresholdAnalyzer)?;
        registry.install(&BasicRuleEvaluator)?;

        Ok(Self {
            platform,
            registry,
            gateway,
            plot_id,
        })
    }

    /// Jalur ingest: M2 -> M1 -> M3.
    pub async fn ingest(&self, raw: RawReading) -> Result<SoilReading> {
        self.gateway.handle(raw).await
    }

    /// Jalur analitik: M3 -> M4 -> M5, dan peringatan ke M3 bila ada risiko tinggi.
    pub async fn assess(
        &self,
        plant: &PlantId,
        stage: GrowthStage,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<(AnalysisResult, Recommendation)> {
        let window = self.platform.window(&self.plot_id, from, to).await?;

        let analyzer = self
            .registry
            .analyzers()
            .into_iter()
            .next()
            .ok_or_else(|| ContractError::Plugin("tidak ada analyzer aktif".into()))?;
        let evaluator = self
            .registry
            .evaluators()
            .into_iter()
            .next()
            .ok_or_else(|| ContractError::Plugin("tidak ada evaluator aktif".into()))?;

        let analysis = analyzer.analyze(&window).await?;
        let recommendation = evaluator.evaluate(plant, stage, &analysis).await?;

        for risk in analysis.risks.iter().filter(|r| r.level >= RiskLevel::High) {
            let what = match risk.kind {
                RiskKind::Waterlogging => "waterlogging",
                RiskKind::Dryness => "kekeringan",
                RiskKind::PhShift => "pergeseran pH",
                RiskKind::EcAnomaly => "anomali EC",
            };
            self.platform
                .raise(Alert {
                    plot_id: self.plot_id,
                    ts: Utc::now(),
                    level: if risk.level == RiskLevel::Critical {
                        AlertLevel::Critical
                    } else {
                        AlertLevel::Warning
                    },
                    message: format!("Risiko {what} {:?}: {}", risk.level, recommendation.action),
                    provenance: analysis.provenance.clone(),
                })
                .await?;
        }

        Ok((analysis, recommendation))
    }
}
