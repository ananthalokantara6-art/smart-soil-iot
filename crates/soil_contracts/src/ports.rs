use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::analysis::{Alert, AnalysisResult};
use crate::error::Result;
use crate::expert::{GrowthStage, Recommendation};
use crate::ids::{DeviceId, PlantId, PlotId, UserId};
use crate::plugin::PluginMeta;
use crate::reading::{RawReading, ReadingWindow, SoilReading};

/// M1: kalibrasi, filter noise, dan deteksi outlier.
pub trait ReadingValidator: Send + Sync {
    fn validate(&self, raw: &RawReading, plot_id: PlotId) -> Result<SoilReading>;
}

/// M2: identitas perangkat dan kepemilikan.
#[async_trait]
pub trait DeviceDirectory: Send + Sync {
    async fn owner_of(&self, device: &DeviceId) -> Result<Option<UserId>>;
    async fn plot_of(&self, device: &DeviceId) -> Result<Option<PlotId>>;
}

/// M3 mengimplementasikan; M2 memanggil saat data masuk.
#[async_trait]
pub trait ReadingSink: Send + Sync {
    async fn ingest(&self, reading: SoilReading) -> Result<()>;
}

/// M3 mengimplementasikan; M4 dan dashboard membaca data historis.
#[async_trait]
pub trait ReadingSource: Send + Sync {
    async fn window(
        &self,
        plot_id: &PlotId,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<ReadingWindow>;

    async fn latest(&self, plot_id: &PlotId) -> Result<Option<SoilReading>>;
}

/// M4 mengimplementasikan (biasanya sebagai plugin Analytics). Tanpa deep learning.
#[async_trait]
pub trait Analyzer: Send + Sync {
    fn meta(&self) -> PluginMeta;
    async fn analyze(&self, window: &ReadingWindow) -> Result<AnalysisResult>;
}

/// M5 mengimplementasikan (biasanya sebagai plugin ExpertRules / PlantModule).
#[async_trait]
pub trait RuleEvaluator: Send + Sync {
    fn meta(&self) -> PluginMeta;
    async fn evaluate(
        &self,
        plant: &PlantId,
        stage: GrowthStage,
        analysis: &AnalysisResult,
    ) -> Result<Recommendation>;
}

/// M3 mengimplementasikan; M4 dan M5 menghasilkan peringatan.
#[async_trait]
pub trait AlertSink: Send + Sync {
    async fn raise(&self, alert: Alert) -> Result<()>;
}
