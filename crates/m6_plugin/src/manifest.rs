use std::path::Path;

use soil_contracts::{ContractError, PluginMeta, Result};

pub fn read_manifest(path: &Path) -> Result<PluginMeta> {
    let content = std::fs::read_to_string(path).map_err(|error| {
        ContractError::Plugin(format!(
            "failed to read manifest {}: {error}",
            path.display()
        ))
    })?;

    toml::from_str::<PluginMeta>(&content).map_err(|error| {
        ContractError::Plugin(format!("invalid manifest {}: {error}", path.display()))
    })
}
