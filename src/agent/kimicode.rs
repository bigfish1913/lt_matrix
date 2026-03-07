// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.


//! KimiCode agent backend implementation
//!
//! This module implements the [`AgentBackend`] trait for the KimiCode CLI tool
//! (`kimi-code`) by 月之暗面 (Moonshot AI).
//!
//! # KimiCode CLI overview
//!
//! KimiCode is invoked via the `kimi-code` binary.  The supported interface is:
//!
//! ```text
//! kimi-code [--model <MODEL>] [--prompt -]
//! ```
//!
//! Prompts are fed through **stdin** (marker `"-"`) and the tool writes its
//! response to **stdout**.  Any diagnostic or error output appears on **stderr**.
//!
//! # Models
//!
//! | Model                | Description                         |
//! |----------------------|-------------------------------------|
//! | `moonshot-v1-128k`   | Default — long-context 128k model   |
//! | `moonshot-v1-32k`    | Faster 32k context window           |
//! | `moonshot-v1-8k`     | Fastest, short-context tasks        |
//!
//! # Session support
//!
//! KimiCode does not expose a persistent process session protocol in the same
//! way as Claude Code.  Sessions are therefore tracked in the shared
//! [`SessionManager`] for audit/reuse accounting, but each invocation spawns
//! a fresh subprocess.
//!
//! # Response parsing
//!
//! KimiCode may include structured output in fenced `` ```json `` blocks.
//! The parser extracts the first such block and attempts JSON deserialisation.
//! Plain-text responses are returned verbatim as `output`.
//!
//! # Completion detection
//!
//! The backend scans the output for positive completion phrases (e.g.
//! "task completed", "done", "finished") while first excluding known negative
//! patterns (e.g. "not done", "error", "failed").

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

// ---------------------------------------------------------------------------
// KimiCodeAgent
// ---------------------------------------------------------------------------

/// KimiCode agent backend.
///
/// Wraps the `kimi-code` CLI and exposes it through the [`AgentBackend`]
/// trait so it can be used anywhere a generic agent is expected.
#[derive(Debug, Clone)]
pub struct KimiCodeAgent {
    /// Agent model / command metadata.
    agent: Agent,

    /// Disk-backed session manager for reuse accounting.
    session_manager: SessionManager,

    /// When `false` the `kimi-code --version` pre-flight is skipped.
    /// Useful in unit-test environments where the CLI is not installed.
    verify_command: bool,
}

impl KimiCodeAgent {
    /// Create a [`KimiCodeAgent`] with the default KimiCode configuration
    /// (`kimi-code`, model `moonshot-v1-128k`, timeout 3600 s).
    pub fn new() -> Result<Self> {
        let agent = Agent::kimicode_default();
        let session_manager =
            SessionManager::default_manager().context("Failed to create session manager")?;

        Ok(KimiCodeAgent {
            agent,
            session_manager,
            verify_command: true,
        })
    }

    /// Create a [`KimiCodeAgent`] from an arbitrary [`Agent`] and
    /// [`SessionManager`].  Used by [`AgentFactory`].
    ///
    /// [`AgentFactory`]: crate::agent::factory::AgentFactory
    pub fn with_agent(agent: Agent, session_manager: SessionManager) -> Self {
        KimiCodeAgent {
            agent,
            session_manager,
            verify_command: true,
        }
    }

    /// Disable the `kimi-code --version` availability check.
    ///
    /// Useful when running tests in CI environments where the CLI is not
    /// installed, so tests can exercise all other logic paths.
    pub fn without_verification(mut self) -> Self {
        self.verify_command = false;
        self
    }

    // ── private helpers ──────────────────────────────────────────────────────

