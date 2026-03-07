//! Integration tests for execution modes (Fast, Standard, Expert)
//!
//! These tests verify:
//! - Pipeline stage execution for each mode
//! - Model selection based on mode and task complexity
//! - Test execution behavior (skipped in fast mode)
//! - Code review execution (expert mode only)
//! - Mode-specific configuration application
//! - CLI flag to mode mapping

use ltmatrix::models::{
    ExecutionMode, ModeConfig, PipelineStage, Task, TaskComplexity, TaskStatus,
};
use ltmatrix::pipeline::orchestrator::{OrchestratorConfig, PipelineOrchestrator};
use ltmatrix::cli::args::ExecutionModeArg;
use clap::Parser;
use ltmatrix::cli::Args;
use tempfile::TempDir;

// =============================================================================
// Helper Functions
// =============================================================================

/// Create a sample task with given complexity
fn create_task_with_complexity(id: &str, title: &str, complexity: TaskComplexity) -> Task {
    let mut task = Task::new(id, title, format!("Description for {}", title));
    task.complexity = complexity;
    task
}

// =============================================================================
// Pipeline Stage Tests
// =============================================================================

/// Test Fast mode pipeline stages
///
/// Fast mode should:
/// - Skip Test stage
/// - Skip Review stage
/// - Execute: Generate → Assess → Execute → Verify → Commit → Memory
#[test]
fn test_fast_mode_pipeline_stages() {
    let stages = PipelineStage::pipeline_for_mode(ExecutionMode::Fast);

    // Verify stage count (6 stages, no Test or Review)
    assert_eq!(stages.len(), 6, "Fast mode should have 6 stages");

    // Verify Test stage is NOT present
    assert!(
        !stages.contains(&PipelineStage::Test),
        "Fast mode should NOT include Test stage"
    );

    // Verify Review stage is NOT present
    assert!(
        !stages.contains(&PipelineStage::Review),
        "Fast mode should NOT include Review stage"
    );

    // Verify all expected stages are present
    assert!(stages.contains(&PipelineStage::Generate), "Should include Generate");
    assert!(stages.contains(&PipelineStage::Assess), "Should include Assess");
    assert!(stages.contains(&PipelineStage::Execute), "Should include Execute");
    assert!(stages.contains(&PipelineStage::Verify), "Should include Verify");
    assert!(stages.contains(&PipelineStage::Commit), "Should include Commit");
    assert!(stages.contains(&PipelineStage::Memory), "Should include Memory");
}

/// Test Standard mode pipeline stages
///
/// Standard mode should:
/// - Include Test stage
/// - Skip Review stage
/// - Execute: Generate → Assess → Execute → Test → Verify → Commit → Memory
#[test]
fn test_standard_mode_pipeline_stages() {
    let stages = PipelineStage::pipeline_for_mode(ExecutionMode::Standard);

    // Verify stage count (7 stages, no Review)
    assert_eq!(stages.len(), 7, "Standard mode should have 7 stages");

    // Verify Test stage IS present
    assert!(
        stages.contains(&PipelineStage::Test),
        "Standard mode should include Test stage"
    );

    // Verify Review stage is NOT present
    assert!(
        !stages.contains(&PipelineStage::Review),
        "Standard mode should NOT include Review stage"
    );

    // Verify all expected stages are present
    assert!(stages.contains(&PipelineStage::Generate), "Should include Generate");
    assert!(stages.contains(&PipelineStage::Assess), "Should include Assess");
    assert!(stages.contains(&PipelineStage::Execute), "Should include Execute");
    assert!(stages.contains(&PipelineStage::Verify), "Should include Verify");
    assert!(stages.contains(&PipelineStage::Commit), "Should include Commit");
    assert!(stages.contains(&PipelineStage::Memory), "Should include Memory");
}

