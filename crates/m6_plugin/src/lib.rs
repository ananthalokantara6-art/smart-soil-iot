//! M6 - Plugin Framework and Extensibility.
//!
//! Implementasi awal: registry di memori dengan pemeriksaan kompatibilitas dan enable/disable.
//! TODO kelompok 6: lifecycle (init/shutdown), discovery dari direktori/manifest, versioning
//! dan rollback, isolasi kegagalan plugin.
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

mod lifecycle;
mod registry;
mod state;

pub use registry::PluginRegistry;
pub use state::PluginState;

#[cfg(test)]
mod tests;
