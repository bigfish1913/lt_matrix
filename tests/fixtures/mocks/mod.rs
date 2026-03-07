//! Mock Agent Implementations for Testing
//!
//! This module provides mock implementations of the AgentBackend trait
//! for use in integration and unit tests. These mocks allow controlled
//! testing of pipeline behavior without requiring actual agent processes.
//!
//! # Available Mocks
//!
//! - [`MockAgent`]: Configurable mock that returns preset responses
//! - [`RecordingMockAgent`]: Records all calls for verification
//! - [`FailingMockAgent`]: Always returns errors
//! - [`DelayedMockAgent`]: Simulates slow responses
//!
//! # Usage Example
//!
//! ```rust,ignore
//! use ltmatrix::testing::mocks::{MockAgent, MockResponse};
//!
//! let mut mock = MockAgent::new();
//! mock.set_response("generate", MockResponse::success("Tasks generated"));
//! mock.set_response("execute", MockResponse::success("Task executed"));
//!
//! // Use mock in tests
//! let response = mock.execute("test prompt", &config).await?;
//! assert!(response.is_complete);
//! ```

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use ltmatrix::agent::backend::{
    AgentBackend, AgentConfig, AgentError, AgentResponse, AgentSession, ExecutionConfig,
    MemorySession,
};
use ltmatrix::models::{Agent, Task};

/// Response configuration for mock agents
#[derive(Debug, Clone)]
pub enum MockResponse {
    /// Successful response with output
    Success {
        output: String,
        structured_data: Option<serde_json::Value>,
    },

    /// Failed response with error
    Failure {
        error: String,
        error_type: MockErrorType,
    },

    /// Delayed response (simulates slow processing)
    Delayed {
        response: Box<MockResponse>,
        delay_ms: u64,
    },

    /// Sequence of responses (each call returns the next)
    Sequence { responses: Vec<MockResponse> },
}

/// Types of errors a mock can return
#[derive(Debug, Clone)]
pub enum MockErrorType {
    /// Command not found
    CommandNotFound,
    /// Execution failed
    ExecutionFailed,
    /// Timeout
    Timeout,
    /// Invalid response
    InvalidResponse,
    /// Config validation error
    ConfigValidation,
}

impl MockResponse {
    /// Create a successful response
    pub fn success(output: impl Into<String>) -> Self {
        MockResponse::Success {
            output: output.into(),
            structured_data: None,
        }
    }

    /// Create a successful response with structured data
    pub fn success_with_data(output: impl Into<String>, data: serde_json::Value) -> Self {
        MockResponse::Success {
            output: output.into(),
            structured_data: Some(data),
        }
    }

    /// Create a failure response
    pub fn failure(error: impl Into<String>) -> Self {
        MockResponse::Failure {
            error: error.into(),
            error_type: MockErrorType::ExecutionFailed,
        }
    }

    /// Create a timeout response
    pub fn timeout() -> Self {
        MockResponse::Failure {
            error: "Operation timed out".to_string(),
            error_type: MockErrorType::Timeout,
        }
    }

    /// Create a delayed response
    pub fn delayed(response: MockResponse, delay_ms: u64) -> Self {
        MockResponse::Delayed {
            response: Box::new(response),
            delay_ms,
        }
    }

    /// Convert to AgentResponse
    pub fn to_agent_response(&self) -> Result<AgentResponse, AgentError> {
        match self {
            MockResponse::Success {
                output,
                structured_data,
            } => Ok(AgentResponse {
                output: output.clone(),
                structured_data: structured_data.clone(),
                is_complete: true,
                error: None,
            }),
            MockResponse::Failure { error, error_type } => {
                let agent_error = match error_type {
                    MockErrorType::CommandNotFound => AgentError::CommandNotFound {
                        command: "mock-agent".to_string(),
                    },
                    MockErrorType::ExecutionFailed => AgentError::ExecutionFailed {
                        command: "mock-agent".to_string(),
                        message: error.clone(),
                    },
                    MockErrorType::Timeout => AgentError::Timeout {
                        command: "mock-agent".to_string(),
                        timeout_secs: 60,
                    },
                    MockErrorType::InvalidResponse => AgentError::InvalidResponse {
                        reason: error.clone(),
                    },
                    MockErrorType::ConfigValidation => AgentError::ConfigValidation {
                        field: "mock".to_string(),
                        message: error.clone(),
                    },
                };
                Err(agent_error.into())
            }
            MockResponse::Delayed { .. } => {
                // Delayed responses should be handled separately
                Ok(AgentResponse::default())
            }
            MockResponse::Sequence { .. } => {
                // Sequence responses should be handled separately
                Ok(AgentResponse::default())
            }
        }
    }
}

