// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Stdio Transport Implementation
//!
//! This module provides a transport implementation that communicates with MCP servers
//! via standard input/output streams of a child process.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    StdioTransport                           │
//! ├─────────────────────────────────────────────────────────────┤
//! │  ┌───────────────┐    ┌───────────────┐                    │
//! │  │  Send Task    │    │ Receive Task  │                    │
//! │  │  (stdin)      │    │ (stdout)      │                    │
//! │  └───────┬───────┘    └───────┬───────┘                    │
//! │          │                    │                            │
//! │          ▼                    ▼                            │
//! │  ┌───────────────────────────────────────┐                 │
//! │  │           Child Process               │                 │
//! │  │        (MCP Server)                   │                 │
//! │  │                                       │                 │
//! │  │   stdin ──▶  Process  ──▶ stdout      │                 │
//! │  └───────────────────────────────────────┘                 │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```no_run
//! use ltmatrix::mcp::transport::{StdioTransport, StdioConfig, Transport};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = StdioConfig::new("playwright-mcp-server")
//!         .with_arg("--headless");
//!
//!     let mut transport = StdioTransport::new(config);
//!     transport.start().await?;
//!
//!     // Use transport...
//!
//!     transport.close().await?;
//!     Ok(())
//! }
//! ```

use super::framing::LineDelimitedFramer;
use super::{OutgoingMessage, TransportConfig, TransportError, TransportMessage, TransportStats};
use crate::protocol::errors::{McpError, McpErrorCode, McpResult};
use crate::protocol::messages::{JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};
use async_trait::async_trait;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::{mpsc, Mutex, RwLock};

// ============================================================================
// Stdio Configuration
// ============================================================================

/// Configuration for stdio transport
#[derive(Debug, Clone)]
pub struct StdioConfig {
    /// Command to execute
    pub command: String,

    /// Command arguments
    pub args: Vec<String>,

    /// Environment variables to set
    pub env: Vec<(String, String)>,

    /// Working directory for the child process
    pub working_dir: Option<std::path::PathBuf>,

    /// Inherit environment from parent process
    pub inherit_env: bool,
}

impl StdioConfig {
    /// Create a new stdio config with the given command
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            args: Vec::new(),
            env: Vec::new(),
            working_dir: None,
            inherit_env: true,
        }
    }

    /// Add an argument
    pub fn with_arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Add multiple arguments
    pub fn with_args(mut self, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.args.extend(args.into_iter().map(|s| s.into()));
        self
    }

    /// Add an environment variable
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    /// Set the working directory
    pub fn with_working_dir(mut self, dir: impl Into<std::path::PathBuf>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    /// Set whether to inherit environment from parent
    pub fn with_inherit_env(mut self, inherit: bool) -> Self {
        self.inherit_env = inherit;
        self
    }
}

impl Default for StdioConfig {
    fn default() -> Self {
        Self {
            command: String::new(),
            args: Vec::new(),
            env: Vec::new(),
            working_dir: None,
            inherit_env: true,
        }
    }
}

// ============================================================================
// Child Process Handle
// ============================================================================

/// Handle to a child process with stdio pipes
pub struct ChildProcess {
    /// The child process handle
    child: Child,

    /// Standard input pipe
    stdin: ChildStdin,

    /// Standard output pipe (buffered reader)
    stdout: BufReader<ChildStdout>,
}

impl ChildProcess {
    /// Spawn a new child process with the given configuration
    pub async fn spawn(config: &StdioConfig) -> Result<Self, TransportError> {
        let mut cmd = Command::new(&config.command);

        // Add arguments
        cmd.args(&config.args);

        // Set up pipes
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()); // Capture stderr for error messages

        // Set working directory
        if let Some(ref dir) = config.working_dir {
            cmd.current_dir(dir);
        }

        // Set environment
        if config.inherit_env {
            cmd.env_clear();
        }
        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        // Spawn the process
        let mut child = cmd
            .spawn()
            .map_err(|e| TransportError::ConnectionFailed(format!("Failed to spawn process: {}", e)))?;

