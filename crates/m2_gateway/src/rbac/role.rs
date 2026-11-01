use serde::{Deserialize, Serialize};

/// Peran pengguna dalam Role-Based Access Control (RBAC).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    /// Admin — akses penuh (Baca, Kontrol semua perangkat, Hapus/Registrasi perangkat).
    Admin,
    /// Operator (Petani/Teknisi Lapangan) — bisa Baca dan Kontrol perangkat miliknya, tidak bisa Hapus.
    Operator,
    /// Viewer (Peneliti/Tamu) — hanya bisa Baca data perangkat.
    Viewer,
}

impl Role {
    /// Fungsi murni: Mengubah string menjadi `Role` (case-insensitive).
    pub fn from_str_role(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "admin" => Some(Self::Admin),
            "operator" | "petani" => Some(Self::Operator),
            "viewer" | "peneliti" => Some(Self::Viewer),
            _ => None,
        }
    }
}