    /// Run `kimi-code --version` to confirm the binary is on PATH.
    async fn verify_kimicode_command(&self) -> Result<()> {
        if !self.verify_command {
            return Ok(());
        }

        let output = Command::new(&self.agent.command)
            .arg("--version")
            .output()
            .await;

        match output {
            Ok(out) if out.status.success() => {
                debug!(
                    "KimiCode command verified: {}",
                    String::from_utf8_lossy(&out.stdout).trim()
                );
                Ok(())
            }
            Ok(out) => Err(anyhow!(
                "KimiCode command returned error status: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            )),
            Err(e) => Err(anyhow!(
                "KimiCode command '{}' not found: {}. \
                 Please install the kimi-code CLI.",
                self.agent.command,
                e
            )),
        }
    }

    /// Construct the argument list for a single `kimi-code` invocation.
    ///
    /// The prompt is always written to stdin; `"-"` signals the CLI to read
    /// from stdin.  Model override is included only when it differs from the
    /// agent's configured default.
    fn build_command(&self, config: &ExecutionConfig) -> Vec<String> {
        let mut args = vec![
            self.agent.command.clone(),
            "--prompt".to_string(),
            "-".to_string(), // read from stdin
        ];

        // Override the model only when explicitly requested via ExecutionConfig.
        if config.model != self.agent.model {
            args.push("--model".to_string());
            args.push(config.model.clone());
        }

        args
    }

    /// Spawn `kimi-code`, write `prompt` to its stdin, collect stdout/stderr
    /// concurrently within `config.timeout` seconds, and parse the response.
    async fn execute_single_attempt(
        &self,
        prompt: &str,
        config: &ExecutionConfig,
        _session: Option<&SessionData>,
    ) -> Result<AgentResponse> {
        let args = self.build_command(config);

        debug!(
            "Spawning KimiCode process: {} with model {}",
            self.agent.command, config.model
        );

        let mut child = Command::new(&args[0])
            .args(&args[1..])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true) // clean up on timeout / early return
            .spawn()
            .context("Failed to spawn KimiCode process")?;

        // Feed the prompt then close stdin so kimi-code sees EOF.
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(prompt.as_bytes())
                .await
                .context("Failed to write prompt to KimiCode stdin")?;
            stdin
                .shutdown()
                .await
                .context("Failed to close KimiCode stdin")?;
        }

        let stdout = child.stdout.take().expect("stdout pipe missing");
        let stderr = child.stderr.take().expect("stderr pipe missing");

        // Read stdout and stderr concurrently to prevent deadlocks when one
        // buffer fills while the process is still writing to the other.
        let stdout_fut = async {
            let mut reader = BufReader::new(stdout).lines();
            let mut buf = String::new();
            while let Some(line) = reader.next_line().await.unwrap_or(None) {
                buf.push_str(&line);
                buf.push('\n');
            }
            buf
        };

        let stderr_fut = async {
            let mut reader = BufReader::new(stderr).lines();
            let mut buf = String::new();
            while let Some(line) = reader.next_line().await.unwrap_or(None) {
                buf.push_str(&line);
                buf.push('\n');
            }
            buf
        };

        let timeout_duration = Duration::from_secs(config.timeout);
        let (stdout_text, stderr_text) = tokio::time::timeout(timeout_duration, async {
            tokio::join!(stdout_fut, stderr_fut)
        })
        .await
        .context("KimiCode execution timed out")?;

        let status = child
            .wait()
            .await
            .context("Failed to wait for KimiCode process")?;

        debug!("KimiCode process exited with status: {}", status);

        let error = if !status.success() {
            Some(format!(
                "KimiCode exited with status {}: {}",
                status,
                stderr_text.trim()
            ))
        } else if !stderr_text.trim().is_empty() {
            Some(format!("KimiCode stderr: {}", stderr_text.trim()))
        } else {
            None
        };

        let structured_data = Self::parse_structured_data(&stdout_text);
        let is_complete = Self::check_completion(&stdout_text);

        Ok(AgentResponse {
            output: stdout_text,
            structured_data,
            is_complete,
            error,
        })
    }

