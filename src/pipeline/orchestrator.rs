// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Pipeline orchestrator
//!
//! This module implements the core orchestration logic for the ltmatrix pipeline.
//! It coordinates all pipeline stages in order:
//! Generate → Assess → Execute → Test → Verify → Commit → Memory
//! (+ Review in expert mode)
//!
//! The orchestrator handles:
//! - Stage transitions and error propagation
//! - Parallel task execution respecting dependencies
//! - Mode-based stage skipping (Fast skips Test, Expert adds Review)
//! - Overall pipeline state tracking and progress reporting
//!
//! # Architecture
//!
//! The orchestrator uses a two-phase execution model:
//! 1. **Sequential Stage Execution**: Each pipeline stage runs in order
//! 2. **Parallel Task Execution**: Within stages, tasks execute in parallel based on dependencies
//!
//! # Example
//!
//! ```no_run
//! use ltmatrix::pipeline::orchestrator::{PipelineOrchestrator, OrchestratorConfig};
//! use ltmatrix::models::ExecutionMode;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let config = OrchestratorConfig::default();
//! let orchestrator = PipelineOrchestrator::new(config)?;
//!
//! let result = orchestrator
//!     .execute_pipeline("Build a REST API", ExecutionMode::Standard)
//!     .await?;
//!
//! println!("Pipeline completed: {} tasks", result.tasks_completed);
//! # Ok(())
//! # }
//! ```

use anyhow::{bail, Result};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument, warn};

use crate::pipeline::assess::{assess_tasks, AssessConfig};
use crate::pipeline::commit::{commit_tasks, CommitConfig};
use crate::pipeline::execute::{execute_tasks, ExecuteConfig};
use crate::pipeline::generate::{generate_tasks, GenerateConfig};
use crate::pipeline::memory::{update_memory, MemoryConfig};
use crate::pipeline::review::{review_tasks, ReviewConfig};
use crate::pipeline::test::{test_tasks, TestConfig};
use crate::pipeline::verify::{verify_tasks, VerifyConfig};
use crate::progress::live_display::{init_display, ProgressStats};
use crate::workspace::{RecoverySummary, WorkspaceState};
use ltmatrix_agent::AgentPool;
use ltmatrix_core::{ExecutionMode, ModeConfig, PipelineStage, Task};

/// Configuration for the pipeline orchestrator
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    /// Execution mode (Fast/Standard/Expert)
    pub execution_mode: ExecutionMode,

    /// Working directory for all operations
    pub work_dir: PathBuf,

    /// Maximum number of parallel tasks to execute
    pub max_parallel_tasks: usize,

    /// Whether to show progress bars
    pub show_progress: bool,

    /// Agent pool for session management
    pub agent_pool: Option<AgentPool>,

    /// Mode-specific configuration
    pub mode_config: ModeConfig,

    /// Generation stage configuration
    pub generate_config: GenerateConfig,

    /// Assess stage configuration
    pub assess_config: AssessConfig,

    /// Execute stage configuration
    pub execute_config: ExecuteConfig,

    /// Test stage configuration
    pub test_config: TestConfig,

    /// Verify stage configuration
    pub verify_config: VerifyConfig,

    /// Commit stage configuration
    pub commit_config: CommitConfig,

    /// Memory stage configuration
    pub memory_config: MemoryConfig,

    /// Review stage configuration (expert mode only)
    pub review_config: ReviewConfig,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        let execution_mode = ExecutionMode::Standard;
        let work_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        OrchestratorConfig {
            execution_mode,
            work_dir: work_dir.clone(),
            max_parallel_tasks: 4,
            show_progress: true,
            agent_pool: None,
            mode_config: ModeConfig::default(),
            generate_config: GenerateConfig::default(),
            assess_config: AssessConfig::default(),
            execute_config: ExecuteConfig::default(),
            test_config: TestConfig::default(),
            verify_config: VerifyConfig::default(),
            commit_config: CommitConfig::default(),
            memory_config: MemoryConfig::default(),
            review_config: ReviewConfig::default(),
        }
    }
}

