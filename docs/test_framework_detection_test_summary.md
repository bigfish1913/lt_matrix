# Test Framework Detection - Test Coverage Summary

## Overview
Comprehensive integration tests for the test framework detection logic in `src/pipeline/test.rs`.

## Test File
`tests/test_framework_detection_test.rs`

## Test Statistics
- **Total Tests**: 28
- **Status**: All passing ✓
- **Test Framework**: Rust's built-in test framework
- **Dependencies**: tempfile (for temporary directory creation)

## Test Coverage

### 1. Framework Detection Tests (17 tests)

#### Cargo/Rust Detection (4 tests)
- `test_detect_cargo_project` - Detects Cargo.toml and #[test] attributes
- `test_detect_cargo_with_tests_directory` - Detects tests/ directory
- `test_detect_cargo_recursive_test_scanning` - Tests nested directory scanning
- `test_framework_priority_cargo_over_others` - Verifies Cargo has highest priority

#### Go Detection (3 tests)
- `test_detect_go_project` - Detects go.mod and _test.go files
- `test_detect_go_with_only_test_files` - Detects Go without go.mod
- `test_detect_go_without_go_mod` - Tests multiple test files detection
- `test_framework_priority_go_over_pytest_and_npm` - Verifies Go priority

#### Pytest Detection (4 tests)
- `test_detect_pytest_with_pytest_ini` - Detects pytest.ini
- `test_detect_pytest_with_pyproject_toml` - Detects [tool.pytest] in pyproject.toml
- `test_detect_pytest_with_test_directory` - Detects tests/ directory with test_*.py files
- `test_detect_pytest_confidence_levels` - Tests confidence scoring (0.6, 0.9, 1.0)
- `test_framework_priority_pytest_over_npm` - Verifies pytest priority

#### NPM Detection (4 tests)
- `test_detect_npm_with_test_script` - Detects package.json with scripts.test
- `test_detect_npm_with_test_directory` - Detects __tests__, tests/, test/, spec/ directories
- `test_detect_npm_with_multiple_test_scripts` - Tests confidence with multiple scripts
- `test_detect_npm_with_jest_dependency` - Detects devDependencies with test frameworks

#### No Framework Detection (1 test)
- `test_detect_no_framework` - Tests empty directory scenario

### 2. Helper Function Tests (11 tests)

#### File System Checks (3 tests)
- `test_file_exists_and_readable_file` - Tests file existence checking
- `test_file_exists_and_readable_directory` - Verifies directories return false
- `test_directory_exists_and_accessible` - Tests directory accessibility

#### File Reading (4 tests)
- `test_read_file_lines` - Tests reading first N lines
- `test_read_file_lines_more_than_available` - Tests requesting more lines than available
- `test_read_file_lines_empty_file` - Tests reading empty files
- `test_read_file_lines_nonexistent` - Tests error handling for missing files

#### TOML Parsing (2 tests)
- `test_parse_toml_section_valid` - Tests parsing existing TOML sections
- `test_parse_toml_section_missing` - Tests handling missing sections
- `test_parse_toml_section_invalid_file` - Tests error handling for invalid files

## Test Scenarios Covered

### 1. Detection Accuracy
- Single framework detection
- Multiple frameworks in same directory (priority testing)
- Minimal project configurations
- Complete project structures

### 2. Confidence Scoring
- Pytest: 0.6 (1 indicator), 0.9 (2 indicators), 1.0 (3+ indicators)
- Go: 0.7 (1 indicator), 1.0 (2 indicators)
- NPM: 0.5 (1 indicator), 0.8 (2 indicators), 1.0 (3+ indicators)
- Cargo: 1.0 (always, if Cargo.toml exists)

### 3. Configuration Detection
- pytest.ini
- pyproject.toml [tool.pytest.ini_options]
- package.json scripts
- package.json devDependencies
- Cargo.toml
- go.mod

### 4. Test Path Detection
- Directory scanning (tests/, __tests__, test/, spec/)
- File pattern matching (test_*.py, *_test.go)
- Recursive directory traversal
- Rust test attributes (#[test], #[cfg(test)])

### 5. Error Handling
- Missing files
- Invalid file formats
- Non-existent directories
- Empty files

## Framework Priority Order
Tests verify the correct priority order:
1. Cargo (highest)
2. Go
3. pytest
4. npm
5. None (no framework)

## Test Utilities

### Helper Functions
- `create_temp_file()` - Creates temporary files with content
- Uses `tempfile::TempDir` for automatic cleanup
- Isolated test environments (no cross-contamination)

## Running the Tests

```bash
# Run all framework detection tests
cargo test --test test_framework_detection_test

# Run with output
cargo test --test test_framework_detection_test -- --nocapture

# Run specific test
cargo test test_detect_cargo_project --test test_framework_detection_test
```

## Integration with Existing Tests

The integration tests complement the existing unit tests in `src/pipeline/test.rs`:
- Unit tests (7 tests): Basic functionality, builder pattern, simple checks
- Integration tests (28 tests): End-to-end detection with real file structures

**Total test coverage: 35 tests** for the test framework detection module.

## Notes

- All tests use temporary directories for isolation
- Automatic cleanup prevents test pollution
- Tests verify both positive and negative scenarios
- Confidence scoring is validated for each framework
- Priority order is explicitly tested
