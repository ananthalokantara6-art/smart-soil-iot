//! Modul registrasi dan manajemen perangkat (Pure Functional & Immutable).

use super::model::Device;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Registry penyimpan perangkat terdaftar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registry {
    devices: HashMap<String, Device>,
}

impl Registry {
    /// Fungsi murni: Membuat `Registry` kosong.
    pub fn new() -> Self {
        Self {
            devices: HashMap::new(),
        }
    }

    /// Fungsi murni & **Immutable**: Mendaftarkan perangkat baru dan mengembalikan `Registry` baru
    /// tanpa memutasi `self` (prinsip Immutability pada Pemrograman Fungsional).
    pub fn registrasi_murni(&self, device: Device) -> Result<Self, String> {
        if device.id.trim().is_empty() {
            return Err("ID perangkat tidak boleh kosong".to_string());
        }
        if self.devices.contains_key(&device.id) {
            return Err(format!("Perangkat dengan ID '{}' sudah terdaftar", device.id));
        }
        let next_devices = self
            .devices
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .chain(std::iter::once((device.id.clone(), device)))
            .collect();

        Ok(Self {
            devices: next_devices,
        })
    }

    /// Fungsi murni & **Immutable**: Memperbarui interval sampling sebuah perangkat
    /// dan mengembalikan `Registry` baru.
    pub fn ubah_interval_murni(&self, id: &str, interval_sec: u64) -> Result<Self, String> {
        let existing = self
            .cari(id)
            .ok_or_else(|| format!("Perangkat '{}' tidak ditemukan", id))?;
        let updated_device = existing.with_sampling_interval(interval_sec)?;

        let next_devices = self
            .devices
            .iter()
            .map(|(k, v)| {
                if k == id {
                    (k.clone(), updated_device.clone())
                } else {
                    (k.clone(), v.clone())
                }
            })
            .collect();

        Ok(Self {
            devices: next_devices,
        })
    }

    /// Fungsi murni & **Immutable**: Menghapus perangkat berdasarkan ID dan menghasilkan `Registry` baru.
    pub fn hapus_murni(&self, id: &str) -> Result<Self, String> {
        if !self.devices.contains_key(id) {
            return Err(format!("Perangkat '{}' tidak ditemukan", id));
        }
        let next_devices = self
            .devices
            .iter()
            .filter(|(k, _)| k.as_str() != id)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        Ok(Self {
            devices: next_devices,
        })
    }

    /// Kompatibilitas method: Mendaftarkan perangkat baru.
    pub fn registrasi(&mut self, device: Device) -> Result<(), String> {
        let updated = self.registrasi_murni(device)?;
        *self = updated;
        Ok(())
    }

    /// Fungsi murni: Mencari perangkat berdasarkan ID.
    pub fn cari(&self, id: &str) -> Option<&Device> {
        self.devices.get(id)
    }

    /// Kompatibilitas method: Menghapus perangkat.
    pub fn hapus(&mut self, id: &str) -> bool {
        match self.hapus_murni(id) {
            Ok(updated) => {
                *self = updated;
                true
            }
            Err(_) => false,
        }
    }

    /// Fungsi murni: Jumlah perangkat terdaftar.
    pub fn jumlah(&self) -> usize {
        self.devices.len()
    }

    /// Fungsi murni: Mengembalikan daftar seluruh perangkat terdaftar (terurut berdasarkan ID).
    pub fn daftar_semua(&self) -> Vec<Device> {
        let mut list: Vec<Device> = self.devices.values().cloned().collect();
        list.sort_by(|a, b| a.id.cmp(&b.id));
        list
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_immutable_registry_operations() {
        let reg0 = Registry::new();
        let dev = Device::new("ESP32-01", "Node Blok A", "ESP32-SIM7000E");

        let reg1 = reg0.registrasi_murni(dev).unwrap();
        assert_eq!(reg0.jumlah(), 0); // reg0 tetap 0 (immutable!)
        assert_eq!(reg1.jumlah(), 1);

        let reg2 = reg1.ubah_interval_murni("ESP32-01", 120).unwrap();
        assert_eq!(reg1.cari("ESP32-01").unwrap().sampling_interval_sec, 60);
        assert_eq!(reg2.cari("ESP32-01").unwrap().sampling_interval_sec, 120);

        let reg3 = reg2.hapus_murni("ESP32-01").unwrap();
        assert_eq!(reg3.jumlah(), 0);
    }
}