/// Test Expert mode pipeline stages
///
/// Expert mode should:
/// - Include Test stage
/// - Include Review stage (between Test and Verify)
/// - Execute: Generate → Assess → Execute → Test → Review → Verify → Commit → Memory
#[test]
fn test_expert_mode_pipeline_stages() {
    let stages = PipelineStage::pipeline_for_mode(ExecutionMode::Expert);

    // Verify stage count (8 stages, including Review)
    assert_eq!(stages.len(), 8, "Expert mode should have 8 stages");

    // Verify Test stage IS present
    assert!(
        stages.contains(&PipelineStage::Test),
        "Expert mode should include Test stage"
    );

    // Verify Review stage IS present
    assert!(
        stages.contains(&PipelineStage::Review),
        "Expert mode should include Review stage"
    );

    // Verify all expected stages are present
    assert!(stages.contains(&PipelineStage::Generate), "Should include Generate");
    assert!(stages.contains(&PipelineStage::Assess), "Should include Assess");
    assert!(stages.contains(&PipelineStage::Execute), "Should include Execute");
    assert!(stages.contains(&PipelineStage::Verify), "Should include Verify");
    assert!(stages.contains(&PipelineStage::Commit), "Should include Commit");
    assert!(stages.contains(&PipelineStage::Memory), "Should include Memory");
}

/// Test that Review stage is positioned between Test and Verify in Expert mode
#[test]
fn test_expert_mode_review_stage_position() {
    let stages = PipelineStage::pipeline_for_mode(ExecutionMode::Expert);

    let test_idx = stages.iter().position(|s| *s == PipelineStage::Test).unwrap();
    let review_idx = stages.iter().position(|s| *s == PipelineStage::Review).unwrap();
    let verify_idx = stages.iter().position(|s| *s == PipelineStage::Verify).unwrap();

    assert!(
        review_idx > test_idx,
        "Review should come after Test (Test at {}, Review at {})",
        test_idx,
        review_idx
    );

    assert!(
        review_idx < verify_idx,
        "Review should come before Verify (Review at {}, Verify at {})",
        review_idx,
        verify_idx
    );
}

/// Test that stage order is consistent across modes for common stages
#[test]
fn test_consistent_stage_order_across_modes() {
    let fast_stages = PipelineStage::pipeline_for_mode(ExecutionMode::Fast);
    let standard_stages = PipelineStage::pipeline_for_mode(ExecutionMode::Standard);
    let expert_stages = PipelineStage::pipeline_for_mode(ExecutionMode::Expert);

    // Helper to get position of a stage
    fn get_position(stages: &[PipelineStage], target: PipelineStage) -> Option<usize> {
        stages.iter().position(|s| *s == target)
    }

    // Generate should always be first
    assert_eq!(get_position(&fast_stages, PipelineStage::Generate), Some(0));
    assert_eq!(get_position(&standard_stages, PipelineStage::Generate), Some(0));
    assert_eq!(get_position(&expert_stages, PipelineStage::Generate), Some(0));

    // Assess should always be second
    assert_eq!(get_position(&fast_stages, PipelineStage::Assess), Some(1));
    assert_eq!(get_position(&standard_stages, PipelineStage::Assess), Some(1));
    assert_eq!(get_position(&expert_stages, PipelineStage::Assess), Some(1));

    // Execute should always be third
    assert_eq!(get_position(&fast_stages, PipelineStage::Execute), Some(2));
    assert_eq!(get_position(&standard_stages, PipelineStage::Execute), Some(2));
    assert_eq!(get_position(&expert_stages, PipelineStage::Execute), Some(2));

    // Memory should always be last
    assert_eq!(get_position(&fast_stages, PipelineStage::Memory), Some(5));
    assert_eq!(get_position(&standard_stages, PipelineStage::Memory), Some(6));
    assert_eq!(get_position(&expert_stages, PipelineStage::Memory), Some(7));
}

// =============================================================================
// Model Selection Tests
// =============================================================================

/// Test Fast mode default model selection
#[test]
fn test_fast_mode_default_model() {
    let model = ExecutionMode::Fast.default_model();

    assert_eq!(
        model, "claude-haiku-4-5",
        "Fast mode should use Haiku by default"
    );
}

/// Test Standard mode default model selection
#[test]
fn test_standard_mode_default_model() {
    let model = ExecutionMode::Standard.default_model();

    assert_eq!(
        model, "claude-sonnet-4-6",
        "Standard mode should use Sonnet by default"
    );
}

/// Test Expert mode default model selection
#[test]
fn test_expert_mode_default_model() {
    let model = ExecutionMode::Expert.default_model();

    assert_eq!(
        model, "claude-opus-4-6",
        "Expert mode should use Opus by default"
    );
}

