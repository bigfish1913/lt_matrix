# Feature Flag System

## Overview

The ltmatrix feature flag system provides comprehensive support for enabling/disabling experimental features, gradual rollout, and A/B testing. This allows for safe deployment of new features and controlled testing with specific user groups.

## Architecture

### Core Components

1. **FeatureFlag Enum**: Defines all available feature flags organized by category
2. **FeatureFlags Manager**: Manages feature flag state and provides query methods
3. **RolloutConfig**: Handles gradual rollout with percentage, whitelist, and blacklist support
4. **FeatureConfig**: TOML-based configuration structure

### Feature Categories

#### Agent Backend Features
- `EnableClaudeOpusBackend`: Enable Claude Opus for complex reasoning (Experimental)
- `EnableOpenCodeBackend`: Enable OpenCode as alternative backend (Opt-in)
- `EnableKimiCodeBackend`: Enable KimiCode backend (Opt-in)
- `EnableCodexBackend`: Enable Codex backend (Opt-in)
- `EnableCustomBackend`: Enable custom agent backends (Experimental)

#### Pipeline Features
- `EnableParallelExecution`: Execute independent tasks in parallel (Stable)
- `EnableSmartCache`: Intelligent caching of intermediate results (Stable)
- `EnableIncrementalBuilds`: Only rebuild changed components (Beta)
- `EnableDistributedTasks`: Execute tasks across machines (Experimental)
- `EnableTaskDependencyGraph`: Advanced dependency resolution (Experimental)
- `EnableTaskBatching`: Batch tasks for efficiency (Beta)
- `EnablePipelineOptimization`: Automated optimization passes (Beta)

#### Scheduler Features
- `EnablePriorityScheduler`: Priority-based task scheduling (Beta)
- `EnableAdaptiveScheduler`: Performance-aware scheduling (Beta)
- `EnableMlScheduler`: ML-based optimal scheduling (Experimental)
- `EnableFairShareScheduler`: Equal resource allocation (Beta)
- `EnableDeadlineScheduler`: Time-sensitive task scheduling (Beta)

#### Monitoring & Observability Features
- `EnableDetailedMetrics`: Comprehensive performance metrics (Opt-in)
- `EnableProfiling`: Performance profiling capabilities (Opt-in)
- `EnableMonitoringDashboard`: Real-time monitoring UI (Experimental)
- `EnableAlerting`: Automated alerting system (Beta)

#### Development & Debugging Features
- `EnableVerboseDebug`: Enhanced debug output (Opt-in)
- `EnableTracing`: Detailed operation tracing (Opt-in)
- `EnableExperimentalCommands`: Unstable CLI commands (Experimental)
- `EnableTestingUtilities`: Testing tools in production (Experimental)

## Configuration

### Basic Configuration

Feature flags are configured in the TOML configuration file:

```toml
# ~/.ltmatrix/config.toml or .ltmatrix/config.toml

[features.agent_backend]
enable_claude_opus_backend = true
enable_opencode_backend = false

[features.pipeline]
enable_parallel_execution = true
enable_smart_cache = true
enable_incremental_builds = false

[features.scheduler]
enable_priority_scheduler = true

[features.monitoring]
enable_detailed_metrics = false

[features.development]
enable_verbose_debug = false
```

### Gradual Rollout Configuration

Control feature rollout with percentage-based allocation:

```toml
[features.rollout.enable_parallel_execution]
percentage = 50  # Enable for 50% of users
users = ["beta_tester1", "beta_tester2"]  # Always enable for these users
excluded_users = ["problematic_user"]  # Never enable for these users
```

#### Rollout Behavior

1. **Blacklist Check**: Users in `excluded_users` are always denied (highest priority)
2. **Whitelist Check**: Users in `users` are always allowed
3. **Percentage Check**: Remaining users are enabled based on consistent hash

This ensures:
- Blacklisted users never get the feature, even at 100%
- Whitelisted users always get the feature, even at 0%
- Other users get consistent results (same user always gets same result)

### Configuration Precedence

Feature flags follow the same precedence as other configuration:

