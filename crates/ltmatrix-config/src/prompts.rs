// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Prompt configuration for ltmatrix
//!
//! This module contains all prompts used by the pipeline stages.
//! Prompts can be customized via configuration files.
//!
//! # Configuration Locations
//!
//! Prompts can be configured in:
//! 1. Global config: `~/.config/ltmatrix/prompts.toml`
//! 2. Project config: `.ltmatrix/prompts.toml`
//! 3. Main config file: `[prompts]` section in `config.toml`

use serde::{Deserialize, Serialize};

/// Prompt configuration containing all stage prompts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptsConfig {
    /// Task generation prompts
    #[serde(default)]
    pub generation: GenerationPrompts,

    /// Task assessment prompts
    #[serde(default)]
    pub assessment: AssessmentPrompts,

    /// Task execution prompts
    #[serde(default)]
    pub execution: ExecutionPrompts,

    /// Task verification prompts
    #[serde(default)]
    pub verification: VerificationPrompts,

    /// Code review prompts (expert mode)
    #[serde(default)]
    pub review: ReviewPrompts,

    /// Fix cycle prompts
    #[serde(default)]
    pub fix: FixPrompts,
}

impl Default for PromptsConfig {
    fn default() -> Self {
        PromptsConfig {
            generation: GenerationPrompts::default(),
            assessment: AssessmentPrompts::default(),
            execution: ExecutionPrompts::default(),
            verification: VerificationPrompts::default(),
            review: ReviewPrompts::default(),
            fix: FixPrompts::default(),
        }
    }
}

/// Task generation prompts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationPrompts {
    /// System role description for generation
    #[serde(default = "default_generation_system")]
    pub system: String,

    /// Task format instructions
    #[serde(default = "default_generation_task_format")]
    pub task_format: String,

    /// Dependency format instructions
    #[serde(default = "default_generation_dependency_format")]
    pub dependency_format: String,

    /// Response format instructions
    #[serde(default = "default_generation_response_format")]
    pub response_format: String,

    /// Additional context for generation
    #[serde(default)]
    pub extra_context: Option<String>,
}

fn default_generation_system() -> String {
    r#"You are an expert software architect and project manager.
Your role is to break down complex software development goals into well-structured, actionable tasks."#.to_string()
}

fn default_generation_task_format() -> String {
    r#"Each task should:
1. Have a clear, descriptive title
2. Include detailed implementation description
3. Be independently testable and verifiable
4. Have explicit dependencies on other tasks if needed
5. Be appropriately scoped (not too large, not too small)

**IMPORTANT: The last task MUST always be a comprehensive testing and fix task that:**
- Runs ALL tests (unit tests, integration tests, e2e tests)
- Ensures full test coverage for all implemented features
- Fixes any failing tests or bugs discovered during testing
- Validates the entire implementation end-to-end
- Depends on ALL other tasks"#.to_string()
}

fn default_generation_dependency_format() -> String {
    r#"Dependencies should:
1. Use task IDs to reference other tasks
2. Form a valid dependency graph (no circular references)
3. Be minimal but complete (only depend on what's truly needed)
4. The final testing task should depend on ALL implementation tasks"#.to_string()
}

fn default_generation_response_format() -> String {
    r#"Respond ONLY with valid JSON in this exact format:
```json
{
  "tasks": [
    {
      "id": "task-1",
      "title": "Task title",
      "description": "Detailed task description",
      "depends_on": ["task-id"]
    }
  ]
}
```

**Remember: The last task MUST be a comprehensive testing and fix task!**"#.to_string()
}

impl Default for GenerationPrompts {
    fn default() -> Self {
        GenerationPrompts {
            system: default_generation_system(),
            task_format: default_generation_task_format(),
            dependency_format: default_generation_dependency_format(),
            response_format: default_generation_response_format(),
            extra_context: None,
        }
    }
}

/// Task assessment prompts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssessmentPrompts {
    /// System role for assessment
    #[serde(default = "default_assessment_system")]
    pub system: String,

    /// Complexity criteria instructions
    #[serde(default = "default_assessment_complexity")]
    pub complexity_criteria: String,

    /// Subtask breakdown instructions
    #[serde(default = "default_assessment_subtasks")]
    pub subtask_instructions: String,

    /// Model recommendation instructions
    #[serde(default = "default_assessment_model")]
    pub model_recommendation: String,

    /// Response format
    #[serde(default = "default_assessment_response_format")]
    pub response_format: String,
}

