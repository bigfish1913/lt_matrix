//! Configuration management
//!
//! This module handles loading, parsing, and managing configuration from
//! TOML files and command-line arguments.

pub mod settings;
pub mod agent;
pub mod modes;

pub use settings::Settings;
pub use agent::AgentConfig;
pub use modes::ExecutionMode;
