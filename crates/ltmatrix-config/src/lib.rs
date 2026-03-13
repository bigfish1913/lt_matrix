// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Configuration management for ltmatrix
//!
//! This crate handles loading, parsing, and managing configuration from
//! TOML files and command-line arguments.

pub mod agent;
pub mod feature;
pub mod mcp;
pub mod modes;
pub mod settings;
pub mod telemetry;

// Re-export commonly used types
pub use feature::FeatureConfig;
pub use settings::Config;
pub use telemetry::TelemetryConfig;