/// Test ModeConfig model selection for different task complexities in Fast mode
#[test]
fn test_fast_mode_model_for_complexity() {
    let config = ModeConfig::fast_mode();

    // Simple and Moderate tasks use model_fast (Haiku)
    assert_eq!(
        config.model_for_complexity(&TaskComplexity::Simple),
        "claude-haiku-4-5",
        "Simple tasks should use fast model"
    );

    assert_eq!(
        config.model_for_complexity(&TaskComplexity::Moderate),
        "claude-haiku-4-5",
        "Moderate tasks should use fast model"
    );

    // Complex tasks use model_smart (Sonnet)
    assert_eq!(
        config.model_for_complexity(&TaskComplexity::Complex),
        "claude-sonnet-4-6",
        "Complex tasks should use smart model"
    );
}

/// Test ModeConfig model selection for different task complexities in Standard mode
#[test]
fn test_standard_mode_model_for_complexity() {
    let config = ModeConfig::default(); // Standard mode is default

    // Simple and Moderate tasks use model_fast (Sonnet)
    assert_eq!(
        config.model_for_complexity(&TaskComplexity::Simple),
        "claude-sonnet-4-6",
        "Simple tasks should use fast model"
    );

    assert_eq!(
        config.model_for_complexity(&TaskComplexity::Moderate),
        "claude-sonnet-4-6",
        "Moderate tasks should use fast model"
    );

    // Complex tasks use model_smart (Opus)
    assert_eq!(
        config.model_for_complexity(&TaskComplexity::Complex),
        "claude-opus-4-6",
        "Complex tasks should use smart model"
    );
}

/// Test ModeConfig model selection for different task complexities in Expert mode
#[test]
fn test_expert_mode_model_for_complexity() {
    let config = ModeConfig::expert_mode();

    // In Expert mode, both fast and smart models are Opus
    assert_eq!(
        config.model_for_complexity(&TaskComplexity::Simple),
        "claude-opus-4-6",
        "Simple tasks should use Opus in expert mode"
    );

    assert_eq!(
        config.model_for_complexity(&TaskComplexity::Moderate),
        "claude-opus-4-6",
        "Moderate tasks should use Opus in expert mode"
    );

    assert_eq!(
        config.model_for_complexity(&TaskComplexity::Complex),
        "claude-opus-4-6",
        "Complex tasks should use Opus in expert mode"
    );
}

// =============================================================================
// Test Execution Behavior Tests
// =============================================================================

/// Test that Fast mode skips tests
#[test]
fn test_fast_mode_skips_tests() {
    // Verify run_tests() returns false for Fast mode
    assert!(
        !ExecutionMode::Fast.run_tests(),
        "Fast mode should skip tests"
    );

    // Verify ModeConfig reflects this
    let config = ModeConfig::fast_mode();
    assert!(
        !config.run_tests,
        "Fast mode config should have run_tests=false"
    );
}

/// Test that Standard mode runs tests
#[test]
fn test_standard_mode_runs_tests() {
    // Verify run_tests() returns true for Standard mode
    assert!(
        ExecutionMode::Standard.run_tests(),
        "Standard mode should run tests"
    );

    // Verify ModeConfig reflects this
    let config = ModeConfig::default();
    assert!(
        config.run_tests,
        "Standard mode config should have run_tests=true"
    );
}

/// Test that Expert mode runs tests
#[test]
fn test_expert_mode_runs_tests() {
    // Verify run_tests() returns true for Expert mode
    assert!(
        ExecutionMode::Expert.run_tests(),
        "Expert mode should run tests"
    );

    // Verify ModeConfig reflects this
    let config = ModeConfig::expert_mode();
    assert!(
        config.run_tests,
        "Expert mode config should have run_tests=true"
    );
}

// =============================================================================
// Code Review Tests
// =============================================================================

/// Test that only Expert mode has Code Review stage
#[test]
fn test_code_review_only_in_expert_mode() {
    let fast_stages = PipelineStage::pipeline_for_mode(ExecutionMode::Fast);
    let standard_stages = PipelineStage::pipeline_for_mode(ExecutionMode::Standard);
    let expert_stages = PipelineStage::pipeline_for_mode(ExecutionMode::Expert);

    // Fast mode should not have Review
    assert!(
        !fast_stages.contains(&PipelineStage::Review),
        "Fast mode should not have Code Review"
    );

    // Standard mode should not have Review
    assert!(
        !standard_stages.contains(&PipelineStage::Review),
        "Standard mode should not have Code Review"
    );

    // Expert mode should have Review
    assert!(
        expert_stages.contains(&PipelineStage::Review),
        "Expert mode should have Code Review"
    );
}

