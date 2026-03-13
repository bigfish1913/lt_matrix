// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Guidelines injection system
//!
//! This module provides functionality for loading and injecting project-specific
//! development guidelines into agent contexts based on agent type.
//!
//! # Guidelines Structure
//!
//! Guidelines are stored in `.ltmatrix/guidelines/` with files for each agent type:
//! - `_common.md` - Shared guidelines for all agent types
//! - `plan.md` - Guidelines for Plan agents
//! - `dev.md` - Guidelines for Dev agents
//! - `test.md` - Guidelines for Test agents
//! - `review.md` - Guidelines for Review agents
//!
//! Alternatively, a single `.ltmatrix/guidelines.md` file can be used.
//!
//! # Example
//!
//! ```no_run
//! use ltmatrix::guidelines::{GuidelinesLoader, inject_guidelines_into_prompt};
//! use ltmatrix_core::AgentType;
//! use std::path::PathBuf;
//!
//! let loader = GuidelinesLoader::new(PathBuf::from(".ltmatrix/guidelines"));
//! let guidelines = loader.load_for_agent_type(AgentType::Dev).unwrap();
//!
//! if let Some(content) = guidelines {
//!     let prompt_with_guidelines = inject_guidelines_into_prompt(
//!         "Implement feature X",
//!         &content
//!     );
//! }
//! ```

mod loader;

pub use loader::{Guidelines, GuidelinesLoader};
use ltmatrix_core::AgentType;

/// Injects guidelines content into a prompt
///
/// This function prepends the guidelines to the prompt in a structured format
/// that helps the agent understand they are project-specific rules.
///
/// # Arguments
///
/// * `prompt` - The original prompt to execute
/// * `guidelines` - The guidelines content to inject
///
/// # Returns
///
/// Returns the prompt with guidelines prepended.
pub fn inject_guidelines_into_prompt(prompt: &str, guidelines: &str) -> String {
    if guidelines.trim().is_empty() {
        return prompt.to_string();
    }

    format!(
        r#"## Project Guidelines

The following guidelines are project-specific rules that you must follow:

{guidelines}

---

## Task

{prompt}"#
    )
}

/// Combines common guidelines with agent-specific guidelines
///
/// This function merges the shared `_common.md` guidelines with the
/// agent-type-specific guidelines.
///
/// # Arguments
///
/// * `common` - Optional common guidelines content
/// * `specific` - Optional agent-specific guidelines content
///
/// # Returns
///
/// Returns the combined guidelines content.
pub fn combine_guidelines(common: Option<&str>, specific: Option<&str>) -> String {
    let mut combined = String::new();

    if let Some(common_content) = common {
        if !common_content.trim().is_empty() {
            combined.push_str(common_content);
        }
    }

    if let Some(specific_content) = specific {
        if !specific_content.trim().is_empty() {
            if !combined.is_empty() {
                combined.push_str("\n\n");
            }
            combined.push_str(specific_content);
        }
    }

    combined
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inject_guidelines_empty() {
        let prompt = "Implement feature X";
        let result = inject_guidelines_into_prompt(prompt, "");
        assert_eq!(result, prompt);
    }

    #[test]
    fn test_inject_guidelines_whitespace_only() {
        let prompt = "Implement feature X";
        let result = inject_guidelines_into_prompt(prompt, "   \n  ");
        assert_eq!(result, prompt);
    }

    #[test]
    fn test_inject_guidelines_basic() {
        let prompt = "Implement feature X";
        let guidelines = "Use snake_case for variables";
        let result = inject_guidelines_into_prompt(prompt, guidelines);

        assert!(result.contains("## Project Guidelines"));
        assert!(result.contains(guidelines));
        assert!(result.contains(prompt));
    }

    #[test]
    fn test_combine_guidelines_both() {
        let common = "Use UTF-8 encoding";
        let specific = "Use snake_case for functions";
        let result = combine_guidelines(Some(common), Some(specific));

        assert!(result.contains(common));
        assert!(result.contains(specific));
    }

    #[test]
    fn test_combine_guidelines_common_only() {
        let common = "Use UTF-8 encoding";
        let result = combine_guidelines(Some(common), None);
        assert_eq!(result, common);
    }

    #[test]
    fn test_combine_guidelines_specific_only() {
        let specific = "Use snake_case for functions";
        let result = combine_guidelines(None, Some(specific));
        assert_eq!(result, specific);
    }

    #[test]
    fn test_combine_guidelines_empty() {
        let result = combine_guidelines(None, None);
        assert!(result.is_empty());
    }

    #[test]
    fn test_combine_guidelines_whitespace() {
        let result = combine_guidelines(Some("   "), Some("   "));
        assert!(result.is_empty() || result.trim().is_empty());
    }
}
