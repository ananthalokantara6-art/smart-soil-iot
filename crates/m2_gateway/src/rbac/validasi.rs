//! Modul validasi hak akses (RBAC) & Otorisasi Kepemilikan Perangkat (100% Pure Functions).

use super::role::Role;
use crate::user::Kepemilikan;
use serde::{Deserialize, Serialize};

/// Aksi yang bisa dilakukan terhadap perangkat IoT.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Aksi {
    /// Melihat data sensor & status perangkat.
    Baca,
    /// Mengontrol perangkat (misal: mengubah interval sampling).
    Kontrol,
    /// Menghapus perangkat dari registry.
    Hapus,
}

impl Aksi {
    /// Fungsi murni: Mengubah string menjadi `Aksi`.
    pub fn from_str_aksi(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "baca" | "read" => Some(Self::Baca),
            "kontrol" | "control" => Some(Self::Kontrol),
            "hapus" | "delete" => Some(Self::Hapus),
            _ => None,
        }
    }
}

/// Fungsi murni: Memvalidasi apakah `Role` boleh melakukan `Aksi` secara umum (Tabel RBAC).
///
/// - `Admin`: Baca (Ya), Kontrol (Ya), Hapus (Ya)
/// - `Operator`: Baca (Ya), Kontrol (Ya), Hapus (Tidak)
/// - `Viewer`: Baca (Ya), Kontrol (Tidak), Hapus (Tidak)
pub fn validasi_akses(role: &Role, aksi: &Aksi) -> bool {
    match (role, aksi) {
        (Role::Admin, _) => true,
        (Role::Operator, Aksi::Baca) => true,
        (Role::Operator, Aksi::Kontrol) => true,
        (Role::Viewer, Aksi::Baca) => true,
        _ => false,
    }
}

/// Fungsi murni: Memvalidasi `X-API-Key` perangkat/klien secara konstan dan aman.
pub fn verify_api_key(provided: Option<&str>, expected: &str) -> Result<(), String> {
    match provided {
        Some(key) if key == expected => Ok(()),
        Some(_) => Err("API Key tidak valid (Unauthorized)".to_string()),
        None => Err("Header X-API-Key tidak ditemukan".to_string()),
    }
}

/// Fungsi murni Komposisi: Menggabungkan validasi RBAC ([`validasi_akses`]) dengan
/// pemeriksaan kepemilikan perangkat ([`Kepemilikan`]).
///
/// Aturan:
/// 1. Cek dulu apakah `Role` diizinkan melakukan `Aksi`. Jika tidak -> `Err`.
/// 2. Jika aksi adalah `Kontrol` oleh `Operator`, pastikan perangkat tersebut benar milik `username`.
///    (`Admin` boleh mengontrol semua perangkat).
pub fn otorisasi_aksi_perangkat(
    username: &str,
    role: &Role,
    aksi: &Aksi,
    device_id: &str,
    kepemilikan: &Kepemilikan,
) -> Result<String, String> {
    if !validasi_akses(role, aksi) {
        return Err(format!(
            "Akses ditolak (RBAC): Role {:?} tidak diizinkan melakukan aksi {:?}",
            role, aksi
        ));
    }

    if *role == Role::Operator && *aksi == Aksi::Kontrol {
        match kepemilikan.pemilik(device_id) {
            Some(owner) if owner == username => Ok(format!(
                "Diizinkan: Operator '{}' adalah pemilik sah perangkat '{}'",
                username, device_id
            )),
            Some(other_owner) => Err(format!(
                "Akses ditolak (Ownership): Perangkat '{}' adalah milik '{}', bukan '{}'",
                device_id, other_owner, username
            )),
            None => Err(format!(
                "Perangkat '{}' belum memiliki pemetaan pemilik",
                device_id
            )),
        }
    } else {
        Ok(format!(
            "Diizinkan: '{}' ({:?}) berhak melakukan {:?} pada '{}'",
            username, role, aksi, device_id
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::PlotMeta;

    #[test]
    fn test_rbac_matrix() {
        assert!(validasi_akses(&Role::Admin, &Aksi::Hapus));
        assert!(validasi_akses(&Role::Operator, &Aksi::Kontrol));
        assert!(!validasi_akses(&Role::Operator, &Aksi::Hapus));
        assert!(validasi_akses(&Role::Viewer, &Aksi::Baca));
        assert!(!validasi_akses(&Role::Viewer, &Aksi::Kontrol));
    }

    #[test]
    fn test_device_ownership_authorization() {
        let plot = PlotMeta {
            plot_id: "PLOT-01".to_string(),
            name: "Petak A".to_string(),
            latitude: -2.2,
            longitude: 113.9,
            area_ha: 1.0,
            plant: "nanas".to_string(),
        };
        let kep = Kepemilikan::new().tetapkan_lengkap_murni("ESP32-01", "petani_budi", plot);

        // Budi (Operator & pemilik) boleh Kontrol ESP32-01
        assert!(otorisasi_aksi_perangkat(
            "petani_budi",
            &Role::Operator,
            &Aksi::Kontrol,
            "ESP32-01",
            &kep
        )
        .is_ok());

        // Andi (Operator tapi BUKAN pemilik ESP32-01) ditolak saat Kontrol ESP32-01
        assert!(otorisasi_aksi_perangkat(
            "petani_andi",
            &Role::Operator,
            &Aksi::Kontrol,
            "ESP32-01",
            &kep
        )
        .is_err());

        // Admin boleh Hapus walaupun pemiliknya petani_budi
        assert!(otorisasi_aksi_perangkat(
            "admin_utama",
            &Role::Admin,
            &Aksi::Hapus,
            "ESP32-01",
            &kep
        )
        .is_ok());
    }
}
