use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::PlotId;
use crate::reading::Parameter;

/// Tingkat kepercayaan 0.0..=1.0 (nilai di luar rentang dipotong).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Confidence(f32);

impl Confidence {
    pub fn new(v: f32) -> Self {
        Self(if v.is_nan() { 0.0 } else { v.clamp(0.0, 1.0) })
    }

    pub fn value(self) -> f32 {
        self.0
    }
}

/// Asal-usul sebuah keputusan: wajib ada di setiap hasil analitik dan rekomendasi
/// (permintaan dosen: provenance dan uncertainty).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Provenance {
    /// Id plugin/modul penghasil, mis. `"m4.rule_risk"`.
    pub source: String,
    pub version: String,
    /// Metode, mis. `"rolling_zscore"`, `"threshold_rules"`.
    pub method: String,
    /// Deskripsi masukan yang dipakai (rentang waktu, jumlah sampel, versi aturan).
    pub inputs: Vec<String>,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskKind {
    Waterlogging,
    Dryness,
    PhShift,
    EcAnomaly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RiskScore {
    pub kind: RiskKind,
    /// 0.0 (aman) .. 1.0 (sangat berisiko).
    pub score: f32,
    pub level: RiskLevel,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Anomaly {
    pub ts: DateTime<Utc>,
    pub parameter: Parameter,
    pub description: String,
    pub severity: RiskLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrendDirection {
    Rising,
    Falling,
    Stable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Trend {
    pub parameter: Parameter,
    pub direction: TrendDirection,
    /// Perubahan per jam dalam satuan parameter.
    pub slope_per_hour: f32,
    pub confidence: Confidence,
}

/// Keluaran M4, masukan M5.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub plot_id: PlotId,
    pub window_from: DateTime<Utc>,
    pub window_to: DateTime<Utc>,
    pub risks: Vec<RiskScore>,
    pub anomalies: Vec<Anomaly>,
    pub trends: Vec<Trend>,
    pub sample_count: usize,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertLevel {
    Info,
    Warning,
    Critical,
}

/// Peringatan untuk ditampilkan M3 (riwayat peringatan di dashboard).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Alert {
    pub plot_id: PlotId,
    pub ts: DateTime<Utc>,
    pub level: AlertLevel,
    pub message: String,
    pub provenance: Provenance,
}