fn default_assessment_system() -> String {
    "You are a task assessment expert. Analyze the following task and determine its complexity."
        .to_string()
}

fn default_assessment_complexity() -> String {
    r#"Assess the task complexity as one of:
- Simple: Straightforward, minimal dependencies, clear implementation path
- Moderate: Some complexity, multiple components, requires careful design
- Complex: High complexity, multiple systems, architectural decisions needed"#.to_string()
}

fn default_assessment_subtasks() -> String {
    r#"If the task is rated as "Complex", break it down into 2-5 subtasks.
Each subtask should be:
- Independently executable (or with clear dependencies)
- Specific and actionable
- Include a clear description"#.to_string()
}

fn default_assessment_model() -> String {
    r#"Recommend the appropriate AI model for execution:
- Simple tasks: claude-haiku-4-5 (fast, cost-effective)
- Moderate tasks: claude-sonnet-4-6 (balanced)
- Complex tasks: claude-opus-4-6 (highest quality)"#.to_string()
}

fn default_assessment_response_format() -> String {
    r#"Respond ONLY with valid JSON in this exact format:
```json
{
  "complexity": "Simple|Moderate|Complex",
  "recommended_model": "claude-haiku-4-5|claude-sonnet-4-6|claude-opus-4-6",
  "estimated_time_minutes": <number or null>,
  "reasoning": "<brief explanation of complexity rating>",
  "subtasks": [
    {
      "id": "<unique subtask ID>",
      "title": "<subtask title>",
      "description": "<detailed description>",
      "depends_on": ["<list of subtask IDs this depends on, or empty array>"]
    }
  ]
}
```

If complexity is Simple or Moderate, subtasks should be an empty array."#.to_string()
}

impl Default for AssessmentPrompts {
    fn default() -> Self {
        AssessmentPrompts {
            system: default_assessment_system(),
            complexity_criteria: default_assessment_complexity(),
            subtask_instructions: default_assessment_subtasks(),
            model_recommendation: default_assessment_model(),
            response_format: default_assessment_response_format(),
        }
    }
}

/// Task execution prompts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPrompts {
    /// System role for execution
    #[serde(default = "default_execution_system")]
    pub system: String,

    /// Implementation instructions
    #[serde(default = "default_execution_instructions")]
    pub instructions: String,

    /// Code quality requirements
    #[serde(default = "default_execution_quality")]
    pub quality_requirements: String,

    /// Completion criteria
    #[serde(default = "default_execution_completion")]
    pub completion_criteria: String,
}

fn default_execution_system() -> String {
    "You are implementing a task for a software development project.".to_string()
}

fn default_execution_instructions() -> String {
    r#"Please implement this task following these requirements:

1. **Complete the task**: Implement all necessary code, tests, and documentation
2. **Follow best practices**: Write clean, maintainable, well-documented code
3. **Handle errors**: Add proper error handling and edge case coverage
4. **Test your code**: Write or update tests to verify the implementation
5. **Update documentation**: Update any relevant documentation or comments"#.to_string()
}

fn default_execution_quality() -> String {
    r#"Code Quality Standards:
- Use consistent formatting and naming conventions
- Write self-documenting code with clear variable/function names
- Add comments for complex logic
- Follow the existing codebase patterns and architecture
- Keep functions focused and reasonably sized"#.to_string()
}

fn default_execution_completion() -> String {
    r#"Mark the task as complete when:
- All required functionality is implemented
- Tests pass (if applicable)
- Code compiles without errors
- Documentation is updated (if needed)
- No obvious bugs or issues remain"#.to_string()
}

