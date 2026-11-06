use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use soil_contracts::{
    Analyzer, ContractError, Plugin, PluginHost, PluginMeta, Result, RuleEvaluator,
};

use crate::state::PluginState;

#[derive(Default)]
pub struct PluginRegistry {
    metas: HashMap<String, PluginMeta>,
    pub(crate) plugins: HashMap<String, Arc<dyn Plugin>>,
    analyzers: Vec<Arc<dyn Analyzer>>,
    evaluators: Vec<Arc<dyn RuleEvaluator>>,
    disabled: HashSet<String>,
    pub(crate) states: HashMap<String, PluginState>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Memasang plugin: periksa kompatibilitas dan duplikasi, lalu biarkan plugin mendaftar.
    pub fn install(&mut self, plugin: Arc<dyn Plugin>) -> Result<()> {
        let meta = plugin.meta();

        if !meta.is_compatible() {
            return Err(ContractError::Plugin(format!(
                "plugin {} butuh kontrak MAJOR {}, tidak kompatibel",
                meta.id, meta.requires_contracts_major
            )));
        }

        if self.metas.contains_key(&meta.id) {
            return Err(ContractError::Plugin(format!(
                "plugin {} sudah terpasang",
                meta.id
            )));
        }

        plugin.register(self)?;

        self.states.insert(meta.id.clone(), PluginState::Installed);

        self.plugins.insert(meta.id.clone(), plugin);

        self.metas.insert(meta.id.clone(), meta);

        Ok(())
    }

    ///

    /// Daftar plugin terpasang, urut berdasarkan id.
    pub fn list(&self) -> Vec<(&PluginMeta, bool)> {
        let mut items: Vec<_> = self
            .metas
            .values()
            .map(|m| (m, !self.disabled.contains(&m.id)))
            .collect();
        items.sort_by(|a, b| a.0.id.cmp(&b.0.id));
        items
    }

    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> Result<()> {
        if !self.metas.contains_key(id) {
            return Err(ContractError::NotFound(format!("plugin {id}")));
        }
        if enabled {
            self.disabled.remove(id);
        } else {
            self.disabled.insert(id.to_string());
        }
        Ok(())
    }

    /// Analyzer yang sedang aktif.
    pub fn analyzers(&self) -> Vec<Arc<dyn Analyzer>> {
        self.analyzers
            .iter()
            .filter(|a| !self.disabled.contains(&a.meta().id))
            .cloned()
            .collect()
    }

    /// Evaluator aturan yang sedang aktif.
    pub fn evaluators(&self) -> Vec<Arc<dyn RuleEvaluator>> {
        self.evaluators
            .iter()
            .filter(|e| !self.disabled.contains(&e.meta().id))
            .cloned()
            .collect()
    }
}

impl PluginHost for PluginRegistry {
    fn register_analyzer(&mut self, analyzer: Arc<dyn Analyzer>) {
        self.analyzers.push(analyzer);
    }

    fn register_rule_evaluator(&mut self, evaluator: Arc<dyn RuleEvaluator>) {
        self.evaluators.push(evaluator);
    }
}
