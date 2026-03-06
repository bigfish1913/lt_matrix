# Test Framework Detection Implementation

## Overview

The test pipeline module now includes comprehensive framework detection logic that automatically identifies which testing framework is in use for a given project.

## Supported Frameworks

| Framework | Detection Method | Command |
|-----------|------------------|---------|
| **pytest** | pytest.ini, pyproject.toml [tool.pytest], test_*.py files | `pytest` |
| **npm** | package.json scripts.test, devDependencies | `npm test` |
| **Go** | go.mod, _test.go files | `go test ./...` |
| **Cargo** | Cargo.toml, tests/ directory, #[test]/#[cfg(test)] | `cargo test` |

## API Usage

```rust
use ltmatrix::pipeline::test::{detect_test_framework, TestFramework};
use std::path::Path;

// Detect framework in current directory
let project_dir = Path::new(".");
let detection = detect_test_framework(project_dir)?;

// Access detection results
println!("Framework: {}", detection.framework.display_name());
println!("Confidence: {:.1}%", detection.confidence * 100.0);
println!("Command: {}", detection.framework.test_command());

// Get config files and test paths
for config in &detection.config_files {
    println!("Config: {}", config.display());
}
for path in &detection.test_paths {
    println!("Test path: {}", path.display());
}
```

## Framework-Specific Detection

### Cargo (Rust)

Detection indicators:
- ✅ `Cargo.toml` exists (required)
- ✅ `tests/` directory exists
- ✅ Files contain `#[test]` attributes
- ✅ Files contain `#[cfg(test)]` module declarations

Confidence scoring:
- Base confidence: 1.0 (Cargo.toml is definitive)

### Go

Detection indicators:
- ✅ `go.mod` exists
- ✅ Files ending with `_test.go`

Confidence scoring:
- 1 indicator: 0.7
- 2 indicators: 1.0

### pytest (Python)

Detection indicators:
- ✅ `pytest.ini` exists
- ✅ `pyproject.toml` contains `[tool.pytest]` section
- ✅ `test_*.py` files in tests/ or test/ directories
- ✅ `test_*.py` files in project root

Confidence scoring:
- 1 indicator: 0.6
- 2 indicators: 0.9
- 3+ indicators: 1.0

### npm (Node.js)

Detection indicators:
- ✅ `package.json` exists (required)
- ✅ `scripts.test` defined
- ✅ Related test scripts exist (test:watch, test:coverage, etc.)
- ✅ Testing frameworks in devDependencies (jest, mocha, etc.)
- ✅ Common test directories exist (tests, test, __tests__, spec)

Confidence scoring:
- 1 indicator: 0.5
- 2 indicators: 0.8
- 3+ indicators: 1.0

## Helper Functions

### File System Utilities

```rust
// Check if file exists and is readable
let exists = file_exists_and_readable(Path::new("Cargo.toml"));

// Check if directory exists and is accessible
let accessible = directory_exists_and_accessible(Path::new("src"));

// Find files with specific suffix (recursive)
let go_tests = find_files_with_suffix(project_dir, "_test.go")?;

// Find files with prefix and extension (recursive)
let py_tests = find_files_with_prefix(project_dir, "test_", ".py")?;

// Read first N lines of a file
let lines = read_file_lines(Path::new("Cargo.toml"), 10)?;
```

### Configuration Parsing

```rust
// Parse TOML file and extract specific section
let section = parse_toml_section(
    Path::new("pyproject.toml"),
    "tool.pytest"
)?;

// Navigate nested sections
let nested = parse_toml_section(
    Path::new("pyproject.toml"),
    "tool.pytest.ini_options"
)?;
```

### Rust-Specific Utilities

```rust
// Scan for #[test] attributes in directory
let has_tests = scan_directory_for_test_attributes(Path::new("src"))?;

// Scan for #[cfg(test)] module declarations
let has_modules = scan_directory_for_test_modules(Path::new("src"))?;
```

## Testing

All framework detection logic includes comprehensive unit tests:

```bash
cargo test --lib pipeline::test
```

Test coverage:
- Framework display names and commands
- Configuration file detection
- Confidence scoring
- File system utilities
- Integration with actual project

## Example Output

```
Testing framework detection on ltmatrix project...
Detected Framework: Cargo
Confidence: 100.0%
Test Command: cargo test
Config Files:
  - .\Cargo.toml
Test Paths:
  - .\tests
  - .\src
```

## Integration with Pipeline

The framework detection is automatically used during the **Test** stage of the pipeline:

1. **Standard Mode**: Full framework detection + test execution
2. **Fast Mode**: Skips framework detection and testing
3. **Expert Mode**: Enhanced detection + additional test coverage analysis

## Implementation Details

- **Error Handling**: All detection functions return `Result<T>` with detailed error context
- **Performance**: Lazy evaluation - stops at first confirmed framework
- **Extensibility**: Easy to add new frameworks by implementing `detect_*` function
- **Recursion**: File system scans are recursive to handle nested structures
- **Confidence**: Each framework includes confidence scoring for ambiguous cases

## Future Enhancements

Potential additions to framework detection:
- Support for more testing frameworks (JUnit, RSpec, etc.)
- Heuristic-based detection for projects without explicit config
- Framework version detection
- Custom test command extraction from config files
- Test coverage analysis integration
