//! Task Generation Demo
//!
//! This example demonstrates how to use the generate stage to break down
//! a user goal into structured tasks using Claude.

use ltmatrix::pipeline::generate::{generate_tasks, GenerateConfig};
use ltmatrix::pipeline::generate::calculate_generation_stats;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("ltmatrix=debug,info")
        .init();

    println!("=== Task Generation Demo ===\n");

    // Example user goals to demonstrate generation
    let goals = vec![
        "Implement a REST API for user management with CRUD operations",
        "Add authentication and authorization to the web application",
        "Create a real-time chat feature using WebSockets",
    ];

    for (i, goal) in goals.iter().enumerate() {
        println!("\n--- Example {} ---", i + 1);
        println!("Goal: {}\n", goal);

        // Use fast mode for quick demonstration
        let config = GenerateConfig::fast_mode();

        match generate_tasks(goal, &config).await {
            Ok(result) => {
                println!("✓ Generation successful!");
                println!("  Tasks generated: {}", result.task_count);
                println!("  Dependency depth: {}", result.dependency_depth);

                // Show validation errors if any
                if !result.validation_errors.is_empty() {
                    println!("\n⚠ Validation warnings:");
                    for error in &result.validation_errors {
                        println!("  - {}", error);
                    }
                }

                // Calculate and display statistics
                let stats = calculate_generation_stats(&result);
                println!("\n{}", stats);

                // Show first few tasks
                println!("\nFirst {} tasks:", result.tasks.len().min(5));
                for (idx, task) in result.tasks.iter().take(5).enumerate() {
                    println!(
                        "  {}. [{}] {} - {}",
                        idx + 1,
                        task.id,
                        task.title,
                        complexity_short(&task.complexity)
                    );
                    if !task.depends_on.is_empty() {
                        println!("     Depends on: {}", task.depends_on.join(", "));
                    }
                }

                if result.tasks.len() > 5 {
                    println!("  ... and {} more tasks", result.tasks.len() - 5);
                }
            }
            Err(e) => {
                eprintln!("✗ Generation failed: {}", e);
            }
        }
    }

    println!("\n=== Demo Complete ===");
    Ok(())
}

/// Returns a short label for task complexity
fn complexity_short(complexity: &ltmatrix::models::TaskComplexity) -> &'static str {
    match complexity {
        ltmatrix::models::TaskComplexity::Simple => "🟢 Simple",
        ltmatrix::models::TaskComplexity::Moderate => "🟡 Moderate",
        ltmatrix::models::TaskComplexity::Complex => "🔴 Complex",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_demo_compiles() {
        // This test just verifies the demo compiles correctly
        let goal = "Simple test goal";
        let config = GenerateConfig::fast_mode();

        // We don't actually run it (would require Claude API),
        // but we verify the types match
        let _ = generate_tasks(goal, &config);
        let _ = calculate_generation_stats;
    }

    #[test]
    fn test_complexity_short() {
        // Test the helper function
        assert_eq!(
            complexity_short(&ltmatrix::models::TaskComplexity::Simple),
            "🟢 Simple"
        );
        assert_eq!(
            complexity_short(&ltmatrix::models::TaskComplexity::Moderate),
            "🟡 Moderate"
        );
        assert_eq!(
            complexity_short(&ltmatrix::models::TaskComplexity::Complex),
            "🔴 Complex"
        );
    }
}
