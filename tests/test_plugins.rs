//! Tests for plugin system

use xls_rs::plugins::{
    PluginFunction, PluginRegistry, PrefixPlugin, UppercasePlugin,
};

#[test]
fn test_plugin_registry_new() {
    let registry = PluginRegistry::new();
    assert_eq!(registry.list_plugins().len(), 0);
}

#[test]
fn test_plugin_registry_default() {
    let registry = PluginRegistry::default();
    let plugins = registry.list_plugins();
    assert!(plugins.len() >= 2);
}

#[test]
fn test_register_plugin() {
    let mut registry = PluginRegistry::new();
    registry.register(UppercasePlugin);

    let plugins = registry.list_plugins();
    assert_eq!(plugins.len(), 1);
    assert_eq!(plugins[0].name, "uppercase");
}

#[test]
fn test_uppercase_plugin_execution() {
    let plugin = UppercasePlugin;
    let data = vec![
        vec!["Name".to_string(), "Age".to_string()],
        vec!["alice".to_string(), "30".to_string()],
        vec!["bob".to_string(), "25".to_string()],
    ];

    let args = vec!["0".to_string()];
    let result = plugin.execute(&args, &data).unwrap();

    assert_eq!(result[0][0], "Name");
    assert_eq!(result[1][0], "ALICE");
    assert_eq!(result[2][0], "BOB");
    assert_eq!(result[1][1], "30");
}

#[test]
fn test_uppercase_plugin_missing_args() {
    let plugin = UppercasePlugin;
    let data = vec![vec!["test".to_string()]];
    let args = vec![];

    let result = plugin.execute(&args, &data);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Column index required"));
}

#[test]
fn test_uppercase_plugin_invalid_column_index() {
    let plugin = UppercasePlugin;
    let data = vec![vec!["test".to_string()]];
    let args = vec!["invalid".to_string()];

    let result = plugin.execute(&args, &data);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid column index"));
}

#[test]
fn test_prefix_plugin_execution() {
    let plugin = PrefixPlugin;
    let data = vec![
        vec!["Name".to_string(), "Age".to_string()],
        vec!["Alice".to_string(), "30".to_string()],
        vec!["Bob".to_string(), "25".to_string()],
    ];

    let args = vec!["0".to_string(), "Mr. ".to_string()];
    let result = plugin.execute(&args, &data).unwrap();

    assert_eq!(result[0][0], "Name");
    assert_eq!(result[1][0], "Mr. Alice");
    assert_eq!(result[2][0], "Mr. Bob");
    assert_eq!(result[1][1], "30");
}

#[test]
fn test_prefix_plugin_missing_args() {
    let plugin = PrefixPlugin;
    let data = vec![vec!["test".to_string()]];
    let args = vec!["0".to_string()];

    let result = plugin.execute(&args, &data);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("prefix required"));
}

#[test]
fn test_prefix_plugin_empty_prefix() {
    let plugin = PrefixPlugin;
    let data = vec![
        vec!["Name".to_string()],
        vec!["Alice".to_string()],
    ];

    let args = vec!["0".to_string(), "".to_string()];
    let result = plugin.execute(&args, &data).unwrap();

    assert_eq!(result[1][0], "Alice");
}

#[test]
fn test_registry_execute() {
    let mut registry = PluginRegistry::new();
    registry.register(UppercasePlugin);

    let data = vec![
        vec!["Name".to_string()],
        vec!["alice".to_string()],
    ];

    let args = vec!["0".to_string()];
    let result = registry.execute("uppercase", &args, &data).unwrap();

    assert_eq!(result[1][0], "ALICE");
}

#[test]
fn test_registry_execute_nonexistent_plugin() {
    let registry = PluginRegistry::new();
    let data = vec![vec!["test".to_string()]];
    let args = vec![];

    let result = registry.execute("nonexistent", &args, &data);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[test]
fn test_plugin_metadata() {
    let plugin = UppercasePlugin;
    let metadata = plugin.metadata();

    assert_eq!(metadata.name, "uppercase");
    assert!(!metadata.description.is_empty());
    assert_eq!(metadata.parameters.len(), 1);
    assert_eq!(metadata.parameters[0].name, "column");
    assert!(metadata.parameters[0].required);
}

#[test]
fn test_get_plugin_metadata() {
    let mut registry = PluginRegistry::new();
    registry.register(UppercasePlugin);

    let metadata = registry.get_metadata("uppercase");
    assert!(metadata.is_some());
    assert_eq!(metadata.unwrap().name, "uppercase");

    let missing = registry.get_metadata("nonexistent");
    assert!(missing.is_none());
}

#[test]
fn test_multiple_plugins() {
    let mut registry = PluginRegistry::new();
    registry.register(UppercasePlugin);
    registry.register(PrefixPlugin);

    let plugins = registry.list_plugins();
    assert_eq!(plugins.len(), 2);

    let data = vec![
        vec!["Name".to_string()],
        vec!["alice".to_string()],
    ];

    let result1 = registry.execute("uppercase", &["0".to_string()], &data).unwrap();
    assert_eq!(result1[1][0], "ALICE");

    let result2 = registry.execute("prefix", &["0".to_string(), "Ms. ".to_string()], &data).unwrap();
    assert_eq!(result2[1][0], "Ms. alice");
}

#[test]
fn test_plugin_with_out_of_bounds_column() {
    let plugin = UppercasePlugin;
    let data = vec![
        vec!["Name".to_string()],
        vec!["alice".to_string()],
    ];

    let args = vec!["5".to_string()];
    let result = plugin.execute(&args, &data).unwrap();

    assert_eq!(result[1][0], "alice");
}
