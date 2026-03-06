//! Example demonstrating CLI integration with config system
//!
//! This example shows how command-line arguments override configuration files.

use clap::Parser;
use ltmatrix::cli::Args;
use ltmatrix::config::settings::load_config_from_args;

fn main() -> anyhow::Result<()> {
    // Example 1: Parse CLI arguments and load config
    println!("=== Example 1: CLI Arguments ===");

    let args = Args::try_parse_from([
        "ltmatrix",
        "--agent", "claude",
        "--mode", "fast",
        "--output", "json",
        "--log-level", "debug",
        "goal: build a REST API"
    ])?;

    let config = load_config_from_args(args)?;

    println!("Default agent: {:?}", config.default);
    println!("Output format: {:?}", config.output.format);
    println!("Log level: {:?}", config.logging.level);

    // Example 2: Show how CLI args override config files
    println!("\n=== Example 2: Precedence ===");
    println!("1. CLI arguments (highest priority)");
    println!("2. Project config: .ltmatrix/config.toml");
    println!("3. Global config: ~/.ltmatrix/config.toml");
    println!("4. Default values (lowest priority)");

    // Example 3: All CLI flags that override config
    println!("\n=== Example 3: Available CLI Overrides ===");
    println!("--agent <NAME>         Override default agent");
    println!("--mode <fast|std|expert> Override execution mode");
    println!("--fast                 Shortcut for --mode fast");
    println!("--expert               Shortcut for --mode expert");
    println!("--output <text|json>    Override output format");
    println!("--log-level <LEVEL>     Override log level");
    println!("--log-file <PATH>       Override log file path");
    println!("--max-retries <NUM>     Override max retries");
    println!("--timeout <SECONDS>     Override timeout");
    println!("--no-color              Disable colored output");

    Ok(())
}
