use serde::{Deserialize, Serialize};

use crate::analysis::{Confidence, Provenance};
use crate::ids::{PlantId, PlotId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrowthStage {
    Seedling,
    Vegetative,
    Flowering,
    Fruiting,
    Harvest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Low,
    Normal,
    Urgent,
}

/// Rujukan ke aturan yang menyala, agar rekomendasi dapat ditelusuri (traceable).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleRef {
    pub rule_id: String,
    pub rule_version: String,
}

/// Keluaran M5. Selalu menyertakan penjelasan dan provenance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recommendation {
    pub plot_id: PlotId,
    pub plant_id: PlantId,
    pub stage: GrowthStage,
    /// Tindakan yang disarankan, mis. irigasi, pengapuran, drainase.
    pub action: String,
    pub priority: Priority,
    pub confidence: Confidence,
    /// Penjelasan langkah demi langkah yang bisa dibaca manusia.
    pub explanation: Vec<String>,
    pub fired_rules: Vec<RuleRef>,
    pub provenance: Provenance,
}
