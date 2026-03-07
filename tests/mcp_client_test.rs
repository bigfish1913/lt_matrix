// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Integration tests for MCP Client Core
//!
//! Tests for:
//! - ConnectionState enum and transitions
//! - McpClientConfig builder pattern
//! - McpClient lifecycle management
//! - State machine validation
//! - Error handling for invalid transitions

use ltmatrix::mcp::{
    ClientCapabilities, ConnectionState, ImplementationInfo, McpClient, McpClientConfig,
    ServerCapabilities, ServerInfo, MCP_PROTOCOL_VERSION,
};
use ltmatrix::mcp::transport::TransportConfig;
use std::time::Duration;

// ============================================================================
// ConnectionState Tests
// ============================================================================

mod connection_state_tests {
    use super::*;

    #[test]
    fn test_state_is_connected() {
        assert!(ConnectionState::Connected.is_connected());
        assert!(!ConnectionState::Disconnected.is_connected());
        assert!(!ConnectionState::Connecting.is_connected());
        assert!(!ConnectionState::Disconnecting.is_connected());
    }

    #[test]
    fn test_state_can_connect() {
        assert!(ConnectionState::Disconnected.can_connect());
        assert!(!ConnectionState::Connected.can_connect());
        assert!(!ConnectionState::Connecting.can_connect());
        assert!(!ConnectionState::Disconnecting.can_connect());
    }

    #[test]
    fn test_state_can_disconnect() {
        assert!(ConnectionState::Connected.can_disconnect());
        assert!(ConnectionState::Connecting.can_disconnect());
        assert!(!ConnectionState::Disconnected.can_disconnect());
        assert!(!ConnectionState::Disconnecting.can_disconnect());
    }

    #[test]
    fn test_state_is_transitioning() {
        assert!(ConnectionState::Connecting.is_transitioning());
        assert!(ConnectionState::Disconnecting.is_transitioning());
        assert!(!ConnectionState::Connected.is_transitioning());
        assert!(!ConnectionState::Disconnected.is_transitioning());
    }

    #[test]
    fn test_state_as_str() {
        assert_eq!(ConnectionState::Disconnected.as_str(), "disconnected");
        assert_eq!(ConnectionState::Connecting.as_str(), "connecting");
        assert_eq!(ConnectionState::Connected.as_str(), "connected");
        assert_eq!(ConnectionState::Disconnecting.as_str(), "disconnecting");
    }

    #[test]
    fn test_state_display() {
        assert_eq!(format!("{}", ConnectionState::Connected), "connected");
        assert_eq!(format!("{}", ConnectionState::Disconnected), "disconnected");
        assert_eq!(format!("{}", ConnectionState::Connecting), "connecting");
        assert_eq!(format!("{}", ConnectionState::Disconnecting), "disconnecting");
    }

    #[test]
    fn test_state_default() {
        assert_eq!(ConnectionState::default(), ConnectionState::Disconnected);
    }

    #[test]
    fn test_state_equality() {
        assert_eq!(ConnectionState::Connected, ConnectionState::Connected);
        assert_ne!(ConnectionState::Connected, ConnectionState::Disconnected);
        assert_ne!(ConnectionState::Connecting, ConnectionState::Disconnecting);
    }

    #[test]
    fn test_state_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(ConnectionState::Connected);
        set.insert(ConnectionState::Disconnected);
        set.insert(ConnectionState::Connected); // Duplicate

        assert_eq!(set.len(), 2);
        assert!(set.contains(&ConnectionState::Connected));
        assert!(set.contains(&ConnectionState::Disconnected));
    }

    #[test]
    fn test_state_clone() {
        let state = ConnectionState::Connected;
        let cloned = state.clone();
        assert_eq!(state, cloned);
    }

    #[test]
    fn test_state_copy() {
        let state = ConnectionState::Connected;
        let copied = state; // Copy trait
        assert_eq!(state, copied);
    }
}

