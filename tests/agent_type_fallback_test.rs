// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Tests for AgentType::from_keywords() auto-assignment functionality
//!
//! These tests verify that the from_keywords() method correctly detects
//! agent types based on task title and description keywords.

use ltmatrix::models::Task;
use ltmatrix_core::AgentType;

fn create_task_with_agent_type(id: &str, title: &str, desc: &str, agent_type: AgentType) -> Task {
    let mut task = Task::new(id, title, desc);
    task.agent_type = agent_type;
    task
}

#[test]
fn test_from_keywords_detects_test_type() {
    let task = create_task_with_agent_type("t1", "Write tests", "Add unit tests", AgentType::Dev);
    let combined = format!("{} {}", task.title, task.description);
    let detected = AgentType::from_keywords(&combined);
    assert_eq!(detected, AgentType::Test);
}

#[test]
fn test_from_keywords_detects_review_type() {
    let task = create_task_with_agent_type("t1", "Code review", "Review the code", AgentType::Dev);
    let combined = format!("{} {}", task.title, task.description);
    let detected = AgentType::from_keywords(&combined);
    assert_eq!(detected, AgentType::Review);
}

#[test]
fn test_from_keywords_detects_plan_type() {
    let task = create_task_with_agent_type(
        "t1",
        "Analyze architecture",
        "Plan the system",
        AgentType::Dev,
    );
    let combined = format!("{} {}", task.title, task.description);
    let detected = AgentType::from_keywords(&combined);
    assert_eq!(detected, AgentType::Plan);
}

#[test]
fn test_from_keywords_keeps_dev_for_implementation() {
    let task = create_task_with_agent_type(
        "t1",
        "Implement feature",
        "Add new functionality",
        AgentType::Dev,
    );
    let combined = format!("{} {}", task.title, task.description);
    let detected = AgentType::from_keywords(&combined);
    assert_eq!(detected, AgentType::Dev);
}

#[test]
fn test_from_keywords_handles_chinese_keywords() {
    let task = create_task_with_agent_type("t1", "测试模块", "添加单元测试", AgentType::Dev);
    let combined = format!("{} {}", task.title, task.description);
    let detected = AgentType::from_keywords(&combined);
    assert_eq!(detected, AgentType::Test);
}