impl Default for ExecutionPrompts {
    fn default() -> Self {
        ExecutionPrompts {
            system: default_execution_system(),
            instructions: default_execution_instructions(),
            quality_requirements: default_execution_quality(),
            completion_criteria: default_execution_completion(),
        }
    }
}

/// Task verification prompts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationPrompts {
    /// System role for verification
    #[serde(default = "default_verification_system")]
    pub system: String,

    /// Verification criteria
    #[serde(default = "default_verification_criteria")]
    pub criteria: String,

    /// Important instructions
    #[serde(default = "default_verification_instructions")]
    pub instructions: String,

    /// Response format
    #[serde(default = "default_verification_response_format")]
    pub response_format: String,
}

fn default_verification_system() -> String {
    "You are verifying that a software development task has been completed correctly."
        .to_string()
}

fn default_verification_criteria() -> String {
    r#"Verification Criteria:

1. **Acceptance Criteria**: Does the implementation fulfill the requirements stated in the task description?
2. **Code Quality**: Is the code well-structured, readable, and maintainable?
3. **Testing**: Have appropriate tests been added or updated?
4. **Documentation**: Has relevant documentation been updated?
5. **Edge Cases**: Are edge cases and error conditions properly handled?"#.to_string()
}

fn default_verification_instructions() -> String {
    r#"Important Instructions:

- Examine the actual code changes made
- Look for test files related to this task
- Check if the described functionality actually works
- Be thorough but fair - minor style issues should not cause failure"#.to_string()
}

fn default_verification_response_format() -> String {
    r#"Respond with a structured assessment in the following format:

```json
{
  "passed": true|false,
  "reasoning": "Detailed explanation of your assessment",
  "unmet_criteria": ["List any acceptance criteria not met"],
  "suggestions": ["List specific fixes for any issues found"],
  "retry_recommended": true|false
}
```"#.to_string()
}

impl Default for VerificationPrompts {
    fn default() -> Self {
        VerificationPrompts {
            system: default_verification_system(),
            criteria: default_verification_criteria(),
            instructions: default_verification_instructions(),
            response_format: default_verification_response_format(),
        }
    }
}

/// Code review prompts (expert mode)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewPrompts {
    /// System role for review
    #[serde(default = "default_review_system")]
    pub system: String,

    /// Review categories
    #[serde(default = "default_review_categories")]
    pub categories: String,

    /// Severity levels
    #[serde(default = "default_review_severity")]
    pub severity_levels: String,

    /// Response format
    #[serde(default = "default_review_response_format")]
    pub response_format: String,
}

fn default_review_system() -> String {
    "You are performing a comprehensive code review for a software development project.".to_string()
}

fn default_review_categories() -> String {
    r#"Review Categories:

1. **Security**: Vulnerabilities, injection risks, authentication issues
2. **Performance**: Inefficient algorithms, memory leaks, resource issues
3. **Code Quality**: Readability, maintainability, complexity
4. **Best Practices**: Design patterns, SOLID principles, DRY
5. **Testing**: Test coverage, test quality, edge cases
6. **Documentation**: Comments, README, API documentation"#.to_string()
}

fn default_review_severity() -> String {
    r#"Severity Levels:

- **Critical**: Must be fixed before merge (security vulnerabilities, data loss risks)
- **High**: Should be fixed soon (significant bugs, performance issues)
- **Medium**: Should be addressed (code quality, maintainability)
- **Low**: Minor improvements (style, optimization opportunities)
- **Info**: Suggestions and observations (non-blocking)"#.to_string()
}

fn default_review_response_format() -> String {
    r#"Respond with a structured review in the following format:

```json
{
  "findings": [
    {
      "category": "security|performance|quality|best_practices|testing|documentation",
      "severity": "critical|high|medium|low|info",
      "file": "path/to/file.rs",
      "line": 42,
      "description": "Description of the issue",
      "suggestion": "Suggested fix or improvement"
    }
  ],
  "summary": "Overall assessment summary",
  "approved": true|false
}
```"#.to_string()
}