impl OrchestratorConfig {
    /// Create config for fast mode
    pub fn fast_mode() -> Self {
        let mut config = Self::default();
        config.execution_mode = ExecutionMode::Fast;
        config.mode_config = ModeConfig::fast_mode();
        config.generate_config = GenerateConfig::fast_mode();
        config.assess_config = AssessConfig::fast_mode();
        config.execute_config = ExecuteConfig::fast_mode();
        config.test_config = TestConfig::fast_mode();
        config.verify_config = VerifyConfig::fast_mode();
        config
    }

    /// Create config for expert mode
    pub fn expert_mode() -> Self {
        let mut config = Self::default();
        config.execution_mode = ExecutionMode::Expert;
        config.mode_config = ModeConfig::expert_mode();
        config.generate_config = GenerateConfig::expert_mode();
        config.assess_config = AssessConfig::expert_mode();
        config.execute_config = ExecuteConfig::expert_mode();
        config.test_config = TestConfig::expert_mode();
        config.verify_config = VerifyConfig::expert_mode();
        config.review_config = ReviewConfig::expert_mode();
        config
    }

    /// Set the working directory
    pub fn with_work_dir(mut self, work_dir: impl Into<PathBuf>) -> Self {
        let work_dir = work_dir.into();
        self.work_dir = work_dir.clone();
        self.execute_config.work_dir = work_dir.clone();
        self.test_config.work_dir = work_dir.clone();
        self.verify_config.work_dir = work_dir.clone();
        self.commit_config.work_dir = work_dir.clone();
        self.memory_config.project_root = Some(work_dir.clone());
        self.review_config.work_dir = work_dir;
        self
    }

    /// Set the agent pool
    pub fn with_agent_pool(mut self, pool: AgentPool) -> Self {
        self.agent_pool = Some(pool.clone());
        self.execute_config.agent_pool = Some(pool);
        self
    }

    /// Enable or disable progress bars
    pub fn with_progress(mut self, show_progress: bool) -> Self {
        self.show_progress = show_progress;
        self
    }

    /// Set max parallel tasks
    pub fn with_max_parallel(mut self, max: usize) -> Self {
        self.max_parallel_tasks = max;
        self
    }

    /// Override the generation model from config
    pub fn with_generation_model(mut self, model: &str) -> Self {
        self.generate_config.generation_model = model.to_string();
        self
    }

    /// Override the assessment model from config
    pub fn with_assessment_model(mut self, model: &str) -> Self {
        self.assess_config.assessment_model = model.to_string();
        self
    }

    /// Use models from pipeline configuration
    pub fn with_pipeline_config(
        mut self,
        generation_model: &str,
        assessment_model: &str,
    ) -> Self {
        self.generate_config.generation_model = generation_model.to_string();
        self.assess_config.assessment_model = assessment_model.to_string();
        self
    }
}

/// Result of pipeline execution
#[derive(Debug, Clone)]
pub struct PipelineResult {
    /// Total number of tasks processed
    pub total_tasks: usize,

    /// Number of tasks completed successfully
    pub tasks_completed: usize,

    /// Number of tasks that failed
    pub tasks_failed: usize,

    /// Number of stages completed
    pub stages_completed: usize,

    /// Total execution time
    pub total_time: Duration,

    /// List of completed tasks
    pub completed_tasks: Vec<Task>,

    /// List of failed tasks
    pub failed_tasks: Vec<Task>,

    /// Whether the pipeline succeeded
    pub success: bool,
}

impl PipelineResult {
    /// Create a new pipeline result
    fn new() -> Self {
        PipelineResult {
            total_tasks: 0,
            tasks_completed: 0,
            tasks_failed: 0,
            stages_completed: 0,
            total_time: Duration::ZERO,
            completed_tasks: Vec::new(),
            failed_tasks: Vec::new(),
            success: false,
        }
    }

    /// Calculate success rate as a percentage
    pub fn success_rate(&self) -> f64 {
        if self.total_tasks == 0 {
            return 100.0;
        }
        (self.tasks_completed as f64 / self.total_tasks as f64) * 100.0
    }
}

