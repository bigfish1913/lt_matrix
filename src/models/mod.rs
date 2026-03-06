//! Core data models and structures for ltmatrix
//!
//! This module defines the fundamental data types used throughout the application,
//! including tasks, agents, execution modes, and pipeline stages.

use serde::{Deserialize, Serialize};

/// Represents a task in the development pipeline
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Task {
    /// Unique identifier for the task
    pub id: String,

    /// Human-readable title
    pub title: String,

    /// Detailed description of what the task should accomplish
    pub description: String,

    /// Current status of the task
    pub status: TaskStatus,

    /// Complexity level of the task (used for agent selection)
    pub complexity: TaskComplexity,

    /// List of task IDs this task depends on
    #[serde(default)]
    pub depends_on: Vec<String>,

    /// Subtasks for complex tasks (max depth: 3)
    #[serde(default)]
    pub subtasks: Vec<Task>,

    /// Number of retries attempted
    #[serde(default)]
    pub retry_count: u32,

    /// Error message if task failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Timestamp when task was created
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Timestamp when task was started
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,

    /// Timestamp when task was completed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl Task {
    /// Creates a new task with the given ID, title, and description
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        let now = chrono::Utc::now();
        Task {
            id: id.into(),
            title: title.into(),
            description: description.into(),
            status: TaskStatus::Pending,
            complexity: TaskComplexity::Moderate,
            depends_on: Vec::new(),
            subtasks: Vec::new(),
            retry_count: 0,
            error: None,
            created_at: now,
            started_at: None,
            completed_at: None,
        }
    }

    /// Returns true if the task has no unresolved dependencies
    pub fn can_execute(&self, completed_tasks: &std::collections::HashSet<String>) -> bool {
        self.depends_on
            .iter()
            .all(|dep_id| completed_tasks.contains(dep_id))
    }

    /// Returns true if the task has failed
    pub fn is_failed(&self) -> bool {
        matches!(self.status, TaskStatus::Failed)
    }

    /// Returns true if the task is complete
    pub fn is_completed(&self) -> bool {
        matches!(self.status, TaskStatus::Completed)
    }

    /// Returns true if the task can be retried
    pub fn can_retry(&self, max_retries: u32) -> bool {
        self.is_failed() && self.retry_count < max_retries
    }
}

/// Current status of a task
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Task is waiting for dependencies to complete
    Pending,

    /// Task is currently being executed
    InProgress,

    /// Task completed successfully
    Completed,

    /// Task failed and may be retried
    Failed,

    /// Task is blocked by an external issue
    Blocked,
}

impl TaskStatus {
    /// Returns true if the status is a terminal state (completed or failed)
    pub fn is_terminal(&self) -> bool {
        matches!(self, TaskStatus::Completed | TaskStatus::Failed)
    }
}

/// Complexity level of a task, determining which agent model to use
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TaskComplexity {
    /// Simple task that can be handled by fast models
    Simple,

    /// Moderate task requiring standard model
    Moderate,

    /// Complex task requiring the most capable model
    Complex,
}

/// Configuration for an AI agent backend
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Agent {
    /// Unique name/identifier for the agent
    pub name: String,

    /// Command to invoke the agent (e.g., "claude", "opencode")
    pub command: String,

    /// Model identifier to use (e.g., "claude-sonnet-4-6")
    pub model: String,

    /// Timeout in seconds for agent operations
    pub timeout: u64,

    /// Whether this agent is the default
    #[serde(default)]
    pub is_default: bool,
}

impl Agent {
    /// Creates a new agent configuration
    pub fn new(
        name: impl Into<String>,
        command: impl Into<String>,
        model: impl Into<String>,
        timeout: u64,
    ) -> Self {
        Agent {
            name: name.into(),
            command: command.into(),
            model: model.into(),
            timeout,
            is_default: false,
        }
    }

    /// Creates the default Claude agent configuration
    pub fn claude_default() -> Self {
        Agent::new("claude", "claude", "claude-sonnet-4-6", 3600).with_default()
    }

    /// Creates the default OpenCode agent configuration
    pub fn opencode_default() -> Self {
        Agent::new("opencode", "opencode", "gpt-4", 3600)
    }

    /// Creates the default KimiCode agent configuration
    pub fn kimicode_default() -> Self {
        Agent::new("kimicode", "kimi-code", "moonshot-v1-128k", 3600)
    }

    /// Marks this agent as the default
    pub fn with_default(mut self) -> Self {
        self.is_default = true;
        self
    }
}

/// Execution mode determining the pipeline strategy
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    /// Fast mode: skip tests, use fast models, minimal verification
    Fast,

    /// Standard mode: full 6-stage pipeline with complete testing
    Standard,

    /// Expert mode: highest quality with code review and thorough testing
    Expert,
}

impl Default for ExecutionMode {
    fn default() -> Self {
        ExecutionMode::Standard
    }
}

impl ExecutionMode {
    /// Returns true if tests should be run in this mode
    pub fn run_tests(&self) -> bool {
        !matches!(self, ExecutionMode::Fast)
    }

    /// Returns the default model to use for this mode
    pub fn default_model(&self) -> &'static str {
        match self {
            ExecutionMode::Fast => "claude-haiku-4-5",
            ExecutionMode::Standard => "claude-sonnet-4-6",
            ExecutionMode::Expert => "claude-opus-4-6",
        }
    }

    /// Returns the maximum task depth for this mode
    pub fn max_depth(&self) -> u32 {
        match self {
            ExecutionMode::Fast => 2,
            ExecutionMode::Standard => 3,
            ExecutionMode::Expert => 3,
        }
    }

    /// Returns the maximum retry count for this mode
    pub fn max_retries(&self) -> u32 {
        match self {
            ExecutionMode::Fast => 1,
            ExecutionMode::Standard => 3,
            ExecutionMode::Expert => 3,
        }
    }
}

