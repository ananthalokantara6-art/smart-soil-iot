//! M6 - Plugin Framework and Extensibility.
//!
//! Implementasi awal: registry di memori dengan pemeriksaan kompatibilitas dan enable/disable.
//! TODO kelompok 6: lifecycle (init/shutdown), discovery dari direktori/manifest, versioning
//! dan rollback, isolasi kegagalan plugin.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use soil_contracts::{
    Analyzer, ContractError, Plugin, PluginHost, PluginMeta, Result, RuleEvaluator,
};

#[derive(Default)]
pub struct PluginRegistry {
    metas: HashMap<String, PluginMeta>,
    analyzers: Vec<Arc<dyn Analyzer>>,
    evaluators: Vec<Arc<dyn RuleEvaluator>>,
    disabled: HashSet<String>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Memasang plugin: periksa kompatibilitas dan duplikasi, lalu biarkan plugin mendaftar.
    pub fn install(&mut self, plugin: &dyn Plugin) -> Result<()> {
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
        self.metas.insert(meta.id.clone(), meta);
        Ok(())
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use soil_contracts::{AnalysisResult, PluginKind, ReadingWindow};

    #[derive(Clone)]
    struct Dummy {
        id: &'static str,
        major: u32,
    }

    #[async_trait]
    impl Analyzer for Dummy {
        fn meta(&self) -> PluginMeta {
            PluginMeta {
                id: self.id.into(),
                name: "dummy".into(),
                version: "0.0.1".into(),
                kind: PluginKind::Analytics,
                description: String::new(),
                requires_contracts_major: self.major,
            }
        }

        async fn analyze(&self, _w: &ReadingWindow) -> Result<AnalysisResult> {
            Err(ContractError::Internal("tidak dipakai di tes".into()))
        }
    }

    impl Plugin for Dummy {
        fn meta(&self) -> PluginMeta {
            Analyzer::meta(self)
        }

        fn register(&self, host: &mut dyn PluginHost) -> Result<()> {
            host.register_analyzer(Arc::new(self.clone()));
            Ok(())
        }
    }

    #[test]
    fn install_registers_analyzer() {
        let mut reg = PluginRegistry::new();
        reg.install(&Dummy {
            id: "t.a",
            major: 0,
        })
        .unwrap();
        assert_eq!(reg.analyzers().len(), 1);
        assert_eq!(reg.list().len(), 1);
    }

    #[test]
    fn incompatible_plugin_is_rejected() {
        let mut reg = PluginRegistry::new();
        let err = reg
            .install(&Dummy {
                id: "t.a",
                major: 99,
            })
            .unwrap_err();
        assert!(matches!(err, ContractError::Plugin(_)));
        assert!(reg.analyzers().is_empty());
    }

    #[test]
    fn duplicate_plugin_is_rejected() {
        let mut reg = PluginRegistry::new();
        reg.install(&Dummy {
            id: "t.a",
            major: 0,
        })
        .unwrap();
        assert!(reg
            .install(&Dummy {
                id: "t.a",
                major: 0
            })
            .is_err());
    }

    #[test]
    fn disabled_plugin_is_hidden() {
        let mut reg = PluginRegistry::new();
        reg.install(&Dummy {
            id: "t.a",
            major: 0,
        })
        .unwrap();
        reg.set_enabled("t.a", false).unwrap();
        assert!(reg.analyzers().is_empty());
        reg.set_enabled("t.a", true).unwrap();
        assert_eq!(reg.analyzers().len(), 1);
        assert!(reg.set_enabled("tidak.ada", true).is_err());
    }
}
