// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Project-level memory management
//!
//! This module provides persistent project-level memory storage that tracks:
//! - Project structure and architecture
//! - Technology stack and dependencies
//! - Coding conventions and patterns
//! - Completed tasks history
//!
//! Memory is stored at `.ltmatrix/memory/project.json`

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, info, warn};

/// Project-level memory structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMemory {
    /// Schema version for future migrations
    pub version: String,

    /// Project name
    pub project_name: String,

    /// When this memory was first created
    pub created_at: DateTime<Utc>,

    /// When this memory was last updated
    pub updated_at: DateTime<Utc>,

    /// Project structure information
    pub structure: ProjectStructure,

    /// Technology stack
    pub tech_stack: TechStack,

    /// Coding conventions
    pub conventions: CodingConventions,

    /// Completed tasks history
    pub completed_tasks: Vec<CompletedTask>,

    /// Architecture decisions
    pub decisions: Vec<ArchitectureDecision>,

    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Project structure information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectStructure {
    /// Project root directory name
    pub root_name: String,

    /// Main source directories
    pub source_dirs: Vec<String>,

    /// Main entry points
    pub entry_points: Vec<String>,

    /// Project type (library, binary, hybrid)
    pub project_type: ProjectType,

    /// Build system
    pub build_system: Option<String>,
}

/// Project type classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProjectType {
    #[default]
    Library,
    Binary,
    Hybrid,
    Workspace,
}

/// Technology stack information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TechStack {
    /// Primary language
    pub language: Option<String>,

    /// Language version
    pub language_version: Option<String>,

    /// Frameworks used
    pub frameworks: Vec<String>,

    /// Key dependencies
    pub dependencies: Vec<Dependency>,

    /// Runtime requirements
    pub runtime: Option<String>,
}

/// Dependency information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    /// Dependency name
    pub name: String,

    /// Version constraint
    pub version: String,

    /// Whether it's a dev dependency
    pub is_dev: bool,
}

/// Coding conventions
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CodingConventions {
    /// Code style preferences
    pub style: HashMap<String, String>,

    /// Naming conventions
    pub naming: HashMap<String, String>,

    /// Documentation style
    pub documentation: Option<String>,

    /// Testing conventions
    pub testing: HashMap<String, String>,

    /// Error handling patterns
    pub error_handling: Vec<String>,
}

/// Record of a completed task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedTask {
    /// Task ID
    pub id: String,

    /// Task title
    pub title: String,

    /// Completion timestamp
    pub completed_at: DateTime<Utc>,

    /// Files affected
    pub files_affected: Vec<String>,

    /// Agent type used
    pub agent_type: String,

    /// Complexity level
    pub complexity: String,

    /// Key outcomes
    pub outcomes: Vec<String>,
}

/// Architecture decision record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureDecision {
    /// Decision ID
    pub id: String,

    /// Decision title
    pub title: String,

    /// When the decision was made
    pub decided_at: DateTime<Utc>,

    /// Decision context
    pub context: String,

    /// Decision rationale
    pub rationale: String,

    /// Consequences
    pub consequences: Vec<String>,

    /// Related tasks
    pub related_tasks: Vec<String>,
}

impl Default for ProjectMemory {
    fn default() -> Self {
        let now = Utc::now();
        ProjectMemory {
            version: "1.0.0".to_string(),
            project_name: String::new(),
            created_at: now,
            updated_at: now,
            structure: ProjectStructure::default(),
            tech_stack: TechStack::default(),
            conventions: CodingConventions::default(),
            completed_tasks: Vec::new(),
            decisions: Vec::new(),
            metadata: HashMap::new(),
        }
    }
}

impl ProjectMemory {
    /// Create a new project memory
    pub fn new(project_name: impl Into<String>) -> Self {
        ProjectMemory {
            project_name: project_name.into(),
            ..Default::default()
        }
    }

