// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.


//! Telemetry sender
//!
//! This module handles sending telemetry events to the analytics endpoint.

use anyhow::{Context, Result};
use reqwest::Client;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn};

use crate::telemetry::config::TelemetryConfig;
use crate::telemetry::event::TelemetryEvent;

/// Telemetry sender that transmits events to the analytics endpoint
#[derive(Debug)]
pub struct TelemetrySender {
    /// HTTP client for sending telemetry
    client: Client,

    /// Telemetry configuration
    config: TelemetryConfig,
}

impl TelemetrySender {
    /// Create a new telemetry sender
    pub fn new(config: TelemetryConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .context("Failed to create HTTP client for telemetry")?;

        Ok(TelemetrySender { client, config })
    }

    /// Send a batch of telemetry events to the analytics endpoint
    pub async fn send_batch(&self, events: Vec<TelemetryEvent>) -> Result<()> {
        if events.is_empty() {
            debug!("No telemetry events to send");
            return Ok(());
        }

        if !self.config.enabled {
            debug!("Telemetry is disabled, not sending events");
            return Ok(());
        }

        debug!("Sending {} telemetry events to {}", events.len(), self.config.endpoint);

        // Serialize events to JSON
        let json_body = serde_json::to_string(&events)
            .context("Failed to serialize telemetry events")?;

        // Send with retry logic
        self.send_with_retry(&json_body).await
    }

    /// Send events with retry logic
    async fn send_with_retry(&self, body: &str) -> Result<()> {
        let mut last_error = None;

        for attempt in 0..self.config.max_retries {
            match self.try_send(body).await {
                Ok(_) => {
                    debug!("Telemetry events sent successfully");
                    return Ok(());
                }
                Err(e) => {
                    warn!(
                        "Failed to send telemetry (attempt {}/{}): {}",
                        attempt + 1,
                        self.config.max_retries,
                        e
                    );
                    last_error = Some(e);

                    // Exponential backoff: 2^attempt seconds
                    let backoff_secs = 2u64.pow(attempt as u32);
                    sleep(Duration::from_secs(backoff_secs)).await;
                }
            }
        }

        Err(anyhow::anyhow!(
            "Failed to send telemetry after {} attempts: {}",
            self.config.max_retries,
            last_error.unwrap_or_else(|| anyhow::anyhow!("Unknown error"))
        ))
    }

    /// Attempt to send telemetry events
    async fn try_send(&self, body: &str) -> Result<()> {
        let response = self.client
            .post(&self.config.endpoint)
            .header("Content-Type", "application/json")
            .header("User-Agent", format!("ltmatrix/{}", env!("CARGO_PKG_VERSION")))
            .body(body.to_string())
            .send()
            .await
            .context("Failed to send telemetry request")?;

        // Check response status
        if response.status().is_success() {
            debug!("Telemetry accepted by server (status {})", response.status());
            Ok(())
        } else {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_else(|_| "Unable to read error body".to_string());
            Err(anyhow::anyhow!(
                "Telemetry server returned error status {}: {}",
                status,
                error_body
            ))
        }
    }

    /// Check if telemetry is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::event::TelemetryEvent;
    use chrono::Utc;
    use uuid::Uuid;

    fn create_test_sender() -> TelemetrySender {
        let config = TelemetryConfig::builder()
            .enabled()
            .endpoint("https://httpbin.org/post") // Use httpbin for testing
            .timeout_secs(5)
            .max_retries(1)
            .build();

        TelemetrySender::new(config).unwrap()
    }

    fn create_test_events() -> Vec<TelemetryEvent> {
        let session_id = Uuid::new_v4();
        vec![
            TelemetryEvent::SessionStart {
                session_id,
                version: "0.1.0".to_string(),
                os: "linux".to_string(),
                arch: "x86_64".to_string(),
                timestamp: Utc::now(),
            },
        ]
    }

    #[tokio::test]
    async fn test_sender_enabled() {
        let sender = create_test_sender();
        assert!(sender.is_enabled());
    }

    #[tokio::test]
    async fn test_sender_disabled() {
        let config = TelemetryConfig::default(); // disabled
        let sender = TelemetrySender::new(config).unwrap();
        assert!(!sender.is_enabled());
    }

    #[tokio::test]
    async fn test_send_empty_batch() {
        let sender = create_test_sender();
        let result = sender.send_batch(vec![]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_disabled_returns_ok() {
        let config = TelemetryConfig::default(); // disabled
        let sender = TelemetrySender::new(config).unwrap();

        let events = create_test_events();
        let result = sender.send_batch(events).await;

        // Should return Ok even though disabled
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_event_serialization() {
        let events = create_test_events();
        let json = serde_json::to_string(&events);
        assert!(json.is_ok());

        let json_str = json.unwrap();
        // With serde(tag = "event_type"), the variant name becomes the tag value
        assert!(json_str.contains("SessionStart") || json_str.contains("session_start"));
        assert!(json_str.contains("0.1.0"));
    }
}
