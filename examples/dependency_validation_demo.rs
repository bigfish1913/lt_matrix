//! Dependency Validation Demo
//!
//! This example demonstrates how to use the dependency validation functions
//! to detect missing references and circular dependencies in task graphs.

use ltmatrix::models::Task;
use ltmatrix::pipeline::generate::{
    validate_dependencies, validate_dependencies_with_stats, DependencyGraphStats,
};

fn main() -> anyhow::Result<()> {
    println!("=== Dependency Validation Demo ===\n");

    // Example 1: Valid task graph
    println!("📋 Example 1: Valid Task Graph");
    println!("─────────────────────────────────");
    let valid_tasks = create_valid_task_graph();
    print_task_graph(&valid_tasks);

    let errors = validate_dependencies(&valid_tasks);
    println!("\nValidation result: {}", if errors.is_empty() { "✅ VALID" } else { "❌ INVALID" });
    if errors.is_empty() {
        println!("No dependency errors found!\n");
    }

    // Example 2: Missing dependency
    println!("📋 Example 2: Missing Dependencies");
    println!("────────────────────────────────────");
    let missing_tasks = create_task_with_missing_dependency();
    print_task_graph(&missing_tasks);

    let errors = validate_dependencies(&missing_tasks);
    println!("\nValidation result: {}", if errors.is_empty() { "✅ VALID" } else { "❌ INVALID" });
    for error in &errors {
        println!("  ❌ {}", error);
    }
    println!();

    // Example 3: Circular dependency
    println!("📋 Example 3: Circular Dependencies");
    println!("────────────────────────────────────");
    let circular_tasks = create_task_with_circular_dependency();
    print_task_graph(&circular_tasks);

    let errors = validate_dependencies(&circular_tasks);
    println!("\nValidation result: {}", if errors.is_empty() { "✅ VALID" } else { "❌ INVALID" });
    for error in &errors {
        println!("  ❌ {}", error);
    }
    println!();

    // Example 4: Complex graph with statistics
    println!("📋 Example 4: Complex Graph with Statistics");
    println!("───────────────────────────────────────────");
    let complex_tasks = create_complex_task_graph();
    print_task_graph(&complex_tasks);

    let result = validate_dependencies_with_stats(&complex_tasks);
    println!("\nValidation result: {}", if result.is_valid { "✅ VALID" } else { "❌ INVALID" });
    print_dependency_stats(&result.stats);

    Ok(())
}

/// Creates a valid linear task graph
fn create_valid_task_graph() -> Vec<Task> {
    vec![
        {
            let mut t = Task::new("setup", "Setup Project", "Initialize project structure");
            t.depends_on = vec![];
            t
        },
        {
            let mut t = Task::new("implement", "Implement Feature", "Write the feature code");
            t.depends_on = vec!["setup".to_string()];
            t
        },
        {
            let mut t = Task::new("test", "Write Tests", "Add unit tests");
            t.depends_on = vec!["implement".to_string()];
            t
        },
        {
            let mut t = Task::new("deploy", "Deploy", "Deploy to production");
            t.depends_on = vec!["test".to_string()];
            t
        },
    ]
}

/// Creates a task graph with missing dependencies
fn create_task_with_missing_dependency() -> Vec<Task> {
    vec![
        {
            let mut t = Task::new("task-1", "Task 1", "First task");
            t.depends_on = vec![];
            t
        },
        {
            let mut t = Task::new("task-2", "Task 2", "Depends on non-existent task");
            t.depends_on = vec!["non-existent".to_string()];
            t
        },
    ]
}

/// Creates a task graph with circular dependencies
fn create_task_with_circular_dependency() -> Vec<Task> {
    vec![
        {
            let mut t = Task::new("design", "Design", "Design the system");
            t.depends_on = vec!["implement".to_string()];
            t
        },
        {
            let mut t = Task::new("implement", "Implement", "Implement the system");
            t.depends_on = vec!["test".to_string()];
            t
        },
        {
            let mut t = Task::new("test", "Test", "Test the system");
            t.depends_on = vec!["design".to_string()];
            t
        },
    ]
}

/// Creates a complex task graph (diamond structure)
fn create_complex_task_graph() -> Vec<Task> {
    vec![
        {
            let mut t = Task::new("root", "Root Task", "Initial setup");
            t.depends_on = vec![];
            t
        },
        {
            let mut t = Task::new("branch-a", "Branch A", "First branch");
            t.depends_on = vec!["root".to_string()];
            t
        },
        {
            let mut t = Task::new("branch-b", "Branch B", "Second branch");
            t.depends_on = vec!["root".to_string()];
            t
        },
        {
            let mut t = Task::new("merge", "Merge", "Merge both branches");
            t.depends_on = vec!["branch-a".to_string(), "branch-b".to_string()];
            t
        },
        {
            let mut t = Task::new("final", "Final", "Final task");
            t.depends_on = vec!["merge".to_string()];
            t
        },
    ]
}

/// Prints a task graph in a readable format
fn print_task_graph(tasks: &[Task]) {
    println!("Tasks ({})", tasks.len());
    for task in tasks {
        print!("  [{}] {} → ", task.id, task.title);
        if task.depends_on.is_empty() {
            print!("(no dependencies)");
        } else {
            print!("depends_on: [{}]", task.depends_on.join(", "));
        }
        println!();
    }
}

/// Prints dependency graph statistics
fn print_dependency_stats(stats: &DependencyGraphStats) {
    println!("📊 Dependency Graph Statistics:");
    println!("  Total tasks: {}", stats.total_tasks);
    println!("  Tasks with dependencies: {}", stats.tasks_with_dependencies);
    println!("  Total dependencies: {}", stats.total_dependencies);
    println!("  Max depth: {}", stats.max_depth);
    println!("  Root tasks (no deps): {}", stats.root_tasks);
    println!("  Leaf tasks (no dependents): {}", stats.leaf_tasks);
    println!("  Missing dependencies: {}", stats.missing_dependencies);
    println!("  Circular dependencies: {}", stats.circular_dependencies);
    println!("  Is DAG: {}", if stats.is_dag { "✅ Yes" } else { "❌ No" });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_task_graph() {
        let tasks = create_valid_task_graph();
        let errors = validate_dependencies(&tasks);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_missing_dependency() {
        let tasks = create_task_with_missing_dependency();
        let errors = validate_dependencies(&tasks);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn test_circular_dependency() {
        let tasks = create_task_with_circular_dependency();
        let errors = validate_dependencies(&tasks);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn test_complex_graph_stats() {
        let tasks = create_complex_task_graph();
        let result = validate_dependencies_with_stats(&tasks);
        assert!(result.is_valid);
        assert_eq!(result.stats.total_tasks, 5);
        assert_eq!(result.stats.max_depth, 3);
        assert!(result.stats.is_dag);
    }

    #[test]
    fn test_diamond_structure() {
        let tasks = create_complex_task_graph();
        let result = validate_dependencies_with_stats(&tasks);
        assert!(result.is_valid, "Diamond structure should be valid");
        assert_eq!(result.stats.root_tasks, 1);
        assert_eq!(result.stats.leaf_tasks, 1);
    }
}