/// Record of a mock agent call
#[derive(Debug, Clone)]
pub struct MockCall {
    /// The prompt that was sent
    pub prompt: String,
    /// The execution config used
    pub config: ExecutionConfig,
    /// Session ID if session was used
    pub session_id: Option<String>,
    /// Timestamp of the call
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Configurable mock agent implementation
pub struct MockAgent {
    /// Agent configuration
    agent: Agent,
    /// Responses keyed by operation type
    responses: HashMap<String, MockResponse>,
    /// Default response when no specific match
    default_response: MockResponse,
    /// Whether health checks should succeed
    healthy: bool,
    /// Record of all calls made
    calls: Arc<Mutex<Vec<MockCall>>>,
    /// Call counter for sequences
    call_counts: Arc<Mutex<HashMap<String, usize>>>,
}

impl MockAgent {
    /// Create a new mock agent
    pub fn new() -> Self {
        MockAgent {
            agent: Agent {
                name: "mock".to_string(),
                model: "mock-model".to_string(),
                command: "mock-command".to_string(),
                timeout_secs: 60,
                max_retries: 3,
                enable_session: false,
            },
            responses: HashMap::new(),
            default_response: MockResponse::success("Default mock response"),
            healthy: true,
            calls: Arc::new(Mutex::new(Vec::new())),
            call_counts: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Set the response for a specific operation
    pub fn set_response(&mut self, operation: impl Into<String>, response: MockResponse) {
        self.responses.insert(operation.into(), response);
    }

    /// Set the default response
    pub fn set_default_response(&mut self, response: MockResponse) {
        self.default_response = response;
    }

    /// Set whether health checks succeed
    pub fn set_healthy(&mut self, healthy: bool) {
        self.healthy = healthy;
    }

    /// Get all recorded calls
    pub fn get_calls(&self) -> Vec<MockCall> {
        self.calls.lock().unwrap().clone()
    }

    /// Get the number of calls made
    pub fn call_count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }

    /// Clear recorded calls
    pub fn clear_calls(&self) {
        self.calls.lock().unwrap().clear();
    }

    /// Record a call
    fn record_call(&self, prompt: &str, config: &ExecutionConfig, session_id: Option<&str>) {
        let call = MockCall {
            prompt: prompt.to_string(),
            config: config.clone(),
            session_id: session_id.map(String::from),
            timestamp: chrono::Utc::now(),
        };
        self.calls.lock().unwrap().push(call);
    }

    /// Get response for an operation
    fn get_response(&self, operation: &str) -> MockResponse {
        let mut counts = self.call_counts.lock().unwrap();
        let count = counts.entry(operation.to_string()).or_insert(0);

        if let Some(response) = self.responses.get(operation) {
            match response {
                MockResponse::Sequence { responses } => {
                    if *count < responses.len() {
                        let resp = responses[*count].clone();
                        *count += 1;
                        return resp;
                    }
                }
                _ => return response.clone(),
            }
        }
        self.default_response.clone()
    }
}

impl Default for MockAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentBackend for MockAgent {
    async fn execute(
        &self,
        prompt: &str,
        config: &ExecutionConfig,
    ) -> anyhow::Result<AgentResponse> {
        self.record_call(prompt, config, None);

        let response = self.get_response("execute");

        match response {
            MockResponse::Delayed {
                response: inner,
                delay_ms,
            } => {
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                inner.to_agent_response()
            }
            _ => response.to_agent_response(),
        }
    }

    async fn execute_with_session(
        &self,
        prompt: &str,
        config: &ExecutionConfig,
        session: &dyn AgentSession,
    ) -> anyhow::Result<AgentResponse> {
        self.record_call(prompt, config, Some(session.session_id()));

        let response = self.get_response("execute_with_session");

        match response {
            MockResponse::Delayed {
                response: inner,
                delay_ms,
            } => {
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                inner.to_agent_response()
            }
            _ => response.to_agent_response(),
        }
    }

    async fn execute_task(
        &self,
        task: &Task,
        context: &str,
        config: &ExecutionConfig,
    ) -> anyhow::Result<AgentResponse> {
        let prompt = format!("Task: {}\nContext: {}", task.title, context);
        self.record_call(&prompt, config, None);

        let response = self.get_response("execute_task");

        match response {
            MockResponse::Delayed {
                response: inner,
                delay_ms,
            } => {
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                inner.to_agent_response()
            }
            _ => response.to_agent_response(),
        }
    }

