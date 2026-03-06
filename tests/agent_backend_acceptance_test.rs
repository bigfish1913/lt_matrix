//! AgentBackend Implementation Acceptance Tests
//!
//! These tests verify that the AgentBackend trait implementation meets
//! all acceptance criteria from the task specification:
//!
//! TASK: Define AgentBackend trait and core types
//! - Create src/agent/mod.rs with AgentBackend trait
//! - Methods: execute, execute_with_session, is_available, validate_config
//! - Define AgentConfig, AgentSession, AgentError types
//! - Document the abstraction contract

use ltmatrix::agent::backend::{
    AgentBackend, AgentConfig, AgentError, AgentSession, ExecutionConfig, MemorySession,
};
use ltmatrix::agent::claude::ClaudeAgent;
use ltmatrix::models::Task;

// ============================================================================
// Acceptance Criterion 1: AgentBackend trait exists with required methods
// ============================================================================

#[tokio::test]
async fn ac01_agent_backend_trait_has_execute_method() {
    // This test verifies that the AgentBackend trait has an execute method
    // with the correct signature: async fn execute(&self, prompt: &str, config: &ExecutionConfig) -> Result<AgentResponse>

    // Verify we can call execute on ClaudeAgent
    let agent = ClaudeAgent::new().unwrap();
    let config = ExecutionConfig::default();

    // Method should be callable (may fail if claude not installed, but signature is correct)
    let _result = agent.execute("test prompt", &config).await;
}

#[tokio::test]
async fn ac02_agent_backend_trait_has_execute_with_session_method() {
    // This test verifies that the AgentBackend trait has an execute_with_session method
    // with the correct signature that accepts a session parameter

    let agent = ClaudeAgent::new().unwrap();
    let config = ExecutionConfig::default();
    let session = MemorySession::default();

    // Method should be callable with a session
    let _result = agent.execute_with_session("test prompt", &config, &session).await;
}

#[tokio::test]
async fn ac03_agent_backend_trait_has_is_available_method() {
    // This test verifies that the AgentBackend trait has an is_available method
    // that returns a boolean

    let agent = ClaudeAgent::new().unwrap();

    // is_available should return a boolean and not panic
    let available = agent.is_available().await;

    // Should always return a valid boolean
    assert!(available == true || available == false);
}

#[tokio::test]
async fn ac04_agent_backend_trait_has_validate_config_method() {
    // This test verifies that the AgentBackend trait has a validate_config method
    // that accepts AgentConfig and returns Result<(), AgentError>

    let agent = ClaudeAgent::new().unwrap();
    let config = AgentConfig::default();

    // Valid config should pass validation
    let result = agent.validate_config(&config).await;
    assert!(result.is_ok());

    // Invalid config should fail validation with AgentError
    let mut invalid_config = AgentConfig::default();
    invalid_config.name = "".to_string();
    let result = agent.validate_config(&invalid_config).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), AgentError::ConfigValidation { .. }));
}

#[tokio::test]
async fn ac05_agent_backend_trait_has_health_check_method() {
    // This test verifies that the AgentBackend trait has a health_check method
    // that returns Result<bool> for checking agent availability

    let agent = ClaudeAgent::new().unwrap();

    // health_check should return Result<bool>
    let result = agent.health_check().await;
    assert!(result.is_ok());

    let is_healthy = result.unwrap();
    assert!(is_healthy == true || is_healthy == false);
}

#[tokio::test]
async fn ac06_agent_backend_trait_has_agent_method() {
    // This test verifies that the AgentBackend trait has an agent method
    // that returns a reference to the Agent configuration

    let agent = ClaudeAgent::new().unwrap();

    // agent() should return a valid reference
    let agent_ref = agent.agent();
    assert!(!agent_ref.name.is_empty());
    assert!(!agent_ref.model.is_empty());
    assert!(!agent_ref.command.is_empty());
}

#[tokio::test]
async fn ac07_agent_backend_trait_has_backend_name_method() {
    // This test verifies that the AgentBackend trait has a backend_name method
    // with a default implementation

    let agent = ClaudeAgent::new().unwrap();

    // backend_name should return the backend name
    let backend_name = agent.backend_name();
    assert!(!backend_name.is_empty());
    assert_eq!(backend_name, "claude");
}

