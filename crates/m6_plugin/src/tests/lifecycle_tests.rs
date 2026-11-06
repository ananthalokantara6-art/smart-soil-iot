use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::{PluginRegistry, PluginState};

use super::common::{Dummy, LifecycleDummy};

#[test]
fn initialize_plugin_changes_state_to_running() {
    let mut reg = PluginRegistry::new();

    reg.install(Arc::new(Dummy {
        id: "t.a",
        major: 0,
    }))
    .unwrap();

    reg.initialize("t.a").unwrap();

    assert_eq!(reg.get_state("t.a").unwrap(), PluginState::Running);
}

#[test]
fn initialize_twice_fails() {
    let mut reg = PluginRegistry::new();

    reg.install(Arc::new(Dummy {
        id: "t.a",
        major: 0,
    }))
    .unwrap();

    reg.initialize("t.a").unwrap();

    assert!(reg.initialize("t.a").is_err());
}

#[test]
fn shutdown_plugin_changes_state_to_stopped() {
    let mut reg = PluginRegistry::new();

    reg.install(Arc::new(Dummy {
        id: "t.a",
        major: 0,
    }))
    .unwrap();

    reg.initialize("t.a").unwrap();
    reg.shutdown("t.a").unwrap();

    assert_eq!(reg.get_state("t.a").unwrap(), PluginState::Stopped);
}

#[test]
fn shutdown_before_initialize_fails() {
    let mut reg = PluginRegistry::new();

    reg.install(Arc::new(Dummy {
        id: "t.a",
        major: 0,
    }))
    .unwrap();

    assert!(reg.shutdown("t.a").is_err());

    assert_eq!(reg.get_state("t.a").unwrap(), PluginState::Installed);
}

#[test]
fn shutdown_twice_fails() {
    let mut reg = PluginRegistry::new();

    reg.install(Arc::new(Dummy {
        id: "t.a",
        major: 0,
    }))
    .unwrap();

    reg.initialize("t.a").unwrap();
    reg.shutdown("t.a").unwrap();

    assert!(reg.shutdown("t.a").is_err());
}

#[test]
fn lifecycle_callbacks_are_called() {
    let mut reg = PluginRegistry::new();

    let plugin = Arc::new(LifecycleDummy {
        initialized: AtomicBool::new(false),
        shutdown_called: AtomicBool::new(false),
    });

    reg.install(plugin.clone()).unwrap();

    reg.initialize("t.lifecycle").unwrap();

    assert!(plugin.initialized.load(std::sync::atomic::Ordering::SeqCst));

    reg.shutdown("t.lifecycle").unwrap();

    assert!(plugin
        .shutdown_called
        .load(std::sync::atomic::Ordering::SeqCst));
}
