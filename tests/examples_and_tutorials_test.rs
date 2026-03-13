//! Tests for usage examples and tutorials (Task: Create usage examples and tutorials)
//!
//! This test module verifies:
//! 1. Required example project directories exist
//! 2. Each example has a README.md explaining purpose and expected output
//! 3. The walkthrough tutorial exists in docs/tutorial.md
//!
//! Expected examples (as implemented):
//! - simple-cli-tool: Simple single-task project
//! - multi-task-workflow: Multi-task with dependencies
//! - web-api-testing: Web project with testing
//! - cross-platform-lib: Cross-platform project

use std::fs;

/// Example project directories with their descriptions
const EXPECTED_EXAMPLES: &[(&str, &str)] = &[
    ("simple-cli-tool", "Simple single-task project"),
    ("multi-task-workflow", "Multi-task with dependencies"),
    ("web-api-testing", "Web project with testing"),
    ("cross-platform-lib", "Cross-platform project"),
];

const EXAMPLES_DIR: &str = "examples";
const DOCS_DIR: &str = "docs";
const TUTORIAL_FILE: &str = "tutorial.md";

fn get_project_root() -> std::path::PathBuf {
    // When running tests, current directory is typically the project root
    std::env::current_dir().expect("Failed to get current directory")
}

// ============================================================================
// Directory Existence Tests
// ============================================================================

#[test]
fn examples_directory_exists() {
    let root = get_project_root();
    let examples_path = root.join(EXAMPLES_DIR);

    assert!(
        examples_path.exists() && examples_path.is_dir(),
        "examples/ directory should exist at {:?}",
        examples_path
    );
}

#[test]
fn simple_cli_tool_example_exists() {
    let root = get_project_root();
    let example_path = root.join(EXAMPLES_DIR).join("simple-cli-tool");

    assert!(
        example_path.exists() && example_path.is_dir(),
        "simple-cli-tool example directory should exist at {:?}",
        example_path
    );
}

#[test]
fn multi_task_workflow_example_exists() {
    let root = get_project_root();
    let example_path = root.join(EXAMPLES_DIR).join("multi-task-workflow");

    assert!(
        example_path.exists() && example_path.is_dir(),
        "multi-task-workflow example directory should exist at {:?}",
        example_path
    );
}

#[test]
fn web_api_testing_example_exists() {
    let root = get_project_root();
    let example_path = root.join(EXAMPLES_DIR).join("web-api-testing");

    assert!(
        example_path.exists() && example_path.is_dir(),
        "web-api-testing example directory should exist at {:?}",
        example_path
    );
}

#[test]
fn cross_platform_lib_example_exists() {
    let root = get_project_root();
    let example_path = root.join(EXAMPLES_DIR).join("cross-platform-lib");

    assert!(
        example_path.exists() && example_path.is_dir(),
        "cross-platform-lib example directory should exist at {:?}",
        example_path
    );
}

// ============================================================================
// README Existence Tests
// ============================================================================

#[test]
fn simple_cli_tool_has_readme() {
    let root = get_project_root();
    let readme_path = root
        .join(EXAMPLES_DIR)
        .join("simple-cli-tool")
        .join("README.md");

    assert!(
        readme_path.exists(),
        "simple-cli-tool example should have a README.md at {:?}",
        readme_path
    );
}

#[test]
fn multi_task_workflow_has_readme() {
    let root = get_project_root();
    let readme_path = root
        .join(EXAMPLES_DIR)
        .join("multi-task-workflow")
        .join("README.md");

    assert!(
        readme_path.exists(),
        "multi-task-workflow example should have a README.md at {:?}",
        readme_path
    );
}

#[test]
fn web_api_testing_has_readme() {
    let root = get_project_root();
    let readme_path = root
        .join(EXAMPLES_DIR)
        .join("web-api-testing")
        .join("README.md");

    assert!(
        readme_path.exists(),
        "web-api-testing example should have a README.md at {:?}",
        readme_path
    );
}

#[test]
fn cross_platform_lib_has_readme() {
    let root = get_project_root();
    let readme_path = root
        .join(EXAMPLES_DIR)
        .join("cross-platform-lib")
        .join("README.md");

    assert!(
        readme_path.exists(),
        "cross-platform-lib example should have a README.md at {:?}",
        readme_path
    );
}

// ============================================================================
// README Content Tests
// ============================================================================

#[test]
fn simple_cli_tool_readme_explains_purpose() {
    let root = get_project_root();
    let readme_path = root
        .join(EXAMPLES_DIR)
        .join("simple-cli-tool")
        .join("README.md");

    let content = fs::read_to_string(&readme_path)
        .expect("Failed to read README.md - run simple_cli_tool_has_readme test first");

    // README should explain purpose
    assert!(
        content.to_lowercase().contains("purpose")
            || content.to_lowercase().contains("goal")
            || content.to_lowercase().contains("demonstrates")
            || content.to_lowercase().contains("this example"),
        "simple-cli-tool README should explain the purpose of the example"
    );

    // README should show expected output
    assert!(
        content.contains("Expected") || content.contains("output") || content.contains("```\n"),
        "simple-cli-tool README should show expected output or behavior"
    );
}

#[test]
fn multi_task_workflow_readme_explains_dependencies() {
    let root = get_project_root();
    let readme_path = root
        .join(EXAMPLES_DIR)
        .join("multi-task-workflow")
        .join("README.md");

    let content = fs::read_to_string(&readme_path)
        .expect("Failed to read README.md - run multi_task_workflow_has_readme test first");

    // README should explain task dependencies
    assert!(
        content.to_lowercase().contains("dependenc")
            || content.to_lowercase().contains("depend")
            || content.to_lowercase().contains("task")
            || content.to_lowercase().contains("workflow"),
        "multi-task-workflow README should explain task dependencies or workflow"
    );
}

