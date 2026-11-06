use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use soil_contracts::{
    Analyzer, ContractError, Plugin, PluginHost, PluginMeta, Result, RuleEvaluator,
};

use crate::state::PluginState;

/// Meyimpan informasi plugin teregistrasi.
#[derive(Default)]
pub struct PluginRegistry {
    /// Metadata plugin, dengan index id plugin.
    metas: HashMap<String, PluginMeta>,
    /// Menyimpan plugin yang sudah terkompilasi.
    pub(crate) plugins: HashMap<String, Arc<dyn Plugin>>,
    /// Menyimpan kapabilitas analiss.
    analyzers: Vec<Arc<dyn Analyzer>>,
    /// Menyimpan kapabilitas evaluator.
    evaluators: Vec<Arc<dyn RuleEvaluator>>,
    /// List plugin yang dimatikan.
    disabled: HashSet<String>,
    /// Status dari semua plugin.
    pub(crate) states: HashMap<String, PluginState>,
}

impl PluginRegistry {
    /// Pure Function: Mengembalikan register plugin.
    pub fn new() -> Self {
        Self::default()
    }

    /// Memasang plugin, periksa kompatibilitas dan duplikasi, lalu biarkan plugin mendaftar.
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

    /// Pure Function: Daftar plugin terpasang, urut berdasarkan id.
    pub fn list(&self) -> Vec<(&PluginMeta, bool)> {
        let mut items: Vec<_> = self
            .metas
            .values()
            .map(|m| (m, !self.disabled.contains(&m.id)))
            .collect();
        items.sort_by(|a, b| a.0.id.cmp(&b.0.id));
        items
    }

    /// Menambah atau menghapus plugin pada atribut `disabled`
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

    /// Pure Function: Kembalikan Analyzer yang sedang aktif.
    pub fn analyzers(&self) -> Vec<Arc<dyn Analyzer>> {
        self.analyzers
            .iter()
            .filter(|a| !self.disabled.contains(&a.meta().id))
            .cloned()
            .collect()
    }

    /// Pure Function: kembalikan Evaluator aturan yang sedang aktif.
    pub fn evaluators(&self) -> Vec<Arc<dyn RuleEvaluator>> {
        self.evaluators
            .iter()
            .filter(|e| !self.disabled.contains(&e.meta().id))
            .cloned()
            .collect()
    }
}

impl PluginHost for PluginRegistry {
    /// Tambahkan analyzer ke register
    fn register_analyzer(&mut self, analyzer: Arc<dyn Analyzer>) {
        self.analyzers.push(analyzer);
    }

    /// Tambahkan evalutaor ke register
    fn register_rule_evaluator(&mut self, evaluator: Arc<dyn RuleEvaluator>) {
        self.evaluators.push(evaluator);
    }
}