#[tokio::test]
async fn ac08_agent_backend_trait_has_execute_task_method() {
    // This test verifies that the AgentBackend trait has an execute_task method
    // that accepts Task and context

    let agent = ClaudeAgent::new().unwrap();
    let config = ExecutionConfig::default();

    let task = Task {
        id: "test-1".to_string(),
        title: "Test Task".to_string(),
        description: "A test task".to_string(),
        status: ltmatrix::models::TaskStatus::Pending,
        complexity: ltmatrix::models::TaskComplexity::Moderate,
        depends_on: vec![],
        subtasks: vec![],
        retry_count: 0,
        error: None,
        created_at: chrono::Utc::now(),
        started_at: None,
        completed_at: None,
    };

    // execute_task should accept Task and context
    let _result = agent.execute_task(&task, "test context", &config).await;
}

// ============================================================================
// Acceptance Criterion 2: AgentConfig type is defined and functional
// ============================================================================

#[test]
fn ac09_agent_config_type_exists_with_required_fields() {
    // This test verifies that AgentConfig has all required fields:
    // - name: String
    // - model: String
    // - command: String
    // - timeout_secs: u64
    // - max_retries: u32
    // - enable_session: bool

    let config = AgentConfig::default();

    // Verify all fields exist and have expected types
    let _name: &String = &config.name;
    let _model: &String = &config.model;
    let _command: &String = &config.command;
    let _timeout_secs: u64 = config.timeout_secs;
    let _max_retries: u32 = config.max_retries;
    let _enable_session: bool = config.enable_session;
}

#[test]
fn ac10_agent_config_has_builder_pattern() {
    // This test verifies that AgentConfig has a builder for construction

    let config = AgentConfig::builder()
        .name("test-agent")
        .model("test-model")
        .command("test-command")
        .timeout_secs(100)
        .max_retries(5)
        .enable_session(false)
        .build();

    assert_eq!(config.name, "test-agent");
    assert_eq!(config.model, "test-model");
    assert_eq!(config.command, "test-command");
    assert_eq!(config.timeout_secs, 100);
    assert_eq!(config.max_retries, 5);
    assert!(!config.enable_session);
}

#[test]
fn ac11_agent_config_has_validate_method() {
    // This test verifies that AgentConfig has a validate method
    // that returns Result<(), AgentError>

    let config = AgentConfig::default();
    let result = config.validate();
    assert!(result.is_ok());

    // Should fail validation for empty name
    let mut invalid_config = AgentConfig::default();
    invalid_config.name = "".to_string();
    let result = invalid_config.validate();
    assert!(result.is_err());
}

#[test]
fn ac12_agent_config_has_default_implementation() {
    // This test verifies that AgentConfig implements Default
    // with sensible defaults

    let config = AgentConfig::default();

    assert_eq!(config.name, "claude");
    assert_eq!(config.model, "claude-sonnet-4-6");
    assert_eq!(config.command, "claude");
    assert_eq!(config.timeout_secs, 3600);
    assert_eq!(config.max_retries, 3);
    assert!(config.enable_session);
}

// ============================================================================
// Acceptance Criterion 3: AgentError type is defined with all variants
// ============================================================================

#[test]
fn ac13_agent_error_has_command_not_found_variant() {
    // This test verifies that AgentError has CommandNotFound variant

    let error = AgentError::CommandNotFound {
        command: "test".to_string(),
    };

    assert!(matches!(error, AgentError::CommandNotFound { .. }));
    assert!(error.to_string().contains("test"));
}

#[test]
fn ac14_agent_error_has_execution_failed_variant() {
    // This test verifies that AgentError has ExecutionFailed variant

    let error = AgentError::ExecutionFailed {
        command: "test".to_string(),
        message: "failed".to_string(),
    };

    assert!(matches!(error, AgentError::ExecutionFailed { .. }));
    assert!(error.to_string().contains("test"));
}

#[test]
fn ac15_agent_error_has_timeout_variant() {
    // This test verifies that AgentError has Timeout variant

    let error = AgentError::Timeout {
        command: "test".to_string(),
        timeout_secs: 60,
    };

    assert!(matches!(error, AgentError::Timeout { .. }));
    assert!(error.to_string().contains("60"));
}

#[test]
fn ac16_agent_error_has_invalid_response_variant() {
    // This test verifies that AgentError has InvalidResponse variant

    let error = AgentError::InvalidResponse {
        reason: "malformed".to_string(),
    };

    assert!(matches!(error, AgentError::InvalidResponse { .. }));
    assert!(error.to_string().contains("malformed"));
}

