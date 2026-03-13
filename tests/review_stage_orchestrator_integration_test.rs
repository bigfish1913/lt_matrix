//! Simple verification test for review stage integration
//!
//! This test verifies that the review stage is properly integrated into the pipeline.

use ltmatrix::models::{ExecutionMode, PipelineStage};

#[test]
fn test_review_stage_exists() {
    // Verify Review variant exists
    let review = PipelineStage::Review;
    assert_eq!(review.display_name(), "Code Review");
}

#[test]
fn test_review_in_expert_mode() {
    let stages = PipelineStage::pipeline_for_mode(ExecutionMode::Expert);
    assert!(stages.contains(&PipelineStage::Review));
}

#[test]
fn test_review_not_in_standard_mode() {
    let stages = PipelineStage::pipeline_for_mode(ExecutionMode::Standard);
    assert!(!stages.contains(&PipelineStage::Review));
}

#[test]
fn test_review_not_in_fast_mode() {
    let stages = PipelineStage::pipeline_for_mode(ExecutionMode::Fast);
    assert!(!stages.contains(&PipelineStage::Review));
}

#[test]
fn test_review_position_in_expert_pipeline() {
    let stages = PipelineStage::pipeline_for_mode(ExecutionMode::Expert);

    let test_pos = stages
        .iter()
        .position(|s| matches!(s, PipelineStage::Test))
        .unwrap();
    let review_pos = stages
        .iter()
        .position(|s| matches!(s, PipelineStage::Review))
        .unwrap();
    let verify_pos = stages
        .iter()
        .position(|s| matches!(s, PipelineStage::Verify))
        .unwrap();

    assert!(review_pos > test_pos, "Review should be after Test");
    assert!(review_pos < verify_pos, "Review should be before Verify");
}

#[test]
fn test_expert_pipeline_complete_flow() {
    let stages = PipelineStage::pipeline_for_mode(ExecutionMode::Expert);

    // Expected order: Generate -> Assess -> Execute -> Test -> Review -> Verify -> Commit -> Memory
    let expected = vec![
        PipelineStage::Generate,
        PipelineStage::Assess,
        PipelineStage::Execute,
        PipelineStage::Test,
        PipelineStage::Review,
        PipelineStage::Verify,
        PipelineStage::Commit,
        PipelineStage::Memory,
    ];

    assert_eq!(stages, expected);
}
