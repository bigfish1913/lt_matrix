//! Telemetry and analytics module
//!
//! This module provides optional anonymous usage telemetry for ltmatrix.
//! All telemetry collection is opt-in only and respects user privacy.
//!
//! # Privacy Guarantees
//!
//! - Fully anonymous (no IP addresses or user IDs)
//! - No code content or project information
//! - Opt-in only (disabled by default)
//! - Open about what data is collected
//!
//! # What We Collect
//!
//! - Execution mode (Fast/Standard/Expert)
//! - Agent backend name (claude/opencode/etc)
//! - Task counts (total, completed, failed)
//! - Pipeline duration
//! - Error categories only (no messages/stacks)
//! - System information (OS, architecture, version)
//!
//! # What We Don't Collect
//!
//! - No IP addresses
//! - No personally identifiable information
//! - No project names or file paths
//! - No code content
//! - No full error messages or stack traces

pub mod collector;
pub mod config;
pub mod event;
pub mod sender;

pub use config::{TelemetryConfig, TelemetryConfigBuilder};
pub use event::{ErrorCategory, TelemetryEvent};
pub use collector::TelemetryCollector;
pub use sender::TelemetrySender;
