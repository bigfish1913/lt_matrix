// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Task resources definitions
//!
//! This module defines the resources that a task can use:
//! - docs: Reference documentation files
//! - skills: Skills to use for execution
//! - mcp_tools: MCP tools to use for execution

use serde::{Deserialize, Serialize};
use std::fmt;

/// Resources required for task execution
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskResources {
    /// Reference documentation paths (detailed business logic)
    #[serde(default)]
    pub docs: Vec<String>,

    /// Skills to use for execution
    #[serde(default)]
    pub skills: Vec<String>,

    /// MCP tools to use for execution
    #[serde(default)]
    pub mcp_tools: Vec<String>,
}

impl TaskResources {
    /// Create new task resources
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a documentation reference
    pub fn with_doc(mut self, doc: impl Into<String>) -> Self {
        self.docs.push(doc.into());
        self
    }

    /// Add a skill
    pub fn with_skill(mut self, skill: impl Into<String>) -> Self {
        self.skills.push(skill.into());
        self
    }

    /// Add an MCP tool
    pub fn with_mcp_tool(mut self, tool: impl Into<String>) -> Self {
        self.mcp_tools.push(tool.into());
        self
    }

    /// Check if any resources are defined
    pub fn is_empty(&self) -> bool {
        self.docs.is_empty() && self.skills.is_empty() && self.mcp_tools.is_empty()
    }

    /// Check if docs are defined
    pub fn has_docs(&self) -> bool {
        !self.docs.is_empty()
    }

    /// Check if skills are defined
    pub fn has_skills(&self) -> bool {
        !self.skills.is_empty()
    }

    /// Check if MCP tools are defined
    pub fn has_mcp_tools(&self) -> bool {
        !self.mcp_tools.is_empty()
    }

    /// Get total resource count
    pub fn count(&self) -> usize {
        self.docs.len() + self.skills.len() + self.mcp_tools.len()
    }
}

impl fmt::Display for TaskResources {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();
        if !self.docs.is_empty() {
            parts.push(format!("docs: {}", self.docs.len()));
        }
        if !self.skills.is_empty() {
            parts.push(format!("skills: {}", self.skills.join(", ")));
        }
        if !self.mcp_tools.is_empty() {
            parts.push(format!("mcp: {}", self.mcp_tools.join(", ")));
        }
        write!(f, "[{}]", parts.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_resources_default() {
        let res = TaskResources::default();
        assert!(res.is_empty());
        assert!(!res.has_docs());
        assert!(!res.has_skills());
        assert!(!res.has_mcp_tools());
    }

    #[test]
    fn test_task_resources_builder() {
        let res = TaskResources::new()
            .with_doc("docs/api-spec.md")
            .with_skill("frontend-design")
            .with_mcp_tool("playwright");

        assert!(!res.is_empty());
        assert!(res.has_docs());
        assert!(res.has_skills());
        assert!(res.has_mcp_tools());
        assert_eq!(res.count(), 3);
        assert_eq!(res.docs, vec!["docs/api-spec.md"]);
        assert_eq!(res.skills, vec!["frontend-design"]);
        assert_eq!(res.mcp_tools, vec!["playwright"]);
    }

    #[test]
    fn test_task_resources_display() {
        let res = TaskResources::new()
            .with_doc("docs/api.md")
            .with_skill("test");

        let display = format!("{}", res);
        assert!(display.contains("docs: 1"));
        assert!(display.contains("skills: test"));
    }

    #[test]
    fn test_task_resources_serialization() {
        let res = TaskResources::new()
            .with_doc("test.md")
            .with_skill("skill1");

        let json = serde_json::to_string(&res).unwrap();
        assert!(json.contains("test.md"));
        assert!(json.contains("skill1"));

        let deserialized: TaskResources = serde_json::from_str(&json).unwrap();
        assert_eq!(res, deserialized);
    }
}