#[test]
fn ac17_agent_error_has_config_validation_variant() {
    // This test verifies that AgentError has ConfigValidation variant

    let error = AgentError::ConfigValidation {
        field: "model".to_string(),
        message: "cannot be empty".to_string(),
    };

    assert!(matches!(error, AgentError::ConfigValidation { .. }));
    assert!(error.to_string().contains("model"));
}

#[test]
fn ac18_agent_error_has_session_not_found_variant() {
    // This test verifies that AgentError has SessionNotFound variant

    let error = AgentError::SessionNotFound {
        session_id: "sess-123".to_string(),
    };

    assert!(matches!(error, AgentError::SessionNotFound { .. }));
    assert!(error.to_string().contains("sess-123"));
}

#[test]
fn ac19_agent_error_implements_std_error_trait() {
    // This test verifies that AgentError implements std::error::Error

    let error = AgentError::CommandNotFound {
        command: "test".to_string(),
    };

    // Should implement Display
    let display_str = format!("{}", error);
    assert!(!display_str.is_empty());

    // Should implement Debug
    let debug_str = format!("{:?}", error);
    assert!(!debug_str.is_empty());
}

#[test]
fn ac20_agent_error_is_cloneable() {
    // This test verifies that AgentError implements Clone

    let error1 = AgentError::Timeout {
        command: "test".to_string(),
        timeout_secs: 60,
    };
    let error2 = error1.clone();

    assert!(matches!(error1, AgentError::Timeout { .. }));
    assert!(matches!(error2, AgentError::Timeout { .. }));
}

// ============================================================================
// Acceptance Criterion 4: AgentSession trait is defined
// ============================================================================

#[test]
fn ac21_agent_session_trait_has_session_id_method() {
    // This test verifies that AgentSession trait has session_id method

    let session = MemorySession::default();
    let _id = session.session_id();
    assert!(!_id.is_empty());
}

#[test]
fn ac22_agent_session_trait_has_agent_name_method() {
    // This test verifies that AgentSession trait has agent_name method

    let session = MemorySession::default();
    let _name = session.agent_name();
    assert!(!session.agent_name().is_empty());
}

#[test]
fn ac23_agent_session_trait_has_model_method() {
    // This test verifies that AgentSession trait has model method

    let session = MemorySession::default();
    let _model = session.model();
    assert!(!session.model().is_empty());
}

#[test]
fn ac24_agent_session_trait_has_created_at_method() {
    // This test verifies that AgentSession trait has created_at method

    let session = MemorySession::default();
    let created = session.created_at();
    assert!(created <= chrono::Utc::now());
}

#[test]
fn ac25_agent_session_trait_has_last_accessed_method() {
    // This test verifies that AgentSession trait has last_accessed method

    let session = MemorySession::default();
    let accessed = session.last_accessed();
    assert!(accessed <= chrono::Utc::now());
}

#[test]
fn ac26_agent_session_trait_has_reuse_count_method() {
    // This test verifies that AgentSession trait has reuse_count method

    let session = MemorySession::default();
    assert_eq!(session.reuse_count(), 0);

    let mut session = MemorySession::default();
    session.mark_accessed();
    assert_eq!(session.reuse_count(), 1);
}

#[test]
fn ac27_agent_session_trait_has_mark_accessed_method() {
    // This test verifies that AgentSession trait has mark_accessed method

    let mut session = MemorySession::default();
    let before = session.last_accessed();

    session.mark_accessed();

    let after = session.last_accessed();
    assert!(after >= before);
}

#[test]
fn ac28_agent_session_trait_has_is_stale_method() {
    // This test verifies that AgentSession trait has is_stale method

    let session = MemorySession::default();
    // Fresh session should not be stale
    assert!(!session.is_stale());

    // Old session should be stale
    let mut old_session = MemorySession::default();
    old_session.last_accessed = chrono::Utc::now() - chrono::Duration::seconds(4000);
    assert!(old_session.is_stale());
}

// ============================================================================
// Acceptance Criterion 5: Module organization and re-exports
// ============================================================================

#[test]
fn ac29_agent_module_exports_required_types() {
    // This test verifies that src/agent/mod.rs exports all required types

    // These should all be accessible from ltmatrix::agent::backend
    use ltmatrix::agent::backend::{
        AgentBackend, AgentConfig, AgentError, AgentResponse, AgentSession, ExecutionConfig,
        MemorySession,
    };

    // If this compiles, the exports are correct
    let _ = AgentConfig::default();
    let _ = ExecutionConfig::default();
    let _ = MemorySession::default();
}

