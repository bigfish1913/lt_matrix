//! Example: Validation Utilities
//!
//! Demonstrates how to use the validation utilities in ltmatrix

use ltmatrix::validate::{
    validate_agent_available, validate_directory_creatable, validate_file_permissions,
    validate_git_repository, validate_goal, validate_task_id_format, validate_task_ids_unique,
    validate_workspace,
};

fn main() {
    println!("=== ltmatrix Validation Utilities Demo ===\n");

    // 1. Goal Validation
    println!("1. Goal Validation");
    println!("-------------------");

    let long_goal = "a".repeat(11_000);
    let goals = vec![
        "Build a REST API with authentication",
        "",         // Invalid: empty
        "!!!",      // Invalid: no alphanumeric
        &long_goal, // Invalid: too long
    ];

    for goal in goals {
        match validate_goal(goal) {
            Ok(()) => println!(
                "✓ Valid goal: '{}{}'",
                goal.chars().take(50).collect::<String>(),
                if goal.len() > 50 { "..." } else { "" }
            ),
            Err(e) => println!("✗ Invalid goal: {}", e),
        }
    }
    println!();

    // 2. Task ID Validation
    println!("2. Task ID Validation");
    println!("----------------------");

    let task_ids = vec![
        "task-1",
        "task-42",
        "task-1-1",   // Subtask
        "task-1-2-3", // Nested subtask
        "invalid",    // Invalid format
        "Task-1",     // Invalid: wrong case
        "task_",      // Invalid: wrong separator
    ];

    for task_id in task_ids {
        match validate_task_id_format(task_id) {
            Ok(()) => println!("✓ Valid task ID: {}", task_id),
            Err(e) => println!("✗ Invalid task ID: {}", e),
        }
    }
    println!();

    // 3. Task ID Uniqueness
    println!("3. Task ID Uniqueness");
    println!("----------------------");

    let unique_ids = vec![
        "task-1".to_string(),
        "task-2".to_string(),
        "task-3".to_string(),
    ];
    match validate_task_ids_unique(&unique_ids) {
        Ok(()) => println!("✓ All task IDs are unique: {:?}", unique_ids),
        Err(e) => println!("✗ Duplicate IDs: {}", e),
    }

    let duplicate_ids = vec![
        "task-1".to_string(),
        "task-2".to_string(),
        "task-1".to_string(),
    ];
    match validate_task_ids_unique(&duplicate_ids) {
        Ok(()) => println!("✓ All task IDs are unique: {:?}", duplicate_ids),
        Err(e) => println!("✗ Duplicate IDs: {}", e),
    }
    println!();

    // 4. Agent Availability
    println!("4. Agent Availability");
    println!("----------------------");

    let agents = vec!["claude", "opencode", "nonexistent_agent_xyz123"];
    for agent in agents {
        match validate_agent_available(agent) {
            Ok(()) => println!("✓ Agent available: {}", agent),
            Err(e) => println!("✗ Agent not available: {}", e),
        }
    }
    println!();

    // 5. Git Repository Validation
    println!("5. Git Repository Validation");
    println!("-----------------------------");

    let current_dir = std::env::current_dir().unwrap();
    match validate_git_repository(&current_dir) {
        Ok(()) => println!("✓ Valid git repository: {}", current_dir.display()),
        Err(e) => println!("✗ Not a valid git repository: {}", e),
    }

    let temp_path = std::env::temp_dir();
    match validate_git_repository(&temp_path) {
        Ok(()) => println!("✓ Valid git repository: {}", temp_path.display()),
        Err(e) => println!("✗ Not a valid git repository: {}", e),
    }
    println!();

    // 6. File Permissions
    println!("6. File Permissions");
    println!("--------------------");

    let current_dir = std::env::current_dir().unwrap();
    match validate_file_permissions(&current_dir, false) {
        Ok(()) => println!("✓ Read permission: {}", current_dir.display()),
        Err(e) => println!("✗ No read permission: {}", e),
    }

    match validate_file_permissions(&current_dir, true) {
        Ok(()) => println!("✓ Write permission: {}", current_dir.display()),
        Err(e) => println!("✗ No write permission: {}", e),
    }
    println!();

    // 7. Directory Creation
    println!("7. Directory Creation Validation");
    println!("----------------------------------");

    let new_dir = current_dir.join("test_new_directory");
    match validate_directory_creatable(&new_dir) {
        Ok(()) => println!("✓ Can create directory: {}", new_dir.display()),
        Err(e) => println!("✗ Cannot create directory: {}", e),
    }
    println!();

    // 8. Workspace Validation
    println!("8. Workspace Validation");
    println!("------------------------");

    match validate_workspace(&current_dir) {
        Ok(()) => println!("✓ Valid workspace: {}", current_dir.display()),
        Err(e) => println!("✗ Invalid workspace: {}", e),
    }
    println!();

    // 9. Combined Workflow
    println!("9. Combined Validation Workflow");
    println!("--------------------------------");
    println!("Validating a complete project setup...\n");

    // Step 1: Validate goal
    match validate_goal("Build a REST API with user authentication and rate limiting") {
        Ok(()) => println!("  ✓ Validating goal: Goal validated"),
        Err(e) => println!("  ✗ Validating goal: {}", e),
    }

    // Step 2: Validate task IDs
    let tasks = vec![
        "task-1".to_string(),
        "task-2".to_string(),
        "task-3".to_string(),
    ];
    match validate_task_ids_unique(&tasks) {
        Ok(()) => println!("  ✓ Validating task IDs: Task IDs validated"),
        Err(e) => println!("  ✗ Validating task IDs: {}", e),
    }

    // Step 3: Validate workspace
    match validate_workspace(&current_dir) {
        Ok(()) => println!("  ✓ Validating workspace: Workspace validated"),
        Err(e) => println!("  ✗ Validating workspace: {}", e),
    }

    // Step 4: Validate permissions
    match validate_file_permissions(&current_dir, true) {
        Ok(()) => println!("  ✓ Validating permissions: Permissions validated"),
        Err(e) => println!("  ✗ Validating permissions: {}", e),
    }

    println!("\n=== Demo Complete ===");
}
