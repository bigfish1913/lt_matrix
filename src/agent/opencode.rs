// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.


//! OpenCode agent backend implementation
//!
//! This module implements the AgentBackend trait for the OpenCode CLI tool,
//! handling subprocess spawning, prompt execution, session management,
//! and response parsing specific to OpenCode.

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tracing::{debug, info, warn};

use crate::agent::backend::{
    AgentBackend, AgentConfig, AgentError, AgentResponse, AgentSession, ExecutionConfig,
};
use crate::agent::session::{SessionData, SessionManager};
use crate::models::Agent;

/// OpenCode agent implementation
#[derive(Debug, Clone)]
pub struct OpenCodeAgent {
    /// Agent configuration
    agent: Agent,

    /// Session manager for reuse
    session_manager: SessionManager,

    /// Whether to verify opencode command availability
    verify_command: bool,
}

impl OpenCodeAgent {
    /// Create a new OpenCode agent with default configuration
    pub fn new() -> Result<Self> {
        let agent = Agent::opencode_default();
        let session_manager =
            SessionManager::default_manager().context("Failed to create session manager")?;

        Ok(OpenCodeAgent {
            agent,
            session_manager,
            verify_command: true,
        })
    }

    /// Create an OpenCode agent with custom configuration
    pub fn with_agent(agent: Agent, session_manager: SessionManager) -> Self {
        OpenCodeAgent {
            agent,
            session_manager,
            verify_command: true,
        }
    }

    /// Disable command verification (useful for testing)
    pub fn without_verification(mut self) -> Self {
        self.verify_command = false;
        self
    }

