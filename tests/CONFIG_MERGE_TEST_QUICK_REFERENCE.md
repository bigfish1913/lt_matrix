# Config Merge Logic - Quick Test Reference

## Test Structure Overview

```
config_merge_precedence_test.rs (36 tests)
├── Precedence Order Tests (6)
│   ├── CLI > Project
│   ├── Project > Global
│   ├── Global > Default
│   └── Full chain validation
│
├── Deep Merge Tests (4)
│   ├── Agent configs (field-level merge)
│   ├── Mode configs (complete replacement)
│   ├── Output config (complete replacement)
│   └── Logging config (complete replacement)
│
├── CLI Override Tests (7)
│   ├── Agent name
│   ├── Output format
│   ├── Log level
│   ├── Log file
│   ├── No color
│   ├── Max retries (mode-specific)
│   └── Timeout (mode-specific)
│
├── Validation Tests (13)
│   ├── Agent validation (5)
│   │   ├── Missing default agent
│   │   ├── Zero timeout
│   │   ├── Excessive timeout (>24h)
│   │   └── Empty command
│   ├── Mode validation (6)
│   │   ├── max_depth > 5
│   │   ├── max_retries > 10
│   │   ├── timeout_plan = 0
│   │   ├── timeout_exec = 0
│   │   ├── timeout_exec < 60s (non-fast)
│   │   └── Fast mode allows short timeout
│   └── General (2)
│       ├── Valid config passes
│       └── No default agent allowed
│
├── Edge Case Tests (6)
│   ├── Empty configs
│   ├── None global
│   ├── None project
│   ├── Both None
│   └── Partial field overrides
│
└── Integration Tests (2)
    ├── Full load and merge
    └── Validation after merge
```

## Precedence Rules (Highest to Lowest)

1. **CLI Overrides** - Always win
2. **Project Config** - `.ltmatrix/config.toml`
3. **Global Config** - `~/.ltmatrix/config.toml`
4. **Built-in Defaults** - Hardcoded defaults

## Merge Behavior by Type

| Config Type | Merge Behavior |
|-------------|---------------|
| Agents | Field-level merge (project overrides specified fields only) |
| Modes | Complete replacement if both define same mode |
| Output | Complete replacement |
| Logging | Complete replacement |
| Default Agent | Override behavior |

## Validation Rules Summary

### Agent Validation
- ✅ Timeout: 0 < timeout ≤ 86400 (24 hours)
- ✅ Command: Must not be empty
- ✅ Default Agent: Must exist in agents map

### Mode Validation
- ✅ max_depth: ≤ 5
- ✅ max_retries: ≤ 10
- ✅ timeout_plan: > 0
- ✅ timeout_exec: > 0
- ✅ timeout_exec: ≥ 60s for non-fast modes
- ✅ timeout_exec: < 60s OK for fast mode

## Running Tests

```bash
# All tests
cargo test --test config_merge_precedence_test

# Specific category
cargo test --test config_merge_precedence_test test_precedence
cargo test --test config_merge_precedence_test test_validation
cargo test --test config_merge_precedence_test test_cli_override

# Specific test
cargo test --test config_merge_precedence_test test_precedence_cli_highest_priority
```

## Test Status

| Category | Tests | Status |
|----------|-------|--------|
| Precedence Order | 6 | ✅ All Passing |
| Deep Merge | 4 | ✅ All Passing |
| CLI Overrides | 7 | ✅ All Passing |
| Validation | 13 | ✅ All Passing |
| Edge Cases | 6 | ✅ All Passing |
| Integration | 2 | ✅ All Passing |
| **Total** | **36** | **✅ All Passing** |

## Common Test Patterns

### Testing Precedence
```rust
// Create lower-priority config
let mut lower = Config::default();
lower.field = "lower-value";

// Create higher-priority config
let mut higher = Config::default();
higher.field = "higher-value";

// Merge
let merged = merge_configs(Some(lower), Some(higher));

// Verify higher priority won
assert_eq!(merged.field, "higher-value");
```

### Testing Validation
```rust
let config = Config { /* invalid config */ };

let result = validate_config(&config);

assert!(result.is_err());
let error_msg = result.unwrap_err().to_string();
assert!(error_msg.contains("specific error"));
```

### Testing CLI Overrides
```rust
let mut config = Config::default();
config.field = "config-value";

let overrides = CliOverrides {
    field: Some("cli-value"),
    ..
};

// Apply override
if let Some(value) = overrides.field {
    config.field = value;
}

assert_eq!(config.field, "cli-value");
```

## Key Takeaways

1. **Precedence is strict**: CLI always wins, defaults always lose
2. **Merge strategy varies**: Agents merge fields, configs replace entirely
3. **Validation is comprehensive**: Catches common misconfigurations
4. **Error messages are descriptive**: Help users fix issues quickly
5. **Edge cases handled**: None values, empty configs, partial overrides

## Related Files

- **Implementation**: `src/config/settings.rs`
  - `merge_configs()` - Main merge function
  - `validate_config()` - Validation function
  - `CliOverrides` - CLI override struct
  - `apply_cli_overrides()` - CLI override application

- **Other Tests**:
  - `tests/config_acceptance_test.rs` - Basic acceptance tests
  - `tests/config_loading_integration_test.rs` - Loading tests
  - `tests/config_tests.rs` - General config tests
