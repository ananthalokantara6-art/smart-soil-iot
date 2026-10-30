use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::ports::{Analyzer, RuleEvaluator};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginKind {
    Analytics,
    ExpertRules,
    PlantModule,
    AiModel,
}

/// Metadata plugin: dasar untuk registrasi, discovery, dan pemeriksaan kompatibilitas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginMeta {
    /// Id unik, mis. `"m4.rule_risk"`.
    pub id: String,
    pub name: String,
    /// Versi plugin (semver).
    pub version: String,
    pub kind: PluginKind,
    pub description: String,
    /// Versi MAJOR `soil_contracts` yang dibutuhkan plugin.
    pub requires_contracts_major: u32,
}

impl PluginMeta {
    /// Kompatibel jika MAJOR kontrak sama.
    pub fn is_compatible(&self) -> bool {
        env!("CARGO_PKG_VERSION_MAJOR")
            .parse::<u32>()
            .map(|major| major == self.requires_contracts_major)
            .unwrap_or(false)
    }
}

/// Disediakan M6; dipakai plugin untuk mendaftarkan kemampuannya.
pub trait PluginHost: Send {
    fn register_analyzer(&mut self, analyzer: Arc<dyn Analyzer>);
    fn register_rule_evaluator(&mut self, evaluator: Arc<dyn RuleEvaluator>);
}

/// Diimplementasikan oleh setiap plugin (termasuk M4 dan M5 sebagai plugin bawaan).
pub trait Plugin: Send + Sync {
    fn meta(&self) -> PluginMeta;
    fn register(&self, host: &mut dyn PluginHost) -> Result<()>;
}