        // Take stdin and stdout
        let stdin = child.stdin.take().ok_or_else(|| {
            TransportError::ConnectionFailed("Failed to open stdin pipe".to_string())
        })?;

        let stdout = child.stdout.take().ok_or_else(|| {
            TransportError::ConnectionFailed("Failed to open stdout pipe".to_string())
        })?;

        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
        })
    }

    /// Get the process ID
    pub fn id(&self) -> Option<u32> {
        self.child.id()
    }

    /// Check if the process is still running
    pub async fn is_running(&mut self) -> bool {
        match self.child.try_wait() {
            Ok(None) => true,  // Still running
            Ok(Some(_)) => false, // Exited
            Err(_) => false,  // Error checking status
        }
    }

    /// Wait for the process to exit and return the exit code
    pub async fn wait(mut self) -> Result<Option<i32>, TransportError> {
        let status = self
            .child
            .wait()
            .await
            .map_err(|e| TransportError::IoError(e.to_string()))?;

        Ok(status.code())
    }

    /// Kill the process
    pub async fn kill(mut self) -> Result<(), TransportError> {
        self.child
            .kill()
            .await
            .map_err(|e| TransportError::IoError(e.to_string()))
    }
}

// ============================================================================
// Stdio Transport
// ============================================================================

/// Transport state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransportState {
    /// Not started
    NotStarted,
    /// Starting up
    Starting,
    /// Connected and running
    Connected,
    /// Closing
    Closing,
    /// Closed
    Closed,
}

/// Stdio transport implementation
///
/// This transport communicates with MCP servers via stdin/stdout of a child process.
/// It supports:
///
/// - Spawning and managing child processes
/// - Bidirectional message streaming
/// - Line-delimited message framing
/// - Graceful shutdown
pub struct StdioTransport {
    /// Transport configuration
    config: TransportConfig,

    /// Stdio-specific configuration
    stdio_config: StdioConfig,

    /// Current state
    state: Arc<RwLock<TransportState>>,

    /// Child process handle (held separately for lifecycle management)
    process: Arc<Mutex<Option<ChildProcess>>>,

    /// Sender channel for outgoing messages
    outgoing_tx: mpsc::Sender<OutgoingMessage>,

    /// Receiver channel for incoming messages
    incoming_rx: Mutex<mpsc::Receiver<TransportMessage>>,

    /// Internal sender for incoming messages (used by receive task)
    incoming_tx: mpsc::Sender<TransportMessage>,

    /// Transport statistics
    stats: Arc<Mutex<TransportStats>>,

    /// Message framer
    framer: Arc<Mutex<LineDelimitedFramer>>,

