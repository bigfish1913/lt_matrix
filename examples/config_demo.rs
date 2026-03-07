//! Configuration Demo
//!
//! This example demonstrates how to use ltmatrix configuration system.
//!
//! # Usage
//!
//! Run this example to see how configuration is loaded and used:
//! ```bash
//! cargo run --example config_demo
//! ```

use ltmatrix::config::settings::{load_config, Config};

fn main() {
    println!("ltmatrix Configuration Demo");
    println!("============================\n");

    // Demonstrate default configuration
    println!("1. Default Configuration:");
    println!("-------------------------");
    let default_config = Config::default();
    display_config(&default_config);
    println!();

    // Demonstrate loading configuration
    println!("2. Configuration Loading:");
    println!("-------------------------");

    match load_config() {
        Ok(config) => {
            println!("✓ Configuration loaded successfully");
            display_config(&config);
        }
        Err(e) => {
            println!("✗ Failed to load configuration: {}", e);
            println!("(Using default configuration)");
            let default_config = Config::default();
            display_config(&default_config);
        }
    }
    println!();

    // Demonstrate configuration precedence
    println!("4. Configuration Precedence:");
    println!("---------------------------");
    println!("1. Built-in defaults (lowest priority)");
    println!("2. Global config (~/.ltmatrix/config.toml)");
    println!("3. Project config (.ltmatrix/config.toml)");
    println!("4. CLI flags (highest priority)");
    println!();

    // Demonstrate key configuration sections
    println!("5. Configuration Sections:");
    println!("-------------------------");
    println!();
    println!("Agent Configuration:");
    println!("  - default: Default agent to use");
    println!("  - agents.<name>: Agent-specific settings");
    println!("    - command: CLI command to invoke");
    println!("    - model: Model identifier");
    println!("    - timeout: Operation timeout in seconds");
    println!();

    println!("Execution Modes:");
    println!("  - modes.fast: Fast execution settings");
    println!("  - modes.standard: Standard execution settings");
    println!("  - modes.expert: Expert execution settings");
    println!();

    println!("Output Configuration:");
    println!("  - output.format: Output format (text, json)");
    println!("  - output.colored: Use ANSI colors");
    println!("  - output.progress: Show progress bars");
    println!();

    println!("Logging Configuration:");
    println!("  - logging.level: Log level (trace, debug, info, warn, error)");
    println!("  - logging.file: Log file path (optional)");
    println!();

    println!("Warmup Configuration:");
    println!("  - warmup.enabled: Enable warmup queries");
    println!("  - warmup.max_queries: Number of warmup queries");
    println!("  - warmup.timeout_seconds: Warmup timeout");
    println!();

    println!("Session Pool Configuration:");
    println!("  - pool.max_sessions: Maximum sessions in pool");
    println!("  - pool.auto_cleanup: Automatically cleanup stale sessions");
    println!("  - pool.cleanup_interval_seconds: Cleanup interval");
    println!("  - pool.stale_threshold_seconds: Session staleness threshold");
    println!("  - pool.enable_reuse: Enable session reuse");
    println!();

    println!("Feature Flags:");
    println!("  - [agent_backend]: Agent backend features");
    println!("  - [pipeline]: Pipeline execution features");
    println!("  - [scheduler]: Task scheduling features");
    println!("  - [rollout.<feature>]: Gradual rollout configuration");
    println!();

    // Example configuration
    println!("6. Example Configuration:");
    println!("-------------------------");
    println!("```toml");
    println!("default = \"claude\"");
    println!();
    println!("[agents.claude]");
    println!("command = \"claude\"");
    println!("model = \"claude-sonnet-4-6\"");
    println!("timeout = 3600");
    println!();
    println!("[modes.standard]");
    println!("model = \"claude-sonnet-4-6\"");
    println!("run_tests = true");
    println!("verify = true");
    println!("max_retries = 3");
    println!("max_depth = 3");
    println!();
    println!("[output]");
    println!("format = \"text\"");
    println!("colored = true");
    println!("progress = true");
    println!();
    println!("[logging]");
    println!("level = \"info\"");
    println!();
    println!("[warmup]");
    println!("enabled = true");
    println!("max_queries = 3");
    println!();
    println!("[pool]");
    println!("max_sessions = 100");
    println!("enable_reuse = true");
    println!("```");
}

fn display_config(config: &Config) {
    println!("Default agent: {:?}", config.default);
    println!("Available agents: {}", config.agents.len());
    println!("Output format: {:?}", config.output.format);
    println!("Log level: {:?}", config.logging.level);
    println!("Warmup enabled: {}", config.warmup.enabled);
    println!("Pool max sessions: {}", config.pool.max_sessions);
}