    async fn health_check(&self) -> anyhow::Result<bool> {
        Ok(self.healthy)
    }

    async fn validate_config(&self, _config: &AgentConfig) -> Result<(), AgentError> {
        if self.healthy {
            Ok(())
        } else {
            Err(AgentError::ConfigValidation {
                field: "mock".to_string(),
                message: "Mock agent is not healthy".to_string(),
            })
        }
    }

    fn agent(&self) -> &Agent {
        &self.agent
    }
}

/// A mock agent that always fails
pub struct FailingMockAgent {
    /// The error to return
    error: AgentError,
}

impl FailingMockAgent {
    /// Create a new failing mock agent
    pub fn new(error: AgentError) -> Self {
        FailingMockAgent { error }
    }

    /// Create a mock that fails with command not found
    pub fn command_not_found() -> Self {
        FailingMockAgent::new(AgentError::CommandNotFound {
            command: "mock-agent".to_string(),
        })
    }

    /// Create a mock that fails with timeout
    pub fn timeout() -> Self {
        FailingMockAgent::new(AgentError::Timeout {
            command: "mock-agent".to_string(),
            timeout_secs: 60,
        })
    }

    /// Create a mock that fails with execution error
    pub fn execution_failed(message: impl Into<String>) -> Self {
        FailingMockAgent::new(AgentError::ExecutionFailed {
            command: "mock-agent".to_string(),
            message: message.into(),
        })
    }
}

#[async_trait]
impl AgentBackend for FailingMockAgent {
    async fn execute(
        &self,
        _prompt: &str,
        _config: &ExecutionConfig,
    ) -> anyhow::Result<AgentResponse> {
        Err(anyhow::anyhow!("{}", self.error))
    }

    async fn execute_with_session(
        &self,
        _prompt: &str,
        _config: &ExecutionConfig,
        _session: &dyn AgentSession,
    ) -> anyhow::Result<AgentResponse> {
        Err(anyhow::anyhow!("{}", self.error))
    }

    async fn execute_task(
        &self,
        _task: &Task,
        _context: &str,
        _config: &ExecutionConfig,
    ) -> anyhow::Result<AgentResponse> {
        Err(anyhow::anyhow!("{}", self.error))
    }

    async fn health_check(&self) -> anyhow::Result<bool> {
        Ok(false)
    }

    async fn validate_config(&self, _config: &AgentConfig) -> Result<(), AgentError> {
        Err(self.error.clone())
    }

    fn agent(&self) -> &Agent {
        static AGENT: std::sync::OnceLock<Agent> = std::sync::OnceLock::new();
        AGENT.get_or_init(|| Agent {
            name: "failing-mock".to_string(),
            model: "none".to_string(),
            command: "none".to_string(),
            timeout_secs: 0,
            max_retries: 0,
            enable_session: false,
        })
    }
}

/// A mock agent that simulates delays
pub struct DelayedMockAgent {
    /// The underlying mock agent
    inner: MockAgent,
    /// Delay to apply to all operations
    delay: Duration,
}

impl DelayedMockAgent {
    /// Create a new delayed mock agent
    pub fn new(delay: Duration) -> Self {
        DelayedMockAgent {
            inner: MockAgent::new(),
            delay,
        }
    }

    /// Create with millisecond delay
    pub fn from_millis(millis: u64) -> Self {
        Self::new(Duration::from_millis(millis))
    }

    /// Set the response for an operation
    pub fn set_response(&mut self, operation: impl Into<String>, response: MockResponse) {
        self.inner.set_response(operation, response);
    }
}

#[async_trait]
impl AgentBackend for DelayedMockAgent {
    async fn execute(
        &self,
        prompt: &str,
        config: &ExecutionConfig,
    ) -> anyhow::Result<AgentResponse> {
        tokio::time::sleep(self.delay).await;
        self.inner.execute(prompt, config).await
    }

    async fn execute_with_session(
        &self,
        prompt: &str,
        config: &ExecutionConfig,
        session: &dyn AgentSession,
    ) -> anyhow::Result<AgentResponse> {
        tokio::time::sleep(self.delay).await;
        self.inner.execute_with_session(prompt, config, session).await
    }

    async fn execute_task(
        &self,
        task: &Task,
        context: &str,
        config: &ExecutionConfig,
    ) -> anyhow::Result<AgentResponse> {
        tokio::time::sleep(self.delay).await;
        self.inner.execute_task(task, context, config).await
    }

    async fn health_check(&self) -> anyhow::Result<bool> {
        tokio::time::sleep(self.delay).await;
        self.inner.health_check().await
    }