// ============================================================================
// McpClientConfig Tests
// ============================================================================

mod client_config_tests {
    use super::*;

    #[test]
    fn test_config_new() {
        let config = McpClientConfig::new("test-client", "1.2.3");

        assert_eq!(config.client_info.name, "test-client");
        assert_eq!(config.client_info.version, "1.2.3");
        assert_eq!(config.protocol_version, MCP_PROTOCOL_VERSION);
    }

    #[test]
    fn test_config_default() {
        let config = McpClientConfig::default();

        assert!(!config.client_info.name.is_empty());
        assert!(!config.client_info.version.is_empty());
        assert!(config.connect_timeout.as_secs() > 0);
        assert!(config.request_timeout.as_secs() > 0);
    }

    #[test]
    fn test_config_with_transport() {
        let transport_config = TransportConfig::stdio_command("test-server");
        let config = McpClientConfig::new("test", "1.0")
            .with_transport(transport_config.clone());

        // Verify transport config is set
        assert!(matches!(config.transport_config.transport_type,
            ltmatrix::mcp::transport::TransportType::Stdio(_)));
    }

    #[test]
    fn test_config_with_capabilities() {
        let capabilities = ClientCapabilities::default();
        let config = McpClientConfig::new("test", "1.0")
            .with_capabilities(capabilities.clone());

        assert_eq!(config.capabilities, capabilities);
    }

    #[test]
    fn test_config_with_timeouts() {
        let config = McpClientConfig::new("test", "1.0")
            .with_connect_timeout(Duration::from_secs(5))
            .with_request_timeout(Duration::from_secs(10));

        assert_eq!(config.connect_timeout, Duration::from_secs(5));
        assert_eq!(config.request_timeout, Duration::from_secs(10));
    }

    #[test]
    fn test_config_with_protocol_version() {
        let config = McpClientConfig::new("test", "1.0")
            .with_protocol_version("2024-01-01");

        assert_eq!(config.protocol_version, "2024-01-01");
    }

    #[test]
    fn test_config_with_debug_logging() {
        let config = McpClientConfig::new("test", "1.0")
            .with_debug_logging(true);

        assert!(config.debug_logging);

        let config = config.with_debug_logging(false);
        assert!(!config.debug_logging);
    }

    #[test]
    fn test_config_builder_chaining() {
        let config = McpClientConfig::new("test", "1.0")
            .with_connect_timeout(Duration::from_secs(5))
            .with_request_timeout(Duration::from_secs(10))
            .with_protocol_version("2025-11-25")
            .with_debug_logging(true);

        assert_eq!(config.client_info.name, "test");
        assert_eq!(config.connect_timeout, Duration::from_secs(5));
        assert_eq!(config.request_timeout, Duration::from_secs(10));
        assert_eq!(config.protocol_version, "2025-11-25");
        assert!(config.debug_logging);
    }

    #[test]
    fn test_config_clone() {
        let config = McpClientConfig::new("test", "1.0");
        let cloned = config.clone();

        assert_eq!(config.client_info.name, cloned.client_info.name);
        assert_eq!(config.client_info.version, cloned.client_info.version);
    }
}

// ============================================================================
// McpClient Creation Tests
// ============================================================================

mod client_creation_tests {
    use super::*;

    #[tokio::test]
    async fn test_client_new() {
        let config = McpClientConfig::new("test", "1.0");
        let client = McpClient::new(config);

        assert_eq!(client.state().await, ConnectionState::Disconnected);
        assert!(!client.is_connected().await);
    }

    #[tokio::test]
    async fn test_client_default() {
        let client = McpClient::default_client();

        assert_eq!(client.state().await, ConnectionState::Disconnected);
        assert!(!client.is_connected().await);
    }

    #[tokio::test]
    async fn test_client_initial_server_info() {
        let client = McpClient::default_client();

        assert!(client.server_info().await.is_none());
        assert!(client.server_capabilities().await.is_none());
    }