1. CLI arguments (highest)
2. Project config (`.ltmatrix/config.toml`)
3. Global config (`~/.ltmatrix/config.toml`)
4. Defaults (lowest)

## Usage Examples

### Basic Feature Checking

```rust
use ltmatrix::feature::{FeatureFlag, FeatureFlags};

// Load feature flags from configuration
let flags = FeatureFlags::stable_enabled();

// Check if a feature is enabled
if flags.is_enabled(FeatureFlag::EnableParallelExecution) {
    // Execute tasks in parallel
} else {
    // Execute tasks sequentially
}
```

### User-Specific Feature Checks

```rust
// Check if a feature is enabled for a specific user
if flags.is_enabled_for_user(
    FeatureFlag::EnableSmartCache,
    "user123"
) {
    // Enable smart cache for this user
}
```

### Listing Enabled Features

```rust
// Get all enabled features
let enabled = flags.enabled_flags();

// Get only experimental features (risky for production)
let experimental = flags.enabled_experimental_flags();

if !experimental.is_empty() {
    eprintln!("Warning: {} experimental features enabled!", experimental.len());
}
```

### Custom Feature Configuration

```rust
use ltmatrix::feature::{FeatureConfig, FeatureFlags};

// Create custom configuration
let mut config = FeatureConfig::default();
config.pipeline.enable_parallel_execution = true;
config.pipeline.enable_smart_cache = true;
config.scheduler.enable_priority_scheduler = true;

// Add rollout configuration
use ltmatrix::feature::RolloutConfig;
use std::collections::HashMap;

let mut rollout = HashMap::new();
rollout.insert(
    "enable_priority_scheduler".to_string(),
    RolloutConfig::new(25)  // 25% rollout
);

config.rollout = rollout;

// Create feature flags manager
let flags = FeatureFlags::new(config);
```

### Loading from File

```rust
use std::path::Path;

// Load feature flags from TOML file
let flags = FeatureFlags::load_from_file(Path::new("features.toml"))?;

// Save feature flags to TOML file
flags.save_to_file(Path::new("features.toml"))?;
```

## Best Practices

### 1. Feature Flag Lifecycle

```rust
// Development phase
if flags.is_enabled(FeatureFlag::EnableMyFeature) {
    // New implementation
} else {
    // Old implementation
}

// Testing phase - enable for beta testers
let flags = FeatureFlags::load_from_file("beta-config.toml")?;
if flags.is_enabled_for_user(FeatureFlag::EnableMyFeature, user_id) {
    // New implementation for beta testers
}

// Production rollout - gradual
// Start at 10%, monitor metrics, increase to 25%, 50%, 100%

// Cleanup phase - remove old code after full rollout
if flags.is_enabled(FeatureFlag::EnableMyFeature) {
    // New implementation (only this remains)
}
```

### 2. Production Safety

```rust
// Warn about experimental features
let experimental = flags.enabled_experimental_flags();
if !experimental.is_empty() {
    eprintln!("Warning: Experimental features enabled:");
    for flag in &experimental {
        eprintln!("  - {} ({})", flag.config_key(), flag.description());
    }
}

// Check feature stability before use
if flags.is_enabled(FeatureFlag::EnableDistributedTasks) {
    if FeatureFlag::EnableDistributedTasks.is_experimental() {
        eprintln!("WARNING: Using experimental distributed tasks!");
        // Log for monitoring
    }
}
```

### 3. Gradual Rollout Strategy

```toml
# Week 1: Enable for 10% of users + beta testers
[features.rollout.my_feature]
percentage = 10
users = ["beta_tester1", "beta_tester2", "internal_team"]

# Week 2: Increase to 25% if metrics look good
percentage = 25

# Week 3: Increase to 50%
percentage = 50

# Week 4: Increase to 100%
percentage = 100

# Post-launch: Remove feature flag entirely
# [features.rollout.my_feature]
# (Delete this section after full rollout)
```

### 4. A/B Testing

```toml
# Test two different implementations
[features.pipeline]
enable_parallel_execution_v1 = true
enable_parallel_execution_v2 = false

[features.rollout.enable_parallel_execution_v2]
percentage = 50  # 50% get v2, 50% get v1
```

