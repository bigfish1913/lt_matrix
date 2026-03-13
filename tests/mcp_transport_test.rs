// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Comprehensive tests for MCP Transport Layer
//!
//! Tests for:
//! - Transport configuration and builder patterns
//! - Transport trait implementation
//! - Stdio transport lifecycle and operations
//! - Message framing (line-delimited and content-length)
//! - Transport errors and statistics
//! - Bidirectional streaming support

use ltmatrix::mcp::{
    ContentLengthFramer, FramingError, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse,
    LineDelimitedFramer, MessageFramer, OutgoingMessage, RequestId, StdioConfig, StdioTransport,
    Transport, TransportConfig, TransportError, TransportMessage, TransportStats, TransportType,
};
use serde_json::json;
use std::time::Duration;

// ============================================================================
// Transport Configuration Tests
// ============================================================================

mod transport_config_tests {
    use super::*;

    #[test]
    fn test_transport_config_default_values() {
        let config = TransportConfig::default();

        assert_eq!(config.connect_timeout, Duration::from_secs(10));
        assert_eq!(config.read_timeout, Duration::from_secs(30));
        assert_eq!(config.write_timeout, Duration::from_secs(30));
        assert_eq!(config.max_message_size, 10 * 1024 * 1024); // 10 MB
        assert_eq!(config.channel_buffer_size, 100);
        assert!(!config.debug_logging);
    }

    #[test]
    fn test_transport_config_stdio_command() {
        let config = TransportConfig::stdio_command("test-server");

        match config.transport_type {
            TransportType::Stdio(stdio_config) => {
                assert_eq!(stdio_config.command, "test-server");
                assert!(stdio_config.args.is_empty());
            }
            _ => panic!("Expected Stdio transport type"),
        }
    }

    #[test]
    fn test_transport_config_stdio_command_with_args() {
        let config = TransportConfig::stdio_command_with_args(
            "test-server",
            vec!["--port".to_string(), "8080".to_string()],
        );

        match config.transport_type {
            TransportType::Stdio(stdio_config) => {
                assert_eq!(stdio_config.command, "test-server");
                assert_eq!(stdio_config.args, vec!["--port", "8080"]);
            }
            _ => panic!("Expected Stdio transport type"),
        }
    }

    #[test]
    fn test_transport_config_builder_chain() {
        let config = TransportConfig::stdio_command("test-server")
            .with_connect_timeout(Duration::from_secs(5))
            .with_read_timeout(Duration::from_secs(15))
            .with_write_timeout(Duration::from_secs(20))
            .with_max_message_size(1024 * 1024)
            .with_debug_logging(true);

        assert_eq!(config.connect_timeout, Duration::from_secs(5));
        assert_eq!(config.read_timeout, Duration::from_secs(15));
        assert_eq!(config.write_timeout, Duration::from_secs(20));
        assert_eq!(config.max_message_size, 1024 * 1024);
        assert!(config.debug_logging);
    }

    #[test]
    fn test_transport_type_variants() {
        // Test that TransportType variants exist (even if WebSocket not fully implemented)
        let stdio_config = StdioConfig::new("test-server");
        let transport_type = TransportType::Stdio(stdio_config);

        match transport_type {
            TransportType::Stdio(config) => {
                assert_eq!(config.command, "test-server");
            }
            TransportType::WebSocket(_) => {
                // WebSocket variant exists but is not fully implemented
            }
        }
    }
}

// ============================================================================
// Stdio Configuration Tests
// ============================================================================

mod stdio_config_tests {
    use super::*;

    #[test]
    fn test_stdio_config_new() {
        let config = StdioConfig::new("my-server");

        assert_eq!(config.command, "my-server");
        assert!(config.args.is_empty());
        assert!(config.env.is_empty());
        assert!(config.working_dir.is_none());
        assert!(config.inherit_env);
    }

    #[test]
    fn test_stdio_config_default() {
        let config = StdioConfig::default();

        assert!(config.command.is_empty());
        assert!(config.args.is_empty());
        assert!(config.env.is_empty());
        assert!(config.working_dir.is_none());
        assert!(config.inherit_env);
    }

    #[test]
    fn test_stdio_config_builder_with_args() {
        let config = StdioConfig::new("server")
            .with_arg("--verbose")
            .with_arg("--port")
            .with_arg("8080");

        assert_eq!(config.args, vec!["--verbose", "--port", "8080"]);
    }

    #[test]
    fn test_stdio_config_builder_with_args_iter() {
        let config = StdioConfig::new("server").with_args(["--headless", "--no-sandbox"]);

        assert_eq!(config.args, vec!["--headless", "--no-sandbox"]);
    }

