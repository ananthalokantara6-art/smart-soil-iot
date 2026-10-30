//! # soil_contracts
//!
//! Satu-satunya crate yang boleh menjadi dependensi bersama antar modul.
//! Modul TIDAK boleh saling import; penyambungan dilakukan di `smart_soil_orchestrator`
//! lewat `Arc<dyn Trait>`.
//!
//! Alur data:
//! `M1 (validasi) -> M2 (transport) -> M3 (simpan) -> M4 (analitik) -> M5 (plant-specific) -> M3 (tampil)`
//! M4 dan M5 didaftarkan sebagai plugin melalui M6.

pub mod analysis;
pub mod error;
pub mod expert;
pub mod ids;
pub mod plugin;
pub mod ports;
pub mod reading;

#[cfg(feature = "mock")]
pub mod mock;

/// Versi kontrak. Perubahan MAJOR = breaking change; plugin memeriksa ini.
pub const CONTRACTS_VERSION: &str = env!("CARGO_PKG_VERSION");

pub use analysis::{
    Alert, AlertLevel, AnalysisResult, Anomaly, Confidence, Provenance, RiskKind, RiskLevel,
    RiskScore, Trend, TrendDirection,
};
pub use error::{ContractError, Result};
pub use expert::{GrowthStage, Priority, Recommendation, RuleRef};
pub use ids::{DeviceId, PlantId, PlotId, UserId};
pub use plugin::{Plugin, PluginHost, PluginKind, PluginMeta};
pub use ports::{
    AlertSink, Analyzer, DeviceDirectory, ReadingSink, ReadingSource, ReadingValidator,
    RuleEvaluator,
};
pub use reading::{Parameter, QualityFlag, RawReading, ReadingWindow, SoilReading};
