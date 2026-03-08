// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Execution mode definitions
//!
//! This module defines the execution modes for ltmatrix:
//! - Fast: Quick execution, skips tests
//! - Standard: Full pipeline with tests
//! - Expert: Full pipeline with review stage

use serde::{Deserialize, Serialize};
use std::fmt;

use super::AgentType;

/// Execution mode determining the pipeline strategy
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Fast mode: skip tests, use fast models, minimal verification
    Fast,
    /// Standard mode: full 6-stage pipeline with complete testing
    #[default]
    Standard,
    /// Expert mode: highest quality with code review and thorough testing
    Expert,
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Mode::Fast => write!(f, "fast"),
            Mode::Standard => write!(f, "standard"),
            Mode::Expert => write!(f, "expert"),
        }
    }
}

impl std::str::FromStr for Mode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "fast" => Ok(Mode::Fast),
            "standard" | "normal" => Ok(Mode::Standard),
            "expert" => Ok(Mode::Expert),
            _ => Err(format!("Invalid mode: {}. Valid options: fast, standard, expert", s)),
        }
    }
}

impl Mode {
    /// Returns true if tests should be run in this mode
    pub fn run_tests(&self) -> bool {
        !matches!(self, Mode::Fast)
    }

    /// Returns true if review stage should be run in this mode
    pub fn run_review(&self) -> bool {
        matches!(self, Mode::Expert)
    }

    /// Returns the enabled agent types for this mode
    pub fn enabled_agents(&self) -> Vec<AgentType> {
        match self {
            Mode::Fast => vec![AgentType::Plan, AgentType::Dev],
            Mode::Standard => vec![AgentType::Plan, AgentType::Dev, AgentType::Test],
            Mode::Expert => vec![AgentType::Plan, AgentType::Dev, AgentType::Test, AgentType::Review],
        }
    }

    /// Check if a specific agent type is enabled in this mode
    pub fn is_agent_enabled(&self, agent_type: AgentType) -> bool {
        self.enabled_agents().contains(&agent_type)
    }

    /// Returns the maximum task depth for this mode
    pub fn max_depth(&self) -> u32 {
        match self {
            Mode::Fast => 2,
            Mode::Standard => 3,
            Mode::Expert => 3,
        }
    }

    /// Returns the maximum retry count for this mode
    pub fn max_retries(&self) -> u32 {
        match self {
            Mode::Fast => 2,
            Mode::Standard => 3,
            Mode::Expert => 5,
        }
    }

    /// Returns the plan timeout in seconds
    pub fn plan_timeout(&self) -> u64 {
        match self {
            Mode::Fast => 60,
            Mode::Standard => 120,
            Mode::Expert => 180,
        }
    }

    /// Returns the task execution timeout in seconds
    pub fn task_timeout(&self) -> u64 {
        match self {
            Mode::Fast => 1800,    // 30 minutes
            Mode::Standard => 3600, // 60 minutes
            Mode::Expert => 7200,   // 120 minutes
        }
    }

    /// Returns the default model for planning
    pub fn plan_model(&self) -> &'static str {
        "claude-opus-4-6"
    }

    /// Returns the default model for execution
    pub fn exec_model(&self) -> &'static str {
        "claude-sonnet-4-6"
    }

    /// Returns the default model for review
    pub fn review_model(&self) -> &'static str {
        "claude-opus-4-6"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mode_enabled_agents() {
        let fast = Mode::Fast;
        assert!(fast.is_agent_enabled(AgentType::Plan));
        assert!(fast.is_agent_enabled(AgentType::Dev));
        assert!(!fast.is_agent_enabled(AgentType::Test));
        assert!(!fast.is_agent_enabled(AgentType::Review));

        let standard = Mode::Standard;
        assert!(standard.is_agent_enabled(AgentType::Test));
        assert!(!standard.is_agent_enabled(AgentType::Review));

        let expert = Mode::Expert;
        assert!(expert.is_agent_enabled(AgentType::Review));
    }

    #[test]
    fn test_mode_defaults() {
        assert_eq!(Mode::default(), Mode::Standard);
    }

    #[test]
    fn test_mode_from_str() {
        assert_eq!("fast".parse::<Mode>(), Ok(Mode::Fast));
        assert_eq!("standard".parse::<Mode>(), Ok(Mode::Standard));
        assert_eq!("expert".parse::<Mode>(), Ok(Mode::Expert));
        assert!("invalid".parse::<Mode>().is_err());
    }

    #[test]
    fn test_mode_timeouts() {
        assert_eq!(Mode::Fast.plan_timeout(), 60);
        assert_eq!(Mode::Standard.plan_timeout(), 120);
        assert_eq!(Mode::Expert.plan_timeout(), 180);
    }

    #[test]
    fn test_mode_max_retries() {
        assert_eq!(Mode::Fast.max_retries(), 2);
        assert_eq!(Mode::Standard.max_retries(), 3);
        assert_eq!(Mode::Expert.max_retries(), 5);
    }
}