```rust
if flags.is_enabled_for_user(FeatureFlag::EnableParallelExecutionV2, user_id) {
    // Use v2 implementation
} else if flags.is_enabled(FeatureFlag::EnableParallelExecutionV1) {
    // Use v1 implementation
}
```

### 5. Emergency Kill Switch

```toml
# Global kill switch for problematic features
[features.pipeline]
enable_distributed_tasks = false  # Disabled immediately
```

## API Reference

### FeatureFlag Enum

```rust
pub enum FeatureFlag {
    // Agent Backend Features
    EnableClaudeOpusBackend,
    EnableOpenCodeBackend,
    EnableKimiCodeBackend,
    EnableCodexBackend,
    EnableCustomBackend,

    // Pipeline Features
    EnableParallelExecution,
    EnableSmartCache,
    EnableIncrementalBuilds,
    EnableDistributedTasks,
    EnableTaskDependencyGraph,
    EnableTaskBatching,
    EnablePipelineOptimization,

    // Scheduler Features
    EnablePriorityScheduler,
    EnableAdaptiveScheduler,
    EnableMlScheduler,
    EnableFairShareScheduler,
    EnableDeadlineScheduler,

    // Monitoring & Observability Features
    EnableDetailedMetrics,
    EnableProfiling,
    EnableMonitoringDashboard,
    EnableAlerting,

    // Development & Debugging Features
    EnableVerboseDebug,
    EnableTracing,
    EnableExperimentalCommands,
    EnableTestingUtilities,
}
```

#### Methods

- `config_key(&self) -> &'static str`: Get the TOML configuration key
- `description(&self) -> &'static str`: Get human-readable description
- `is_experimental(&self) -> bool`: Check if feature is experimental
- `is_stable(&self) -> bool`: Check if feature is stable

### FeatureFlags Manager

```rust
pub struct FeatureFlags {
    config: FeatureConfig,
}
```

#### Methods

- `new(config: FeatureConfig) -> Self`: Create from configuration
- `all_disabled() -> Self`: Create with all features disabled
- `stable_enabled() -> Self`: Create with stable features enabled
- `is_enabled(&self, flag: FeatureFlag) -> bool`: Check if feature is enabled
- `is_enabled_for_user(&self, flag: FeatureFlag, user_id: &str) -> bool`: Check for specific user
- `enabled_flags(&self) -> Vec<FeatureFlag>`: List all enabled flags
- `enabled_experimental_flags(&self) -> Vec<FeatureFlag>`: List experimental flags
- `load_from_file(path: &Path) -> Result<Self>`: Load from TOML file
- `save_to_file(&self, path: &Path) -> Result<()>`: Save to TOML file

### RolloutConfig

```rust
pub struct RolloutConfig {
    pub percentage: u32,
    pub users: HashSet<String>,
    pub excluded_users: HashSet<String>,
}
```

#### Methods

- `new(percentage: u32) -> Self`: Create with percentage
- `with_user(self, user: impl Into<String>) -> Self`: Add user to whitelist
- `with_excluded_user(self, user: impl Into<String>) -> Self`: Add user to blacklist
- `is_enabled_for(&self, user_id: &str) -> bool`: Check if user gets feature

## Testing

Feature flags make testing easier:

```rust
#[test]
fn test_new_implementation() {
    let flags = FeatureFlags::all_disabled();

    // Test with feature disabled
    assert!(!flags.is_enabled(FeatureFlag::EnableMyFeature));

    // Test with feature enabled
    let mut config = FeatureConfig::default();
    config.pipeline.enable_my_feature = true;
    let flags = FeatureFlags::new(config);
    assert!(flags.is_enabled(FeatureFlag::EnableMyFeature));
}
```

## Monitoring and Observability

When using gradual rollout, monitor:

1. **Error Rates**: Compare error rates between enabled/disabled groups
2. **Performance Metrics**: Latency, throughput, resource usage
3. **User Feedback**: Collect feedback from whitelisted beta testers
4. **Business Metrics**: Conversion, engagement, retention

Example monitoring code:

