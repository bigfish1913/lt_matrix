//! Task hierarchy tree visualization demo
//!
//! This example demonstrates the ASCII tree visualization for task hierarchies

use ltmatrix::models::{Task, TaskStatus, TaskComplexity};

fn main() {
    println!("=== Task Hierarchy Tree Visualization Demo ===\n");

    // Example 1: Simple single task
    println!("1. Single Task:");
    println!("{}", "=".repeat(50));
    let task1 = Task::new("task-1", "Implement feature X", "Add new feature");
    let tree1 = ltmatrix::tasks::tree::format_tree(&task1);
    println!("{}", tree1);
    println!();

    // Example 2: Task with subtasks
    println!("2. Task with Subtasks:");
    println!("{}", "=".repeat(50));
    let mut task2 = Task::new("task-2", "Build Authentication System", "Full auth implementation");
    task2.complexity = TaskComplexity::Complex;
    task2.subtasks = vec![
        Task::new("task-3", "Design Database Schema", "Create users table"),
        Task::new("task-4", "Implement Login API", "POST /login endpoint"),
        Task::new("task-5", "Add JWT Token Support", "Token generation and validation"),
    ];
    let tree2 = ltmatrix::tasks::tree::format_tree(&task2);
    println!("{}", tree2);
    println!();

    // Example 3: Nested task hierarchy (3 levels)
    println!("3. Nested Task Hierarchy (3 levels):");
    println!("{}", "=".repeat(50));
    let mut grandchild3 = Task::new("task-7", "Write Unit Tests", "Test coverage for API");
    grandchild3.status = TaskStatus::Completed;

    let mut child2 = Task::new("task-6", "Implement API Endpoints", "REST API");
    child2.subtasks = vec![
        Task::new("task-8", "GET Endpoint", "Retrieve data"),
        grandchild3,
    ];

    let mut root = Task::new("task-5", "E-commerce Backend", "Complete backend system");
    root.complexity = TaskComplexity::Complex;
    root.subtasks = vec![
        Task::new("task-9", "Setup Project", "Initialize repo"),
        child2,
    ];
    root.status = TaskStatus::InProgress;

    let tree3 = ltmatrix::tasks::tree::format_tree(&root);
    println!("{}", tree3);
    println!();

    // Example 4: Task with different statuses
    println!("4. Task with Various Statuses:");
    println!("{}", "=".repeat(50));
    let mut child1 = Task::new("task-11", "Design", "System design");
    child1.status = TaskStatus::Completed;

    let mut child2 = Task::new("task-12", "Implementation", "Code implementation");
    child2.status = TaskStatus::InProgress;

    let mut child3 = Task::new("task-13", "Testing", "Unit and integration tests");
    child3.status = TaskStatus::Pending;

    let mut child4 = Task::new("task-14", "Documentation", "API docs");
    child4.status = TaskStatus::Blocked;

    let mut root2 = Task::new("task-10", "Complete Feature", "Full feature lifecycle");
    root2.subtasks = vec![child1, child2, child3, child4];

    let tree4 = ltmatrix::tasks::tree::format_tree(&root2);
    println!("{}", tree4);
    println!();

    // Example 5: Deep hierarchy (max depth 3)
    println!("5. Deep Hierarchy (demonstrates max depth):");
    println!("{}", "=".repeat(50));
    let mut level4 = Task::new("task-15", "Level 4 Task", "Beyond max depth");

    let mut level3 = Task::new("task-16", "Level 3 Task", "Third level");
    level3.subtasks = vec![level4];

    let mut level2 = Task::new("task-17", "Level 2 Task", "Second level");
    level2.subtasks = vec![level3];

    let mut level1 = Task::new("task-18", "Level 1 Task", "First level");
    level1.subtasks = vec![level2];

    let mut root3 = Task::new("task-19", "Root Task", "Top level");
    root3.subtasks = vec![level1];

    let tree5 = ltmatrix::tasks::tree::format_tree(&root3);
    println!("{}", tree5);
    println!();

    // Legend
    println!("Status Legend:");
    println!("  ○ = Pending");
    println!("  ⚙ = In Progress");
    println!("  ✓ = Completed");
    println!("  ✗ = Failed");
    println!("  ⚠ = Blocked");
}
