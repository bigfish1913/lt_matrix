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
//! - Dynamic libraries (.so, .dylib, .dll)
//! - WASM modules (.wasm)
//! - Built-in plugins (compiled into ltmatrix)
//!
//! # Example
//!
//! ```no_run
//! use ltmatrix::plugin::{PluginManager, PluginType};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let mut manager = PluginManager::new();
//! manager.discover_plugins().await?;
//! manager.load_plugins(PluginType::AgentBackend).await?;
//! # Ok(())
//! # }
//! ```

pub mod agent;
pub mod discovery;
pub mod formatter;
pub mod loader;
pub mod manager;
pub mod stage;
pub mod traits;
pub mod validator;

pub use manager::PluginManager;
pub use traits::{Plugin, PluginMetadata, PluginType};
