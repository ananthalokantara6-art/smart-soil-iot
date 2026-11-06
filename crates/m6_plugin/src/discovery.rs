use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use soil_contracts::{ContractError, PluginMeta, Result};

use crate::manifest::read_manifest;

/// Metadata for a discovered plugin.
///
/// This does not contain an executable plugin instance.
#[derive(Debug, Clone)]
pub struct DiscoveredPlugin {
    pub meta: PluginMeta,
    pub manifest_path: PathBuf,
}

/// Discovers plugins from immediate subdirectories.
///
/// Expected structure:
///
/// plugins/
///   plugin_a/plugin.toml
///   plugin_b/plugin.toml
pub fn discover_plugins(directory: impl AsRef<Path>) -> Result<Vec<DiscoveredPlugin>> {
    let directory = directory.as_ref();

    let entries = fs::read_dir(directory).map_err(|error| {
        ContractError::Plugin(format!(
            "failed to read plugin directory {}: {error}",
            directory.display()
        ))
    })?;

    let mut discovered = Vec::new();
    let mut plugin_ids = HashSet::new();

    for entry in entries {
        let entry = entry.map_err(|error| {
            ContractError::Plugin(format!("failed to read directory entry: {error}"))
        })?;

        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let manifest_path = path.join("plugin.toml");

        // Directories without a manifest are not considered plugins.
        if !manifest_path.is_file() {
            continue;
        }

        let meta = read_manifest(&manifest_path)?;

        if !meta.is_compatible() {
            return Err(ContractError::Plugin(format!(
                "plugin {} requires contracts MAJOR {}, incompatible",
                meta.id, meta.requires_contracts_major
            )));
        }

        if !plugin_ids.insert(meta.id.clone()) {
            return Err(ContractError::Plugin(format!(
                "duplicate discovered plugin ID: {}",
                meta.id
            )));
        }

        discovered.push(DiscoveredPlugin {
            meta,
            manifest_path,
        });
    }

    discovered.sort_by(|a, b| a.meta.id.cmp(&b.meta.id));

    Ok(discovered)
}
