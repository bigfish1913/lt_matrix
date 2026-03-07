// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Message Framing for MCP Transport
//!
//! This module provides message framing utilities for delimiting messages in a byte stream.
//! MCP supports two framing styles:
//!
//! - **Line-delimited**: Each message is terminated by a newline character (`\n`)
//! - **Content-length**: Messages are prefixed with a `Content-Length` header
//!
//! # Example
//!
//! ```
//! use ltmatrix::mcp::transport::framing::{MessageFramer, LineDelimitedFramer};
//!
//! let mut framer = LineDelimitedFramer::new();
//!
//! // Frame a message for sending
//! let framed = framer.frame_message(br#"{"jsonrpc":"2.0","method":"ping"}"#);
//! assert!(framed.ends_with(b"\n"));
//!
//! // Parse incoming data
//! framer.feed_data(b"Line 1\nLine 2\n");
//! while let Some(message) = framer.next_message() {
//!     println!("Received: {:?}", String::from_utf8_lossy(&message));
//! }
//! ```

use std::collections::VecDeque;

/// Errors that can occur during message framing
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FramingError {
    /// Message exceeds maximum allowed size
    MessageTooLarge {
        actual: usize,
        max: usize,
    },

    /// Invalid content-length header
    InvalidContentLength(String),

    /// Missing content-length header
    MissingContentLength,

    /// Incomplete message (need more data)
    IncompleteMessage,

    /// Invalid UTF-8 in message
    InvalidUtf8,

    /// Header parsing error
    HeaderParseError(String),
}

impl std::fmt::Display for FramingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FramingError::MessageTooLarge { actual, max } => {
                write!(f, "Message too large: {} bytes (max: {})", actual, max)
            }
            FramingError::InvalidContentLength(msg) => {
                write!(f, "Invalid Content-Length header: {}", msg)
            }
            FramingError::MissingContentLength => {
                write!(f, "Missing Content-Length header")
            }
            FramingError::IncompleteMessage => {
                write!(f, "Incomplete message")
            }
            FramingError::InvalidUtf8 => {
                write!(f, "Invalid UTF-8 in message")
            }
            FramingError::HeaderParseError(msg) => {
                write!(f, "Header parse error: {}", msg)
            }
        }
    }
}

impl std::error::Error for FramingError {}

// ============================================================================
// Message Framer Trait
// ============================================================================

/// Trait for message framing implementations
///
/// A message framer handles:
/// - Framing outgoing messages (adding delimiters/headers)
/// - Parsing incoming data into complete messages
/// - Handling partial messages (buffering)
pub trait MessageFramer: Send + Sync {
    /// Frame a message for sending
    ///
    /// Takes a raw message bytes and returns the framed version
    /// (with delimiters or headers added).
    fn frame_message(&self, message: &[u8]) -> Vec<u8>;

    /// Feed incoming data to the framer
    ///
    /// This data will be buffered and parsed into complete messages.
    fn feed_data(&mut self, data: &[u8]);

    /// Get the next complete message, if available
    ///
    /// Returns `None` if no complete message is available yet.
    fn next_message(&mut self) -> Option<Vec<u8>>;

    /// Check if there are pending messages
    fn has_pending_messages(&self) -> bool;

    /// Get the number of pending bytes in the buffer
    fn pending_bytes(&self) -> usize;

    /// Clear the internal buffer
    fn clear(&mut self);

    /// Set the maximum message size
    fn set_max_message_size(&mut self, max_size: usize);

    /// Get the maximum message size
    fn max_message_size(&self) -> usize;
}

// ============================================================================
// Line-Delimited Framer
// ============================================================================

/// Line-delimited message framer
///
/// Each message is terminated by a newline character (`\n`).
/// This is the simplest framing style and is commonly used for JSON-RPC.
#[derive(Debug, Clone)]
pub struct LineDelimitedFramer {
    /// Internal buffer for incomplete messages
    buffer: Vec<u8>,