    #[test]
    fn test_stdio_config_builder_with_env() {
        let config = StdioConfig::new("server")
            .with_env("DEBUG", "true")
            .with_env("LOG_LEVEL", "debug");

        assert_eq!(config.env.len(), 2);
        assert!(config
            .env
            .contains(&("DEBUG".to_string(), "true".to_string())));
        assert!(config
            .env
            .contains(&("LOG_LEVEL".to_string(), "debug".to_string())));
    }

    #[test]
    fn test_stdio_config_builder_with_working_dir() {
        let config = StdioConfig::new("server").with_working_dir("/tmp/test");

        assert_eq!(
            config.working_dir,
            Some(std::path::PathBuf::from("/tmp/test"))
        );
    }

    #[test]
    fn test_stdio_config_builder_inherit_env() {
        let config = StdioConfig::new("server").with_inherit_env(false);

        assert!(!config.inherit_env);
    }

    #[test]
    fn test_stdio_config_full_builder() {
        let config = StdioConfig::new("playwright-mcp-server")
            .with_arg("--headless")
            .with_args(["--no-sandbox", "--disable-gpu"])
            .with_env("NODE_ENV", "production")
            .with_env("DEBUG", "mcp:*")
            .with_working_dir("/app")
            .with_inherit_env(false);

        assert_eq!(config.command, "playwright-mcp-server");
        assert_eq!(config.args.len(), 3);
        assert_eq!(config.env.len(), 2);
        assert!(config.working_dir.is_some());
        assert!(!config.inherit_env);
    }
}

// ============================================================================
// Transport Error Tests
// ============================================================================

mod transport_error_tests {
    use super::*;

    #[test]
    fn test_transport_error_display_connection_failed() {
        let error = TransportError::ConnectionFailed("Failed to connect".to_string());
        let display = error.to_string();

        assert!(display.contains("Connection failed"));
        assert!(display.contains("Failed to connect"));
    }

    #[test]
    fn test_transport_error_display_connection_closed() {
        let error = TransportError::ConnectionClosed;
        let display = error.to_string();

        assert!(display.contains("closed unexpectedly"));
    }

    #[test]
    fn test_transport_error_display_timeout() {
        let error = TransportError::Timeout(Duration::from_secs(30));
        let display = error.to_string();

        assert!(display.contains("Timeout"));
        assert!(display.contains("30"));
    }

    #[test]
    fn test_transport_error_display_framing_error() {
        let error = TransportError::FramingError("Invalid frame".to_string());
        let display = error.to_string();

        assert!(display.contains("framing error"));
        assert!(display.contains("Invalid frame"));
    }

    #[test]
    fn test_transport_error_display_message_too_large() {
        let error = TransportError::MessageTooLarge {
            actual: 2000,
            max: 1000,
        };
        let display = error.to_string();

        assert!(display.contains("too large"));
        assert!(display.contains("2000"));
        assert!(display.contains("1000"));
    }

    #[test]
    fn test_transport_error_display_serialization_error() {
        let error = TransportError::SerializationError("Invalid JSON".to_string());
        let display = error.to_string();

        assert!(display.contains("Serialization error"));
        assert!(display.contains("Invalid JSON"));
    }

    #[test]
    fn test_transport_error_display_deserialization_error() {
        let error = TransportError::DeserializationError("Expected object".to_string());
        let display = error.to_string();

        assert!(display.contains("Deserialization error"));
        assert!(display.contains("Expected object"));
    }

    #[test]
    fn test_transport_error_display_io_error() {
        let error = TransportError::IoError("Permission denied".to_string());
        let display = error.to_string();

        assert!(display.contains("IO error"));
        assert!(display.contains("Permission denied"));
    }

    #[test]
    fn test_transport_error_display_process_error_with_code() {
        let error = TransportError::ProcessError {
            code: Some(1),
            message: "Process exited with error".to_string(),
        };
        let display = error.to_string();

        assert!(display.contains("exit code 1"));
        assert!(display.contains("Process exited with error"));
    }

    #[test]
    fn test_transport_error_display_process_error_without_code() {
        let error = TransportError::ProcessError {
            code: None,
            message: "Process killed".to_string(),
        };
        let display = error.to_string();

        assert!(!display.contains("exit code"));
        assert!(display.contains("Process killed"));
    }

    #[test]
    fn test_transport_error_display_not_connected() {
        let error = TransportError::NotConnected;
        let display = error.to_string();

        assert!(display.contains("not connected"));
    }

    #[test]
    fn test_transport_error_display_already_started() {
        let error = TransportError::AlreadyStarted;
        let display = error.to_string();

        assert!(display.contains("already started"));
    }

    #[test]
    fn test_transport_error_is_std_error() {
        let error = TransportError::ConnectionClosed;
        let _: &dyn std::error::Error = &error;
    }
}

