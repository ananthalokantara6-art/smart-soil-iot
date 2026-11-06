
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;

use soil_contracts::{
    AnalysisResult, Analyzer, Plugin, PluginHost, PluginKind, PluginMeta, ReadingWindow, Result,
};

#[derive(Clone)]
pub struct Dummy {
    pub id: &'static str,
    pub major: u32,
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
        Err(soil_contracts::ContractError::Internal(
            "tidak dipakai di tes".into(),
        ))
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

pub struct LifecycleDummy {
    pub initialized: AtomicBool,
    pub shutdown_called: AtomicBool,
}

impl Plugin for LifecycleDummy {
    fn meta(&self) -> PluginMeta {
        PluginMeta {
            id: "t.lifecycle".into(),
            name: "lifecycle dummy".into(),
            version: "0.0.1".into(),
            kind: PluginKind::Analytics,
            description: String::new(),
            requires_contracts_major: 0,
        }
    }

    fn register(&self, _host: &mut dyn PluginHost) -> Result<()> {
        Ok(())
    }

    fn init(&self) -> Result<()> {
        self.initialized.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        self.shutdown_called.store(true, Ordering::SeqCst);
        Ok(())
    }
}
