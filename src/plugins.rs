//! Plugin system for custom functions
//!
//! Provides a trait-based plugin system for extending xls-rs with custom operations.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: Option<String>,
    pub functions: Vec<FunctionMetadata>,
}

/// Function metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionMetadata {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ParameterMetadata>,
    pub return_type: String,
}

/// Parameter metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterMetadata {
    pub name: String,
    pub param_type: String,
    pub required: bool,
    pub default: Option<String>,
    pub description: Option<String>,
}

/// Trait for plugin functions
pub trait PluginFunction: Send + Sync {
    fn name(&self) -> &str;
    fn execute(&self, args: &[String], data: &[Vec<String>]) -> Result<Vec<Vec<String>>>;
    fn metadata(&self) -> FunctionMetadata;
}

/// Plugin registry
pub struct PluginRegistry {
    plugins: HashMap<String, Box<dyn PluginFunction>>,
    metadata: HashMap<String, PluginMetadata>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    /// Register a plugin function
    pub fn register<F>(&mut self, function: F)
    where
        F: PluginFunction + 'static,
    {
        let name = function.name().to_string();
        let func_metadata = function.metadata();

        // Update or create plugin metadata
        let plugin_meta = self
            .metadata
            .entry(name.clone())
            .or_insert_with(|| PluginMetadata {
                name: name.clone(),
                version: "1.0.0".to_string(),
                description: format!("Plugin: {}", name),
                author: None,
                functions: Vec::new(),
            });

        plugin_meta.functions.push(func_metadata);
        self.plugins.insert(name, Box::new(function));
    }

    /// Execute a plugin function
    pub fn execute(
        &self,
        function_name: &str,
        args: &[String],
        data: &[Vec<String>],
    ) -> Result<Vec<Vec<String>>> {
        let function = self
            .plugins
            .get(function_name)
            .ok_or_else(|| anyhow::anyhow!("Plugin function '{}' not found", function_name))?;

        function.execute(args, data)
    }

    /// List all registered plugins
    pub fn list_plugins(&self) -> Vec<&PluginMetadata> {
        self.metadata.values().collect()
    }

    /// Get plugin metadata
    pub fn get_metadata(&self, name: &str) -> Option<&PluginMetadata> {
        self.metadata.get(name)
    }
}

/// Example plugin: Uppercase transformation
pub struct UppercasePlugin;

impl PluginFunction for UppercasePlugin {
    fn name(&self) -> &str {
        "uppercase"
    }

    fn execute(&self, args: &[String], data: &[Vec<String>]) -> Result<Vec<Vec<String>>> {
        if args.is_empty() {
            anyhow::bail!("Column index required");
        }

        let col_idx: usize = args[0]
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid column index: {}", args[0]))?;

        let mut result = data.to_vec();

        for row in result.iter_mut().skip(1) {
            // Skip header
            if let Some(cell) = row.get_mut(col_idx) {
                *cell = cell.to_uppercase();
            }
        }

        Ok(result)
    }

    fn metadata(&self) -> FunctionMetadata {
        FunctionMetadata {
            name: "uppercase".to_string(),
            description: "Convert column values to uppercase".to_string(),
            parameters: vec![ParameterMetadata {
                name: "column".to_string(),
                param_type: "usize".to_string(),
                required: true,
                default: None,
                description: Some("Column index to transform".to_string()),
            }],
            return_type: "Vec<Vec<String>>".to_string(),
        }
    }
}

/// Example plugin: Add prefix
pub struct PrefixPlugin;

impl PluginFunction for PrefixPlugin {
    fn name(&self) -> &str {
        "prefix"
    }

    fn execute(&self, args: &[String], data: &[Vec<String>]) -> Result<Vec<Vec<String>>> {
        if args.len() < 2 {
            anyhow::bail!("Column index and prefix required");
        }

        let col_idx: usize = args[0]
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid column index: {}", args[0]))?;
        let prefix = &args[1];

        let mut result = data.to_vec();

        for row in result.iter_mut().skip(1) {
            // Skip header
            if let Some(cell) = row.get_mut(col_idx) {
                *cell = format!("{}{}", prefix, cell);
            }
        }

        Ok(result)
    }

    fn metadata(&self) -> FunctionMetadata {
        FunctionMetadata {
            name: "prefix".to_string(),
            description: "Add prefix to column values".to_string(),
            parameters: vec![
                ParameterMetadata {
                    name: "column".to_string(),
                    param_type: "usize".to_string(),
                    required: true,
                    default: None,
                    description: Some("Column index to transform".to_string()),
                },
                ParameterMetadata {
                    name: "prefix".to_string(),
                    param_type: "String".to_string(),
                    required: true,
                    default: None,
                    description: Some("Prefix to add".to_string()),
                },
            ],
            return_type: "Vec<Vec<String>>".to_string(),
        }
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        let mut registry = Self::new();

        // Register built-in plugins
        registry.register(UppercasePlugin);
        registry.register(PrefixPlugin);

        registry
    }
}