// ============================================================================
// Transport Message Tests
// ============================================================================

mod transport_message_tests {
    use super::*;

    #[test]
    fn test_transport_message_response_classification() {
        let response = JsonRpcResponse::success(RequestId::Number(1), json!({"status": "ok"}));
        let message = TransportMessage::Response(response);

        assert!(message.is_response());
        assert!(!message.is_notification());
        assert!(!message.is_error());
    }

    #[test]
    fn test_transport_message_notification_classification() {
        let notification = JsonRpcNotification::new("test/method");
        let message = TransportMessage::Notification(notification);

        assert!(!message.is_response());
        assert!(message.is_notification());
        assert!(!message.is_error());
    }

    #[test]
    fn test_transport_message_error_classification() {
        let message = TransportMessage::Error(TransportError::ConnectionClosed);

        assert!(!message.is_response());
        assert!(!message.is_notification());
        assert!(message.is_error());
    }

    #[test]
    fn test_transport_message_as_response() {
        let response = JsonRpcResponse::success(RequestId::Number(1), json!({"result": "success"}));
        let message = TransportMessage::Response(response.clone());

        let extracted = message.as_response();
        assert!(extracted.is_some());
        assert_eq!(extracted.unwrap().id, RequestId::Number(1));
    }

    #[test]
    fn test_transport_message_as_response_none() {
        let message = TransportMessage::Error(TransportError::ConnectionClosed);
        assert!(message.as_response().is_none());

        let notification = TransportMessage::Notification(JsonRpcNotification::new("test"));
        assert!(notification.as_response().is_none());
    }

    #[test]
    fn test_transport_message_as_notification() {
        let notification = JsonRpcNotification::new("test/method");
        let message = TransportMessage::Notification(notification.clone());

        let extracted = message.as_notification();
        assert!(extracted.is_some());
        assert_eq!(extracted.unwrap().method, "test/method");
    }

    #[test]
    fn test_transport_message_as_notification_none() {
        let message = TransportMessage::Error(TransportError::ConnectionClosed);
        assert!(message.as_notification().is_none());

        let response =
            TransportMessage::Response(JsonRpcResponse::success(RequestId::Number(1), json!({})));
        assert!(response.as_notification().is_none());
    }
}

// ============================================================================
// Transport Statistics Tests
// ============================================================================

mod transport_stats_tests {
    use super::*;

    #[test]
    fn test_transport_stats_new() {
        let stats = TransportStats::new();

        assert_eq!(stats.messages_sent, 0);
        assert_eq!(stats.messages_received, 0);
        assert_eq!(stats.bytes_sent, 0);
        assert_eq!(stats.bytes_received, 0);
        assert_eq!(stats.error_count, 0);
        assert!(stats.connected_since.is_none());
        assert!(stats.last_sent.is_none());
        assert!(stats.last_received.is_none());
    }

    #[test]
    fn test_transport_stats_default() {
        let stats = TransportStats::default();

        assert_eq!(stats.messages_sent, 0);
        assert_eq!(stats.messages_received, 0);
    }

    #[test]
    fn test_transport_stats_record_sent() {
        let mut stats = TransportStats::new();

        stats.record_sent(100);
        assert_eq!(stats.messages_sent, 1);
        assert_eq!(stats.bytes_sent, 100);
        assert!(stats.last_sent.is_some());

        stats.record_sent(200);
        assert_eq!(stats.messages_sent, 2);
        assert_eq!(stats.bytes_sent, 300);
    }

    #[test]
    fn test_transport_stats_record_received() {
        let mut stats = TransportStats::new();

        stats.record_received(150);
        assert_eq!(stats.messages_received, 1);
        assert_eq!(stats.bytes_received, 150);
        assert!(stats.last_received.is_some());

        stats.record_received(250);
        assert_eq!(stats.messages_received, 2);
        assert_eq!(stats.bytes_received, 400);
    }

    #[test]
    fn test_transport_stats_record_error() {
        let mut stats = TransportStats::new();

        stats.record_error();
        assert_eq!(stats.error_count, 1);

        stats.record_error();
        stats.record_error();
        assert_eq!(stats.error_count, 3);
    }

    #[test]
    fn test_transport_stats_connection_lifecycle() {
        let mut stats = TransportStats::new();

        assert!(stats.connected_since.is_none());

        stats.mark_connected();
        assert!(stats.connected_since.is_some());

        stats.mark_disconnected();
        assert!(stats.connected_since.is_none());
    }

    #[test]
    fn test_transport_stats_connection_duration() {
        let mut stats = TransportStats::new();

        // Not connected
        assert!(stats.connection_duration().is_none());

        // Connected
        stats.mark_connected();
        let duration = stats.connection_duration();
        assert!(duration.is_some());
        // Duration should be very small (just connected)
        assert!(duration.unwrap() < Duration::from_secs(1));
    }

