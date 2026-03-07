//! Testing framework detection
//!
//! This module provides functionality to detect which testing framework
//! is being used in a project and map it to the appropriate test command.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Supported testing frameworks
///
/// Each variant represents a different testing framework or toolchain
/// that can be automatically detected and used for running tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Framework {
    /// Python's pytest framework
    ///
    /// Detected by the presence of:
    /// - `pytest.ini` or `pyproject.toml` with `[tool.pytest]`
    /// - `tests/` directory with Python test files
    /// - Test files named `test_*.py` or `*_test.py`
    Pytest,

    /// JavaScript/TypeScript npm test scripts
    ///
    /// Detected by the presence of:
    /// - `package.json` with `scripts.test` defined
    /// - `node_modules/` directory
    Npm,

    /// Go's built-in testing framework
    ///
    /// Detected by the presence of:
    /// - `*.go` files with `*_test.go` naming pattern
    /// - `go.mod` file
    Go,

    /// Rust's built-in testing framework via Cargo
    ///
    /// Detected by the presence of:
    /// - `Cargo.toml` file
    /// - Rust test files (in `tests/` or `src/` with `#[cfg(test)]`)
    Cargo,

    /// No testing framework detected
    ///
    /// Returned when no known testing framework files or patterns
    /// are found in the project directory.
    None,
}

impl Framework {
    /// Get the display name for this framework
    pub fn name(&self) -> &str {
        match self {
            Framework::Pytest => "pytest",
            Framework::Npm => "npm",
            Framework::Go => "go test",
            Framework::Cargo => "cargo test",
            Framework::None => "none",
        }
    }

    /// Get the command used to run tests for this framework
    pub fn command(&self) -> &[&str] {
        match self {
            Framework::Pytest => &["pytest", "-v"],
            Framework::Npm => &["npm", "test"],
            Framework::Go => &["go", "test", "./..."],
            Framework::Cargo => &["cargo", "test"],
            Framework::None => &[],
        }
    }
}

impl std::fmt::Display for Framework {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Test command configuration
///
/// Represents a complete test command with all necessary arguments
/// and environment variables for running tests in a specific framework.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestCommand {
    /// The framework this command is for
    pub framework: Framework,

    /// Command to execute (e.g., "pytest", "cargo", "npm")
    pub program: String,

    /// Arguments to pass to the command
    pub args: Vec<String>,

    /// Environment variables to set when running the command
    pub env_vars: Vec<(String, String)>,

    /// Working directory for the command (None = current directory)
    pub work_dir: Option<String>,
}

impl TestCommand {
    /// Create a new test command for the given framework
    pub fn new(framework: Framework) -> Self {
        let (program, args) = match framework {
            Framework::Pytest => ("pytest", vec!["-v".to_string()]),
            Framework::Npm => ("npm", vec!["test".to_string()]),
            Framework::Go => ("go", vec!["test".to_string(), "./...".to_string()]),
            Framework::Cargo => ("cargo", vec!["test".to_string()]),
            Framework::None => ("echo", vec!["No tests configured".to_string()]),
        };

        TestCommand {
            framework,
            program: program.to_string(),
            args,
            env_vars: Vec::new(),
            work_dir: None,
        }
    }

    /// Create a test command for the given framework with defaults
    pub fn for_framework(framework: &Framework) -> Self {
        Self::new(*framework)
    }

    /// Add an argument to the command
    pub fn with_arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Add multiple arguments to the command
    pub fn with_args(mut self, args: &[impl AsRef<str>]) -> Self {
        self.args.extend(args.iter().map(|s| s.as_ref().to_string()));
        self
    }

    /// Add an environment variable to the command
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.push((key.into(), value.into()));
        self
    }

    /// Set the working directory for the command
    pub fn with_work_dir(mut self, dir: impl Into<String>) -> Self {
        self.work_dir = Some(dir.into());
        self
    }

    /// Convert to a command-like format suitable for execution
    pub fn to_command_line(&self) -> String {
        let cmd = format!("{} {}", self.program, self.args.join(" "));
        if let Some(dir) = &self.work_dir {
            format!("(cd {} && {})", dir, cmd)
        } else {
            cmd
        }
    }
}