/// Pipeline state tracking
#[derive(Debug)]
struct PipelineState {
    /// Current stage
    current_stage: Option<PipelineStage>,

    /// All tasks being managed
    tasks: Vec<Task>,

    /// Completed task IDs
    completed_tasks: HashSet<String>,

    /// Failed task IDs
    failed_tasks: HashSet<String>,

    /// Start time
    start_time: std::time::Instant,
}

impl PipelineState {
    fn new(_show_progress: bool) -> Self {
        PipelineState {
            current_stage: None,
            tasks: Vec::new(),
            completed_tasks: HashSet::new(),
            failed_tasks: HashSet::new(),
            start_time: std::time::Instant::now(),
        }
    }
}

/// Pipeline orchestrator
///
/// The orchestrator coordinates all pipeline stages and manages the overall
/// execution flow.
#[derive(Debug)]
pub struct PipelineOrchestrator {
    config: OrchestratorConfig,
    state: Arc<RwLock<PipelineState>>,
}

impl PipelineOrchestrator {
    /// Create a new pipeline orchestrator
    pub fn new(config: OrchestratorConfig) -> Result<Self> {
        // Validate working directory
        if !config.work_dir.exists() {
            bail!(
                "Working directory does not exist: {}",
                config.work_dir.display()
            );
        }

        Ok(PipelineOrchestrator {
            config,
            state: Arc::new(RwLock::new(PipelineState::new(true))),
        })
    }

    /// Execute the complete pipeline for a given goal
    ///
    /// This is the main entry point for pipeline execution. It runs all stages
    /// in sequence based on the execution mode.
    #[instrument(skip(self, goal), fields(goal = %goal.chars().take(50).collect::<String>()))]
    pub async fn execute_pipeline(
        &self,
        goal: &str,
        mode: ExecutionMode,
    ) -> Result<PipelineResult> {
        info!("Starting pipeline execution in {:?} mode", mode);
        info!("Goal: {}", goal);

        // Initialize live display
        let display = init_display(self.config.show_progress);
        display.start();
        display.info("pipeline", &format!("开始执行 - {:?} 模式", mode));

        let mut result = PipelineResult::new();
        let start_time = std::time::Instant::now();

        // Initialize state
        {
            let mut state = self.state.write().await;
            state.start_time = start_time;
            state.tasks.clear();
            state.completed_tasks.clear();
            state.failed_tasks.clear();
        }

        // Get the pipeline stages for this mode
        let stages = PipelineStage::pipeline_for_mode(mode);
        let total_stages = stages.len();

        // Execute each stage in sequence
        for (stage_index, stage) in stages.iter().enumerate() {
            self.set_current_stage(*stage).await;

            // Update live display with current stage
            display.update_stats(ProgressStats {
                stage: stage.display_name().to_string(),
                stage_index,
                total_stages,
                total_tasks: result.total_tasks,
                completed_tasks: result.tasks_completed,
                failed_tasks: result.tasks_failed,
                current_task: None,
            });
            display.info("pipeline", &format!("执行阶段: {}", stage.display_name()));

            match self.execute_stage(goal, *stage, stage_index).await {
                Ok(stage_result) => {
                    result.stages_completed = stage_index + 1;
                    result.total_tasks = stage_result.len();

                    // Update task results
                    for task in &stage_result {
                        if task.is_completed() {
                            result.tasks_completed += 1;
                            result.completed_tasks.push(task.clone());
                        } else if task.is_failed() {
                            result.tasks_failed += 1;
                            result.failed_tasks.push(task.clone());
                        }
                    }

                    display.success("pipeline", &format!("阶段完成: {} ({} 任务)", stage.display_name(), stage_result.len()));
                    info!("Stage {:?} completed: {} tasks", stage, stage_result.len());
                }
                Err(e) => {
                    error!("Stage {:?} failed: {}", stage, e);
                    display.error("pipeline", &format!("阶段失败: {} - {}", stage.display_name(), e));
                    result.total_time = start_time.elapsed();
                    result.success = false;

                    // Show final summary
                    let summary = format!(
                        "Pipeline 失败于阶段: {}\n错误: {}",
                        stage.display_name(), e
                    );
                    display.finish(false, &summary);
                    return Ok(result);
                }
            }
        }

        result.total_time = start_time.elapsed();
        result.success = result.tasks_failed == 0;

        info!(
            "Pipeline completed: {} tasks completed, {} failed in {:.2}s",
            result.tasks_completed,
            result.tasks_failed,
            result.total_time.as_secs_f64()
        );

        // Show final summary
        let summary = format!(
            "Pipeline 执行摘要:\n  总任务: {}\n  完成: {}\n  失败: {}\n  阶段完成: {}\n  成功率: {:.1}%\n  总耗时: {:.2}s",
            result.total_tasks,
            result.tasks_completed,
            result.tasks_failed,
            result.stages_completed,
            result.success_rate(),
            result.total_time.as_secs_f64()
        );
        display.finish(result.success, &summary);

        Ok(result)
    }