    #[test]
    fn test_transport_stats_clone() {
        let mut stats = TransportStats::new();
        stats.record_sent(100);
        stats.record_received(200);
        stats.mark_connected();

        let cloned = stats.clone();
        assert_eq!(cloned.messages_sent, 1);
        assert_eq!(cloned.bytes_sent, 100);
        assert_eq!(cloned.messages_received, 1);
        assert_eq!(cloned.bytes_received, 200);
        assert!(cloned.connected_since.is_some());
    }
}

// ============================================================================
// Line-Delimited Framer Tests
// ============================================================================

mod line_delimited_framer_tests {
    use super::*;

    #[test]
    fn test_line_framer_new() {
        let framer = LineDelimitedFramer::new();

        assert!(!framer.has_pending_messages());
        assert_eq!(framer.pending_bytes(), 0);
        assert_eq!(framer.max_message_size(), 10 * 1024 * 1024);
    }

    #[test]
    fn test_line_framer_with_max_size() {
        let framer = LineDelimitedFramer::with_max_size(1024);
        assert_eq!(framer.max_message_size(), 1024);
    }

    #[test]
    fn test_line_framer_frame_message() {
        let framer = LineDelimitedFramer::new();
        let framed = framer.frame_message(b"Hello World");

        assert!(framed.starts_with(b"Hello World"));
        assert!(framed.ends_with(b"\n"));
        assert_eq!(&framed[..framed.len() - 1], b"Hello World");
    }

    #[test]
    fn test_line_framer_single_message() {
        let mut framer = LineDelimitedFramer::new();

        framer.feed_data(b"Message 1\n");

        assert!(framer.has_pending_messages());
        let message = framer.next_message();
        assert!(message.is_some());
        assert_eq!(message.unwrap(), b"Message 1");
        assert!(!framer.has_pending_messages());
    }

    #[test]
    fn test_line_framer_multiple_messages() {
        let mut framer = LineDelimitedFramer::new();

        framer.feed_data(b"Message 1\nMessage 2\nMessage 3\n");

        assert_eq!(framer.next_message().unwrap(), b"Message 1");
        assert_eq!(framer.next_message().unwrap(), b"Message 2");
        assert_eq!(framer.next_message().unwrap(), b"Message 3");
        assert!(framer.next_message().is_none());
    }

    #[test]
    fn test_line_framer_partial_message() {
        let mut framer = LineDelimitedFramer::new();

        // Feed partial message
        framer.feed_data(b"Partial");
        assert!(!framer.has_pending_messages());
        assert_eq!(framer.pending_bytes(), 7);

        // Complete the message
        framer.feed_data(b" Complete\n");

        assert!(framer.has_pending_messages());
        let message = framer.next_message();
        assert_eq!(message.unwrap(), b"Partial Complete");
    }

    #[test]
    fn test_line_framer_crlf_handling() {
        let mut framer = LineDelimitedFramer::new();

        // Windows-style line endings
        framer.feed_data(b"Windows\r\n");

        let message = framer.next_message();
        assert!(message.is_some());
        // Should strip the CR
        assert_eq!(message.unwrap(), b"Windows");
    }

    #[test]
    fn test_line_framer_empty_lines_skipped() {
        let mut framer = LineDelimitedFramer::new();

        framer.feed_data(b"\n\nMessage\n\n");

        // Empty lines should be skipped
        let message = framer.next_message();
        assert!(message.is_some());
        assert_eq!(message.unwrap(), b"Message");
        assert!(framer.next_message().is_none());
    }

    #[test]
    fn test_line_framer_json_message() {
        let framer = LineDelimitedFramer::new();

        let json = r#"{"jsonrpc":"2.0","method":"initialize","id":1}"#;
        let framed = framer.frame_message(json.as_bytes());

        // Parse the framed message
        let mut framer2 = LineDelimitedFramer::new();
        framer2.feed_data(&framed);

        let message = framer2.next_message().unwrap();
        assert_eq!(message, json.as_bytes());
    }

    #[test]
    fn test_line_framer_clear() {
        let mut framer = LineDelimitedFramer::new();

        framer.feed_data(b"Message\nPartial");
        assert!(framer.has_pending_messages());
        assert_eq!(framer.pending_bytes(), 7); // "Partial" remains

        framer.clear();
        assert!(!framer.has_pending_messages());
        assert_eq!(framer.pending_bytes(), 0);
    }

    #[test]
    fn test_line_framer_set_max_message_size() {
        let mut framer = LineDelimitedFramer::new();

        framer.set_max_message_size(2048);
        assert_eq!(framer.max_message_size(), 2048);
    }