    /// Queue of complete messages ready to be read
    messages: VecDeque<Vec<u8>>,

    /// Maximum message size
    max_message_size: usize,
}

impl LineDelimitedFramer {
    /// Create a new line-delimited framer
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            messages: VecDeque::new(),
            max_message_size: 10 * 1024 * 1024, // 10 MB default
        }
    }

    /// Create a framer with a specific max message size
    pub fn with_max_size(max_size: usize) -> Self {
        Self {
            buffer: Vec::new(),
            messages: VecDeque::new(),
            max_message_size: max_size,
        }
    }

    /// Parse the buffer for complete messages
    fn parse_buffer(&mut self) {
        while let Some(newline_pos) = self.buffer.iter().position(|&b| b == b'\n') {
            // Extract the message (without the newline)
            let message: Vec<u8> = self.buffer.drain(..newline_pos).collect();

            // Remove the newline
            self.buffer.drain(..1);

            // Also remove any trailing carriage return (Windows line endings)
            let message = if message.last().map(|&b| b) == Some(b'\r') {
                message[..message.len() - 1].to_vec()
            } else {
                message
            };

            // Skip empty messages
            if !message.is_empty() {
                self.messages.push_back(message);
            }
        }
    }
}

impl Default for LineDelimitedFramer {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageFramer for LineDelimitedFramer {
    fn frame_message(&self, message: &[u8]) -> Vec<u8> {
        let mut framed = Vec::with_capacity(message.len() + 1);
        framed.extend_from_slice(message);
        framed.push(b'\n');
        framed
    }

    fn feed_data(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);

        // Check for max size
        if self.buffer.len() > self.max_message_size {
            // Clear buffer and add error (in real impl, we'd handle this better)
            self.buffer.clear();
        }

        self.parse_buffer();
    }

    fn next_message(&mut self) -> Option<Vec<u8>> {
        self.messages.pop_front()
    }

    fn has_pending_messages(&self) -> bool {
        !self.messages.is_empty()
    }

    fn pending_bytes(&self) -> usize {
        self.buffer.len()
    }

    fn clear(&mut self) {
        self.buffer.clear();
        self.messages.clear();
    }

    fn set_max_message_size(&mut self, max_size: usize) {
        self.max_message_size = max_size;
    }

    fn max_message_size(&self) -> usize {
        self.max_message_size
    }
}

// ============================================================================
// Content-Length Framer
// ============================================================================

/// Content-length based message framer
///
/// Messages are prefixed with headers that include the content length.
/// This is similar to HTTP chunked encoding and is used by LSP (Language Server Protocol).
///
/// Format:
/// ```text
/// Content-Length: <length>\r\n
/// \r\n
/// <message bytes>
/// ```
#[derive(Debug, Clone)]
pub struct ContentLengthFramer {
    /// Internal buffer for incomplete data
    buffer: Vec<u8>,

    /// Queue of complete messages ready to be read
    messages: VecDeque<Vec<u8>>,

    /// Maximum message size
    max_message_size: usize,
}