    /// Execute a single pipeline stage
    async fn execute_stage(
        &self,
        goal: &str,
        stage: PipelineStage,
        _stage_index: usize,
    ) -> Result<Vec<Task>> {
        info!("Executing stage: {:?}", stage);

        let result = match stage {
            PipelineStage::Generate => self.execute_generate_stage(goal).await?,
            PipelineStage::Assess => {
                let tasks = self.get_current_tasks().await?;
                self.execute_assess_stage(tasks).await?
            }
            PipelineStage::Execute => {
                let tasks = self.get_current_tasks().await?;
                self.execute_execute_stage(tasks).await?
            }
            PipelineStage::Test => {
                let tasks = self.get_completed_tasks().await?;
                self.execute_test_stage(tasks).await?
            }
            PipelineStage::Review => {
                let tasks = self.get_completed_tasks().await?;
                self.execute_review_stage(tasks).await?
            }
            PipelineStage::Verify => {
                let tasks = self.get_completed_tasks().await?;
                self.execute_verify_stage(tasks).await?
            }
            PipelineStage::Commit => {
                let tasks = self.get_completed_tasks().await?;
                self.execute_commit_stage(tasks).await?
            }
            PipelineStage::Memory => {
                let tasks = self.get_completed_tasks().await?;
                self.execute_memory_stage(tasks).await?
            }
        };

        // Update state with new tasks
        self.update_tasks(result.clone()).await?;

        Ok(result)
    }

    /// Execute the Generate stage
    async fn execute_generate_stage(
        &self,
        goal: &str,
    ) -> Result<Vec<Task>> {
        let generation_result = generate_tasks(goal, &self.config.generate_config).await?;

        info!("Generated {} tasks from goal", generation_result.task_count);
        Ok(generation_result.tasks)
    }

    /// Execute the Assess stage
    async fn execute_assess_stage(
        &self,
        tasks: Vec<Task>,
    ) -> Result<Vec<Task>> {
        let assessed_tasks = assess_tasks(tasks, &self.config.assess_config).await?;

        Ok(assessed_tasks)
    }

    /// Execute the Execute stage with parallel task execution
    async fn execute_execute_stage(
        &self,
        tasks: Vec<Task>,
    ) -> Result<Vec<Task>> {
        if tasks.is_empty() {
            return Ok(tasks);
        }

        let (executed_tasks, _stats) = execute_tasks(tasks, &self.config.execute_config).await?;

        Ok(executed_tasks)
    }

    /// Execute the Test stage (skipped in Fast mode)
    async fn execute_test_stage(
        &self,
        tasks: Vec<Task>,
    ) -> Result<Vec<Task>> {
        if !self.config.execution_mode.run_tests() {
            debug!("Test stage skipped in Fast mode");
            return Ok(tasks);
        }

        if tasks.is_empty() {
            return Ok(tasks);
        }

        let tested_tasks = test_tasks(tasks, &self.config.test_config).await?;

        Ok(tested_tasks)
    }

