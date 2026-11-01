//! Kontrak Data Antar-Modul (`soil_contracts` & Integrasi Modul 1 -> Modul 2 -> Modul 3).
//!
//! File ini mendefinisikan struktur data imutabel yang menyambungkan:
//! - **Modul 1 (Sensor Hardware RAB)**: Mengirimkan [`RawReading`] dari ESP32 + SIM7000E.
//! - **Modul 2 (Gateway, Edge & RBAC)**: Memvalidasi, mendeteksi anomali, memetakan pemilik & petak,
//!   serta menangani offline buffering.
//! - **Modul 3 (Platform & Visualisasi)**: Menerima [`SoilReading`] melalui trait [`ReadingSink`]
//!   beserta metadata lahan [`PlotMeta`].

use serde::{Deserialize, Serialize};

/// Parameter tanah minimum wajib untuk pertanian lahan gambut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Parameter {
    Moisture,
    Temperature,
    Ph,
    Ec,
}

/// Status kualitas pembacaan sensor sesuai `soil_contracts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityFlag {
    /// Lolos validasi rentang fisik dan batas wajar lahan gambut.
    Good,
    /// Mendekati batas ekstrem lahan gambut; tetap disimpan namun perlu perhatian.
    Suspect,
    /// Di luar rentang fisik sensor / indikasi sensor rusak atau terputus.
    Bad,
    /// Sensor sedang dalam proses kalibrasi buffer (pH 4.00/6.86/9.18 atau EC 1413 uS/cm).
    Calibrating,
}

/// Data mentah dari node ESP32 (Modul 1) sebelum diproses di Edge/Gateway (Modul 2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawReading {
    /// ID unik event pengiriman untuk mencegah duplikasi saat retry (idempotency).
    pub event_id: String,
    /// ID perangkat ESP32 pengirim data.
    pub device_id: String,
    /// Waktu pembacaan sensor dalam format ISO 8601 UTC (contoh: `"2026-11-01T10:30:00Z"`).
    pub ts: String,
    /// Waktu pembacaan dalam detik (epoch) untuk pengecekan interval sampling.
    pub timestamp_sec: u64,
    /// Pembacaan kelembapan dari DFRobot Capacitive Soil Moisture Sensor + ADS1115 (%).
    pub moisture_raw: f32,
    /// Pembacaan suhu tanah dari DS18B20 Waterproof Probe (°C).
    pub temperature_raw: f32,
    /// Pembacaan pH dari Sensor pH Tanah Analog + DMS (skala 0-14).
    pub ph_raw: f32,
    /// Pembacaan konduktivitas listrik dari DFRobot Gravity Analog TDS/EC Sensor (mS/cm).
    pub ec_raw: f32,
}

/// Data sensor tanah tervalidasi yang dikirimkan dari Modul 2 ke Modul 3 (`ReadingSink`).
///
/// Seluruh nama atribut disesuaikan 100% dengan kontrak JSON Modul 3:
/// `device_id`, `plot_id`, `ts`, `moisture_pct`, `temperature_c`, `ph`, `ec_ms_cm`, `quality`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoilReading {
    pub device_id: String,
    pub plot_id: String,
    pub ts: String,
    pub moisture_pct: f32,
    pub temperature_c: f32,
    pub ph: f32,
    pub ec_ms_cm: f32,
    pub quality: QualityFlag,
}

impl SoilReading {
    /// Fungsi murni: mengecek apakah pembacaan layak dipakai oleh analitik (bukan `Bad` atau `Calibrating`).
    pub fn is_usable(&self) -> bool {
        matches!(self.quality, QualityFlag::Good | QualityFlag::Suspect)
    }

    /// Fungsi murni: mengambil nilai numerik berdasarkan jenis [`Parameter`].
    pub fn value(&self, param: Parameter) -> f32 {
        match param {
            Parameter::Moisture => self.moisture_pct,
            Parameter::Temperature => self.temperature_c,
            Parameter::Ph => self.ph,
            Parameter::Ec => self.ec_ms_cm,
        }
    }
}

/// Informasi metadata petak lahan gambut (`PlotMeta`) sesuai kebutuhan Modul 3.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlotMeta {
    pub plot_id: String,
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub area_ha: f32,
    pub plant: String,
}

/// Paket pengiriman lengkap dari Modul 2 ke Modul 3 (`ReadingSink`),
/// menggabungkan [`SoilReading`] dan [`PlotMeta`] serta status alarm fisik.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct M3IngestEnvelope {
    pub reading: SoilReading,
    pub plot_meta: PlotMeta,
    pub owner_username: String,
    pub anomalies: Vec<String>,
    /// Indikator Buzzer Active 5V (RAB M1 No. 9): menyala jika sensor terputus/rusak (`QualityFlag::Bad`).
    pub buzzer_alarm_active: bool,
}

/// Trait kontrak `ReadingSink` (diimplementasikan oleh Modul 3, dipanggil oleh Modul 2).
pub trait ReadingSink: Send + Sync {
    fn ingest(&self, reading: SoilReading, plot_meta: &PlotMeta) -> Result<(), String>;
}