    /// Verify that the opencode command is available
    async fn verify_opencode_command(&self) -> Result<()> {
        if !self.verify_command {
            return Ok(());
        }

        let output = Command::new(&self.agent.command)
            .arg("--version")
            .output()
            .await;

        match output {
            Ok(output) if output.status.success() => {
                debug!(
                    "OpenCode command verified: {}",
                    String::from_utf8_lossy(&output.stdout)
                );
                Ok(())
            }
            Ok(output) => Err(anyhow!(
                "OpenCode command failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )),
            Err(e) => Err(anyhow!(
                "OpenCode command not found: {}. Please install OpenCode CLI",
                e
            )),
        }
    }

    /// Build the command line for invoking OpenCode
    fn build_command(&self, config: &ExecutionConfig) -> Vec<String> {
        let mut args = vec![
            self.agent.command.clone(),
            "--prompt".to_string(),
            "-".to_string(), // Read from stdin
        ];

        // Add model selection if specified and different from default
        if config.model != self.agent.model {
            args.push("--model".to_string());
            args.push(config.model.clone());
        }

        args
    }

    /// Execute a prompt with retry logic
    async fn execute_with_retry(
        &self,
        prompt: &str,
        config: &ExecutionConfig,
        session: Option<&SessionData>,
    ) -> Result<AgentResponse> {
        let mut last_error = None;

        for attempt in 0..=config.max_retries {
            if attempt > 0 {
                // Exponential backoff
                let delay = Duration::from_millis(100 * 2_u64.pow(attempt - 1));
                warn!("Retry attempt {} after {:?}", attempt, delay);
                tokio::time::sleep(delay).await;
            }

            match self.execute_single_attempt(prompt, config, session).await {
                Ok(response) => {
                    if attempt > 0 {
                        info!("Execution succeeded on attempt {}", attempt + 1);
                    }
                    return Ok(response);
                }
                Err(e) => {
                    warn!("Execution attempt {} failed: {}", attempt + 1, e);
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("All retry attempts exhausted")))
    }

    /// Execute a single attempt
    async fn execute_single_attempt(
        &self,
        prompt: &str,
        config: &ExecutionConfig,
        _session: Option<&SessionData>,
    ) -> Result<AgentResponse> {
        let args = self.build_command(config);

        debug!(
            "Spawning OpenCode process: {} with model {}",
            self.agent.command, config.model
        );

        // Spawn the OpenCode process
        let mut child = Command::new(&args[0])
            .args(&args[1..])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .context("Failed to spawn OpenCode process")?;

        // Write prompt to stdin
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(prompt.as_bytes())
                .await
                .context("Failed to write prompt to OpenCode stdin")?;

            stdin
                .shutdown()
                .await
                .context("Failed to close OpenCode stdin")?;
        }

        // Read stdout and stderr concurrently
        let stdout = child.stdout.take().expect("Failed to get stdout");
        let stderr = child.stderr.take().expect("Failed to get stderr");

        let stdout_future = async {
            let mut reader = BufReader::new(stdout).lines();
            let mut output = String::new();

            while let Some(line) = reader.next_line().await.unwrap_or(None) {
                output.push_str(&line);
                output.push('\n');
            }

            output
        };

        let stderr_future = async {
            let mut reader = BufReader::new(stderr).lines();
            let mut errors = String::new();

            while let Some(line) = reader.next_line().await.unwrap_or(None) {
                errors.push_str(&line);
                errors.push('\n');
            }

            errors
        };

        // Wait for both stdout and stderr with timeout
        let timeout_duration = Duration::from_secs(config.timeout);
        let output = tokio::time::timeout(timeout_duration, async {
            tokio::join!(stdout_future, stderr_future)
        })
        .await
        .context("OpenCode execution timed out")?;

        let (stdout_text, stderr_text) = output;

        // Wait for process to complete
        let status = child
            .wait()
            .await
            .context("Failed to wait for OpenCode process")?;

        debug!("OpenCode process exited with status: {}", status);

        // Check for errors
        let error = if !status.success() {
            Some(format!(
                "OpenCode exited with status {}: {}",
                status, stderr_text
            ))
        } else if !stderr_text.is_empty() {
            Some(format!("OpenCode stderr: {}", stderr_text))
        } else {
            None
        };

        // Parse structured data from response
        let structured_data = Self::parse_structured_data(&stdout_text);

        // Determine if task is complete
        let is_complete = Self::check_completion(&stdout_text);

        Ok(AgentResponse {
            output: stdout_text,
            structured_data,
            is_complete,
            error,
        })
    }

    /// Parse structured data (JSON) from OpenCode's response
    fn parse_structured_data(output: &str) -> Option<serde_json::Value> {
        // Look for JSON blocks in the response
        // OpenCode may use different markers than Claude
        let json_start = output.find("```json")?;
        let json_start = json_start + 7; // Skip past ```json
        let json_end = output[json_start..].find("```")?;
        let json_str = &output[json_start..json_start + json_end];

        // Try to parse as JSON
        serde_json::from_str(json_str).ok()
    }

    /// Check if OpenCode indicates the task is complete
    pub fn check_completion(output: &str) -> bool {
        let output_lower = output.to_lowercase();

        // First, exclude negative patterns - if we find these, it's NOT complete
        if output_lower.contains("not done")
            || output_lower.contains("not finished")
            || output_lower.contains("not complete")
            || output_lower.contains("not completed")
            || output_lower.contains("incomplete")
            || output_lower.contains("error")
            || output_lower.contains("failed")
        {
            return false;
        }

        // Now look for positive completion indicators
        output_lower.contains("task completed")
            || output_lower.contains("implementation complete")
            || output_lower.contains("done")
            || output_lower.contains("finished")
            || output_lower.contains("complete")
            || output_lower.contains("completed")
            || output_lower.contains("success")
    }

    /// Get or create a session for this execution
    async fn get_session(&self, config: &ExecutionConfig) -> Result<Option<SessionData>> {
        if !config.enable_session {
            return Ok(None);
        }

        // Try to reuse existing session
        // For now, we'll create a new session each time
        let session = self
            .session_manager
            .create_session("opencode", &config.model)
            .await?;

        debug!("Created new session: {}", session.session_id);

        Ok(Some(session))
    }
}

#[async_trait]
impl AgentBackend for OpenCodeAgent {
    async fn execute(&self, prompt: &str, config: &ExecutionConfig) -> Result<AgentResponse> {
        info!("Executing OpenCode prompt with model {}", config.model);

        // Validate prompt
        if prompt.trim().is_empty() {
            return Err(anyhow::anyhow!("Prompt cannot be empty"));
        }

        // Verify OpenCode command is available
        self.verify_opencode_command().await?;

        // Get session
        let session = self.get_session(config).await?;

        // Execute with retry
        let response = self
            .execute_with_retry(prompt, config, session.as_ref())
            .await?;

        Ok(response)
    }

    async fn execute_task(
        &self,
        task: &crate::models::Task,
        context: &str,
        config: &ExecutionConfig,
    ) -> Result<AgentResponse> {
        // Build prompt with task context
        let prompt = format!(
            "Task: {}\n\nDescription: {}\n\nContext:\n{}\n\nPlease complete this task.",
            task.title, task.description, context
        );

        self.execute(&prompt, config).await
    }

    async fn health_check(&self) -> Result<bool> {
        match self.verify_opencode_command().await {
            Ok(()) => Ok(true),
            Err(e) => {
                warn!("OpenCode health check failed: {}", e);
                Ok(false)
            }
        }
    }

    async fn execute_with_session(
        &self,
        prompt: &str,
        config: &ExecutionConfig,
        session: &dyn AgentSession,
    ) -> Result<AgentResponse> {
        // Validate prompt first (before verifying command availability)
        if prompt.trim().is_empty() {
            return Err(anyhow::anyhow!("Prompt cannot be empty"));
        }

        info!(
            "Executing OpenCode prompt with session {} (model {})",
            session.session_id(),
            config.model
        );

        // Verify OpenCode command is available
        self.verify_opencode_command().await?;

        // Execute with retry (session is already provided)
        let response = self.execute_with_retry(prompt, config, None).await?;

        Ok(response)
    }

    async fn validate_config(&self, config: &AgentConfig) -> Result<(), AgentError> {
        // Validate using the AgentConfig's validate method
        config.validate()?;

        // Additional OpenCode-specific validation
        if config.name != "opencode" {
            return Err(AgentError::ConfigValidation {
                field: "name".to_string(),
                message: format!("Expected 'opencode', got '{}'", config.name),
            });
        }

        // Verify the command exists
        let output = tokio::process::Command::new(&config.command)
            .arg("--version")
            .output()
            .await;

        match output {
            Ok(_) => Ok(()),
            Err(_) => Err(AgentError::CommandNotFound {
                command: config.command.clone(),
            }),
        }
    }

    fn agent(&self) -> &Agent {
        &self.agent
    }
}

impl Default for OpenCodeAgent {
    fn default() -> Self {
        Self::new().expect("Failed to create OpenCode agent")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_opencode_agent_creation() {
        let agent = OpenCodeAgent::new();
        assert!(agent.is_ok());
    }

    #[tokio::test]
    async fn test_parse_structured_data() {
        let response = r#"
Some text before.

```json
{
  "tasks": [
    {"id": "1", "title": "Task 1"}
  ]
}
```

Some text after.
"#;

        let data = OpenCodeAgent::parse_structured_data(response);
        assert!(data.is_some());

        if let Some(json) = data {
            assert!(json.get("tasks").is_some());
        }
    }

    #[test]
    fn test_check_completion() {
        assert!(OpenCodeAgent::check_completion(
            "Task completed successfully"
        ));
        assert!(OpenCodeAgent::check_completion("Implementation complete"));
        assert!(OpenCodeAgent::check_completion("Success!"));
        assert!(!OpenCodeAgent::check_completion("Still working on it"));
        assert!(!OpenCodeAgent::check_completion("Error: something failed"));
    }

    #[tokio::test]
    async fn test_default_opencode_agent() {
        let agent = OpenCodeAgent::default();
        assert_eq!(agent.agent().name, "opencode");
        assert_eq!(agent.agent().model, "gpt-4");
    }
}