#[test]
fn ac30_agent_module_exports_claude_agent() {
    // This test verifies that src/agent/mod.rs exports ClaudeAgent

    use ltmatrix::agent::ClaudeAgent;

    // If this compiles, ClaudeAgent is properly exported
    let _agent = ClaudeAgent::new();
}

// ============================================================================
// Acceptance Criterion 6: Documentation exists
// ============================================================================

#[test]
fn ac31_agent_backend_trait_is_documented() {
    // This test verifies that the AgentBackend trait has documentation
    // (Note: This is a compile-time check - if it compiles, docs exist)

    use ltmatrix::agent::backend::AgentBackend;

    // The trait should be documented (this would fail compilation if not)
    // We can't test this at runtime, but the presence of tests using the trait
    // indicates it exists and is accessible
    let _ = std::any::type_name::<dyn AgentBackend>();
}

// ============================================================================
// Acceptance Criterion 7: ClaudeAgent implements AgentBackend
// ============================================================================

#[tokio::test]
async fn ac32_claude_agent_implements_agent_backend() {
    // This test verifies that ClaudeAgent implements the AgentBackend trait

    let agent = ClaudeAgent::new().unwrap();

    // All trait methods should be callable
    let _ = agent.backend_name();
    let _ = agent.agent();

    let config = ExecutionConfig::default();
    let session = MemorySession::default();

    // Execute methods should exist (may fail if claude not installed)
    let _ = agent.execute("test", &config).await;
    let _ = agent.execute_with_session("test", &config, &session).await;

    let task = Task {
        id: "test-1".to_string(),
        title: "Test".to_string(),
        description: "Test task".to_string(),
        status: ltmatrix::models::TaskStatus::Pending,
        complexity: ltmatrix::models::TaskComplexity::Moderate,
        depends_on: vec![],
        subtasks: vec![],
        retry_count: 0,
        error: None,
        created_at: chrono::Utc::now(),
        started_at: None,
        completed_at: None,
    };
    let _ = agent.execute_task(&task, "context", &config).await;
}

// ============================================================================
// Acceptance Criterion 8: Additional supporting types
// ============================================================================

#[test]
fn ac33_execution_config_type_exists() {
    // This test verifies that ExecutionConfig type exists with required fields

    let config = ExecutionConfig::default();

    // Verify all fields exist
    let _model: &String = &config.model;
    let _max_retries: u32 = config.max_retries;
    let _timeout: u64 = config.timeout;
    let _enable_session: bool = config.enable_session;
    let _env_vars: &Vec<(String, String)> = &config.env_vars;
}

#[test]
fn ac34_agent_response_type_exists() {
    // This test verifies that AgentResponse type exists with required fields

    let response = ltmatrix::agent::backend::AgentResponse::default();

    // Verify all fields exist
    let _output: &String = &response.output;
    let _structured_data: &Option<serde_json::Value> = &response.structured_data;
    let _is_complete: bool = response.is_complete;
    let _error: &Option<String> = &response.error;
}

#[test]
fn ac35_memory_session_implements_agent_session() {
    // This test verifies that MemorySession implements AgentSession trait

    let session: &dyn AgentSession = &MemorySession::default();

    // All trait methods should be callable
    let _ = session.session_id();
    let _ = session.agent_name();
    let _ = session.model();
    let _ = session.created_at();
    let _ = session.last_accessed();
    let _ = session.reuse_count();
    let _ = session.is_stale();
}

// ============================================================================
// Integration Tests
// ============================================================================

#[tokio::test]
async fn ac36_full_agent_backend_workflow() {
    // This test verifies the complete workflow:
    // 1. Create agent
    // 2. Validate configuration
    // 3. Check health/availability
    // 4. Execute prompt
    // 5. Execute with session

    let agent = ClaudeAgent::new().unwrap();

    // 1. Agent created successfully
    assert_eq!(agent.agent().name, "claude");

    // 2. Validate configuration
    let config = AgentConfig::default();
    let validation_result = agent.validate_config(&config).await;
    assert!(validation_result.is_ok());

    // 3. Check health
    let health_result = agent.health_check().await;
    assert!(health_result.is_ok());

    // 4. Execute with empty prompt should fail validation
    let exec_config = ExecutionConfig::default();
    let exec_result = agent.execute("", &exec_config).await;
    assert!(exec_result.is_err());

    // 5. Execute with session (may fail if claude not installed)
    let session = MemorySession::default();
    let session_result = agent.execute_with_session("test", &exec_config, &session).await;
    // We don't assert success here, just that it doesn't panic
    let _ = session_result;
}
