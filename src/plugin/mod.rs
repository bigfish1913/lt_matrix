//! Plugin system for ltmatrix extensibility
//!
//! This module provides a plugin architecture that allows extending ltmatrix with:
//! - Custom agent backends
//! - Custom pipeline stages
//! - Custom output formatters
//! - Custom validation rules
//!
//! # Plugin Discovery
//!
//! Plugins are discovered from:
//! 1. `~/.ltmatrix/plugins/` - User plugins
//! 2. `.ltmatrix/plugins/` - Project-specific plugins
//! 3. Environment variable `LTMATRIX_PLUGIN_PATH`
//!
//! # Plugin Loading
//!
//! Plugins can be:
//! - Dynamic libraries (.so, .dylib, .dll) - requires `dynamic-plugins` feature
//! - WASM modules (.wasm) - planned for future
//! - Built-in plugins (compiled into ltmatrix)
//!
//! # Custom Pipeline Stages
//!
//! Custom stages can be inserted at specific positions in the pipeline:
//!
//! ```toml
//! # ~/.ltmatrix/config.toml
//! [custom_stages.my-stage]
//! name = "My Custom Stage"
//! description = "A custom stage that does something"
//! position = { after = "execute" }  # or { before = "test" }, "first", "last"
//! enabled = true
//! timeout_seconds = 300
//!
//! [custom_stages.my-stage.config]
//! custom_option = "value"
//! ```
//!
//! # Example: Creating a Custom Stage Plugin
//!
//! ```ignore
//! use ltmatrix::plugin::{Plugin, PluginMetadata, PluginType, PipelineStagePlugin};
//! use ltmatrix::plugin::{CustomStageConfig, StageResult};
//! use ltmatrix::models::Task;
//!
//! struct MyStage {
//!     metadata: PluginMetadata,
//!     config: CustomStageConfig,
//! }
//!
//! #[async_trait]
//! impl Plugin for MyStage {
//!     fn metadata(&self) -> &PluginMetadata { &self.metadata }
//!     fn set_enabled(&mut self, enabled: bool) { self.config.enabled = enabled; }
//! }
//!
//! #[async_trait]
//! impl PipelineStagePlugin for MyStage {
//!     fn config(&self) -> &CustomStageConfig { &self.config }
//!     fn config_mut(&mut self) -> &mut CustomStageConfig { &mut self.config }
//!
//!     async fn execute(
//!         &self,
//!         tasks: Vec<Task>,
//!         config: &HashMap<String, serde_json::Value>,
//!         progress: Option<&ProgressBar>,
//!     ) -> Result<StageResult> {
//!         // Process tasks...
//!         Ok(StageResult::success(tasks))
//!     }
//! }
//! ```

pub mod agent;
pub mod discovery;
pub mod formatter;
pub mod loader;
pub mod manager;
pub mod stage;
pub mod traits;
pub mod validator;

// Re-export commonly used types
pub use manager::PluginManager;
pub use traits::{
    CustomStageConfig, Plugin, PluginMetadata, PluginType, StagePosition, StageResult,
    StandardStage,
};
pub use stage::PipelineStagePlugin;

/// Initialize the plugin manager with built-in plugins
pub async fn initialize() -> std::result::Result<PluginManager, anyhow::Error> {
    let manager = PluginManager::new();

    // Register built-in plugins
    manager
        .register_stage_plugin(stage::LoggingStage::new())
        .await?;
    manager
        .register_stage_plugin(stage::DelayStage::new())
        .await?;

    // Initialize all plugins
    manager.initialize_all().await?;

    Ok(manager)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_plugin_initialization() {
        let manager = initialize().await;
        assert!(manager.is_ok());

        let manager = manager.unwrap();
        let plugins = manager.list_plugins().await;
        assert!(!plugins.is_empty());
    }
}