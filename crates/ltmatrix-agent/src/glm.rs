// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! GLM agent backend implementation
//!
//! This module implements the AgentBackend trait for Zhipu AI's GLM models,
//! using direct HTTP API calls instead of CLI.

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, info, warn};

use crate::backend::{
    AgentBackend, AgentConfig, AgentError, AgentResponse, AgentSession, ExecutionConfig,
};
use crate::session::{SessionData, SessionManager};
use ltmatrix_core::Agent;

/// GLM API endpoint
const GLM_API_URL: &str = "https://open.bigmodel.cn/api/paas/v4/chat/completions";

/// Default GLM model
const DEFAULT_GLM_MODEL: &str = "glm-4-flash";

/// GLM agent implementation
#[derive(Debug, Clone)]
pub struct GlmAgent {
    /// Agent configuration
    agent: Agent,

    /// Session manager for reuse
    session_manager: SessionManager,

    /// HTTP client
    client: Client,

    /// API key (from environment or config)
    api_key: Option<String>,
}

/// GLM API request
#[derive(Debug, Serialize)]
struct GlmRequest {
    model: String,
    messages: Vec<GlmMessage>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
}

/// GLM API message
#[derive(Debug, Serialize, Clone, Deserialize)]
struct GlmMessage {
    role: String,
    content: String,
}

/// GLM API response
#[derive(Debug, Deserialize)]
struct GlmResponse {
    choices: Vec<GlmChoice>,
    #[serde(default)]
    usage: Option<GlmUsage>,
    #[serde(default)]
    error: Option<GlmError>,
}

/// GLM API choice
#[derive(Debug, Deserialize)]
struct GlmChoice {
    message: GlmMessage,
    finish_reason: Option<String>,
}

/// GLM API usage
#[derive(Debug, Deserialize)]
struct GlmUsage {
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
}

/// GLM API error
#[derive(Debug, Deserialize)]
struct GlmError {
    message: String,
    #[serde(rename = "type")]
    error_type: String,
}

impl GlmAgent {
    /// Create a new GLM agent with default configuration
    pub fn new() -> Result<Self> {
        let agent = Agent {
            name: "glm".to_string(),
            command: "glm".to_string(),
            model: DEFAULT_GLM_MODEL.to_string(),
            timeout: 3600,
            api_key: None,
            is_default: false,
        };

        let session_manager =
            SessionManager::default_manager().context("Failed to create session manager")?;

        // Get API key from environment
        let api_key = std::env::var("ZHIPU_API_KEY")
            .or_else(|_| std::env::var("GLM_API_KEY"))
            .ok();

        let client = Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(GlmAgent {
            agent,
            session_manager,
            client,
            api_key,
        })
    }

    /// Create a GLM agent with custom configuration
    pub fn with_agent(agent: Agent, session_manager: SessionManager) -> Self {
        // Get API key from environment or agent config
        let api_key = agent
            .api_key
            .clone()
            .or_else(|| std::env::var("ZHIPU_API_KEY").ok())
            .or_else(|| std::env::var("GLM_API_KEY").ok());

        let client = Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .expect("Failed to create HTTP client");

        GlmAgent {
            agent,
            session_manager,
            client,
            api_key,
        }
    }