#[test]
fn web_api_testing_readme_explains_testing() {
    let root = get_project_root();
    let readme_path = root
        .join(EXAMPLES_DIR)
        .join("web-api-testing")
        .join("README.md");

    let content = fs::read_to_string(&readme_path)
        .expect("Failed to read README.md - run web_api_testing_has_readme test first");

    // README should explain testing aspect
    assert!(
        content.to_lowercase().contains("test")
            || content.to_lowercase().contains("e2e")
            || content.to_lowercase().contains("integration")
            || content.to_lowercase().contains("api"),
        "web-api-testing README should explain testing approach or API testing"
    );
}

#[test]
fn cross_platform_lib_readme_explains_platforms() {
    let root = get_project_root();
    let readme_path = root
        .join(EXAMPLES_DIR)
        .join("cross-platform-lib")
        .join("README.md");

    let content = fs::read_to_string(&readme_path)
        .expect("Failed to read README.md - run cross_platform_lib_has_readme test first");

    // README should explain cross-platform considerations
    assert!(
        content.to_lowercase().contains("platform")
            || content.to_lowercase().contains("windows")
            || content.to_lowercase().contains("linux")
            || content.to_lowercase().contains("macos")
            || content.to_lowercase().contains("cross"),
        "cross-platform-lib README should explain platform considerations"
    );
}

// ============================================================================
// Tutorial Tests
// ============================================================================

#[test]
fn tutorial_exists() {
    let root = get_project_root();
    let tutorial_path = root.join(DOCS_DIR).join(TUTORIAL_FILE);

    assert!(
        tutorial_path.exists(),
        "docs/tutorial.md should exist at {:?}",
        tutorial_path
    );
}

#[test]
fn tutorial_is_walkthrough() {
    let root = get_project_root();
    let tutorial_path = root.join(DOCS_DIR).join(TUTORIAL_FILE);

    let content = fs::read_to_string(&tutorial_path)
        .expect("Failed to read tutorial.md - run tutorial_exists test first");

    // Tutorial should have substantial content
    assert!(
        content.lines().count() > 50,
        "Tutorial should have substantial content (more than 50 lines), found {}",
        content.lines().count()
    );

    // Tutorial should have headings/structure
    assert!(
        content.contains("# ") || content.contains("## "),
        "Tutorial should have markdown headings for structure"
    );

    // Tutorial should explain how to use ltmatrix
    assert!(
        content.to_lowercase().contains("ltmatrix")
            || content.to_lowercase().contains("usage")
            || content.to_lowercase().contains("getting started"),
        "Tutorial should explain ltmatrix usage"
    );
}

#[test]
fn tutorial_covers_examples() {
    let root = get_project_root();
    let tutorial_path = root.join(DOCS_DIR).join(TUTORIAL_FILE);

    let content = fs::read_to_string(&tutorial_path)
        .expect("Failed to read tutorial.md - run tutorial_exists test first");

    // Tutorial should reference or cover the examples
    let covers_examples = content.to_lowercase().contains("example")
        || content.to_lowercase().contains("simple")
        || content.to_lowercase().contains("cli")
        || content.to_lowercase().contains("task");

    assert!(
        covers_examples,
        "Tutorial should reference or cover the example projects"
    );
}

// ============================================================================
// Aggregate Tests
// ============================================================================

#[test]
fn all_example_directories_have_readme_files() {
    let root = get_project_root();
    let examples_dir = root.join(EXAMPLES_DIR);

    let mut missing_readmes: Vec<String> = Vec::new();

    for entry in fs::read_dir(&examples_dir).expect("Failed to read examples directory") {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();

        if path.is_dir() {
            let readme_path = path.join("README.md");
            if !readme_path.exists() {
                missing_readmes.push(path.file_name().unwrap().to_string_lossy().to_string());
            }
        }
    }

    assert!(
        missing_readmes.is_empty(),
        "Example directories missing README.md: {:?}",
        missing_readmes
    );
}

#[test]
fn all_example_readmes_contain_command_examples() {
    let root = get_project_root();
    let examples_dir = root.join(EXAMPLES_DIR);

    let mut readmes_without_commands: Vec<String> = Vec::new();
    let mut checked_count = 0;

    for entry in fs::read_dir(&examples_dir).expect("Failed to read examples directory") {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();

        if path.is_dir() {
            let readme_path = path.join("README.md");
            if readme_path.exists() {
                let content = fs::read_to_string(&readme_path).expect("Failed to read README");

                // READMEs should show command examples
                if !content.contains("ltmatrix") && !content.contains("```") {
                    readmes_without_commands
                        .push(path.file_name().unwrap().to_string_lossy().to_string());
                }
                checked_count += 1;
            }
        }
    }

    assert!(
        checked_count > 0,
        "At least one example README should exist"
    );
    assert!(
        readmes_without_commands.is_empty(),
        "Example READMEs without command examples: {:?}",
        readmes_without_commands
    );
}

#[test]
fn expected_example_count() {
    let root = get_project_root();
    let examples_dir = root.join(EXAMPLES_DIR);

    let mut example_count = 0;
    for entry in fs::read_dir(&examples_dir).expect("Failed to read examples directory") {
        let entry = entry.expect("Failed to read directory entry");
        if entry.path().is_dir() {
            example_count += 1;
        }
    }

    assert!(
        example_count >= EXPECTED_EXAMPLES.len(),
        "Expected at least {} example directories, found {}",
        EXPECTED_EXAMPLES.len(),
        example_count
    );
}