    /// Execute the Review stage (expert mode only)
    async fn execute_review_stage(
        &self,
        tasks: Vec<Task>,
    ) -> Result<Vec<Task>> {
        if !self.config.review_config.should_run() {
            debug!("Review stage skipped: only runs in expert mode with verify enabled");
            return Ok(tasks);
        }

        if tasks.is_empty() {
            return Ok(tasks);
        }

        let (reviewed_tasks, review_summary) =
            review_tasks(tasks, &self.config.review_config).await?;

        // Count blocking issues
        let blocking_count = review_summary
            .all_issues
            .iter()
            .filter(|issue| issue.blocking)
            .count();

        // Log review summary
        info!(
            "Review completed: {} tasks assessed, {} blocking issues found",
            review_summary.total_tasks, blocking_count
        );

        // If there are critical issues that block the pipeline, log them
        if blocking_count > 0 {
            warn!(
                "Found {} blocking issues that must be addressed:",
                blocking_count
            );

            // Group blocking issues by category for better reporting
            use std::collections::HashMap;
            let mut blocking_by_category: HashMap<
                crate::pipeline::review::IssueCategory,
                Vec<&crate::pipeline::review::CodeIssue>,
            > = HashMap::new();

            for issue in &review_summary.all_issues {
                if issue.blocking {
                    blocking_by_category
                        .entry(issue.category)
                        .or_insert_with(Vec::new)
                        .push(issue);
                }
            }

            for (category, issues) in blocking_by_category {
                warn!("  {:?}: {} blocking issues", category, issues.len());
                for issue in issues.iter().take(5) {
                    warn!("    - {}: {}", issue.title, issue.description);
                }
                if issues.len() > 5 {
                    warn!("    ... and {} more", issues.len() - 5);
                }
            }
        }

        Ok(reviewed_tasks)
    }

    /// Execute the Verify stage
    async fn execute_verify_stage(
        &self,
        tasks: Vec<Task>,
    ) -> Result<Vec<Task>> {
        if tasks.is_empty() {
            return Ok(tasks);
        }

        let (verified_tasks, _summary) = verify_tasks(tasks, &self.config.verify_config).await?;

        Ok(verified_tasks)
    }

    /// Execute the Commit stage
    async fn execute_commit_stage(
        &self,
        tasks: Vec<Task>,
    ) -> Result<Vec<Task>> {
        if tasks.is_empty() {
            return Ok(tasks);
        }

        let (committed_tasks, _summary) = commit_tasks(tasks, &self.config.commit_config).await?;

        Ok(committed_tasks)
    }

    /// Execute the Memory stage
    async fn execute_memory_stage(
        &self,
        tasks: Vec<Task>,
    ) -> Result<Vec<Task>> {
        if tasks.is_empty() {
            return Ok(tasks);
        }

        update_memory(&tasks, &self.config.memory_config).await?;

        Ok(tasks)
    }

    /// Get current tasks from state
    async fn get_current_tasks(&self) -> Result<Vec<Task>> {
        let state = self.state.read().await;
        Ok(state.tasks.clone())
    }

    /// Get completed tasks from state
    async fn get_completed_tasks(&self) -> Result<Vec<Task>> {
        let state = self.state.read().await;
        Ok(state
            .tasks
            .iter()
            .filter(|t| t.is_completed())
            .cloned()
            .collect())
    }

    /// Update tasks in state
    async fn update_tasks(&self, tasks: Vec<Task>) -> Result<()> {
        let mut state = self.state.write().await;
        state.tasks = tasks;
        Ok(())
    }

    /// Set the current stage
    async fn set_current_stage(&self, stage: PipelineStage) {
        let mut state = self.state.write().await;
        state.current_stage = Some(stage);
    }