impl Default for ReviewPrompts {
    fn default() -> Self {
        ReviewPrompts {
            system: default_review_system(),
            categories: default_review_categories(),
            severity_levels: default_review_severity(),
            response_format: default_review_response_format(),
        }
    }
}

/// Fix cycle prompts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixPrompts {
    /// System role for fixing
    #[serde(default = "default_fix_system")]
    pub system: String,

    /// Fix instructions
    #[serde(default = "default_fix_instructions")]
    pub instructions: String,

    /// Response format
    #[serde(default = "default_fix_response_format")]
    pub response_format: String,
}

fn default_fix_system() -> String {
    "You are fixing issues identified during testing or code review.".to_string()
}

fn default_fix_instructions() -> String {
    r#"Fix Instructions:

1. **Analyze the issue**: Understand the root cause of the problem
2. **Minimal changes**: Make the smallest change that fixes the issue
3. **Preserve functionality**: Don't break existing working code
4. **Add tests**: If applicable, add tests to prevent regression
5. **Document changes**: Update comments or documentation if needed"#.to_string()
}

fn default_fix_response_format() -> String {
    r#"After fixing, summarize what was changed:

```json
{
  "fixed": true|false,
  "changes": ["List of files modified"],
  "description": "What was changed and why",
  "tests_added": true|false
}
```"#.to_string()
}

impl Default for FixPrompts {
    fn default() -> Self {
        FixPrompts {
            system: default_fix_system(),
            instructions: default_fix_instructions(),
            response_format: default_fix_response_format(),
        }
    }
}

