use serde::{Deserialize, Serialize};
use soil_contracts::{ContractError, PlotId, Result};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlotMeta {
    pub plot_id: PlotId,
    pub name: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub area_ha: Option<f64>,
    pub plant: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct NewPlot {
    pub name: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub area_ha: Option<f64>,
    pub plant: Option<String>,
}

impl PlotMeta {
    pub fn from_new(new: NewPlot) -> Result<Self> {
        Self::from_new_with_id(PlotId::new(), new)
    }

    pub fn from_new_with_id(plot_id: PlotId, new: NewPlot) -> Result<Self> {
        let name = new.name.trim().to_string();
        if name.is_empty() {
            return Err(ContractError::Validation("nama petak kosong".into()));
        }
        if new.latitude.is_some_and(|v| !(-90.0..=90.0).contains(&v)) {
            return Err(ContractError::Validation("latitude di luar -90..90".into()));
        }
        if new
            .longitude
            .is_some_and(|v| !(-180.0..=180.0).contains(&v))
        {
            return Err(ContractError::Validation(
                "longitude di luar -180..180".into(),
            ));
        }
        if new.area_ha.is_some_and(|v| !v.is_finite() || v <= 0.0) {
            return Err(ContractError::Validation("luas harus lebih dari 0".into()));
        }
        Ok(Self {
            plot_id,
            name,
            latitude: new.latitude,
            longitude: new.longitude,
            area_ha: new.area_ha,
            plant: new.plant,
        })
    }
}

#[cfg(test)]
#[path = "tests/plots.rs"]
mod tests;