    /// Load project memory from file
    pub async fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            debug!("Project memory file not found at {:?}, creating new", path);
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path)
            .await
            .context("Failed to read project memory file")?;

        let memory: ProjectMemory = serde_json::from_str(&content)
            .context("Failed to parse project memory JSON")?;

        info!("Loaded project memory from {:?}", path);
        Ok(memory)
    }

    /// Save project memory to file
    pub async fn save(&self, path: &Path) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .context("Failed to create memory directory")?;
        }

        let content = serde_json::to_string_pretty(self)
            .context("Failed to serialize project memory")?;

        fs::write(path, content)
            .await
            .context("Failed to write project memory file")?;

        debug!("Saved project memory to {:?}", path);
        Ok(())
    }

    /// Record a completed task
    pub fn record_completed_task(&mut self, task: CompletedTask) {
        // Check if task already exists
        if let Some(existing) = self.completed_tasks.iter_mut().find(|t| t.id == task.id) {
            *existing = task;
        } else {
            self.completed_tasks.push(task);
        }
        self.updated_at = Utc::now();
    }

    /// Add an architecture decision
    pub fn add_decision(&mut self, decision: ArchitectureDecision) {
        // Check if decision already exists
        if let Some(existing) = self.decisions.iter_mut().find(|d| d.id == decision.id) {
            *existing = decision;
        } else {
            self.decisions.push(decision);
        }
        self.updated_at = Utc::now();
    }

    /// Update tech stack
    pub fn update_tech_stack(&mut self, tech_stack: TechStack) {
        self.tech_stack = tech_stack;
        self.updated_at = Utc::now();
    }

    /// Update conventions
    pub fn update_conventions(&mut self, conventions: CodingConventions) {
        self.conventions = conventions;
        self.updated_at = Utc::now();
    }

    /// Get recent completed tasks (last N)
    pub fn get_recent_tasks(&self, limit: usize) -> &[CompletedTask] {
        let start = self.completed_tasks.len().saturating_sub(limit);
        &self.completed_tasks[start..]
    }

    /// Get tasks by agent type
    pub fn get_tasks_by_agent_type(&self, agent_type: &str) -> Vec<&CompletedTask> {
        self.completed_tasks
            .iter()
            .filter(|t| t.agent_type == agent_type)
            .collect()
    }

    /// Generate a summary for context injection
    pub fn generate_summary(&self) -> String {
        let mut summary = String::new();

        summary.push_str(&format!("# Project: {}\n\n", self.project_name));

        // Tech stack
        if let Some(ref lang) = self.tech_stack.language {
            summary.push_str(&format!("**Language**: {}", lang));
            if let Some(ref version) = self.tech_stack.language_version {
                summary.push_str(&format!(" {}", version));
            }
            summary.push('\n');
        }

        if !self.tech_stack.frameworks.is_empty() {
            summary.push_str(&format!(
                "**Frameworks**: {}\n",
                self.tech_stack.frameworks.join(", ")
            ));
        }

        // Project structure
        if !self.structure.source_dirs.is_empty() {
            summary.push_str(&format!(
                "**Source dirs**: {}\n",
                self.structure.source_dirs.join(", ")
            ));
        }

        // Recent activity
        if !self.completed_tasks.is_empty() {
            summary.push_str(&format!(
                "\n## Recent Tasks ({} total)\n",
                self.completed_tasks.len()
            ));
            for task in self.get_recent_tasks(5) {
                summary.push_str(&format!(
                    "- {} ({})\n",
                    task.title, task.agent_type
                ));
            }
        }

        // Key decisions
        if !self.decisions.is_empty() {
            summary.push_str(&format!("\n## Architecture Decisions ({} total)\n", self.decisions.len()));
            for decision in self.decisions.iter().take(3) {
                summary.push_str(&format!("- {}: {}\n", decision.title, decision.rationale));
            }
        }

        summary
    }
}

/// Get the default project memory path
pub fn get_project_memory_path(project_root: &Path) -> PathBuf {
    project_root.join(".ltmatrix").join("memory").join("project.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_project_memory_new() {
        let memory = ProjectMemory::new("test-project");
        assert_eq!(memory.project_name, "test-project");
        assert!(memory.completed_tasks.is_empty());
    }

    #[test]
    fn test_record_completed_task() {
        let mut memory = ProjectMemory::new("test");
        let task = CompletedTask {
            id: "task-1".to_string(),
            title: "Test task".to_string(),
            completed_at: Utc::now(),
            files_affected: vec!["src/main.rs".to_string()],
            agent_type: "dev".to_string(),
            complexity: "moderate".to_string(),
            outcomes: vec!["Implemented feature".to_string()],
        };

        memory.record_completed_task(task);
        assert_eq!(memory.completed_tasks.len(), 1);
    }

    #[test]
    fn test_get_recent_tasks() {
        let mut memory = ProjectMemory::new("test");

        for i in 0..10 {
            memory.record_completed_task(CompletedTask {
                id: format!("task-{}", i),
                title: format!("Task {}", i),
                completed_at: Utc::now(),
                files_affected: vec![],
                agent_type: "dev".to_string(),
                complexity: "simple".to_string(),
                outcomes: vec![],
            });
        }

        let recent = memory.get_recent_tasks(3);
        assert_eq!(recent.len(), 3);
    }

    #[test]
    fn test_generate_summary() {
        let mut memory = ProjectMemory::new("test-project");
        memory.tech_stack.language = Some("Rust".to_string());
        memory.tech_stack.language_version = Some("1.75".to_string());
        memory.structure.source_dirs = vec!["src".to_string()];

        let summary = memory.generate_summary();
        assert!(summary.contains("test-project"));
        assert!(summary.contains("Rust"));
        assert!(summary.contains("src"));
    }

    #[tokio::test]
    async fn test_save_and_load() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("project.json");

        let mut memory = ProjectMemory::new("test");
        memory.tech_stack.language = Some("Rust".to_string());
        memory.save(&path).await.unwrap();

        let loaded = ProjectMemory::load(&path).await.unwrap();
        assert_eq!(loaded.project_name, "test");
        assert_eq!(loaded.tech_stack.language, Some("Rust".to_string()));
    }
}
