use ltmatrix::memory::{ProjectMemory, RunMemory};
use ltmatrix::pipeline::execute::load_unified_memory;
use std::path::Path;
use tempfile::TempDir;

#[tokio::test]
async fn test_load_unified_memory_returns_empty_string_when_no_memory() {
    let temp_dir = TempDir::new().unwrap();
    let result = load_unified_memory(temp_dir.path()).await.unwrap();
    assert!(result.contains("No project memory available"));
}

#[tokio::test]
async fn test_load_unified_memory_loads_project_memory() {
    let temp_dir = TempDir::new().unwrap();

    // Create project memory
    let memory_dir = temp_dir.path().join(".ltmatrix").join("memory");
    tokio::fs::create_dir_all(&memory_dir).await.unwrap();

    let mut project_mem = ProjectMemory::new("test-project");
    project_mem.tech_stack.language = Some("Rust".to_string());
    project_mem
        .save(&memory_dir.join("project.json"))
        .await
        .unwrap();

    let result = load_unified_memory(temp_dir.path()).await.unwrap();
    assert!(result.contains("test-project"));
    assert!(result.contains("Rust"));
}

#[tokio::test]
async fn test_load_unified_memory_falls_back_to_legacy() {
    let temp_dir = TempDir::new().unwrap();

    // Create old-style memory.md
    let claude_dir = temp_dir.path().join(".claude");
    tokio::fs::create_dir_all(&claude_dir).await.unwrap();
    tokio::fs::write(
        claude_dir.join("memory.md"),
        b"# Legacy Memory\nThis is the old format",
    )
    .await
    .unwrap();

    let result = load_unified_memory(temp_dir.path()).await.unwrap();
    assert!(result.contains("Legacy Memory"));
}
