//! Example demonstrating CLI and config integration
//!
//! Run with:
//!   cargo run --example cli_config_integration -- --help
//!   cargo run --example cli_config_integration -- --agent opencode --log-level debug "test goal"

use ltmatrix::cli::Args;
use ltmatrix::config::settings::{CliOverrides, load_config_with_overrides};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== CLI-Config Integration Example ===\n");

    // Parse CLI arguments
    let args = Args::parse();

    // Convert args to overrides (using From trait which takes ownership)
    let overrides = CliOverrides::from(args.clone());

    println!("CLI Overrides:");
    println!("  Agent: {:?}", overrides.agent);
    println!("  Mode: {:?}", overrides.mode);
    println!("  Log level: {:?}", overrides.log_level);
    println!("  Output format: {:?}", overrides.output_format);
    println!("  Max retries: {:?}", overrides.max_retries);
    println!("  Timeout: {:?}", overrides.timeout);
    println!("  Dry run: {}", overrides.dry_run);
    println!("  Resume: {}", overrides.resume);
    println!("  Ask: {}", overrides.ask);
    println!("  Regenerate plan: {}", overrides.regenerate_plan);
    println!("  On blocked: {:?}", overrides.on_blocked);
    println!("  MCP config: {:?}", overrides.mcp_config);
    println!("  No color: {:?}", overrides.no_color);
    println!("  Config file: {:?}", overrides.config_file);
    println!();

    // Load config with overrides
    println!("Loading configuration...");
    let config = load_config_with_overrides(Some(overrides))?;

    println!("Final Configuration:");
    println!("  Default agent: {:?}", config.default);
    println!("  Agents configured: {}", config.agents.len());
    println!("  Log level: {:?}", config.logging.level);
    println!("  Log file: {:?}", config.logging.file);
    println!("  Output format: {:?}", config.output.format);
    println!("  Colored output: {}", config.output.colored);
    println!("  Progress bars: {}", config.output.progress);
    println!();

    // Display mode configurations if they exist
    if let Some(fast) = &config.modes.fast {
        println!("Fast Mode:");
        println!("  Model: {:?}", fast.model);
        println!("  Run tests: {}", fast.run_tests);
        println!("  Verify: {}", fast.verify);
        println!("  Max retries: {}", fast.max_retries);
        println!("  Max depth: {}", fast.max_depth);
        println!("  Timeout (plan): {}s", fast.timeout_plan);
        println!("  Timeout (exec): {}s", fast.timeout_exec);
        println!();
    }

    if let Some(standard) = &config.modes.standard {
        println!("Standard Mode:");
        println!("  Model: {:?}", standard.model);
        println!("  Run tests: {}", standard.run_tests);
        println!("  Verify: {}", standard.verify);
        println!("  Max retries: {}", standard.max_retries);
        println!("  Max depth: {}", standard.max_depth);
        println!("  Timeout (plan): {}s", standard.timeout_plan);
        println!("  Timeout (exec): {}s", standard.timeout_exec);
        println!();
    }

    if let Some(expert) = &config.modes.expert {
        println!("Expert Mode:");
        println!("  Model: {:?}", expert.model);
        println!("  Run tests: {}", expert.run_tests);
        println!("  Verify: {}", expert.verify);
        println!("  Max retries: {}", expert.max_retries);
        println!("  Max depth: {}", expert.max_depth);
        println!("  Timeout (plan): {}s", expert.timeout_plan);
        println!("  Timeout (exec): {}s", expert.timeout_exec);
    }

    Ok(())
}
