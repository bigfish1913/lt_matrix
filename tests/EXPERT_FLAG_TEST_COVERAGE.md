# --expert Flag Test Coverage

## Overview
Comprehensive acceptance tests for the `--expert` flag implementation in ltmatrix.

## Test File
`tests/expert_flag_integration_test.rs`

## Test Categories

### 1. CLI Argument Parsing Tests (6 tests)
- ✅ `test_expert_flag_parses_correctly` - Verifies --expert flag is accepted
- ✅ `test_expert_flag_sets_execution_mode` - Verifies get_execution_mode() returns Expert
- ✅ `test_expert_flag_conflicts_with_fast` - Verifies conflict with --fast
- ✅ `test_expert_flag_conflicts_with_mode` - Verifies conflict with --mode
- ✅ `test_expert_flag_can_be_used_alone` - Verifies --expert works standalone
- ✅ `test_expert_flag_with_subcommand` - Verifies --expert works with subcommands

### 2. CliOverrides Conversion Tests (4 tests)
- ✅ `test_expert_flag_maps_to_expert_mode_in_overrides` - Verifies --expert maps to "expert" mode
- ✅ `test_no_mode_flag_maps_to_none_in_overrides` - Verifies default behavior
- ✅ `test_fast_flag_maps_to_fast_mode_in_overrides` - Comparison test for --fast
- ✅ `test_expert_overrides_other_overrides_fields` - Verifies other fields work with --expert

### 3. Config Merge Integration Tests (2 tests)
- ✅ `test_expert_flag_with_config_file_mode_settings` - Verifies --expert with config files
- ✅ `test_expert_flag_cli_overrides_mode_settings` - Verifies CLI overrides for mode settings

### 4. Precedence Tests (2 tests)
- ✅ `test_expert_cli_overrides_config_file_default_mode` - Verifies CLI > config file
- ✅ `test_precedence_order_expert` - Verifies full precedence chain

### 5. Edge Cases Tests (3 tests)
- ✅ `test_expert_flag_with_all_other_flags` - Verifies --expert with all other flags
- ✅ `test_expert_flag_default_without_other_mode_flags` - Verifies default mode
- ✅ `test_expert_mode_string_value` - Verifies string representation

### 6. Integration Acceptance Tests (1 test)
- ✅ `test_complete_expert_flag_integration` - Full end-to-end integration test

## Acceptance Criteria Coverage

✅ **1. --expert flag is parsed correctly as a CLI argument**
- Tests verify clap accepts --expert flag
- Tests verify expert field is set to true

✅ **2. --expert flag conflicts with --fast and --mode flags**
- Tests verify clap enforces conflicts
- Tests verify appropriate error messages

✅ **3. --expert maps to "expert" mode in CliOverrides**
- Tests verify From<Args> for CliOverrides sets mode to "expert"
- Tests verify get_execution_mode() returns Expert

✅ **4. --expert flag properly integrates with config merge logic**
- Tests verify load_config_with_overrides works with --expert
- Tests verify config files are loaded with --expert

✅ **5. Precedence is correct: CLI > config file > defaults**
- Tests verify CLI overrides take precedence
- Tests verify config file values are used when CLI not specified
- Tests verify defaults are used when neither specified

✅ **6. Mode-specific settings are applied when using --expert**
- Tests verify max_retries, timeout, and other mode settings work
- Tests verify CLI mode overrides work with config file settings

## Test Results
```
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Running the Tests
```bash
cargo test --test expert_flag_integration_test
```

## Notes
- Tests use tempfile for temporary directories
- Tests properly handle directory changes and restoration
- All tests are independent and can run in parallel
- No test data pollution between tests