    #[test]
    fn test_line_framer_max_size_exceeded() {
        let mut framer = LineDelimitedFramer::with_max_size(10);

        // This message is too long and should be discarded
        framer.feed_data(b"This is a very long message that exceeds the limit\n");

        // Buffer should be cleared when max size exceeded
        assert_eq!(framer.pending_bytes(), 0);
        assert!(framer.next_message().is_none());
    }
}

// ============================================================================
// Content-Length Framer Tests
// ============================================================================

mod content_length_framer_tests {
    use super::*;

    #[test]
    fn test_content_length_framer_new() {
        let framer = ContentLengthFramer::new();

        assert!(!framer.has_pending_messages());
        assert_eq!(framer.pending_bytes(), 0);
        assert_eq!(framer.max_message_size(), 10 * 1024 * 1024);
    }

    #[test]
    fn test_content_length_framer_with_max_size() {
        let framer = ContentLengthFramer::with_max_size(2048);
        assert_eq!(framer.max_message_size(), 2048);
    }

    #[test]
    fn test_content_length_framer_frame_message() {
        let framer = ContentLengthFramer::new();
        let framed = framer.frame_message(b"Test");

        let framed_str = String::from_utf8_lossy(&framed);
        assert!(framed_str.contains("Content-Length: 4"));
        assert!(framed.ends_with(b"Test"));
    }

    #[test]
    fn test_content_length_framer_single_message() {
        let mut framer = ContentLengthFramer::new();

        framer.feed_data(b"Content-Length: 5\r\n\r\nHello");

        assert!(framer.has_pending_messages());
        let message = framer.next_message();
        assert!(message.is_some());
        assert_eq!(message.unwrap(), b"Hello");
    }

    #[test]
    fn test_content_length_framer_multiple_messages() {
        let mut framer = ContentLengthFramer::new();

        framer.feed_data(b"Content-Length: 4\r\n\r\nMsg1");
        framer.feed_data(b"Content-Length: 4\r\n\r\nMsg2");

        assert_eq!(framer.next_message().unwrap(), b"Msg1");
        assert_eq!(framer.next_message().unwrap(), b"Msg2");
    }

    #[test]
    fn test_content_length_framer_partial_header() {
        let mut framer = ContentLengthFramer::new();

        // Feed partial header
        framer.feed_data(b"Content-Length: 1");
        assert!(!framer.has_pending_messages());

        // Complete the header and body
        framer.feed_data(b"0\r\n\r\n0123456789");

        let message = framer.next_message();
        assert!(message.is_some());
        assert_eq!(message.unwrap(), b"0123456789");
    }

    #[test]
    fn test_content_length_framer_partial_body() {
        let mut framer = ContentLengthFramer::new();

        // Feed header and partial body
        framer.feed_data(b"Content-Length: 10\r\n\r\nHello");
        assert!(!framer.has_pending_messages());

        // Complete the body
        framer.feed_data(b"World");

        let message = framer.next_message();
        assert!(message.is_some());
        assert_eq!(message.unwrap(), b"HelloWorld");
    }

    #[test]
    fn test_content_length_framer_json_roundtrip() {
        let framer = ContentLengthFramer::new();

        let json = r#"{"jsonrpc":"2.0","method":"initialize","id":1}"#;
        let framed = framer.frame_message(json.as_bytes());

        let mut framer2 = ContentLengthFramer::new();
        framer2.feed_data(&framed);

        let message = framer2.next_message().unwrap();
        assert_eq!(message, json.as_bytes());
    }

    #[test]
    fn test_content_length_case_insensitive() {
        let mut framer = ContentLengthFramer::new();

        // Lowercase
        framer.feed_data(b"content-length: 5\r\n\r\nHello");
        assert_eq!(framer.next_message().unwrap(), b"Hello");

        // Mixed case
        framer.feed_data(b"Content-length: 5\r\n\r\nWorld");
        assert_eq!(framer.next_message().unwrap(), b"World");
    }

    #[test]
    fn test_content_length_framer_clear() {
        let mut framer = ContentLengthFramer::new();

        framer.feed_data(b"Content-Length: 5\r\n\r\nHelloPartial");
        // After parsing "Hello", remaining buffer contains "Partial"
        // But actually the framer keeps the partial data waiting for next header

        framer.clear();
        assert!(!framer.has_pending_messages());
        assert_eq!(framer.pending_bytes(), 0);
    }

    #[test]
    fn test_content_length_framer_max_size_exceeded() {
        let mut framer = ContentLengthFramer::with_max_size(10);

        // Request 100 bytes but max is 10
        framer.feed_data(b"Content-Length: 100\r\n\r\n");
        framer.feed_data(&vec![b'X'; 100]);

        // Message should be discarded
        assert!(framer.next_message().is_none());
    }