/// Test that Code Review requires agent interaction
#[test]
fn test_code_review_requires_agent() {
    assert!(
        PipelineStage::Review.requires_agent(),
        "Code Review stage should require agent interaction"
    );
}

// =============================================================================
// Mode Configuration Tests
// =============================================================================

/// Test Fast mode configuration
#[test]
fn test_fast_mode_config() {
    let config = ModeConfig::fast_mode();

    assert_eq!(config.model_fast, "claude-haiku-4-5");
    assert_eq!(config.model_smart, "claude-sonnet-4-6");
    assert!(!config.run_tests);
    assert!(config.verify);
    assert_eq!(config.max_retries, 1);
    assert_eq!(config.max_depth, 2);
    assert_eq!(config.timeout_plan, 60);
    assert_eq!(config.timeout_exec, 1800);
}

/// Test Standard mode configuration
#[test]
fn test_standard_mode_config() {
    let config = ModeConfig::default(); // Standard is default

    assert_eq!(config.model_fast, "claude-sonnet-4-6");
    assert_eq!(config.model_smart, "claude-opus-4-6");
    assert!(config.run_tests);
    assert!(config.verify);
    assert_eq!(config.max_retries, 3);
    assert_eq!(config.max_depth, 3);
    assert_eq!(config.timeout_plan, 120);
    assert_eq!(config.timeout_exec, 3600);
}

/// Test Expert mode configuration
#[test]
fn test_expert_mode_config() {
    let config = ModeConfig::expert_mode();

    assert_eq!(config.model_fast, "claude-opus-4-6");
    assert_eq!(config.model_smart, "claude-opus-4-6");
    assert!(config.run_tests);
    assert!(config.verify);
    assert_eq!(config.max_retries, 3);
    assert_eq!(config.max_depth, 3);
    assert_eq!(config.timeout_plan, 180);
    assert_eq!(config.timeout_exec, 7200);
}

/// Test max_depth varies by mode
#[test]
fn test_max_depth_by_mode() {
    assert_eq!(
        ExecutionMode::Fast.max_depth(),
        2,
        "Fast mode should have max_depth=2"
    );

    assert_eq!(
        ExecutionMode::Standard.max_depth(),
        3,
        "Standard mode should have max_depth=3"
    );

    assert_eq!(
        ExecutionMode::Expert.max_depth(),
        3,
        "Expert mode should have max_depth=3"
    );
}

/// Test max_retries varies by mode
#[test]
fn test_max_retries_by_mode() {
    assert_eq!(
        ExecutionMode::Fast.max_retries(),
        1,
        "Fast mode should have max_retries=1"
    );

    assert_eq!(
        ExecutionMode::Standard.max_retries(),
        3,
        "Standard mode should have max_retries=3"
    );

    assert_eq!(
        ExecutionMode::Expert.max_retries(),
        3,
        "Expert mode should have max_retries=3"
    );
}

// =============================================================================
// CLI Flag Mapping Tests
// =============================================================================

/// Test --fast flag maps to Fast mode
#[test]
fn test_fast_flag_maps_to_fast_mode() {
    let args = Args::try_parse_from(["ltmatrix", "--fast", "test goal"])
        .expect("Should parse --fast flag");

    assert_eq!(
        args.get_execution_mode(),
        ExecutionModeArg::Fast,
        "--fast flag should map to Fast mode"
    );
}

/// Test --expert flag maps to Expert mode
#[test]
fn test_expert_flag_maps_to_expert_mode() {
    let args = Args::try_parse_from(["ltmatrix", "--expert", "test goal"])
        .expect("Should parse --expert flag");

    assert_eq!(
        args.get_execution_mode(),
        ExecutionModeArg::Expert,
        "--expert flag should map to Expert mode"
    );
}

