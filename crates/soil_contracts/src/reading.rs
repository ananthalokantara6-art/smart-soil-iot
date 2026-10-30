use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{DeviceId, PlotId};

/// Parameter tanah minimum yang diwajibkan dosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Parameter {
    Moisture,
    Temperature,
    Ph,
    Ec,
}

/// Hasil validasi M1 untuk satu pembacaan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityFlag {
    /// Lolos kalibrasi, filter noise, dan deteksi outlier.
    Good,
    /// Mencurigakan (mis. mendekati batas); boleh dipakai dengan kepercayaan rendah.
    Suspect,
    /// Outlier / sensor rusak; tidak boleh dipakai analitik.
    Bad,
    /// Sensor sedang dikalibrasi ulang.
    Calibrating,
}

/// Pembacaan mentah dari node, sebelum validasi M1. Satuan = keluaran sensor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawReading {
    pub device_id: DeviceId,
    pub ts: DateTime<Utc>,
    pub moisture_raw: f32,
    pub temperature_raw: f32,
    pub ph_raw: f32,
    pub ec_raw: f32,
}

/// Pembacaan tervalidasi. Inilah data standar yang mengalir M2 -> M3 -> M4.
///
/// Satuan: kelembapan = % VWC, suhu = derajat Celsius, pH = skala pH, EC = mS/cm.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoilReading {
    pub device_id: DeviceId,
    pub plot_id: PlotId,
    pub ts: DateTime<Utc>,
    pub moisture_pct: f32,
    pub temperature_c: f32,
    pub ph: f32,
    pub ec_ms_cm: f32,
    pub quality: QualityFlag,
}

impl SoilReading {
    /// Boleh dipakai analitik (bukan `Bad` atau `Calibrating`).
    pub fn is_usable(&self) -> bool {
        matches!(self.quality, QualityFlag::Good | QualityFlag::Suspect)
    }

    pub fn value(&self, p: Parameter) -> f32 {
        match p {
            Parameter::Moisture => self.moisture_pct,
            Parameter::Temperature => self.temperature_c,
            Parameter::Ph => self.ph,
            Parameter::Ec => self.ec_ms_cm,
        }
    }
}

/// Jendela data historis untuk satu petak, dipakai M4.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReadingWindow {
    pub plot_id: PlotId,
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub readings: Vec<SoilReading>,
}

impl ReadingWindow {
    /// Deret waktu satu parameter, hanya dari pembacaan yang `is_usable`.
    pub fn series(&self, p: Parameter) -> Vec<(DateTime<Utc>, f32)> {
        self.readings
            .iter()
            .filter(|r| r.is_usable())
            .map(|r| (r.ts, r.value(p)))
            .collect()
    }
}