/// Individual stage in the pipeline
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum PipelineStage {
    /// Generate task breakdown from goal
    Generate,

    /// Assess task complexity and create subtasks if needed
    Assess,

    /// Execute the task implementation
    Execute,

    /// Write and run tests
    Test,

    /// Verify task completion
    Verify,

    /// Commit changes to git
    Commit,

    /// Update project memory
    Memory,
}

impl PipelineStage {
    /// Returns all stages in order for a given execution mode
    pub fn pipeline_for_mode(mode: ExecutionMode) -> Vec<PipelineStage> {
        match mode {
            ExecutionMode::Fast => vec![
                PipelineStage::Generate,
                PipelineStage::Assess,
                PipelineStage::Execute,
                PipelineStage::Verify,
                PipelineStage::Commit,
                PipelineStage::Memory,
            ],
            ExecutionMode::Standard | ExecutionMode::Expert => vec![
                PipelineStage::Generate,
                PipelineStage::Assess,
                PipelineStage::Execute,
                PipelineStage::Test,
                PipelineStage::Verify,
                PipelineStage::Commit,
                PipelineStage::Memory,
            ],
        }
    }

    /// Returns the display name of the stage
    pub fn display_name(&self) -> &'static str {
        match self {
            PipelineStage::Generate => "Generate",
            PipelineStage::Assess => "Assess",
            PipelineStage::Execute => "Execute",
            PipelineStage::Test => "Test",
            PipelineStage::Verify => "Verify",
            PipelineStage::Commit => "Commit",
            PipelineStage::Memory => "Memory",
        }
    }

    /// Returns true if the stage involves external agent interaction
    pub fn requires_agent(&self) -> bool {
        !matches!(self, PipelineStage::Commit | PipelineStage::Memory)
    }
}

/// Configuration for execution modes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeConfig {
    /// Model to use for simple tasks
    pub model_fast: String,

    /// Model to use for complex tasks
    pub model_smart: String,

    /// Whether to run tests
    pub run_tests: bool,

    /// Whether to verify task completion
    pub verify: bool,

    /// Maximum number of retries per task
    pub max_retries: u32,

    /// Maximum depth for task decomposition
    pub max_depth: u32,

    /// Timeout for planning stage (seconds)
    pub timeout_plan: u64,

    /// Timeout for execution stage (seconds)
    pub timeout_exec: u64,
}

impl Default for ModeConfig {
    fn default() -> Self {
        ModeConfig {
            model_fast: "claude-sonnet-4-6".to_string(),
            model_smart: "claude-opus-4-6".to_string(),
            run_tests: true,
            verify: true,
            max_retries: 3,
            max_depth: 3,
            timeout_plan: 120,
            timeout_exec: 3600,
        }
    }
}

impl ModeConfig {
    /// Returns the appropriate model for the given task complexity
    pub fn model_for_complexity(&self, complexity: &TaskComplexity) -> &str {
        match complexity {
            TaskComplexity::Simple => &self.model_fast,
            TaskComplexity::Moderate => &self.model_fast,
            TaskComplexity::Complex => &self.model_smart,
        }
    }

    /// Creates config for fast mode
    pub fn fast_mode() -> Self {
        ModeConfig {
            model_fast: "claude-haiku-4-5".to_string(),
            model_smart: "claude-sonnet-4-6".to_string(),
            run_tests: false,
            verify: true,
            max_retries: 1,
            max_depth: 2,
            timeout_plan: 60,
            timeout_exec: 1800,
        }
    }

    /// Creates config for expert mode
    pub fn expert_mode() -> Self {
        ModeConfig {
            model_fast: "claude-opus-4-6".to_string(),
            model_smart: "claude-opus-4-6".to_string(),
            run_tests: true,
            verify: true,
            max_retries: 3,
            max_depth: 3,
            timeout_plan: 180,
            timeout_exec: 7200,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = Task::new("task-1", "Test Task", "A test task description");
        assert_eq!(task.id, "task-1");
        assert_eq!(task.status, TaskStatus::Pending);
        assert_eq!(task.complexity, TaskComplexity::Moderate);
        assert!(task.depends_on.is_empty());
        assert!(task.subtasks.is_empty());
    }

    #[test]
    fn test_task_can_execute() {
        let mut task = Task::new("task-1", "Test", "Description");
        task.depends_on = vec!["task-0".to_string()];

        let mut completed = std::collections::HashSet::new();
        assert!(!task.can_execute(&completed));

        completed.insert("task-0".to_string());
        assert!(task.can_execute(&completed));
    }

    #[test]
    fn test_execution_mode_defaults() {
        assert_eq!(ExecutionMode::default(), ExecutionMode::Standard);
    }

    #[test]
    fn test_execution_mode_tests() {
        assert!(!ExecutionMode::Fast.run_tests());
        assert!(ExecutionMode::Standard.run_tests());
        assert!(ExecutionMode::Expert.run_tests());
    }

    #[test]
    fn test_pipeline_stage_display() {
        assert_eq!(PipelineStage::Generate.display_name(), "Generate");
        assert_eq!(PipelineStage::Execute.display_name(), "Execute");
    }

    #[test]
    fn test_mode_config_model_selection() {
        let config = ModeConfig::default();
        assert_eq!(
            config.model_for_complexity(&TaskComplexity::Simple),
            "claude-sonnet-4-6"
        );
        assert_eq!(
            config.model_for_complexity(&TaskComplexity::Complex),
            "claude-opus-4-6"
        );
    }
}
