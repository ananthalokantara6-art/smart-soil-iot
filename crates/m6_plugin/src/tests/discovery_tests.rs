use std::fs;

use crate::discover_plugins;

#[test]
fn discovers_plugin_from_manifest() {
    let directory = tempfile::tempdir().unwrap();

    let plugin_dir = directory.path().join("m4_threshold");
    fs::create_dir_all(&plugin_dir).unwrap();

    let manifest = r#"
id = "m4.threshold_risk"
name = "Threshold risk analyzer"
version = "0.1.0"
kind = "analytics"
description = "Soil analytics"
requires_contracts_major = 0
"#;

    fs::write(plugin_dir.join("plugin.toml"), manifest).unwrap();

    let discovered = discover_plugins(directory.path()).unwrap();

    assert_eq!(discovered.len(), 1);
    assert_eq!(discovered[0].meta.id, "m4.threshold_risk");
}

#[test]
fn ignores_directories_without_manifest() {
    let directory = tempfile::tempdir().unwrap();

    fs::create_dir_all(directory.path().join("empty_plugin")).unwrap();

    let discovered = discover_plugins(directory.path()).unwrap();

    assert!(discovered.is_empty());
}

#[test]
fn rejects_invalid_manifest() {
    let directory = tempfile::tempdir().unwrap();

    let plugin_dir = directory.path().join("invalid_plugin");
    fs::create_dir_all(&plugin_dir).unwrap();

    fs::write(plugin_dir.join("plugin.toml"), "this is not valid toml = =").unwrap();

    assert!(discover_plugins(directory.path()).is_err());
}