/// Detect the testing framework used in a project directory
///
/// This function examines the directory structure and configuration files
/// to determine which testing framework is being used. Detection is performed
/// in a specific priority order to handle projects with multiple frameworks.
///
/// # Detection Priority
///
/// 1. **Cargo** - Checks for `Cargo.toml`
/// 2. **Go** - Checks for `go.mod` and `*_test.go` files
/// 3. **Pytest** - Checks for `pytest.ini`, `pyproject.toml` with `[tool.pytest]`,
///    or `tests/` directory with `test_*.py` files
/// 4. **Npm** - Checks for `package.json` with `scripts.test`
/// 5. **None** - No framework detected
///
/// # Arguments
///
/// * `project_dir` - Path to the project directory to analyze
///
/// # Returns
///
/// - `Ok(Framework)` - The detected framework (may be `Framework::None`)
/// - `Err(anyhow::Error)` - Error reading directory or checking files
///
/// # Example
///
/// ```no_run
/// use ltmatrix::testing::detect_framework;
///
/// # async fn example() -> anyhow::Result<()> {
/// let framework = detect_framework(".").await?;
/// println!("Detected framework: {}", framework);
/// # Ok(())
/// # }
/// ```
///
/// # Note
///
/// This is currently a stub that returns `Framework::None`. The actual
/// detection logic will be implemented in a follow-up task.
pub async fn detect_framework(project_dir: impl AsRef<std::path::Path>) -> Result<Framework> {
    let _project_dir = project_dir.as_ref();

    // TODO: Implement framework detection logic
    // This will include:
    // - Checking for configuration files (Cargo.toml, go.mod, package.json, etc.)
    // - Scanning directory structure for test files
    // - Analyzing configuration files for test settings
    // - Returning the appropriate framework or None

    Ok(Framework::None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_framework_display_names() {
        assert_eq!(Framework::Pytest.name(), "pytest");
        assert_eq!(Framework::Npm.name(), "npm");
        assert_eq!(Framework::Go.name(), "go test");
        assert_eq!(Framework::Cargo.name(), "cargo test");
        assert_eq!(Framework::None.name(), "none");
    }

    #[test]
    fn test_framework_commands() {
        assert_eq!(Framework::Pytest.command(), &["pytest", "-v"] as &[&str]);
        assert_eq!(Framework::Npm.command(), &["npm", "test"] as &[&str]);
        assert_eq!(Framework::Go.command(), &["go", "test", "./..."] as &[&str]);
        assert_eq!(Framework::Cargo.command(), &["cargo", "test"] as &[&str]);
        assert_eq!(Framework::None.command(), &[] as &[&str]);
    }

    #[test]
    fn test_test_command_creation() {
        let cmd = TestCommand::new(Framework::Pytest);
        assert_eq!(cmd.framework, Framework::Pytest);
        assert_eq!(cmd.program, "pytest");
        assert_eq!(cmd.args, vec!["-v"]);
        assert!(cmd.env_vars.is_empty());
        assert!(cmd.work_dir.is_none());
    }

    #[test]
    fn test_test_command_builder() {
        let cmd = TestCommand::new(Framework::Cargo)
            .with_arg("--release")
            .with_args(&["--no-fail-fast", "--verbose"])
            .with_env("RUST_LOG", "debug")
            .with_work_dir("/tmp/test");

        assert_eq!(cmd.program, "cargo");
        assert_eq!(cmd.args, vec!["test", "--release", "--no-fail-fast", "--verbose"]);
        assert_eq!(cmd.env_vars, vec![("RUST_LOG".to_string(), "debug".to_string())]);
        assert_eq!(cmd.work_dir, Some("/tmp/test".to_string()));
    }

    #[test]
    fn test_test_command_for_framework() {
        let cmd = TestCommand::for_framework(&Framework::Go);
        assert_eq!(cmd.framework, Framework::Go);
        assert_eq!(cmd.program, "go");
        assert_eq!(cmd.args, vec!["test", "./..."]);
    }

    #[test]
    fn test_test_command_to_command_line() {
        let cmd1 = TestCommand::new(Framework::Pytest);
        assert_eq!(cmd1.to_command_line(), "pytest -v");

        let cmd2 = TestCommand::new(Framework::Cargo)
            .with_work_dir("/my/project");
        assert_eq!(cmd2.to_command_line(), "(cd /my/project && cargo test)");
    }

    #[test]
    fn test_framework_equality() {
        assert_eq!(Framework::Pytest, Framework::Pytest);
        assert_ne!(Framework::Pytest, Framework::Npm);
    }

    #[test]
    fn test_framework_display() {
        assert_eq!(format!("{}", Framework::Pytest), "pytest");
        assert_eq!(format!("{}", Framework::Cargo), "cargo test");
    }

    #[tokio::test]
    async fn test_detect_framework_stub() {
        // The current implementation returns None
        let result = detect_framework(".").await.unwrap();
        assert_eq!(result, Framework::None);
    }
}
