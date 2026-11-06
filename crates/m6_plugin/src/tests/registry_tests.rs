use std::sync::Arc;

use soil_contracts::ContractError;

use crate::{PluginRegistry, PluginState};

use super::common::Dummy;

#[test]
fn install_registers_analyzer() {
    let mut reg = PluginRegistry::new();

    reg.install(Arc::new(Dummy {
        id: "t.a",
        major: 0,
    }))
    .unwrap();

    assert_eq!(reg.analyzers().len(), 1);
    assert_eq!(reg.list().len(), 1);
}

#[test]
fn incompatible_plugin_is_rejected() {
    let mut reg = PluginRegistry::new();

    let err = reg
        .install(Arc::new(Dummy {
            id: "t.a",
            major: 99,
        }))
        .unwrap_err();

    assert!(matches!(err, ContractError::Plugin(_)));
    assert!(reg.analyzers().is_empty());
}

#[test]
fn duplicate_plugin_is_rejected() {
    let mut reg = PluginRegistry::new();

    reg.install(Arc::new(Dummy {
        id: "t.a",
        major: 0,
    }))
    .unwrap();

    assert!(reg
        .install(Arc::new(Dummy {
            id: "t.a",
            major: 0,
        }))
        .is_err());
}

#[test]
fn disabled_plugin_is_hidden() {
    let mut reg = PluginRegistry::new();

    reg.install(Arc::new(Dummy {
        id: "t.a",
        major: 0,
    }))
    .unwrap();

    reg.set_enabled("t.a", false).unwrap();
    assert!(reg.analyzers().is_empty());

    reg.set_enabled("t.a", true).unwrap();
    assert_eq!(reg.analyzers().len(), 1);

    assert!(reg.set_enabled("tidak.ada", true).is_err());
}

#[test]
fn installed_plugin_has_installed_state() {
    let mut reg = PluginRegistry::new();

    reg.install(Arc::new(Dummy {
        id: "t.a",
        major: 0,
    }))
    .unwrap();

    assert_eq!(reg.get_state("t.a").unwrap(), PluginState::Installed);
}
