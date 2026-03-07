// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.


//! Command-line interface parsing and handling
//!
//! This module provides argument parsing and CLI utilities for ltmatrix.

pub mod args;
pub mod command;

pub use args::Args;
pub use command::execute_command;
