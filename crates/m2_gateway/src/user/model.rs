//! Modul model pengguna & pemetaan kepemilikan perangkat (`Device -> User & PlotMeta`).

use crate::contracts::PlotMeta;
use crate::rbac::Role;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Representasi pengguna dalam sistem Smart Soil IoT.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    /// Username unik pengguna.
    pub username: String,
    /// Role pengguna (RBAC: `Admin`, `Operator`, `Viewer`).
    pub role: Role,
    /// Daftar ID perangkat yang dimiliki user ini.
    pub perangkat: Vec<String>,
}

impl User {
    /// Fungsi murni: Membuat `User` baru.
    pub fn new(username: &str, role: Role) -> Self {
        Self {
            username: username.to_string(),
            role,
            perangkat: Vec::new(),
        }
    }

    /// Fungsi murni & **Immutable**: Menghasilkan `User` baru dengan perangkat tambahan
    /// tanpa mengubah struct `User` asli.
    pub fn tambah_perangkat_murni(&self, device_id: &str) -> Self {
        let sudah_ada = self.perangkat.iter().any(|id| id == device_id);
        let perangkat_baru = if sudah_ada {
            self.perangkat.clone()
        } else {
            self.perangkat
                .iter()
                .cloned()
                .chain(std::iter::once(device_id.to_string()))
                .collect()
        };
        Self {
            username: self.username.clone(),
            role: self.role.clone(),
            perangkat: perangkat_baru,
        }
    }

    /// Kompatibilitas method: Menambahkan perangkat ke kepemilikan user.
    pub fn tambah_perangkat(&mut self, device_id: &str) {
        *self = self.tambah_perangkat_murni(device_id);
    }

    /// Fungsi murni: Cek apakah user memiliki perangkat tertentu.
    pub fn memiliki(&self, device_id: &str) -> bool {
        self.perangkat.iter().any(|id| id == device_id)
    }
}

/// Pemetaan kepemilikan perangkat (`device_id -> username`) dan petak lahan (`device_id -> PlotMeta`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Kepemilikan {
    mapping: HashMap<String, String>,
    plots: HashMap<String, PlotMeta>,
}

impl Kepemilikan {
    /// Fungsi murni: Membuat `Kepemilikan` kosong.
    pub fn new() -> Self {
        Self {
            mapping: HashMap::new(),
            plots: HashMap::new(),
        }
    }

    /// Fungsi murni & **Immutable**: Menetapkan pemilik dan metadata petak lahan (`PlotMeta`)
    /// untuk sebuah perangkat, menghasilkan instance `Kepemilikan` baru.
    pub fn tetapkan_lengkap_murni(
        &self,
        device_id: &str,
        username: &str,
        plot: PlotMeta,
    ) -> Self {
        let next_mapping = self
            .mapping
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .chain(std::iter::once((
                device_id.to_string(),
                username.to_string(),
            )))
            .collect();

        let next_plots = self
            .plots
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .chain(std::iter::once((device_id.to_string(), plot)))
            .collect();

        Self {
            mapping: next_mapping,
            plots: next_plots,
        }
    }

    /// Kompatibilitas method: Menetapkan pemilik sebuah perangkat.
    pub fn tetapkan(&mut self, device_id: &str, username: &str) {
        self.mapping
            .insert(device_id.to_string(), username.to_string());
    }

    /// Fungsi murni: Mendapatkan username pemilik perangkat.
    pub fn pemilik(&self, device_id: &str) -> Option<&String> {
        self.mapping.get(device_id)
    }

    /// Fungsi murni: Mendapatkan metadata petak (`PlotMeta`) dari perangkat untuk dikirim ke Modul 3.
    pub fn plot_of(&self, device_id: &str) -> Option<&PlotMeta> {
        self.plots.get(device_id)
    }

    /// Fungsi murni (*Higher-Order / Filter*): Mencari semua `device_id` yang dimiliki oleh `username` tertentu.
    pub fn perangkat_milik(&self, username: &str) -> Vec<String> {
        self.mapping
            .iter()
            .filter(|(_, owner)| owner.as_str() == username)
            .map(|(dev_id, _)| dev_id.clone())
            .collect()
    }
}

impl Default for Kepemilikan {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_and_ownership_immutability() {
        let u0 = User::new("petani_budi", Role::Operator);
        let u1 = u0.tambah_perangkat_murni("ESP32-01");
        assert!(!u0.memiliki("ESP32-01"));
        assert!(u1.memiliki("ESP32-01"));

        let plot = PlotMeta {
            plot_id: "PLOT-01".to_string(),
            name: "Lahan Gambut A".to_string(),
            latitude: -2.21,
            longitude: 113.92,
            area_ha: 2.0,
            plant: "nanas".to_string(),
        };
        let k0 = Kepemilikan::new();
        let k1 = k0.tetapkan_lengkap_murni("ESP32-01", "petani_budi", plot);
        assert!(k0.pemilik("ESP32-01").is_none());
        assert_eq!(k1.pemilik("ESP32-01").unwrap(), "petani_budi");
        assert_eq!(k1.plot_of("ESP32-01").unwrap().plant, "nanas");
    }
}