/// Test default mode (no flags) is Standard
#[test]
fn test_default_mode_is_standard() {
    let args = Args::try_parse_from(["ltmatrix", "test goal"])
        .expect("Should parse without mode flags");

    assert_eq!(
        args.get_execution_mode(),
        ExecutionModeArg::Standard,
        "Default mode should be Standard"
    );
}

/// Test --fast and --expert flags conflict
#[test]
fn test_fast_and_expert_conflict() {
    let result = Args::try_parse_from(["ltmatrix", "--fast", "--expert", "test goal"]);

    assert!(
        result.is_err(),
        "--fast and --expert should conflict"
    );
}

// =============================================================================
// Execution Mode Display Tests
// =============================================================================

/// Test ExecutionMode Display trait
#[test]
fn test_execution_mode_display() {
    assert_eq!(ExecutionMode::Fast.to_string(), "fast");
    assert_eq!(ExecutionMode::Standard.to_string(), "standard");
    assert_eq!(ExecutionMode::Expert.to_string(), "expert");
}

/// Test ExecutionMode Default trait
#[test]
fn test_execution_mode_default() {
    assert_eq!(
        ExecutionMode::default(),
        ExecutionMode::Standard,
        "Default execution mode should be Standard"
    );
}

// =============================================================================
// Pipeline Stage Display Tests
// =============================================================================

/// Test PipelineStage display names
#[test]
fn test_pipeline_stage_display_names() {
    assert_eq!(PipelineStage::Generate.display_name(), "Generate");
    assert_eq!(PipelineStage::Assess.display_name(), "Assess");
    assert_eq!(PipelineStage::Execute.display_name(), "Execute");
    assert_eq!(PipelineStage::Test.display_name(), "Test");
    assert_eq!(PipelineStage::Review.display_name(), "Code Review");
    assert_eq!(PipelineStage::Verify.display_name(), "Verify");
    assert_eq!(PipelineStage::Commit.display_name(), "Commit");
    assert_eq!(PipelineStage::Memory.display_name(), "Memory");
}

/// Test PipelineStage requires_agent method
#[test]
fn test_pipeline_stage_requires_agent() {
    // Stages requiring agent
    assert!(PipelineStage::Generate.requires_agent());
    assert!(PipelineStage::Assess.requires_agent());
    assert!(PipelineStage::Execute.requires_agent());
    assert!(PipelineStage::Test.requires_agent());
    assert!(PipelineStage::Review.requires_agent());
    assert!(PipelineStage::Verify.requires_agent());

    // Stages NOT requiring agent
    assert!(!PipelineStage::Commit.requires_agent());
    assert!(!PipelineStage::Memory.requires_agent());
}

// =============================================================================
// Orchestrator Integration Tests
// =============================================================================

/// Test orchestrator with Fast mode configuration
#[tokio::test]
async fn test_orchestrator_fast_mode() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    let config = OrchestratorConfig::fast_mode()
        .with_work_dir(temp_dir.path())
        .with_progress(false);

    let orchestrator = PipelineOrchestrator::new(config)
        .expect("Failed to create orchestrator");

    let result = orchestrator
        .execute_pipeline("Test goal", ExecutionMode::Fast)
        .await;

    assert!(
        result.is_ok(),
        "Fast mode pipeline execution should succeed"
    );
}

/// Test orchestrator with Standard mode configuration
#[tokio::test]
async fn test_orchestrator_standard_mode() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    let config = OrchestratorConfig::default()
        .with_work_dir(temp_dir.path())
        .with_progress(false);

    let orchestrator = PipelineOrchestrator::new(config)
        .expect("Failed to create orchestrator");

    let result = orchestrator
        .execute_pipeline("Test goal", ExecutionMode::Standard)
        .await;

    assert!(
        result.is_ok(),
        "Standard mode pipeline execution should succeed"
    );
}

/// Test orchestrator with Expert mode configuration
#[tokio::test]
async fn test_orchestrator_expert_mode() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    let config = OrchestratorConfig::expert_mode()
        .with_work_dir(temp_dir.path())
        .with_progress(false);

    let orchestrator = PipelineOrchestrator::new(config)
        .expect("Failed to create orchestrator");

    let result = orchestrator
        .execute_pipeline("Test goal", ExecutionMode::Expert)
        .await;

    assert!(
        result.is_ok(),
        "Expert mode pipeline execution should succeed"
    );
}

// =============================================================================
// Mode Comparison Tests
// =============================================================================