    /// Retry wrapper with exponential back-off.
    ///
    /// Attempt 0 is the initial attempt.  Retries start at attempt 1 and
    /// wait `100 * 2^(attempt-1)` milliseconds before the next try.
    async fn execute_with_retry(
        &self,
        prompt: &str,
        config: &ExecutionConfig,
        session: Option<&SessionData>,
    ) -> Result<AgentResponse> {
        let mut last_error = None;

        for attempt in 0..=config.max_retries {
            if attempt > 0 {
                let delay = Duration::from_millis(100 * 2_u64.pow(attempt - 1));
                warn!("KimiCode retry attempt {} after {:?}", attempt, delay);
                tokio::time::sleep(delay).await;
            }

            match self.execute_single_attempt(prompt, config, session).await {
                Ok(response) => {
                    if attempt > 0 {
                        info!("KimiCode execution succeeded on attempt {}", attempt + 1);
                    }
                    return Ok(response);
                }
                Err(e) => {
                    warn!("KimiCode attempt {} failed: {}", attempt + 1, e);
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("All KimiCode retry attempts exhausted")))
    }

    /// Obtain (or create) a session for the current execution, when session
    /// reuse is enabled in `config`.
    async fn get_session(&self, config: &ExecutionConfig) -> Result<Option<SessionData>> {
        if !config.enable_session {
            return Ok(None);
        }

        let session = self
            .session_manager
            .create_session("kimicode", &config.model)
            .await?;

        debug!("KimiCode: created session {}", session.session_id);
        Ok(Some(session))
    }

    // ── response-parsing helpers (pub for testing) ───────────────────────────

    /// Extract the first `` ```json … ``` `` block from `output` and parse it.
    ///
    /// Returns `None` when no block is found or the JSON is malformed.
    pub fn parse_structured_data(output: &str) -> Option<serde_json::Value> {
        let json_start = output.find("```json")?;
        let content_start = json_start + 7; // skip past "```json"
        let json_end = output[content_start..].find("```")?;
        let json_str = &output[content_start..content_start + json_end];
        serde_json::from_str(json_str.trim()).ok()
    }

    /// Return `true` when `output` contains a positive completion phrase and
    /// does **not** contain a negating phrase.
    ///
    /// Negative phrases take precedence: if both "done" and "not done" appear,
    /// the function returns `false`.
    pub fn check_completion(output: &str) -> bool {
        let lower = output.to_lowercase();

        // Negative patterns — any one of these makes the result false.
        let negative_patterns = [
            "not done",
            "not finished",
            "not complete",
            "not completed",
            "incomplete",
            "error",
            "failed",
        ];
        if negative_patterns.iter().any(|p| lower.contains(p)) {
            return false;
        }

        // Positive completion indicators.
        let positive_patterns = [
            "task completed",
            "implementation complete",
            "done",
            "finished",
            "complete",
            "completed",
            "success",
        ];
        positive_patterns.iter().any(|p| lower.contains(p))
    }
}

// ---------------------------------------------------------------------------
// AgentBackend implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl AgentBackend for KimiCodeAgent {
    /// Execute `prompt` against KimiCode with the given runtime configuration.
    ///
    /// Rejects empty / whitespace-only prompts immediately.  Verifies the CLI
    /// binary is available, obtains a session, then delegates to the retry
    /// helper.
    async fn execute(&self, prompt: &str, config: &ExecutionConfig) -> Result<AgentResponse> {
        if prompt.trim().is_empty() {
            return Err(anyhow!("Prompt cannot be empty"));
        }

        info!("Executing KimiCode prompt with model {}", config.model);

        self.verify_kimicode_command().await?;

        let session = self.get_session(config).await?;
        self.execute_with_retry(prompt, config, session.as_ref())
            .await
    }