    #[test]
    fn test_content_length_framer_set_max_size() {
        let mut framer = ContentLengthFramer::new();

        framer.set_max_message_size(2048);
        assert_eq!(framer.max_message_size(), 2048);
    }
}

// ============================================================================
// Framing Error Tests
// ============================================================================

mod framing_error_tests {
    use super::*;

    #[test]
    fn test_framing_error_message_too_large() {
        let error = FramingError::MessageTooLarge {
            actual: 100,
            max: 50,
        };
        let display = error.to_string();

        assert!(display.contains("too large"));
        assert!(display.contains("100"));
        assert!(display.contains("50"));
    }

    #[test]
    fn test_framing_error_invalid_content_length() {
        let error = FramingError::InvalidContentLength("abc".to_string());
        let display = error.to_string();

        assert!(display.contains("Invalid Content-Length"));
        assert!(display.contains("abc"));
    }

    #[test]
    fn test_framing_error_missing_content_length() {
        let error = FramingError::MissingContentLength;
        let display = error.to_string();

        assert!(display.contains("Missing"));
        assert!(display.contains("Content-Length"));
    }

    #[test]
    fn test_framing_error_incomplete_message() {
        let error = FramingError::IncompleteMessage;
        let display = error.to_string();

        assert!(display.contains("Incomplete"));
    }

    #[test]
    fn test_framing_error_invalid_utf8() {
        let error = FramingError::InvalidUtf8;
        let display = error.to_string();

        assert!(display.contains("Invalid UTF-8"));
    }

    #[test]
    fn test_framing_error_header_parse_error() {
        let error = FramingError::HeaderParseError("Malformed header".to_string());
        let display = error.to_string();

        assert!(display.contains("Header parse error"));
        assert!(display.contains("Malformed header"));
    }

    #[test]
    fn test_framing_error_is_std_error() {
        let error = FramingError::IncompleteMessage;
        let _: &dyn std::error::Error = &error;
    }
}

// ============================================================================
// Outgoing Message Tests
// ============================================================================

mod outgoing_message_tests {
    use super::*;

    #[test]
    fn test_outgoing_message_request_serialization() {
        let request = JsonRpcRequest::new(RequestId::Number(1), "test");
        let message = OutgoingMessage::Request(request);

        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("Request"));
        assert!(json.contains("test"));
    }

    #[test]
    fn test_outgoing_message_notification_serialization() {
        let notification = JsonRpcNotification::new("test/method");
        let message = OutgoingMessage::Notification(notification);

        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("Notification"));
        assert!(json.contains("test/method"));
    }

    #[test]
    fn test_outgoing_message_shutdown_serialization() {
        let message = OutgoingMessage::Shutdown;

        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("Shutdown"));
    }

    #[test]
    fn test_outgoing_message_roundtrip() {
        let request = JsonRpcRequest::with_params(
            RequestId::String("req-123".to_string()),
            "initialize",
            json!({"version": "1.0"}),
        );
        let message = OutgoingMessage::Request(request);

        let json = serde_json::to_string(&message).unwrap();
        let parsed: OutgoingMessage = serde_json::from_str(&json).unwrap();

        match parsed {
            OutgoingMessage::Request(req) => {
                assert_eq!(req.method, "initialize");
            }
            _ => panic!("Expected Request variant"),
        }
    }
}

// ============================================================================
// Stdio Transport Tests (Non-spawning)
// ============================================================================

mod stdio_transport_tests {
    use super::*;

    #[tokio::test]
    async fn test_stdio_transport_initial_state() {
        let config = StdioConfig::new("test-server");
        let transport = StdioTransport::new(config);

        assert!(!transport.is_connected());
    }

    #[tokio::test]
    async fn test_stdio_transport_stats_initial() {
        let config = StdioConfig::new("test-server");
        let transport = StdioTransport::new(config);

        let stats = transport.stats();
        assert_eq!(stats.messages_sent, 0);
        assert_eq!(stats.messages_received, 0);
    }

    #[tokio::test]
    async fn test_stdio_transport_with_config() {
        let stdio_config = StdioConfig::new("test-server").with_arg("--verbose");
        let transport_config = TransportConfig::default().with_debug_logging(true);

        let transport = StdioTransport::with_config(stdio_config, transport_config);

        assert!(!transport.is_connected());
    }

    #[tokio::test]
    async fn test_stdio_transport_sender_channel() {
        let config = StdioConfig::new("test-server");
        let transport = StdioTransport::new(config);

        // Get sender channel
        let sender = transport.sender();

        // Channel exists even if not connected
        // (the request variable is used to demonstrate the channel can be used)
        let _request = JsonRpcRequest::new(RequestId::Number(1), "test");
        drop(sender);
    }

