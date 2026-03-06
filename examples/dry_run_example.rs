//! Example usage of dry-run mode
//!
//! This example demonstrates how to use the dry-run mode to preview
//! task plans without executing them.

use ltmatrix::dryrun::{run_dry_run, DryRunConfig};
use ltmatrix::models::ExecutionMode;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Define the goal
    let goal = "build a REST API with user authentication";

    // Create dry-run configuration
    let config = DryRunConfig {
        execution_mode: ExecutionMode::Standard,
        ..Default::default()
    };

    println!("Starting dry-run for goal: {}", goal);
    println!("This will generate and assess tasks without executing them.\n");

    // Run dry-run mode
    let result = run_dry_run(goal, &config).await?;

    // You can also access the result programmatically
    println!("\nProgrammatic access to results:");
    println!("Total tasks: {}", result.statistics.total_tasks);
    println!("Execution depth: {}", result.statistics.execution_depth);
    println!("Simple tasks: {}", result.statistics.simple_tasks);
    println!("Moderate tasks: {}", result.statistics.moderate_tasks);
    println!("Complex tasks: {}", result.statistics.complex_tasks);

    Ok(())
}