```rust
if flags.is_enabled_for_user(FeatureFlag::EnableMyFeature, user_id) {
    let start = std::time::Instant::now();

    // Execute feature

    let duration = start.elapsed();
    metrics::histogram!("feature.duration", duration, "feature" => "my_feature");
}
```

## Migration Guide

### Adding a New Feature Flag

1. **Add to enum**: Add the feature to `FeatureFlag` enum in `src/feature/mod.rs`
2. **Implement methods**: Add `config_key()` and `description()` implementations
3. **Add to config struct**: Add to appropriate feature group (e.g., `PipelineFeatures`)
4. **Update defaults**: Set appropriate default (usually `false` for new features)
5. **Use in code**: Add conditional logic based on feature flag
6. **Add tests**: Test both enabled and disabled states
7. **Document**: Update this documentation with description and stability status

### Removing an Old Feature Flag

1. **Fully roll out**: Ensure feature is at 100% rollout
2. **Remove conditional code**: Remove if/else branches, keep only new code
3. **Remove from config**: Delete from feature configuration structures
4. **Update tests**: Remove feature flag tests, add direct tests
5. **Update docs**: Remove feature flag from documentation

## Troubleshooting

### Feature Not Working

```rust
// Debug: Check if feature is actually enabled
if !flags.is_enabled(FeatureFlag::EnableMyFeature) {
    eprintln!("Feature is disabled in configuration");
    return Err(...);
}

// Debug: Check configuration loading
let flags = match FeatureFlags::load_from_file("config.toml") {
    Ok(f) => f,
    Err(e) => {
        eprintln!("Failed to load feature flags: {}", e);
        return Err(...);
    }
};
```

### Rollout Not Working as Expected

```rust
// Verify rollout configuration
if let Some(rollout) = flags.rollout_config(FeatureFlag::EnableMyFeature) {
    eprintln!("Rollout percentage: {}", rollout.percentage);
    eprintln!("Whitelisted users: {:?}", rollout.users);
    eprintln!("Blacklisted users: {:?}", rollout.excluded_users);

    // Test specific user
    let enabled = rollout.is_enabled_for(user_id);
    eprintln!("User {} enabled: {}", user_id, enabled);
}
```

## Performance Considerations

Feature flag checks are very fast:
- Simple boolean check: ~1-2 ns
- User-specific check with rollout: ~50-100 ns (hash-based)
- No I/O or network calls after initial loading

For performance-critical code, cache the result:

```rust
// Cache feature flag check
let use_parallel = flags.is_enabled(FeatureFlag::EnableParallelExecution);

// Use cached value in loop
for task in tasks {
    if use_parallel {
        // Parallel execution
    } else {
        // Sequential execution
    }
}
```

## Security Considerations

1. **Configuration File Security**: Feature flags are in configuration files - protect sensitive flags
2. **User ID Privacy**: Rollout hashing uses user IDs - ensure privacy compliance
3. **Experimental Features**: Warn users about experimental features in production
4. **Audit Logging**: Log when experimental features are enabled

## Future Enhancements

Potential future improvements:

1. **Remote Configuration**: Load feature flags from remote service
2. **Dynamic Updates**: Update feature flags without restart
3. **Analytics Integration**: Built-in A/B testing analytics
4. **Feature Dependencies**: Express dependencies between features
5. **Time-Based Rollout**: Enable features at specific times
6. **Geographic Rollout**: Enable features based on user location
7. **Environment-Aware Defaults**: Different defaults for dev/staging/prod

## Summary

The ltmatrix feature flag system provides:

✓ **Comprehensive Coverage**: 20+ feature flags across 5 categories
✓ **Gradual Rollout**: Percentage-based with whitelist/blacklist
✓ **Production Ready**: Stable defaults, experimental flags clearly marked
✓ **Easy to Use**: Simple API, TOML configuration
✓ **Well Tested**: Comprehensive test coverage
✓ **Well Documented**: Complete documentation with examples

Use feature flags to:
- Safely deploy new features
- Test with beta testers before full rollout
- Kill problematic features instantly
- Run A/B tests
- Enable experimental features for development

For more examples, see `examples/feature_flags.rs`.