    /// Set API key
    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.api_key = Some(api_key);
        self
    }

    /// Get API key
    fn get_api_key(&self) -> Result<&str> {
        self.api_key
            .as_deref()
            .ok_or_else(|| anyhow!("GLM API key not configured. Set ZHIPU_API_KEY or GLM_API_KEY environment variable"))
    }

    /// Execute a prompt with retry logic
    async fn execute_with_retry(
        &self,
        prompt: &str,
        config: &ExecutionConfig,
        _session: Option<&SessionData>,
    ) -> Result<AgentResponse> {
        let mut last_error = None;

        for attempt in 0..=config.max_retries {
            if attempt > 0 {
                let delay = Duration::from_millis(100 * 2_u64.pow(attempt - 1));
                warn!("Retry attempt {} after {:?}", attempt, delay);
                tokio::time::sleep(delay).await;
            }

            match self.execute_single_attempt(prompt, config).await {
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

    /// Execute a single API call
    async fn execute_single_attempt(
        &self,
        prompt: &str,
        config: &ExecutionConfig,
    ) -> Result<AgentResponse> {
        let api_key = self.get_api_key()?;

        let model = if config.model.is_empty() || config.model == "glm" {
            self.agent.model.clone()
        } else {
            config.model.clone()
        };

        debug!("Calling GLM API with model: {}", model);

        let request = GlmRequest {
            model: model.clone(),
            messages: vec![GlmMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            temperature: Some(0.7),
            max_tokens: Some(4096),
        };

        let response = self
            .client
            .post(GLM_API_URL)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .timeout(Duration::from_secs(config.timeout))
            .send()
            .await
            .context("Failed to send request to GLM API")?;

        let status = response.status();
        let body = response
            .text()
            .await
            .context("Failed to read response body")?;

        if !status.is_success() {
            return Err(anyhow!("GLM API error ({}): {}", status, body));
        }

        // Parse response
        let glm_response: GlmResponse =
            serde_json::from_str(&body).context("Failed to parse GLM API response")?;

        // Check for API error
        if let Some(error) = glm_response.error {
            return Err(anyhow!("GLM API error: {} ({})", error.message, error.error_type));
        }

        // Extract content
        let content = glm_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        // Log usage
        if let Some(usage) = glm_response.usage {
            debug!(
                "GLM API usage: {} prompt + {} completion = {} total tokens",
                usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
            );
        }

        // Parse structured data
        let structured_data = Self::parse_structured_data(&content);

        // Check completion
        let is_complete = Self::check_completion(&content);

        Ok(AgentResponse {
            output: content,
            structured_data,
            is_complete,
            error: None,
        })
    }

    /// Parse structured data (JSON) from response
    fn parse_structured_data(output: &str) -> Option<serde_json::Value> {
        // Try ```json block first
        if let Some(json_start) = output.find("```json") {
            let json_start = json_start + 7;
            if let Some(json_end) = output[json_start..].find("```") {
                let json_str = &output[json_start..json_start + json_end];
                if let Ok(json) = serde_json::from_str(json_str.trim()) {
                    return Some(json);
                }
            }
        }

        // Try generic ``` block
        if let Some(code_start) = output.find("```") {
            let code_start = code_start + 3;
            let rest = &output[code_start..];
            // Skip language identifier if present
            let content_start = rest.find('\n').unwrap_or(0);
            let content = &rest[content_start..];
            if let Some(code_end) = content.find("```") {
                let content = content[..code_end].trim();
                if content.starts_with('{') || content.starts_with('[') {
                    if let Ok(json) = serde_json::from_str(content) {
                        return Some(json);
                    }
                }
            }
        }

        // Try to find JSON object directly
        let mut brace_depth = 0;
        let mut json_start = None;
        let mut json_end = None;

        for (i, c) in output.chars().enumerate() {
            match c {
                '{' | '[' => {
                    if brace_depth == 0 {
                        json_start = Some(i);
                    }
                    brace_depth += 1;
                }
                '}' | ']' => {
                    brace_depth -= 1;
                    if brace_depth == 0 && json_start.is_some() {
                        json_end = Some(i + 1);
                        break;
                    }
                }
                _ => {}
            }
        }

        if let (Some(start), Some(end)) = (json_start, json_end) {
            let json_str = output[start..end].trim();
            if let Ok(json) = serde_json::from_str(json_str) {
                return Some(json);
            }
        }

        None
    }

    /// Check if response indicates task completion
    fn check_completion(output: &str) -> bool {
        let output_lower = output.to_lowercase();

        if output_lower.contains("not done")
            || output_lower.contains("not finished")
            || output_lower.contains("not complete")
            || output_lower.contains("error")
            || output_lower.contains("failed")
        {
            return false;
        }

        output_lower.contains("task completed")
            || output_lower.contains("implementation complete")
            || output_lower.contains("done")
            || output_lower.contains("finished")
            || output_lower.contains("complete")
            || output_lower.contains("success")
    }

    /// Health check - verify API key is configured
    async fn health_check_async(&self) -> Result<bool> {
        match self.get_api_key() {
            Ok(_) => Ok(true),
            Err(e) => {
                warn!("GLM health check failed: {}", e);
                Ok(false)
            }
        }
    }
}

#[async_trait]
impl AgentBackend for GlmAgent {
    async fn execute(&self, prompt: &str, config: &ExecutionConfig) -> Result<AgentResponse> {
        info!("Executing GLM prompt with model {}", config.model);

        if prompt.trim().is_empty() {
            return Err(anyhow!("Prompt cannot be empty"));
        }

        self.execute_with_retry(prompt, config, None).await
    }

    async fn execute_task(
        &self,
        task: &ltmatrix_core::Task,
        context: &str,
        config: &ExecutionConfig,
    ) -> Result<AgentResponse> {
        let prompt = format!(
            "Task: {}\n\nDescription: {}\n\nContext:\n{}\n\nPlease complete this task.",
            task.title, task.description, context
        );

        self.execute(&prompt, config).await
    }

    async fn health_check(&self) -> Result<bool> {
        self.health_check_async().await
    }

    async fn execute_with_session(
        &self,
        prompt: &str,
        config: &ExecutionConfig,
        _session: &dyn AgentSession,
    ) -> Result<AgentResponse> {
        if prompt.trim().is_empty() {
            return Err(anyhow!("Prompt cannot be empty"));
        }

        info!("Executing GLM prompt with session (model {})", config.model);

        self.execute_with_retry(prompt, config, None).await
    }

    async fn validate_config(&self, config: &AgentConfig) -> Result<(), AgentError> {
        config.validate()?;

        if config.name != "glm" {
            return Err(AgentError::ConfigValidation {
                field: "name".to_string(),
                message: format!("Expected 'glm', got '{}'", config.name),
            });
        }

        // Check API key
        if self.api_key.is_none() && std::env::var("ZHIPU_API_KEY").is_err() && std::env::var("GLM_API_KEY").is_err() {
            return Err(AgentError::ConfigValidation {
                field: "api_key".to_string(),
                message: "GLM API key not configured. Set ZHIPU_API_KEY or GLM_API_KEY environment variable".to_string(),
            });
        }

        Ok(())
    }

    fn agent(&self) -> &Agent {
        &self.agent
    }
}

impl Default for GlmAgent {
    fn default() -> Self {
        Self::new().expect("Failed to create GLM agent")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glm_agent_creation() {
        let agent = GlmAgent::new();
        assert!(agent.is_ok());
    }

    #[test]
    fn test_parse_structured_data() {
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

        let data = GlmAgent::parse_structured_data(response);
        assert!(data.is_some());
    }

    #[test]
    fn test_check_completion() {
        assert!(GlmAgent::check_completion("Task completed successfully"));
        assert!(GlmAgent::check_completion("Implementation complete"));
        assert!(!GlmAgent::check_completion("Still working on it"));
        assert!(!GlmAgent::check_completion("Error: something failed"));
    }

    #[test]
    fn test_default_glm_agent() {
        let agent = GlmAgent::default();
        assert_eq!(agent.agent().name, "glm");
    }
}