    async fn validate_config(&self, config: &AgentConfig) -> Result<(), AgentError> {
        self.inner.validate_config(config).await
    }

    fn agent(&self) -> &Agent {
        self.inner.agent()
    }
}

/// Builder for creating mock agents with specific behaviors
pub struct MockAgentBuilder {
    agent: MockAgent,
}

impl MockAgentBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        MockAgentBuilder {
            agent: MockAgent::new(),
        }
    }

    /// Set the agent name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.agent.agent.name = name.into();
        self
    }

    /// Set the model
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.agent.agent.model = model.into();
        self
    }

    /// Set response for generate operation
    pub fn generate_response(mut self, response: MockResponse) -> Self {
        self.agent.set_response("generate", response);
        self
    }

    /// Set response for execute operation
    pub fn execute_response(mut self, response: MockResponse) -> Self {
        self.agent.set_response("execute", response);
        self
    }

    /// Set response for execute_task operation
    pub fn task_response(mut self, response: MockResponse) -> Self {
        self.agent.set_response("execute_task", response);
        self
    }

    /// Set response for verify operation
    pub fn verify_response(mut self, response: MockResponse) -> Self {
        self.agent.set_response("verify", response);
        self
    }

    /// Set default response for unmatched operations
    pub fn default_response(mut self, response: MockResponse) -> Self {
        self.agent.set_default_response(response);
        self
    }

    /// Set health status
    pub fn healthy(mut self, healthy: bool) -> Self {
        self.agent.set_healthy(healthy);
        self
    }

    /// Build the mock agent
    pub fn build(self) -> MockAgent {
        self.agent
    }
}

impl Default for MockAgentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_agent_success() {
        let mut mock = MockAgent::new();
        mock.set_response("execute", MockResponse::success("Test output"));

        let config = ExecutionConfig::default();
        let result = mock.execute("test prompt", &config).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.output, "Test output");
        assert!(response.is_complete);
    }

    #[tokio::test]
    async fn test_mock_agent_failure() {
        let mut mock = MockAgent::new();
        mock.set_response("execute", MockResponse::failure("Test error"));

        let config = ExecutionConfig::default();
        let result = mock.execute("test prompt", &config).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_agent_records_calls() {
        let mock = MockAgent::new();

        let config = ExecutionConfig::default();
        let _ = mock.execute("first prompt", &config).await;
        let _ = mock.execute("second prompt", &config).await;

        assert_eq!(mock.call_count(), 2);

        let calls = mock.get_calls();
        assert_eq!(calls[0].prompt, "first prompt");
        assert_eq!(calls[1].prompt, "second prompt");
    }

    #[tokio::test]
    async fn test_mock_agent_sequence() {
        let mut mock = MockAgent::new();
        mock.set_response(
            "execute",
            MockResponse::Sequence {
                responses: vec![
                    MockResponse::success("First"),
                    MockResponse::success("Second"),
                    MockResponse::failure("Done"),
                ],
            },
        );

        let config = ExecutionConfig::default();

        let r1 = mock.execute("prompt", &config).await.unwrap();
        assert_eq!(r1.output, "First");

        let r2 = mock.execute("prompt", &config).await.unwrap();
        assert_eq!(r2.output, "Second");

        let r3 = mock.execute("prompt", &config).await;
        assert!(r3.is_err());
    }

    #[tokio::test]
    async fn test_mock_agent_builder() {
        let mock = MockAgentBuilder::new()
            .name("test-agent")
            .model("test-model")
            .execute_response(MockResponse::success("Built response"))
            .build();

        assert_eq!(mock.agent().name, "test-agent");
        assert_eq!(mock.agent().model, "test-model");

        let config = ExecutionConfig::default();
        let response = mock.execute("test", &config).await.unwrap();
        assert_eq!(response.output, "Built response");
    }

    #[tokio::test]
    async fn test_failing_mock_agent() {
        let mock = FailingMockAgent::execution_failed("Always fails");

        let config = ExecutionConfig::default();
        let result = mock.execute("any prompt", &config).await;

        assert!(result.is_err());
        assert!(mock.health_check().await.is_ok());
        assert!(!mock.health_check().await.unwrap());
    }

    #[tokio::test]
    async fn test_delayed_mock_agent() {
        let mock = DelayedMockAgent::from_millis(50);

        let config = ExecutionConfig::default();
        let start = std::time::Instant::now();
        let _ = mock.execute("test", &config).await;
        let elapsed = start.elapsed();

        assert!(elapsed >= Duration::from_millis(50));
    }
}