    #[tokio::test]
    async fn test_client_config_accessor() {
        let config = McpClientConfig::new("my-client", "2.0");
        let client = McpClient::new(config);

        let config = client.config();
        assert_eq!(config.client_info.name, "my-client");
        assert_eq!(config.client_info.version, "2.0");
    }

    #[tokio::test]
    async fn test_client_transport_stats_initial() {
        let client = McpClient::default_client();
        let stats = client.transport_stats();

        assert_eq!(stats.messages_sent, 0);
        assert_eq!(stats.messages_received, 0);
        assert!(stats.connected_since.is_none());
    }
}

// ============================================================================
// State Machine Validation Tests
// ============================================================================

mod state_machine_tests {
    use super::*;

    /// Verify that state transitions follow the expected pattern:
    /// Disconnected -> Connecting -> Connected -> Disconnecting -> Disconnected
    #[test]
    fn test_valid_state_sequence() {
        let mut state = ConnectionState::Disconnected;

        // Verify initial state
        assert!(state.can_connect());
        assert!(!state.can_disconnect());

        // Transition to Connecting (via connect())
        state = ConnectionState::Connecting;
        assert!(state.is_transitioning());
        assert!(!state.is_connected());
        assert!(state.can_disconnect());

        // Transition to Connected (after handshake)
        state = ConnectionState::Connected;
        assert!(state.is_connected());
        assert!(!state.is_transitioning());
        assert!(state.can_disconnect());
        assert!(!state.can_connect());

        // Transition to Disconnecting (via disconnect())
        state = ConnectionState::Disconnecting;
        assert!(state.is_transitioning());
        assert!(!state.is_connected());

        // Transition to Disconnected
        state = ConnectionState::Disconnected;
        assert!(!state.is_connected());
        assert!(state.can_connect());
        assert!(!state.can_disconnect());
    }

    #[test]
    fn test_invalid_transition_from_connected_to_connecting() {
        // Cannot connect from Connected state
        let state = ConnectionState::Connected;
        assert!(!state.can_connect());
    }

    #[test]
    fn test_invalid_transition_from_disconnected_to_disconnect() {
        // Cannot disconnect from Disconnected state
        let state = ConnectionState::Disconnected;
        assert!(!state.can_disconnect());
    }

    #[test]
    fn test_can_disconnect_during_connecting() {
        // Should be able to abort a connection attempt
        let state = ConnectionState::Connecting;
        assert!(state.can_disconnect());
    }
}

// ============================================================================
// ServerInfo Tests
// ============================================================================

mod server_info_tests {
    use super::*;

    #[test]
    fn test_server_info_from_initialize_result() {
        let init_result = ltmatrix::mcp::InitializeResult {
            protocol_version: "2025-11-25".to_string(),
            capabilities: ServerCapabilities::default(),
            server_info: ImplementationInfo::new("test-server", "1.0.0"),
            instructions: Some("Welcome to the server!".to_string()),
        };

        let server_info = ServerInfo::from(init_result);

        assert_eq!(server_info.info.name, "test-server");
        assert_eq!(server_info.info.version, "1.0.0");
        assert_eq!(server_info.protocol_version, "2025-11-25");
        assert_eq!(server_info.instructions, Some("Welcome to the server!".to_string()));
    }

    #[test]
    fn test_server_info_without_instructions() {
        let init_result = ltmatrix::mcp::InitializeResult {
            protocol_version: "2025-11-25".to_string(),
            capabilities: ServerCapabilities::default(),
            server_info: ImplementationInfo::new("test-server", "1.0.0"),
            instructions: None,
        };

        let server_info = ServerInfo::from(init_result);
        assert!(server_info.instructions.is_none());
    }