impl ContentLengthFramer {
    /// Create a new content-length framer
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            messages: VecDeque::new(),
            max_message_size: 10 * 1024 * 1024, // 10 MB default
        }
    }

    /// Create a framer with a specific max message size
    pub fn with_max_size(max_size: usize) -> Self {
        Self {
            buffer: Vec::new(),
            messages: VecDeque::new(),
            max_message_size: max_size,
        }
    }

    /// Parse the buffer for complete messages
    fn parse_buffer(&mut self) {
        loop {
            // Look for header terminator
            let header_end = self.find_header_end();
            if header_end.is_none() {
                break;
            }

            let header_end = header_end.unwrap();

            // Parse headers
            let header_bytes = &self.buffer[..header_end];
            let content_length = match self.parse_content_length(header_bytes) {
                Ok(len) => len,
                Err(_) => {
                    // Invalid headers, try to recover by discarding
                    self.buffer.drain(..header_end + 4); // +4 for \r\n\r\n
                    continue;
                }
            };

            // Check if we have the complete message
            let message_start = header_end + 4; // After \r\n\r\n
            let message_end = message_start + content_length;

            if self.buffer.len() < message_end {
                // Incomplete message, wait for more data
                break;
            }

            // Check max size
            if content_length > self.max_message_size {
                // Message too large, discard
                self.buffer.drain(..message_end);
                continue;
            }

            // Extract the message
            let message: Vec<u8> = self.buffer.drain(..message_end).collect();
            let message_content = message[message_start..].to_vec();

            self.messages.push_back(message_content);
        }
    }

    /// Find the end of headers (marked by \r\n\r\n or \n\n)
    fn find_header_end(&self) -> Option<usize> {
        // Look for \r\n\r\n
        for i in 0..self.buffer.len().saturating_sub(3) {
            if self.buffer[i..i + 4] == [b'\r', b'\n', b'\r', b'\n'] {
                return Some(i);
            }
        }

        // Look for \n\n
        for i in 0..self.buffer.len().saturating_sub(1) {
            if self.buffer[i..i + 2] == [b'\n', b'\n'] {
                return Some(i);
            }
        }

        None
    }

    /// Parse Content-Length from headers
    fn parse_content_length(&self, header_bytes: &[u8]) -> Result<usize, FramingError> {
        let header_str = std::str::from_utf8(header_bytes)
            .map_err(|_| FramingError::InvalidUtf8)?;

        for line in header_str.lines() {
            let line = line.trim();
            if line.to_lowercase().starts_with("content-length:") {
                let value = line[15..].trim();
                return value
                    .parse::<usize>()
                    .map_err(|_| FramingError::InvalidContentLength(value.to_string()));
            }
        }

        Err(FramingError::MissingContentLength)
    }
}

impl Default for ContentLengthFramer {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageFramer for ContentLengthFramer {
    fn frame_message(&self, message: &[u8]) -> Vec<u8> {
        let header = format!("Content-Length: {}\r\n\r\n", message.len());
        let mut framed = Vec::with_capacity(header.len() + message.len());
        framed.extend_from_slice(header.as_bytes());
        framed.extend_from_slice(message);
        framed
    }

    fn feed_data(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
        self.parse_buffer();
    }

    fn next_message(&mut self) -> Option<Vec<u8>> {
        self.messages.pop_front()
    }

    fn has_pending_messages(&self) -> bool {
        !self.messages.is_empty()
    }

    fn pending_bytes(&self) -> usize {
        self.buffer.len()
    }

    fn clear(&mut self) {
        self.buffer.clear();
        self.messages.clear();
    }

    fn set_max_message_size(&mut self, max_size: usize) {
        self.max_message_size = max_size;
    }

