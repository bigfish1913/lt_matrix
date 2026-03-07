// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.


//! Testing framework detection and execution
//!
//! This module provides automatic detection and execution of testing frameworks
//! across multiple programming languages and ecosystems. It supports pytest,
//! npm test, go test, and cargo test, with the ability to run tests and analyze
//! results.
//!
//! # Architecture
//!
//! The testing module is organized into several components:
//!
//! - **detector**: Framework detection logic and command mapping
//! - **executor**: Test execution and result collection (future)
//! - **parser**: Test output parsing for various frameworks (future)
//!
//! # Features
//!
//! - **Automatic detection**: Detects which testing framework is used in a project
//! - **Cross-language support**: Python (pytest), JavaScript/TypeScript (npm), Go, Rust
//! - **Command mapping**: Maps detected frameworks to appropriate test commands
//! - **Result analysis**: Parses test output to determine pass/fail status
//!
//! # Example
//!
//! ```no_run
//! use ltmatrix::testing::{Framework, detect_framework, TestCommand};
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Detect the testing framework in the current directory
//! let framework = detect_framework(".").await?;
//!
//! match framework {
//!     Framework::Pytest => {
//!         println!("Detected pytest");
//!         let cmd = TestCommand::for_framework(&framework);
//!         // Execute test command...
//!     }
//!     Framework::Npm => {
//!         println!("Detected npm test");
//!     }
//!     Framework::Go => {
//!         println!("Detected Go tests");
//!     }
//!     Framework::Cargo => {
//!         println!("Detected Cargo tests");
//!     }
//!     Framework::None => {
//!         println!("No testing framework detected");
//!     }
//! }
//! # Ok(())
//! # }
//! ```

pub mod detector;

// Re-exports for convenience
pub use detector::{detect_framework, Framework, TestCommand};