/// Test that modes are comparable
#[test]
fn test_mode_equality() {
    assert_eq!(ExecutionMode::Fast, ExecutionMode::Fast);
    assert_eq!(ExecutionMode::Standard, ExecutionMode::Standard);
    assert_eq!(ExecutionMode::Expert, ExecutionMode::Expert);

    assert_ne!(ExecutionMode::Fast, ExecutionMode::Standard);
    assert_ne!(ExecutionMode::Standard, ExecutionMode::Expert);
    assert_ne!(ExecutionMode::Fast, ExecutionMode::Expert);
}

/// Test mode serialization
#[test]
fn test_mode_serialization() {
    let modes = vec![
        ExecutionMode::Fast,
        ExecutionMode::Standard,
        ExecutionMode::Expert,
    ];

    for mode in modes {
        let serialized = serde_json::to_string(&mode).expect("Should serialize");
        let deserialized: ExecutionMode = serde_json::from_str(&serialized).expect("Should deserialize");
        assert_eq!(mode, deserialized, "Mode should round-trip through serialization");
    }
}

/// Test that PipelineStage is hashable (for use in HashSet/HashMap)
#[test]
fn test_pipeline_stage_hash() {
    use std::collections::HashSet;

    let mut stages = HashSet::new();
    stages.insert(PipelineStage::Generate);
    stages.insert(PipelineStage::Execute);
    stages.insert(PipelineStage::Test);

    assert!(stages.contains(&PipelineStage::Generate));
    assert!(stages.contains(&PipelineStage::Execute));
    assert!(stages.contains(&PipelineStage::Test));
    assert!(!stages.contains(&PipelineStage::Review));
}

// =============================================================================
// Task Complexity Model Selection Integration
// =============================================================================

/// Test that task complexity affects model selection appropriately
#[test]
fn test_complexity_model_selection_integration() {
    let simple_task = create_task_with_complexity("task-1", "Simple", TaskComplexity::Simple);
    let moderate_task = create_task_with_complexity("task-2", "Moderate", TaskComplexity::Moderate);
    let complex_task = create_task_with_complexity("task-3", "Complex", TaskComplexity::Complex);

    // Verify complexity is set correctly
    assert_eq!(simple_task.complexity, TaskComplexity::Simple);
    assert_eq!(moderate_task.complexity, TaskComplexity::Moderate);
    assert_eq!(complex_task.complexity, TaskComplexity::Complex);

    // Test model selection for each mode
    let fast_config = ModeConfig::fast_mode();
    let standard_config = ModeConfig::default();
    let expert_config = ModeConfig::expert_mode();

    // Fast mode: simple/moderate -> Haiku, complex -> Sonnet
    assert!(fast_config.model_for_complexity(&simple_task.complexity).contains("haiku"));
    assert!(fast_config.model_for_complexity(&moderate_task.complexity).contains("haiku"));
    assert!(fast_config.model_for_complexity(&complex_task.complexity).contains("sonnet"));

    // Standard mode: simple/moderate -> Sonnet, complex -> Opus
    assert!(standard_config.model_for_complexity(&simple_task.complexity).contains("sonnet"));
    assert!(standard_config.model_for_complexity(&moderate_task.complexity).contains("sonnet"));
    assert!(standard_config.model_for_complexity(&complex_task.complexity).contains("opus"));

    // Expert mode: all -> Opus
    assert!(expert_config.model_for_complexity(&simple_task.complexity).contains("opus"));
    assert!(expert_config.model_for_complexity(&moderate_task.complexity).contains("opus"));
    assert!(expert_config.model_for_complexity(&complex_task.complexity).contains("opus"));
}

// =============================================================================
// Performance Characteristics Tests
// =============================================================================

/// Test that Fast mode has shorter timeouts
#[test]
fn test_fast_mode_shorter_timeouts() {
    let fast_config = ModeConfig::fast_mode();
    let standard_config = ModeConfig::default();

    assert!(
        fast_config.timeout_plan < standard_config.timeout_plan,
        "Fast mode should have shorter plan timeout"
    );

    assert!(
        fast_config.timeout_exec < standard_config.timeout_exec,
        "Fast mode should have shorter execution timeout"
    );
}