    fn max_message_size(&self) -> usize {
        self.max_message_size
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- LineDelimitedFramer Tests ----

    #[test]
    fn test_line_framer_single_message() {
        let mut framer = LineDelimitedFramer::new();

        framer.feed_data(b"Hello World\n");

        let message = framer.next_message();
        assert!(message.is_some());
        assert_eq!(message.unwrap(), b"Hello World");
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

        framer.feed_data(b"Partial");

        assert!(framer.next_message().is_none());
        assert_eq!(framer.pending_bytes(), 7);

        framer.feed_data(b" Complete\n");

        let message = framer.next_message();
        assert!(message.is_some());
        assert_eq!(message.unwrap(), b"Partial Complete");
    }

    #[test]
    fn test_line_framer_crlf() {
        let mut framer = LineDelimitedFramer::new();

        framer.feed_data(b"Windows Style\r\n");

        let message = framer.next_message();
        assert!(message.is_some());
        assert_eq!(message.unwrap(), b"Windows Style");
    }

    #[test]
    fn test_line_framer_empty_lines() {
        let mut framer = LineDelimitedFramer::new();

        framer.feed_data(b"\n\nMessage\n\n");

        // Empty lines should be skipped
        let message = framer.next_message();
        assert!(message.is_some());
        assert_eq!(message.unwrap(), b"Message");
        assert!(framer.next_message().is_none());
    }

    #[test]
    fn test_line_framer_frame_message() {
        let framer = LineDelimitedFramer::new();
        let framed = framer.frame_message(b"Test Message");

        assert_eq!(&framed[..framed.len() - 1], b"Test Message");
        assert_eq!(framed[framed.len() - 1], b'\n');
    }

    #[test]
    fn test_line_framer_clear() {
        let mut framer = LineDelimitedFramer::new();

        // Feed data with a complete message and partial message
        framer.feed_data(b"Message\nPartial");
        // After parsing, "Message" is extracted, "Partial" (7 chars) remains in buffer
        assert!(framer.has_pending_messages());
        assert_eq!(framer.pending_bytes(), 7); // Only "Partial" remains in buffer

        framer.clear();
        assert!(!framer.has_pending_messages());
        assert_eq!(framer.pending_bytes(), 0);
    }

    // ---- ContentLengthFramer Tests ----

    #[test]
    fn test_content_length_framer_single_message() {
        let mut framer = ContentLengthFramer::new();

        let data = b"Content-Length: 5\r\n\r\nHello";
        framer.feed_data(data);

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

        framer.feed_data(b"Content-Length: 1");
        assert!(framer.next_message().is_none());

        framer.feed_data(b"0\r\n\r\n0123456789");

        let message = framer.next_message();
        assert!(message.is_some());
        assert_eq!(message.unwrap(), b"0123456789");
    }

    #[test]
    fn test_content_length_framer_partial_body() {
        let mut framer = ContentLengthFramer::new();

        framer.feed_data(b"Content-Length: 10\r\n\r\nHello");
        assert!(framer.next_message().is_none());

        framer.feed_data(b"World");

        let message = framer.next_message();
        assert!(message.is_some());
        assert_eq!(message.unwrap(), b"HelloWorld");
    }

    #[test]
    fn test_content_length_framer_frame_message() {
        let framer = ContentLengthFramer::new();
        let framed = framer.frame_message(b"Test");

        assert!(String::from_utf8_lossy(&framed).contains("Content-Length: 4"));
        assert!(framed.ends_with(b"Test"));
    }

    #[test]
    fn test_content_length_framer_json_message() {
        let mut framer = ContentLengthFramer::new();

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

        // Test lowercase
        framer.feed_data(b"content-length: 5\r\n\r\nHello");
        assert_eq!(framer.next_message().unwrap(), b"Hello");

        // Test mixed case
        framer.feed_data(b"Content-length: 5\r\n\r\nWorld");
        assert_eq!(framer.next_message().unwrap(), b"World");
    }

    // ---- FramingError Tests ----

    #[test]
    fn test_framing_error_display() {
        let error = FramingError::MessageTooLarge {
            actual: 100,
            max: 50,
        };
        assert!(error.to_string().contains("100"));
        assert!(error.to_string().contains("50"));

        let error = FramingError::InvalidContentLength("abc".to_string());
        assert!(error.to_string().contains("abc"));

        let error = FramingError::MissingContentLength;
        assert!(error.to_string().contains("Missing"));
    }

    // ---- Max Size Tests ----

    #[test]
    fn test_line_framer_max_size() {
        let mut framer = LineDelimitedFramer::with_max_size(10);

        // This message is too long and should be discarded
        framer.feed_data(b"This is a very long message that exceeds the limit\n");

        // Buffer should be cleared
        assert_eq!(framer.pending_bytes(), 0);
        assert!(framer.next_message().is_none());
    }

    #[test]
    fn test_content_length_framer_max_size() {
        let mut framer = ContentLengthFramer::with_max_size(10);

        // Request 100 bytes but max is 10
        framer.feed_data(b"Content-Length: 100\r\n\r\n");
        framer.feed_data(&vec![b'X'; 100]);

        // Message should be discarded
        assert!(framer.next_message().is_none());
    }
}