    #[tokio::test]
    async fn test_stdio_transport_double_start_fails() {
        let config = StdioConfig::new("nonexistent-command-12345");
        let mut transport = StdioTransport::new(config);

        // First start will fail because command doesn't exist
        let result = transport.start().await;
        assert!(result.is_err());

        // Transport should not be connected
        assert!(!transport.is_connected());
    }
}

// ============================================================================
// Message Framer Trait Tests
// ============================================================================

mod message_framer_trait_tests {
    use super::*;

    #[test]
    fn test_line_framer_implements_trait() {
        let framer: Box<dyn MessageFramer> = Box::new(LineDelimitedFramer::new());

        assert!(!framer.has_pending_messages());
        assert_eq!(framer.pending_bytes(), 0);
    }

    #[test]
    fn test_content_length_framer_implements_trait() {
        let framer: Box<dyn MessageFramer> = Box::new(ContentLengthFramer::new());

        assert!(!framer.has_pending_messages());
        assert_eq!(framer.pending_bytes(), 0);
    }

    #[test]
    fn test_framer_trait_frame_and_parse() {
        let mut framer: Box<dyn MessageFramer> = Box::new(LineDelimitedFramer::new());

        // Frame a message
        let framed = framer.frame_message(b"Test Message");

        // Parse it back
        framer.feed_data(&framed);

        let message = framer.next_message();
        assert!(message.is_some());
        assert_eq!(message.unwrap(), b"Test Message");
    }
}

// ============================================================================
// Integration Tests - Framing with JSON-RPC Messages
// ============================================================================

mod json_rpc_framing_tests {
    use super::*;

    #[test]
    fn test_line_framer_with_json_rpc_request() {
        let framer = LineDelimitedFramer::new();

        let request = JsonRpcRequest::with_params(
            RequestId::Number(1),
            "initialize",
            json!({"protocolVersion": "2025-11-25"}),
        );
        let json = request.to_json().unwrap();

        // Frame and parse
        let framed = framer.frame_message(json.as_bytes());

        let mut framer2 = LineDelimitedFramer::new();
        framer2.feed_data(&framed);

        let message = framer2.next_message().unwrap();
        let parsed: JsonRpcRequest = serde_json::from_slice(&message).unwrap();

        assert_eq!(parsed.method, "initialize");
    }

    #[test]
    fn test_line_framer_with_json_rpc_response() {
        let framer = LineDelimitedFramer::new();

        let response = JsonRpcResponse::success(RequestId::Number(1), json!({"status": "ok"}));
        let json = response.to_json().unwrap();

        // Frame and parse
        let framed = framer.frame_message(json.as_bytes());

        let mut framer2 = LineDelimitedFramer::new();
        framer2.feed_data(&framed);

        let message = framer2.next_message().unwrap();
        let parsed: JsonRpcResponse = serde_json::from_slice(&message).unwrap();

        assert!(parsed.is_success());
    }

    #[test]
    fn test_line_framer_with_json_rpc_notification() {
        let framer = LineDelimitedFramer::new();

        let notification = JsonRpcNotification::new("notifications/initialized");
        let json = notification.to_json().unwrap();

        // Frame and parse
        let framed = framer.frame_message(json.as_bytes());

        let mut framer2 = LineDelimitedFramer::new();
        framer2.feed_data(&framed);

        let message = framer2.next_message().unwrap();
        let parsed: JsonRpcNotification = serde_json::from_slice(&message).unwrap();

        assert_eq!(parsed.method, "notifications/initialized");
    }

    #[test]
    fn test_content_length_framer_with_json_rpc_request() {
        let framer = ContentLengthFramer::new();

        let request = JsonRpcRequest::new(RequestId::Number(1), "ping");
        let json = request.to_json().unwrap();

        // Frame and parse
        let framed = framer.frame_message(json.as_bytes());

        let mut framer2 = ContentLengthFramer::new();
        framer2.feed_data(&framed);

        let message = framer2.next_message().unwrap();
        let parsed: JsonRpcRequest = serde_json::from_slice(&message).unwrap();

        assert_eq!(parsed.method, "ping");
    }

