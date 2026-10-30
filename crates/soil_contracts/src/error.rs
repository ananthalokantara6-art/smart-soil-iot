use thiserror::Error;

/// Error standar lintas modul. Modul boleh punya error internal sendiri,
/// tetapi harus mengonversinya ke tipe ini di batas trait.
#[derive(Debug, Error)]
pub enum ContractError {
    #[error("validasi gagal: {0}")]
    Validation(String),
    #[error("tidak ditemukan: {0}")]
    NotFound(String),
    #[error("tidak diizinkan")]
    Unauthorized,
    #[error("kesalahan penyimpanan: {0}")]
    Storage(String),
    #[error("kesalahan plugin: {0}")]
    Plugin(String),
    #[error("kesalahan internal: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, ContractError>;
