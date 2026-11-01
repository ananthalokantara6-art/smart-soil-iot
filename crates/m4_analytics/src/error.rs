use soil_contracts::ContractError;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum SoilError {
    #[error("dataset kosong: tidak ada pembacaan valid")]
    EmptyDataset,

    #[error("nilai fisik di luar batas wajar: {0}")]
    InvalidPhysicalRange(String),

    #[error("perhitungan gagal: {0}")]
    ComputationError(String),
}

impl From<SoilError> for ContractError {
    fn from(err: SoilError) -> Self {
        ContractError::Validation(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, SoilError>;