/// Test that Expert mode has longer timeouts
#[test]
fn test_expert_mode_longer_timeouts() {
    let expert_config = ModeConfig::expert_mode();
    let standard_config = ModeConfig::default();

    assert!(
        expert_config.timeout_plan > standard_config.timeout_plan,
        "Expert mode should have longer plan timeout"
    );

    assert!(
        expert_config.timeout_exec > standard_config.timeout_exec,
        "Expert mode should have longer execution timeout"
    );
}

/// Test that Fast mode has fewer retries
#[test]
fn test_fast_mode_fewer_retries() {
    let fast_config = ModeConfig::fast_mode();
    let standard_config = ModeConfig::default();
    let expert_config = ModeConfig::expert_mode();

    assert!(
        fast_config.max_retries < standard_config.max_retries,
        "Fast mode should have fewer retries than Standard"
    );

    assert_eq!(
        standard_config.max_retries,
        expert_config.max_retries,
        "Standard and Expert should have same retry count"
    );
}

// =============================================================================
// Serialization Tests
// =============================================================================

/// Test ModeConfig serialization
#[test]
fn test_mode_config_serialization() {
    let config = ModeConfig::fast_mode();

    let serialized = serde_json::to_string(&config).expect("Should serialize");
    let deserialized: ModeConfig = serde_json::from_str(&serialized).expect("Should deserialize");

    assert_eq!(config.model_fast, deserialized.model_fast);
    assert_eq!(config.model_smart, deserialized.model_smart);
    assert_eq!(config.run_tests, deserialized.run_tests);
    assert_eq!(config.max_retries, deserialized.max_retries);
    assert_eq!(config.max_depth, deserialized.max_depth);
}

// =============================================================================
// Stage Order Verification Tests
// =============================================================================

/// Verify that stage order is correct for Fast mode
#[test]
fn test_fast_mode_stage_order() {
    let stages = PipelineStage::pipeline_for_mode(ExecutionMode::Fast);

    assert_eq!(stages[0], PipelineStage::Generate, "Stage 0 should be Generate");
    assert_eq!(stages[1], PipelineStage::Assess, "Stage 1 should be Assess");
    assert_eq!(stages[2], PipelineStage::Execute, "Stage 2 should be Execute");
    assert_eq!(stages[3], PipelineStage::Verify, "Stage 3 should be Verify");
    assert_eq!(stages[4], PipelineStage::Commit, "Stage 4 should be Commit");
    assert_eq!(stages[5], PipelineStage::Memory, "Stage 5 should be Memory");
}

/// Verify that stage order is correct for Standard mode
#[test]
fn test_standard_mode_stage_order() {
    let stages = PipelineStage::pipeline_for_mode(ExecutionMode::Standard);

    assert_eq!(stages[0], PipelineStage::Generate, "Stage 0 should be Generate");
    assert_eq!(stages[1], PipelineStage::Assess, "Stage 1 should be Assess");
    assert_eq!(stages[2], PipelineStage::Execute, "Stage 2 should be Execute");
    assert_eq!(stages[3], PipelineStage::Test, "Stage 3 should be Test");
    assert_eq!(stages[4], PipelineStage::Verify, "Stage 4 should be Verify");
    assert_eq!(stages[5], PipelineStage::Commit, "Stage 5 should be Commit");
    assert_eq!(stages[6], PipelineStage::Memory, "Stage 6 should be Memory");
}

/// Verify that stage order is correct for Expert mode
#[test]
fn test_expert_mode_stage_order() {
    let stages = PipelineStage::pipeline_for_mode(ExecutionMode::Expert);

    assert_eq!(stages[0], PipelineStage::Generate, "Stage 0 should be Generate");
    assert_eq!(stages[1], PipelineStage::Assess, "Stage 1 should be Assess");
    assert_eq!(stages[2], PipelineStage::Execute, "Stage 2 should be Execute");
    assert_eq!(stages[3], PipelineStage::Test, "Stage 3 should be Test");
    assert_eq!(stages[4], PipelineStage::Review, "Stage 4 should be Review");
    assert_eq!(stages[5], PipelineStage::Verify, "Stage 5 should be Verify");
    assert_eq!(stages[6], PipelineStage::Commit, "Stage 6 should be Commit");
    assert_eq!(stages[7], PipelineStage::Memory, "Stage 7 should be Memory");
}