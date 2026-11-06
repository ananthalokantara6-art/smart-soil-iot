use crate::registry::PluginRegistry;
use crate::state::PluginState;
use soil_contracts::{ContractError, Result};

impl PluginRegistry {
    // initialize()
    // shutdown()
    // get_state()
    pub fn initialize(&mut self, id: &str) -> Result<()> {
        let plugin = self
            .plugins
            .get(id)
            .cloned()
            .ok_or_else(|| ContractError::NotFound(format!("plugin {id}")))?;

        let state = self.get_state(id)?;

        if state != PluginState::Installed && state != PluginState::Stopped {
            return Err(ContractError::Plugin(format!(
                "plugin {id} cannot be initialized from state {state:?}"
            )));
        }

        match plugin.init() {
            Ok(()) => {
                self.states.insert(id.to_string(), PluginState::Running);
                Ok(())
            }
            Err(err) => {
                self.states.insert(id.to_string(), PluginState::Failed);
                Err(err)
            }
        }
    }

    ///
    pub fn shutdown(&mut self, id: &str) -> Result<()> {
        let plugin = self
            .plugins
            .get(id)
            .cloned()
            .ok_or_else(|| ContractError::NotFound(format!("plugin {id}")))?;

        let state = self.get_state(id)?;

        if state != PluginState::Running {
            return Err(ContractError::Plugin(format!(
                "plugin {id} cannot be shut down from state {state:?}"
            )));
        }

        match plugin.shutdown() {
            Ok(()) => {
                self.states.insert(id.to_string(), PluginState::Stopped);
                Ok(())
            }
            Err(err) => {
                self.states.insert(id.to_string(), PluginState::Failed);
                Err(err)
            }
        }
    }

    /// Dapatkan state dari plugin
    pub fn get_state(&self, id: &str) -> Result<PluginState> {
        self.states
            .get(id)
            .copied()
            .ok_or_else(|| ContractError::NotFound(format!("plugin {id}")))
    }
}
