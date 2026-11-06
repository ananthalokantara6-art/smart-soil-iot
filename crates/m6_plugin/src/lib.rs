//! M6 - Plugin Framework and Extensibility.
//!
//! Implementasi awal: registry di memori dengan pemeriksaan kompatibilitas dan enable/disable.
//! TODO kelompok 6: lifecycle (init/shutdown), discovery dari direktori/manifest, versioning
//! dan rollback, isolasi kegagalan plugin.
mod discovery;
mod lifecycle;
mod manifest;
mod registry;
mod state;

pub use discovery::{discover_plugins, DiscoveredPlugin};
pub use registry::PluginRegistry;
pub use state::PluginState;

#[cfg(test)]
mod tests;