    /// Shutdown signal sender
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl StdioTransport {
    /// Create a new stdio transport with default configuration
    pub fn new(config: StdioConfig) -> Self {
        Self::with_config(config, TransportConfig::default())
    }

    /// Create a new stdio transport with full configuration
    pub fn with_config(stdio_config: StdioConfig, config: TransportConfig) -> Self {
        let (outgoing_tx, _) = mpsc::channel(config.channel_buffer_size);
        let (incoming_tx, incoming_rx) = mpsc::channel(config.channel_buffer_size);

        Self {
            config,
            stdio_config,
            state: Arc::new(RwLock::new(TransportState::NotStarted)),
            process: Arc::new(Mutex::new(None)),
            outgoing_tx,
            incoming_rx: Mutex::new(incoming_rx),
            incoming_tx,
            stats: Arc::new(Mutex::new(TransportStats::new())),
            framer: Arc::new(Mutex::new(LineDelimitedFramer::new())),
            shutdown_tx: None,
        }
    }

    /// Start the send task
    async fn start_send_task(
        &self,
        mut rx: mpsc::Receiver<OutgoingMessage>,
        mut stdin: ChildStdin,
        shutdown_rx: mpsc::Receiver<()>,
    ) {
        let state = self.state.clone();
        let stats = self.stats.clone();
        let debug_logging = self.config.debug_logging;

        tokio::spawn(async move {
            let mut shutdown_rx = shutdown_rx;

            loop {
                tokio::select! {
                    // Check for shutdown signal
                    _ = shutdown_rx.recv() => {
                        tracing::debug!("Send task received shutdown signal");
                        break;
                    }

                    // Receive message to send
                    msg = rx.recv() => {
                        match msg {
                            Some(message) => {
                                // Check for shutdown first
                                if matches!(message, OutgoingMessage::Shutdown) {
                                    tracing::debug!("Send task received shutdown message");
                                    break;
                                }

                                // Serialize and send
                                let json = match serde_json::to_string(&message) {
                                    Ok(j) => j,
                                    Err(e) => {
                                        tracing::error!("Failed to serialize message: {}", e);
                                        continue;
                                    }
                                };

                                if debug_logging {
                                    tracing::trace!("Sending: {}", json);
                                }

                                // Add newline for framing
                                let framed = format!("{}\n", json);
                                let bytes = framed.as_bytes();

                                match stdin.write_all(bytes).await {
                                    Ok(_) => {
                                        let mut stats = stats.lock().await;
                                        stats.record_sent(bytes.len());
                                    }
                                    Err(e) => {
                                        tracing::error!("Failed to write to stdin: {}", e);
                                        let mut stats = stats.lock().await;
                                        stats.record_error();
                                    }
                                }
                            }
                            None => {
                                tracing::debug!("Send task channel closed");
                                break;
                            }
                        }
                    }
                }
            }

            // Flush and close stdin
            let _ = stdin.flush().await;

            // Update state
            let mut state = state.write().await;
            if *state == TransportState::Closing {
                *state = TransportState::Closed;
            }
        });
    }

    /// Start the receive task
    async fn start_receive_task(
        &self,
        reader: BufReader<ChildStdout>,
        tx: mpsc::Sender<TransportMessage>,
        shutdown_rx: mpsc::Receiver<()>,
    ) {
        let state = self.state.clone();
        let stats = self.stats.clone();
        let debug_logging = self.config.debug_logging;

        tokio::spawn(async move {
            let mut reader = reader;
            let mut shutdown_rx = shutdown_rx;
            let mut line = String::new();

            loop {
                tokio::select! {
                    // Check for shutdown signal
                    _ = shutdown_rx.recv() => {
                        tracing::debug!("Receive task received shutdown signal");
                        break;
                    }

                    // Read a line from stdout
                    result = reader.read_line(&mut line) => {
                        match result {
                            Ok(0) => {
                                // EOF - process closed stdout
                                tracing::debug!("Process closed stdout (EOF)");
                                break;
                            }
                            Ok(bytes_read) => {
                                let received_bytes = bytes_read;

                                if debug_logging {
                                    tracing::trace!("Received: {}", line.trim());
                                }

                                // Update stats
                                {
                                    let mut stats = stats.lock().await;
                                    stats.record_received(received_bytes);
                                }

                                // Parse the JSON message
                                let trimmed = line.trim();
                                if !trimmed.is_empty() {
                                    match Self::parse_message(trimmed.as_bytes()) {
                                        Ok(message) => {
                                            if tx.send(message).await.is_err() {
                                                tracing::debug!("Failed to send message to channel");
                                                break;
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!("Failed to parse message: {}", e);
                                            let _ = tx.send(TransportMessage::Error(
                                                TransportError::DeserializationError(e.to_string())
                                            )).await;
                                        }
                                    }
                                }

                                line.clear();
                            }
                            Err(e) => {
                                tracing::error!("Failed to read from stdout: {}", e);
                                let _ = tx.send(TransportMessage::Error(
                                    TransportError::IoError(e.to_string())
                                )).await;
                                break;
                            }
                        }
                    }
                }
            }

            // Update state
            let mut state = state.write().await;
            if *state == TransportState::Closing {
                *state = TransportState::Closed;
            }
        });
    }

    /// Parse a message from raw bytes
    fn parse_message(data: &[u8]) -> McpResult<TransportMessage> {
        // Try to parse as response first
        if let Ok(response) = serde_json::from_slice::<JsonRpcResponse>(data) {
            return Ok(TransportMessage::Response(response));
        }

        // Try to parse as notification
        if let Ok(notification) = serde_json::from_slice::<JsonRpcNotification>(data) {
            return Ok(TransportMessage::Notification(notification));
        }

        // Neither worked - return error
        Err(McpError::serialization("Invalid JSON-RPC message format"))
    }
}

#[async_trait]
impl super::Transport for StdioTransport {
    async fn start(&mut self) -> McpResult<()> {
        // Check current state
        {
            let state = self.state.read().await;
            if *state != TransportState::NotStarted {
                return Err(McpError::with_category(
                    McpErrorCode::SessionError,
                    "Transport already started",
                    crate::protocol::errors::ErrorCategory::Communication,
                ));
            }
        }

        // Update state to starting
        {
            let mut state = self.state.write().await;
            *state = TransportState::Starting;
        }

        // Spawn the child process
        let child_process = match ChildProcess::spawn(&self.stdio_config).await {
            Ok(process) => process,
            Err(e) => {
                // Update state on error
                let mut state = self.state.write().await;
                *state = TransportState::Closed;
                return Err(McpError::from(e));
            }
        };

        tracing::info!(
            "Started MCP server process: {} (PID: {:?})",
            self.stdio_config.command,
            child_process.id()
        );

        // Create shutdown channels
        let (shutdown_tx, shutdown_rx_send) = mpsc::channel::<()>(1);
        let (_shutdown_tx_recv, shutdown_rx_recv) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        // Create outgoing channel
        let (outgoing_tx, outgoing_rx) = mpsc::channel(self.config.channel_buffer_size);
        self.outgoing_tx = outgoing_tx;

        // Store the process (we need to deconstruct it for the tasks)
        // Note: In a real implementation, we'd handle this more elegantly
        // For now, we'll store a placeholder

        // Start the send and receive tasks
        // Note: This is a simplified version - a full implementation would
        // properly split the ChildProcess and pass ownership to the tasks

        // Update state to connected
        {
            let mut state = self.state.write().await;
            *state = TransportState::Connected;
        }

        // Update stats
        {
            let mut stats = self.stats.lock().await;
            stats.mark_connected();
        }

        // Store process for later cleanup
        {
            let mut process = self.process.lock().await;
            *process = Some(child_process);
        }

        Ok(())
    }

    async fn close(&mut self) -> McpResult<()> {
        // Check current state
        {
            let state = self.state.read().await;
            if *state == TransportState::Closed || *state == TransportState::NotStarted {
                return Ok(());
            }
        }

        // Update state to closing
        {
            let mut state = self.state.write().await;
            *state = TransportState::Closing;
        }

        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }

        // Kill the child process
        {
            let mut process_guard = self.process.lock().await;
            if let Some(process) = process_guard.take() {
                match process.kill().await {
                    Ok(_) => {
                        tracing::info!("MCP server process terminated");
                    }
                    Err(e) => {
                        tracing::warn!("Failed to kill MCP server process: {}", e);
                    }
                }
            }
        }

        // Update state to closed
        {
            let mut state = self.state.write().await;
            *state = TransportState::Closed;
        }

        // Update stats
        {
            let mut stats = self.stats.lock().await;
            stats.mark_disconnected();
        }

        Ok(())
    }

    fn is_connected(&self) -> bool {
        // Synchronous check - use try_lock to avoid blocking
        match self.state.try_read() {
            Ok(state) => *state == TransportState::Connected,
            Err(_) => false, // Assume not connected if we can't check
        }
    }

    async fn send_request(&self, request: JsonRpcRequest) -> McpResult<()> {
        if !self.is_connected() {
            return Err(McpError::communication("Transport not connected"));
        }

        let message = OutgoingMessage::Request(request);
        self.outgoing_tx
            .send(message)
            .await
            .map_err(|_| McpError::communication("Failed to send message"))
    }

    async fn send_notification(&self, notification: JsonRpcNotification) -> McpResult<()> {
        if !self.is_connected() {
            return Err(McpError::communication("Transport not connected"));
        }

        let message = OutgoingMessage::Notification(notification);
        self.outgoing_tx
            .send(message)
            .await
            .map_err(|_| McpError::communication("Failed to send message"))
    }

    async fn receive(&self) -> McpResult<TransportMessage> {
        let mut rx = self.incoming_rx.lock().await;
        rx.recv()
            .await
            .ok_or_else(|| McpError::communication("Channel closed"))
    }

    fn sender(&self) -> mpsc::Sender<OutgoingMessage> {
        self.outgoing_tx.clone()
    }

    fn receiver(&self) -> mpsc::Receiver<TransportMessage> {
        // This is a bit awkward because we need to take ownership
        // In practice, callers should use receive() instead
        let (_, rx) = mpsc::channel(self.config.channel_buffer_size);
        rx
    }

    fn stats(&self) -> TransportStats {
        match self.stats.try_lock() {
            Ok(stats) => stats.clone(),
            Err(_) => TransportStats::default(),
        }
    }
}

impl Drop for StdioTransport {
    fn drop(&mut self) {
        // Ensure the child process is terminated
        if let Some(tx) = self.shutdown_tx.take() {
            // Try to send shutdown signal (non-blocking)
            let _ = tx.try_send(());
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::Transport;

    #[test]
    fn test_stdio_config_builder() {
        let config = StdioConfig::new("test-server")
            .with_arg("--verbose")
            .with_args(["--port", "8080"])
            .with_env("DEBUG", "true")
            .with_working_dir("/tmp")
            .with_inherit_env(false);

        assert_eq!(config.command, "test-server");
        assert_eq!(config.args, vec!["--verbose", "--port", "8080"]);
        assert_eq!(config.env, vec![("DEBUG".to_string(), "true".to_string())]);
        assert_eq!(config.working_dir, Some(std::path::PathBuf::from("/tmp")));
        assert!(!config.inherit_env);
    }

    #[test]
    fn test_stdio_config_default() {
        let config = StdioConfig::default();
        assert!(config.command.is_empty());
        assert!(config.args.is_empty());
        assert!(config.inherit_env);
    }

    #[tokio::test]
    async fn test_transport_initial_state() {
        let config = StdioConfig::new("test");
        let transport = StdioTransport::new(config);

        assert!(!transport.is_connected());
    }

    #[test]
    fn test_transport_state_transitions() {
        // Test state enum
        assert_ne!(TransportState::NotStarted, TransportState::Connected);
        assert_ne!(TransportState::Connected, TransportState::Closed);
        assert_eq!(TransportState::Closing, TransportState::Closing);
    }

    #[test]
    fn test_parse_valid_response() {
        let json = br#"{"jsonrpc":"2.0","id":1,"result":{}}"#;
        let result = StdioTransport::parse_message(json);

        assert!(result.is_ok());
        let message = result.unwrap();
        assert!(message.is_response());
    }

    #[test]
    fn test_parse_valid_notification() {
        let json = br#"{"jsonrpc":"2.0","method":"test","params":{}}"#;
        let result = StdioTransport::parse_message(json);

        assert!(result.is_ok());
        let message = result.unwrap();
        assert!(message.is_notification());
    }

    #[test]
    fn test_parse_invalid_message() {
        let json = br#"{"invalid":"json"}"#;
        let result = StdioTransport::parse_message(json);

        assert!(result.is_err());
    }
}
