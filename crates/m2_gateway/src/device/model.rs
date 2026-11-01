//! Modul model untuk perangkat IoT Edge (ESP32 + SIM7000E).

use serde::{Deserialize, Serialize};

/// Representasi sebuah perangkat sensor tanah di lapangan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Device {
    /// ID unik perangkat (misal: `"ESP32-GAMBUT-01"`).
    pub id: String,
    /// Nama perangkat di lapangan.
    pub nama: String,
    /// Tipe hardware sesuai RAB Modul 1 (misal: `"ESP32-DOIT + SIM7000E + ADS1115"`).
    pub tipe: String,
    /// Interval pengiriman data dalam detik (Sampling Interval Control).
    pub sampling_interval_sec: u64,
}

impl Device {
    /// Fungsi murni: Membuat `Device` baru dengan interval sampling default 60 detik.
    pub fn new(id: &str, nama: &str, tipe: &str) -> Self {
        Self {
            id: id.to_string(),
            nama: nama.to_string(),
            tipe: tipe.to_string(),
            sampling_interval_sec: 60,
        }
    }

    /// Fungsi murni & **Immutable**: Menghasilkan instance `Device` baru dengan interval sampling baru
    /// tanpa mengubah (`mut`) struct asli.
    pub fn with_sampling_interval(&self, interval_sec: u64) -> Result<Self, String> {
        if !(5..=86_400).contains(&interval_sec) {
            return Err("Interval sampling harus berada di antara 5 hingga 86400 detik".to_string());
        }
        Ok(Self {
            sampling_interval_sec: interval_sec,
            ..self.clone()
        })
    }
}