    /// Execute `prompt` using an externally-managed session.
    ///
    /// Empty / whitespace-only prompts are rejected before the CLI check so
    /// callers get a clear error without needing the binary installed.
    async fn execute_with_session(
        &self,
        prompt: &str,
        config: &ExecutionConfig,
        session: &dyn AgentSession,
    ) -> Result<AgentResponse> {
        if prompt.trim().is_empty() {
            return Err(anyhow!("Prompt cannot be empty"));
        }

        info!(
            "Executing KimiCode prompt with session {} (model {})",
            session.session_id(),
            config.model
        );

        self.verify_kimicode_command().await?;

        // The external session is used for accounting; subprocess invocations
        // are still independent (KimiCode CLI does not support session attach).
        self.execute_with_retry(prompt, config, None).await
    }

    /// Format the task as a structured prompt and execute it.
    async fn execute_task(
        &self,
        task: &crate::models::Task,
        context: &str,
        config: &ExecutionConfig,
    ) -> Result<AgentResponse> {
        let prompt = format!(
            "Task: {}\n\nDescription: {}\n\nContext:\n{}\n\nPlease complete this task.",
            task.title, task.description, context
        );
        self.execute(&prompt, config).await
    }

    /// Return `Ok(true)` if `kimi-code --version` exits successfully, else
    /// log a warning and return `Ok(false)`.  Never panics.
    async fn health_check(&self) -> Result<bool> {
        match self.verify_kimicode_command().await {
            Ok(()) => Ok(true),
            Err(e) => {
                warn!("KimiCode health check failed: {}", e);
                Ok(false)
            }
        }
    }

    /// Validate `config` against the generic rules and KimiCode-specific rules.
    ///
    /// KimiCode-specific rules:
    /// - `config.name` must be `"kimicode"`
    /// - `config.command` binary must exist on PATH (checked via `--version`)
    async fn validate_config(&self, config: &AgentConfig) -> Result<(), AgentError> {
        // Generic field checks (empty name/model/command, zero timeout, …).
        config.validate()?;

        // KimiCode-specific: the config must be for this backend.
        if config.name != "kimicode" {
            return Err(AgentError::ConfigValidation {
                field: "name".to_string(),
                message: format!("Expected 'kimicode', got '{}'", config.name),
            });
        }

        // Verify the binary exists.
        let result = tokio::process::Command::new(&config.command)
            .arg("--version")
            .output()
            .await;

        match result {
            Ok(_) => Ok(()),
            Err(_) => Err(AgentError::CommandNotFound {
                command: config.command.clone(),
            }),
        }
    }

    /// Return the underlying [`Agent`] configuration.
    fn agent(&self) -> &Agent {
        &self.agent
    }
}

