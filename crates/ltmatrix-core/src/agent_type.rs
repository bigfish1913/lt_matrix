// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Agent type definitions for ltmatrix
//!
//! This module defines the types of agents available in the system.

use serde::{Deserialize, Serialize};

/// Agent type enum representing different agent roles
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentType {
    /// Planning agent - responsible for task breakdown and complexity assessment
    Plan,
    /// Development agent - responsible for code implementation
    #[default]
    Dev,
    /// Testing agent - responsible for writing and running tests
    Test,
    /// Review agent - responsible for code review (Expert mode only)
    Review,
}

impl AgentType {
    /// Returns all agent types
    pub fn all() -> &'static [AgentType; 4] {
        static TYPES: [AgentType; 4] = [
            AgentType::Plan,
            AgentType::Dev,
            AgentType::Test,
            AgentType::Review,
        ];
        &TYPES
    }

    /// Detects agent type from task description text using keyword matching
    pub fn from_keywords(text: &str) -> Self {
        let text_lower = text.to_lowercase();

        // Plan keywords
        if text_lower.contains("分析")
            || text_lower.contains("规划")
            || text_lower.contains("拆分")
            || text_lower.contains("评估")
            || text_lower.contains("analyze")
            || text_lower.contains("plan")
            || text_lower.contains("breakdown")
        {
            return AgentType::Plan;
        }

        // Test keywords
        if text_lower.contains("测试")
            || text_lower.contains("验证")
            || text_lower.contains("覆盖率")
            || text_lower.contains("断言")
            || text_lower.contains("test")
            || text_lower.contains("verify")
            || text_lower.contains("coverage")
            || text_lower.contains("assert")
        {
            return AgentType::Test;
        }

        // Review keywords
        if text_lower.contains("审查")
            || text_lower.contains("检查")
            || text_lower.contains("审计")
            || text_lower.contains("评审")
            || text_lower.contains("review")
            || text_lower.contains("audit")
            || text_lower.contains("inspect")
        {
            return AgentType::Review;
        }

        // Default to Dev for implementation tasks
        AgentType::Dev
    }

    /// Returns the display name of the agent type
    pub fn display_name(&self) -> &'static str {
        match self {
            AgentType::Plan => "Planning Agent",
            AgentType::Dev => "Development Agent",
            AgentType::Test => "Testing Agent",
            AgentType::Review => "Review Agent",
        }
    }

    /// Returns true if this agent type is enabled in the given mode
    pub fn is_enabled_in_mode(&self, mode: crate::ExecutionMode) -> bool {
        mode.enabled_agents().contains(self)
    }
}

impl std::fmt::Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentType::Plan => write!(f, "plan"),
            AgentType::Dev => write!(f, "dev"),
            AgentType::Test => write!(f, "test"),
            AgentType::Review => write!(f, "review"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_type_from_keywords_plan() {
        assert_eq!(AgentType::from_keywords("分析系统架构"), AgentType::Plan);
        assert_eq!(AgentType::from_keywords("规划任务拆分"), AgentType::Plan);
        assert_eq!(
            AgentType::from_keywords("analyze the requirements"),
            AgentType::Plan
        );
    }

    #[test]
    fn test_agent_type_from_keywords_test() {
        assert_eq!(AgentType::from_keywords("编写测试用例"), AgentType::Test);
        assert_eq!(AgentType::from_keywords("验证功能正确性"), AgentType::Test);
        assert_eq!(AgentType::from_keywords("run unit tests"), AgentType::Test);
    }

    #[test]
    fn test_agent_type_from_keywords_review() {
        assert_eq!(AgentType::from_keywords("审查代码质量"), AgentType::Review);
        assert_eq!(AgentType::from_keywords("代码审计"), AgentType::Review);
        assert_eq!(
            AgentType::from_keywords("review the changes"),
            AgentType::Review
        );
    }

    #[test]
    fn test_agent_type_from_keywords_dev() {
        assert_eq!(AgentType::from_keywords("实现用户登录功能"), AgentType::Dev);
        assert_eq!(AgentType::from_keywords("修复bug"), AgentType::Dev);
        assert_eq!(
            AgentType::from_keywords("implement new feature"),
            AgentType::Dev
        );
    }

    #[test]
    fn test_agent_type_default() {
        assert_eq!(AgentType::default(), AgentType::Dev);
    }

    #[test]
    fn test_agent_type_display() {
        assert_eq!(format!("{}", AgentType::Plan), "plan");
        assert_eq!(format!("{}", AgentType::Dev), "dev");
        assert_eq!(format!("{}", AgentType::Test), "test");
        assert_eq!(format!("{}", AgentType::Review), "review");
    }
}