    #[test]
    fn test_server_info_clone() {
        let server_info = ServerInfo {
            info: ImplementationInfo::new("test", "1.0"),
            capabilities: ServerCapabilities::default(),
            protocol_version: "2025-11-25".to_string(),
            instructions: Some("test".to_string()),
        };

        let cloned = server_info.clone();
        assert_eq!(server_info.info.name, cloned.info.name);
        assert_eq!(server_info.protocol_version, cloned.protocol_version);
    }
}

// ============================================================================
// Error Handling Tests
// ============================================================================

mod error_handling_tests {
    use super::*;
    use ltmatrix::mcp::protocol::errors::{McpError, McpErrorCode};

    #[test]
    fn test_state_transition_error_display() {
        let error = ltmatrix::mcp::StateTransitionError {
            current: ConnectionState::Connected,
            action: "connect",
            required_states: &[ConnectionState::Disconnected],
        };

        let message = error.to_string();
        assert!(message.contains("Cannot connect"));
        assert!(message.contains("connected"));
        assert!(message.contains("disconnected"));
    }

    #[test]
    fn test_state_transition_error_to_mcp_error() {
        let error = ltmatrix::mcp::StateTransitionError {
            current: ConnectionState::Connected,
            action: "connect",
            required_states: &[ConnectionState::Disconnected],
        };

        let mcp_error: McpError = error.into();
        assert_eq!(mcp_error.code, McpErrorCode::SessionError);
    }
}

// ============================================================================
// Edge Cases Tests
// ============================================================================

mod edge_case_tests {
    use super::*;

    #[test]
    fn test_empty_client_name() {
        let config = McpClientConfig::new("", "1.0");
        assert_eq!(config.client_info.name, "");
    }

    #[test]
    fn test_empty_client_version() {
        let config = McpClientConfig::new("test", "");
        assert_eq!(config.client_info.version, "");
    }

    #[test]
    fn test_zero_timeout() {
        let config = McpClientConfig::new("test", "1.0")
            .with_connect_timeout(Duration::ZERO)
            .with_request_timeout(Duration::ZERO);

        assert_eq!(config.connect_timeout, Duration::ZERO);
        assert_eq!(config.request_timeout, Duration::ZERO);
    }

    #[test]
    fn test_very_long_timeout() {
        let long_timeout = Duration::from_secs(86400); // 1 day
        let config = McpClientConfig::new("test", "1.0")
            .with_connect_timeout(long_timeout);

        assert_eq!(config.connect_timeout, long_timeout);
    }

    #[test]
    fn test_unicode_in_client_name() {
        let config = McpClientConfig::new("测试客户端-🚀", "1.0");
        assert!(config.client_info.name.contains("测试"));
        assert!(config.client_info.name.contains("🚀"));
    }

    #[tokio::test]
    async fn test_multiple_state_queries() {
        let client = McpClient::default_client();

        // Multiple queries should work
        for _ in 0..100 {
            let state = client.state().await;
            assert_eq!(state, ConnectionState::Disconnected);
        }
    }

    #[tokio::test]
    async fn test_concurrent_state_queries() {
        let client = std::sync::Arc::new(McpClient::default_client());
        let mut handles = vec![];

        // Spawn multiple tasks querying state concurrently
        for _ in 0..10 {
            let client_clone = client.clone();
            handles.push(tokio::spawn(async move {
                let state = client_clone.state().await;
                assert_eq!(state, ConnectionState::Disconnected);
            }));
        }

        // Wait for all handles
        for handle in handles {
            handle.await.unwrap();
        }
    }
}

// ============================================================================
// Debug Trait Tests
// ============================================================================

mod debug_trait_tests {
    use super::*;

    #[test]
    fn test_connection_state_debug() {
        let state = ConnectionState::Connected;
        let debug_str = format!("{:?}", state);
        assert!(debug_str.contains("Connected"));
    }

    #[test]
    fn test_client_config_debug() {
        let config = McpClientConfig::new("test", "1.0");
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("test"));
        assert!(debug_str.contains("1.0"));
    }
}