    /// Check for recovery opportunity at startup
    ///
    /// This method checks if there are incomplete tasks from a previous run
    /// that could be resumed.
    ///
    /// # Returns
    ///
    /// Returns `Some(RecoverySummary)` if there are incomplete tasks, `None` otherwise.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ltmatrix::pipeline::orchestrator::{PipelineOrchestrator, OrchestratorConfig};
    /// use std::path::PathBuf;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let config = OrchestratorConfig::default()
    ///     .with_work_dir("/path/to/project");
    /// let orchestrator = PipelineOrchestrator::new(config)?;
    ///
    /// if let Some(recovery) = orchestrator.check_recovery().await? {
    ///     println!("Found {} incomplete tasks. Resume?", recovery.total_incomplete);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn check_recovery(&self) -> Result<Option<RecoverySummary>> {
        if !WorkspaceState::exists(&self.config.work_dir) {
            debug!("No existing workspace state found, starting fresh");
            return Ok(None);
        }

        // Load the existing workspace state
        match WorkspaceState::load_with_transform(self.config.work_dir.clone()) {
            Ok(state) => {
                let summary = state.get_recovery_summary();
                if summary.can_resume {
                    info!(
                        "Found {} incomplete tasks from previous run ({} pending, {} in-progress, {} failed, {} blocked)",
                        summary.total_incomplete,
                        summary.pending_count,
                        summary.in_progress_count,
                        summary.failed_count,
                        summary.blocked_count
                    );
                    return Ok(Some(summary));
                }
                Ok(None)
            }
            Err(e) => {
                warn!("Failed to load workspace state for recovery check: {}", e);
                Ok(None)
            }
        }
    }

    /// Resume pipeline execution from a previous interrupted run
    ///
    /// This method loads the existing workspace state and continues execution
    /// from where it left off. In-progress tasks are reset to Pending.
    ///
    /// # Returns
    ///
    /// Returns `Ok(PipelineResult)` with the results of the resumed pipeline execution.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No existing workspace state exists
    /// - The workspace state cannot be loaded
    /// - Pipeline execution fails
    pub async fn resume_pipeline(&self) -> Result<PipelineResult> {
        info!("Resuming pipeline from previous run");

        // Load the workspace state with transformation (resets InProgress to Pending)
        let state = WorkspaceState::load_with_transform(self.config.work_dir.clone())?;

        let recovery = state.get_recovery_summary();
        if !recovery.can_resume {
            info!("No incomplete tasks found, nothing to resume");
            return Ok(PipelineResult::new());
        }

        info!("Resuming {} incomplete tasks", recovery.total_incomplete);

        // Initialize state with loaded tasks
        {
            let mut pipeline_state = self.state.write().await;
            pipeline_state.tasks = state.tasks.clone();
            pipeline_state.start_time = std::time::Instant::now();
        }

        // Get the pipeline stages for this mode
        let stages = PipelineStage::pipeline_for_mode(self.config.execution_mode);

        // Start from Assess stage (tasks already generated)
        let result = self.execute_from_stage(stages[1]).await?;

        Ok(result)
    }

    /// Execute pipeline starting from a specific stage
    ///
    /// This is used for resuming execution after recovery.
    async fn execute_from_stage(&self, start_stage: PipelineStage) -> Result<PipelineResult> {
        let mut result = PipelineResult::new();
        let start_time = std::time::Instant::now();

        // Get the pipeline stages for this mode
        let stages = PipelineStage::pipeline_for_mode(self.config.execution_mode);

        // Find the starting stage index
        let start_index = stages.iter().position(|&s| s == start_stage).unwrap_or(0);

        // Execute stages starting from the specified stage
        for (stage_index, stage) in stages.iter().enumerate().skip(start_index) {
            self.set_current_stage(*stage).await;

            match self.execute_stage_from_state(*stage, stage_index).await {
                Ok(stage_result) => {
                    result.stages_completed = stage_index + 1;
                    result.total_tasks = stage_result.len();

                    // Update task results
                    for task in &stage_result {
                        if task.is_completed() {
                            result.tasks_completed += 1;
                            result.completed_tasks.push(task.clone());
                        } else if task.is_failed() {
                            result.tasks_failed += 1;
                            result.failed_tasks.push(task.clone());
                        }
                    }

                    info!("Stage {:?} completed: {} tasks", stage, stage_result.len());
                }
                Err(e) => {
                    error!("Stage {:?} failed: {}", stage, e);
                    result.total_time = start_time.elapsed();
                    result.success = false;
                    return Ok(result);
                }
            }
        }

        result.total_time = start_time.elapsed();
        result.success = result.tasks_failed == 0;

        info!(
            "Pipeline completed: {} tasks completed, {} failed in {:.2}s",
            result.tasks_completed,
            result.tasks_failed,
            result.total_time.as_secs_f64()
        );

        Ok(result)
    }

    /// Execute a stage using the current state
    async fn execute_stage_from_state(
        &self,
        stage: PipelineStage,
        _stage_index: usize,
    ) -> Result<Vec<Task>> {
        info!("Executing stage: {:?}", stage);

        let tasks = self.get_current_tasks().await?;
        let result = match stage {
            PipelineStage::Generate => {
                // For resume, we skip generation and use existing tasks
                return Ok(tasks);
            }
            PipelineStage::Assess => self.execute_assess_stage(tasks).await?,
            PipelineStage::Execute => self.execute_execute_stage(tasks).await?,
            PipelineStage::Test => {
                let completed = self.get_completed_tasks().await?;
                self.execute_test_stage(completed).await?
            }
            PipelineStage::Review => {
                let completed = self.get_completed_tasks().await?;
                self.execute_review_stage(completed).await?
            }
            PipelineStage::Verify => {
                let completed = self.get_completed_tasks().await?;
                self.execute_verify_stage(completed).await?
            }
            PipelineStage::Commit => {
                let completed = self.get_completed_tasks().await?;
                self.execute_commit_stage(completed).await?
            }
            PipelineStage::Memory => {
                let completed = self.get_completed_tasks().await?;
                self.execute_memory_stage(completed).await?
            }
        };

        // Update state with new tasks
        self.update_tasks(result.clone()).await?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_orchestrator_config_default() {
        let config = OrchestratorConfig::default();
        assert_eq!(config.execution_mode, ExecutionMode::Standard);
        assert_eq!(config.max_parallel_tasks, 4);
        assert!(config.show_progress);
    }

    #[test]
    fn test_orchestrator_config_fast_mode() {
        let config = OrchestratorConfig::fast_mode();
        assert_eq!(config.execution_mode, ExecutionMode::Fast);
        assert_eq!(config.mode_config.max_retries, 1);
    }

    #[test]
    fn test_orchestrator_config_expert_mode() {
        let config = OrchestratorConfig::expert_mode();
        assert_eq!(config.execution_mode, ExecutionMode::Expert);
        assert_eq!(config.mode_config.max_retries, 3);
    }

    #[test]
    fn test_orchestrator_config_builder() {
        let temp_dir = TempDir::new().unwrap();
        let config = OrchestratorConfig::default()
            .with_work_dir(temp_dir.path())
            .with_max_parallel(8)
            .with_progress(false);

        assert_eq!(config.work_dir, temp_dir.path());
        assert_eq!(config.max_parallel_tasks, 8);
        assert!(!config.show_progress);
    }

    #[test]
    fn test_pipeline_result_new() {
        let result = PipelineResult::new();
        assert_eq!(result.total_tasks, 0);
        assert_eq!(result.tasks_completed, 0);
        assert_eq!(result.tasks_failed, 0);
        assert!(!result.success);
    }

    #[test]
    fn test_pipeline_result_success_rate() {
        let mut result = PipelineResult::new();
        assert_eq!(result.success_rate(), 100.0);

        result.total_tasks = 10;
        result.tasks_completed = 8;
        assert_eq!(result.success_rate(), 80.0);
    }

    #[tokio::test]
    async fn test_orchestrator_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = OrchestratorConfig::default().with_work_dir(temp_dir.path());

        let orchestrator = PipelineOrchestrator::new(config);
        assert!(orchestrator.is_ok());
    }

    #[tokio::test]
    async fn test_orchestrator_invalid_work_dir() {
        let config = OrchestratorConfig::default().with_work_dir("/nonexistent/path");

        let orchestrator = PipelineOrchestrator::new(config);
        assert!(orchestrator.is_err());
    }
}
