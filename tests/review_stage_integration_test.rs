//! Integration tests for review stage integration in the pipeline orchestrator
//!
//! These tests verify that the review stage is properly integrated into the pipeline
//! flow and behaves correctly in different execution modes.

use ltmatrix::models::{ExecutionMode, PipelineStage};
use ltmatrix::pipeline::orchestrator::{OrchestratorConfig, PipelineOrchestrator};
use ltmatrix::pipeline::review::ReviewConfig;
use tempfile::TempDir;

#[test]
fn test_pipeline_stage_includes_review() {
    // Verify that Review variant exists in PipelineStage
    let stages = ExecutionMode::Expert.pipeline_stages();

    assert!(
        stages.contains(&PipelineStage::Review),
        "Expert mode pipeline should include Review stage"
    );
}

#[test]
fn test_review_stage_position_in_expert_mode() {
    let stages = ExecutionMode::Expert.pipeline_stages();

    let test_idx = stages
        .iter()
        .position(|s| matches!(s, PipelineStage::Test))
        .expect("Test stage should exist");
    let review_idx = stages
        .iter()
        .position(|s| matches!(s, PipelineStage::Review))
        .expect("Review stage should exist");
    let verify_idx = stages
        .iter()
        .position(|s| matches!(s, PipelineStage::Verify))
        .expect("Verify stage should exist");

    assert!(
        review_idx > test_idx,
        "Review should come after Test in expert mode"
    );
    assert!(
        review_idx < verify_idx,
        "Review should come before Verify in expert mode"
    );
}

#[test]
fn test_review_stage_not_in_fast_mode() {
    let stages = ExecutionMode::Fast.pipeline_stages();

    assert!(
        !stages.contains(&PipelineStage::Review),
        "Fast mode should not include Review stage"
    );
}

#[test]
fn test_review_stage_not_in_standard_mode() {
    let stages = ExecutionMode::Standard.pipeline_stages();

    assert!(
        !stages.contains(&PipelineStage::Review),
        "Standard mode should not include Review stage"
    );
}

#[test]
fn test_orchestrator_config_expert_mode_has_review_enabled() {
    let config = OrchestratorConfig::expert_mode();

    assert!(
        config.review_config.should_run(),
        "Review stage should be enabled in expert mode"
    );
}

#[test]
fn test_orchestrator_config_fast_mode_review_disabled() {
    let config = OrchestratorConfig::fast_mode();

    assert!(
        !config.review_config.should_run(),
        "Review stage should be disabled in fast mode"
    );
}

#[test]
fn test_orchestrator_config_standard_mode_review_disabled() {
    let config = OrchestratorConfig::default(); // Default is Standard mode

    assert!(
        !config.review_config.should_run(),
        "Review stage should be disabled in standard mode"
    );
}

#[test]
fn test_review_stage_display_name() {
    let name = PipelineStage::Review.display_name();
    assert_eq!(name, "Code Review");
}

#[test]
fn test_review_stage_requires_agent() {
    assert!(
        PipelineStage::Review.requires_agent(),
        "Review stage should require agent interaction"
    );
}

#[test]
fn test_expert_mode_pipeline_order() {
    let stages = ExecutionMode::Expert.pipeline_stages();
    let expected_order = vec![
        PipelineStage::Generate,
        PipelineStage::Assess,
        PipelineStage::Execute,
        PipelineStage::Test,
        PipelineStage::Review,
        PipelineStage::Verify,
        PipelineStage::Commit,
        PipelineStage::Memory,
    ];

    assert_eq!(stages, expected_order, "Expert mode pipeline should have the correct stage order");
}

#[test]
fn test_standard_mode_pipeline_order() {
    let stages = ExecutionMode::Standard.pipeline_stages();
    let expected_order = vec![
        PipelineStage::Generate,
        PipelineStage::Assess,
        PipelineStage::Execute,
        PipelineStage::Test,
        PipelineStage::Verify,
        PipelineStage::Commit,
        PipelineStage::Memory,
    ];

    assert_eq!(stages, expected_order, "Standard mode pipeline should have the correct stage order");
}

#[test]
fn test_fast_mode_pipeline_order() {
    let stages = ExecutionMode::Fast.pipeline_stages();
    let expected_order = vec![
        PipelineStage::Generate,
        PipelineStage::Assess,
        PipelineStage::Execute,
        PipelineStage::Verify,
        PipelineStage::Commit,
        PipelineStage::Memory,
    ];

    assert_eq!(stages, expected_order, "Fast mode pipeline should have the correct stage order");
}

#[tokio::test]
async fn test_orchestrator_creates_successfully_with_review_config() {
    let temp_dir = TempDir::new().unwrap();
    let config = OrchestratorConfig::expert_mode()
        .with_work_dir(temp_dir.path());

    let orchestrator = PipelineOrchestrator::new(config);
    assert!(orchestrator.is_ok(), "Orchestrator should create successfully with review config");
}

#[test]
fn test_review_config_expert_mode_properties() {
    let config = ReviewConfig::expert_mode();

    assert!(config.enabled, "Review should be enabled in expert mode");
    assert_eq!(config.mode_config.verify, true, "Verify should be enabled in expert mode");
    assert!(config.review_model.contains("opus"), "Expert mode should use Opus model");
    assert_eq!(config.timeout, 900, "Expert mode should have 15min timeout");
}

// Helper trait to get pipeline stages
trait PipelineStagesExt {
    fn pipeline_stages(&self) -> Vec<PipelineStage>;
}

impl PipelineStagesExt for ExecutionMode {
    fn pipeline_stages(&self) -> Vec<PipelineStage> {
        PipelineStage::pipeline_for_mode(*self)
    }
}