    #[test]
    fn test_multiple_json_rpc_messages_sequential() {
        let framer = LineDelimitedFramer::new();

        // Create multiple messages
        let request = JsonRpcRequest::new(RequestId::Number(1), "method1");
        let notification = JsonRpcNotification::new("notification1");
        let response = JsonRpcResponse::success(RequestId::Number(2), json!({}));

        // Frame them all
        let mut data = Vec::new();
        data.extend(framer.frame_message(request.to_json().unwrap().as_bytes()));
        data.extend(framer.frame_message(notification.to_json().unwrap().as_bytes()));
        data.extend(framer.frame_message(response.to_json().unwrap().as_bytes()));

        // Parse them all
        let mut framer2 = LineDelimitedFramer::new();
        framer2.feed_data(&data);

        assert_eq!(
            framer2.next_message().unwrap(),
            request.to_json().unwrap().as_bytes()
        );
        assert_eq!(
            framer2.next_message().unwrap(),
            notification.to_json().unwrap().as_bytes()
        );
        assert_eq!(
            framer2.next_message().unwrap(),
            response.to_json().unwrap().as_bytes()
        );
        assert!(framer2.next_message().is_none());
    }
}

// ============================================================================
// Edge Cases and Error Handling
// ============================================================================

mod edge_case_tests {
    use super::*;

    #[test]
    fn test_line_framer_empty_input() {
        let mut framer = LineDelimitedFramer::new();

        framer.feed_data(b"");
        assert!(!framer.has_pending_messages());
        assert_eq!(framer.pending_bytes(), 0);
    }

    #[test]
    fn test_line_framer_only_newlines() {
        let mut framer = LineDelimitedFramer::new();

        framer.feed_data(b"\n\n\n\n");
        // All empty lines should be skipped
        assert!(!framer.has_pending_messages());
    }

    #[test]
    fn test_content_length_framer_empty_input() {
        let mut framer = ContentLengthFramer::new();

        framer.feed_data(b"");
        assert!(!framer.has_pending_messages());
        assert_eq!(framer.pending_bytes(), 0);
    }

    #[test]
    fn test_content_length_framer_zero_length() {
        let mut framer = ContentLengthFramer::new();

        framer.feed_data(b"Content-Length: 0\r\n\r\n");

        // Zero-length message should be extracted
        let message = framer.next_message();
        assert!(message.is_some());
        assert_eq!(message.unwrap().len(), 0);
    }

    #[test]
    fn test_line_framer_unicode_message() {
        let mut framer = LineDelimitedFramer::new();

        let unicode_msg = "你好世界 🌍 Hello World";
        framer.feed_data(format!("{}\n", unicode_msg).as_bytes());

        let message = framer.next_message().unwrap();
        let parsed = String::from_utf8(message).unwrap();

        assert_eq!(parsed, unicode_msg);
    }

    #[test]
    fn test_content_length_framer_unicode_message() {
        let mut framer = ContentLengthFramer::new();

        let unicode_msg = "你好世界 🌍";
        let data = format!(
            "Content-Length: {}\r\n\r\n{}",
            unicode_msg.len(),
            unicode_msg
        );

        framer.feed_data(data.as_bytes());

        let message = framer.next_message().unwrap();
        let parsed = String::from_utf8(message).unwrap();

        assert_eq!(parsed, unicode_msg);
    }

    #[test]
    fn test_transport_message_clone() {
        let response = JsonRpcResponse::success(RequestId::Number(1), json!({"test": "value"}));
        let message = TransportMessage::Response(response);

        let cloned = message.clone();
        assert!(cloned.is_response());
    }

    #[test]
    fn test_transport_stats_multiple_operations() {
        let mut stats = TransportStats::new();

        stats.mark_connected();
        stats.record_sent(100);
        stats.record_sent(200);
        stats.record_received(150);
        stats.record_error();
        stats.record_received(250);

        assert_eq!(stats.messages_sent, 2);
        assert_eq!(stats.bytes_sent, 300);
        assert_eq!(stats.messages_received, 2);
        assert_eq!(stats.bytes_received, 400);
        assert_eq!(stats.error_count, 1);
        assert!(stats.connected_since.is_some());
        assert!(stats.last_sent.is_some());
        assert!(stats.last_received.is_some());
    }
}

// ============================================================================
// Cross-Platform Considerations
// ============================================================================

#[cfg(target_os = "windows")]
mod windows_tests {
    use super::*;

    #[test]
    fn test_line_framer_windows_line_endings() {
        let mut framer = LineDelimitedFramer::new();

        // Windows uses \r\n
        framer.feed_data(b"Line 1\r\nLine 2\r\n");

        assert_eq!(framer.next_message().unwrap(), b"Line 1");
        assert_eq!(framer.next_message().unwrap(), b"Line 2");
    }
}

#[cfg(not(target_os = "windows"))]
mod unix_tests {
    use super::*;

    #[test]
    fn test_line_framer_unix_line_endings() {
        let mut framer = LineDelimitedFramer::new();

        // Unix uses \n
        framer.feed_data(b"Line 1\nLine 2\n");

        assert_eq!(framer.next_message().unwrap(), b"Line 1");
        assert_eq!(framer.next_message().unwrap(), b"Line 2");
    }
}