impl PromptsConfig {
    /// Load prompts from a TOML file
    pub fn from_file(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: PromptsConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Load prompts from the default locations
    pub fn load() -> Self {
        // Try project config first
        let project_config = std::path::Path::new(".ltmatrix/prompts.toml");
        if project_config.exists() {
            if let Ok(config) = Self::from_file(project_config) {
                return config;
            }
        }

        // Try global config
        let global_config = dirs::config_dir()
            .map(|p| p.join("ltmatrix/prompts.toml"));
        if let Some(path) = global_config {
            if path.exists() {
                if let Ok(config) = Self::from_file(&path) {
                    return config;
                }
            }
        }

        // Return defaults
        Self::default()
    }

    /// Build the full generation prompt
    pub fn build_generation_prompt(&self, goal: &str, mode: &str) -> String {
        let mode_context = match mode {
            "fast" => "\n\n## Mode: Fast\nGenerate fewer, simpler tasks. Focus on quick wins.",
            "expert" => "\n\n## Mode: Expert\nGenerate comprehensive tasks with thorough coverage. Include edge cases and advanced scenarios.",
            _ => ""
        };

        let extra = self.generation.extra_context.as_deref().unwrap_or("");

        format!(
            r#"{}

{}

## Goal

{}

{}

## Task Format

{}

## Dependencies

{}

## Response Format

{}

Begin task generation now."#,
            self.generation.system,
            mode_context,
            goal,
            extra,
            self.generation.task_format,
            self.generation.dependency_format,
            self.generation.response_format
        )
    }

    /// Build the full assessment prompt
    pub fn build_assessment_prompt(&self, task_id: &str, task_title: &str, task_description: &str, depth: u32) -> String {
        let depth_context = if depth > 0 {
            format!("\n(Subtask depth: {})", depth)
        } else {
            String::new()
        };

        format!(
            r#"{}

Task ID: {} {}
Title: {}
Description: {}

## Complexity Assessment

{}

## Subtask Breakdown

{}

## Model Recommendation

{}

## Response Format

{}"#,
            self.assessment.system,
            task_id, depth_context, task_title, task_description,
            self.assessment.complexity_criteria,
            self.assessment.subtask_instructions,
            self.assessment.model_recommendation,
            self.assessment.response_format
        )
    }

    /// Build the full execution prompt
    pub fn build_execution_prompt(&self, task_title: &str, task_description: &str, context: &str) -> String {
        format!(
            r#"{}

{}

## Your Task

**{}**

{}

## Instructions

{}

## Code Quality

{}

## Completion

{}"#,
            self.execution.system,
            context,
            task_title,
            task_description,
            self.execution.instructions,
            self.execution.quality_requirements,
            self.execution.completion_criteria
        )
    }

    /// Build the full verification prompt
    pub fn build_verification_prompt(&self, task_id: &str, task_title: &str, task_description: &str) -> String {
        format!(
            r#"{}

## Original Task Description

**Task ID**: {}
**Title**: {}
**Description**: {}

## Your Task

Review the current state of the codebase and determine if this task has been completed successfully.

## {}

## {}

## Response Format

{}

Begin your verification now. Examine the codebase thoroughly and provide your assessment."#,
            self.verification.system,
            task_id, task_title, task_description,
            self.verification.criteria,
            self.verification.instructions,
            self.verification.response_format
        )
    }

    /// Build the full review prompt
    pub fn build_review_prompt(&self, files_changed: &[&str], diff: &str) -> String {
        let files_list = files_changed.join("\n- ");

        format!(
            r#"{}

## Files Changed

- {}

## Diff

```
{}
```

## Review Categories

{}

## Severity Levels

{}

## Response Format

{}

Begin your code review now."#,
            self.review.system,
            files_list,
            diff,
            self.review.categories,
            self.review.severity_levels,
            self.review.response_format
        )
    }

    /// Build the full fix prompt
    pub fn build_fix_prompt(&self, issue_description: &str, context: &str) -> String {
        format!(
            r#"{}

## Issue

{}

## Context

{}

## {}

## Response Format

{}

Begin fixing the issue now."#,
            self.fix.system,
            issue_description,
            context,
            self.fix.instructions,
            self.fix.response_format
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_prompts_config() {
        let config = PromptsConfig::default();
        assert!(!config.generation.system.is_empty());
        assert!(!config.assessment.system.is_empty());
        assert!(!config.execution.system.is_empty());
        assert!(!config.verification.system.is_empty());
        assert!(!config.review.system.is_empty());
        assert!(!config.fix.system.is_empty());
    }

    #[test]
    fn test_build_generation_prompt() {
        let config = PromptsConfig::default();
        let prompt = config.build_generation_prompt("Build a REST API", "standard");
        assert!(prompt.contains("Build a REST API"));
        assert!(prompt.contains("expert software architect"));
    }

    #[test]
    fn test_build_assessment_prompt() {
        let config = PromptsConfig::default();
        let prompt = config.build_assessment_prompt("task-1", "Test Task", "Description", 0);
        assert!(prompt.contains("task-1"));
        assert!(prompt.contains("Test Task"));
        assert!(prompt.contains("Description"));
    }

    #[test]
    fn test_build_execution_prompt() {
        let config = PromptsConfig::default();
        let prompt = config.build_execution_prompt("Test Task", "Description", "Context info");
        assert!(prompt.contains("Test Task"));
        assert!(prompt.contains("Description"));
        assert!(prompt.contains("Context info"));
    }

    #[test]
    fn test_build_verification_prompt() {
        let config = PromptsConfig::default();
        let prompt = config.build_verification_prompt("task-1", "Test", "Description");
        assert!(prompt.contains("task-1"));
        assert!(prompt.contains("Test"));
    }

    #[test]
    fn test_assessment_with_depth() {
        let config = PromptsConfig::default();
        let prompt = config.build_assessment_prompt("sub-1", "Subtask", "Desc", 2);
        assert!(prompt.contains("(Subtask depth: 2)"));
    }

    #[test]
    fn test_serialization_deserialization() {
        let config = PromptsConfig::default();
        let toml_str = toml::to_string(&config).unwrap();
        let parsed: PromptsConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(config.generation.system, parsed.generation.system);
    }
}