impl Default for KimiCodeAgent {
    fn default() -> Self {
        Self::new().expect("Failed to create KimiCode agent")
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_kimicode_agent_creation() {
        let agent = KimiCodeAgent::new();
        assert!(agent.is_ok());
    }

    #[test]
    fn test_kimicode_default_fields() {
        let agent = KimiCodeAgent::default();
        assert_eq!(agent.agent().name, "kimicode");
        assert_eq!(agent.agent().command, "kimi-code");
        assert_eq!(agent.agent().model, "moonshot-v1-128k");
        assert_eq!(agent.agent().timeout, 3600);
    }

    #[test]
    fn test_backend_name() {
        let agent = KimiCodeAgent::default();
        assert_eq!(agent.backend_name(), "kimicode");
    }

    #[test]
    fn test_parse_structured_data_valid_json() {
        let output = r#"
Here is the result:

```json
{"tasks": [{"id": "1", "title": "Init"}]}
```

Done.
"#;
        let data = KimiCodeAgent::parse_structured_data(output);
        assert!(data.is_some());
        assert!(data.unwrap().get("tasks").is_some());
    }

    #[test]
    fn test_parse_structured_data_no_block() {
        assert!(KimiCodeAgent::parse_structured_data("No JSON here").is_none());
    }

    #[test]
    fn test_parse_structured_data_malformed_json() {
        let bad = "```json\n{ not valid json\n```";
        assert!(KimiCodeAgent::parse_structured_data(bad).is_none());
    }

    #[test]
    fn test_parse_structured_data_nested() {
        let output = "```json\n{\"a\":{\"b\":[1,2,3]}}\n```";
        let data = KimiCodeAgent::parse_structured_data(output);
        assert!(data.is_some());
    }

    #[test]
    fn test_check_completion_positive() {
        assert!(KimiCodeAgent::check_completion(
            "Task completed successfully."
        ));
        assert!(KimiCodeAgent::check_completion("Implementation complete."));
        assert!(KimiCodeAgent::check_completion("done"));
        assert!(KimiCodeAgent::check_completion("finished"));
        assert!(KimiCodeAgent::check_completion("Success!"));
    }

    #[test]
    fn test_check_completion_negative_overrides_positive() {
        // "not done" must win over "done"
        assert!(!KimiCodeAgent::check_completion("not done yet"));
        assert!(!KimiCodeAgent::check_completion("task not completed"));
        assert!(!KimiCodeAgent::check_completion(
            "Error: something went wrong"
        ));
        assert!(!KimiCodeAgent::check_completion("Build failed!"));
        assert!(!KimiCodeAgent::check_completion(
            "incomplete implementation"
        ));
    }

    #[test]
    fn test_check_completion_neutral() {
        // Neither positive nor negative
        assert!(!KimiCodeAgent::check_completion("Still working on it"));
        assert!(!KimiCodeAgent::check_completion(""));
    }

    #[test]
    fn test_check_completion_case_insensitive() {
        assert!(KimiCodeAgent::check_completion("DONE"));
        assert!(KimiCodeAgent::check_completion("TASK COMPLETED"));
        assert!(!KimiCodeAgent::check_completion("ERROR"));
    }

    #[tokio::test]
    async fn test_execute_rejects_empty_prompt() {
        let agent = KimiCodeAgent::default().without_verification();
        let config = ExecutionConfig::default();
        let result = agent.execute("", &config).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .to_lowercase()
            .contains("empty"));
    }

    #[tokio::test]
    async fn test_execute_rejects_whitespace_prompt() {
        let agent = KimiCodeAgent::default().without_verification();
        let config = ExecutionConfig::default();
        let result = agent.execute("   \t\n  ", &config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_execute_with_session_rejects_empty_prompt() {
        let agent = KimiCodeAgent::default().without_verification();
        let config = ExecutionConfig::default();
        let session = crate::agent::backend::MemorySession::default();
        let result = agent.execute_with_session("", &config, &session).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_health_check_returns_ok() {
        // health_check must never return Err — only Ok(bool)
        let agent = KimiCodeAgent::default();
        let result = agent.health_check().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_is_available_returns_bool() {
        let agent = KimiCodeAgent::default();
        // is_available is a convenience wrapper around health_check
        let _available: bool = agent.is_available().await;
    }

    #[tokio::test]
    async fn test_validate_config_wrong_name() {
        let agent = KimiCodeAgent::default();
        let config = AgentConfig::builder()
            .name("claude")
            .command("kimi-code")
            .model("moonshot-v1-128k")
            .timeout_secs(3600)
            .build();

        let result = agent.validate_config(&config).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AgentError::ConfigValidation { .. }
        ));
    }

    #[tokio::test]
    async fn test_validate_config_empty_model() {
        let agent = KimiCodeAgent::default();
        let config = AgentConfig::builder()
            .name("kimicode")
            .command("kimi-code")
            .model("")
            .timeout_secs(3600)
            .build();

        let result = agent.validate_config(&config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_config_zero_timeout() {
        let agent = KimiCodeAgent::default();
        let config = AgentConfig {
            name: "kimicode".to_string(),
            command: "kimi-code".to_string(),
            model: "moonshot-v1-128k".to_string(),
            timeout_secs: 0,
            max_retries: 3,
            enable_session: true,
        };

        let result = agent.validate_config(&config).await;
        assert!(result.is_err());
    }
}
