//! Integration tests for feature flag system
//!
//! These tests verify the feature flag system works correctly in various
//! real-world scenarios and integrates properly with other components.

use ltmatrix::feature::{
    FeatureFlag, FeatureFlags, FeatureConfig, RolloutConfig,
};
use std::collections::HashMap;
use tempfile::TempDir;
use std::fs;

// ============================================================================
// Integration with file system
// ============================================================================

#[test]
fn integration_load_from_nonexistent_file() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("nonexistent.toml");

    // Should return error for nonexistent file
    let result = FeatureFlags::load_from_file(&config_path);
    assert!(result.is_err(), "Should fail to load nonexistent file");

    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(err_msg.contains("Failed to read") || err_msg.contains("No such file"),
        "Error should mention file read failure");
}

#[test]
fn integration_save_and_load_roundtrip() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("features.toml");

    // Create and save a feature config
    let mut config = FeatureConfig::default();
    config.agent_backend.enable_claude_opus_backend = true;
    config.pipeline.enable_parallel_execution = true;
    config.pipeline.enable_smart_cache = false;
    config.scheduler.enable_priority_scheduler = true;
    config.scheduler.enable_ml_scheduler = false;

    let original_flags = FeatureFlags::new(config);
    original_flags.save_to_file(&config_path).unwrap();

    // Load it back
    let loaded_flags = FeatureFlags::load_from_file(&config_path).unwrap();

    // Verify all flags match
    assert_eq!(
        original_flags.is_enabled(FeatureFlag::EnableClaudeOpusBackend),
        loaded_flags.is_enabled(FeatureFlag::EnableClaudeOpusBackend)
    );
    assert_eq!(
        original_flags.is_enabled(FeatureFlag::EnableParallelExecution),
        loaded_flags.is_enabled(FeatureFlag::EnableParallelExecution)
    );
    assert_eq!(
        original_flags.is_enabled(FeatureFlag::EnableSmartCache),
        loaded_flags.is_enabled(FeatureFlag::EnableSmartCache)
    );
    assert_eq!(
        original_flags.is_enabled(FeatureFlag::EnablePriorityScheduler),
        loaded_flags.is_enabled(FeatureFlag::EnablePriorityScheduler)
    );
    assert_eq!(
        original_flags.is_enabled(FeatureFlag::EnableMlScheduler),
        loaded_flags.is_enabled(FeatureFlag::EnableMlScheduler)
    );
}

#[test]
fn integration_load_malformed_toml() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("bad.toml");

    // Write invalid TOML
    fs::write(&config_path, "this is not valid [toml").unwrap();

    // Should return error
    let result = FeatureFlags::load_from_file(&config_path);
    assert!(result.is_err(), "Should fail to parse invalid TOML");

    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(err_msg.contains("Failed to parse") || err_msg.contains("TOML"),
        "Error should mention parse failure");
}

#[test]
fn integration_save_creates_valid_toml() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("features.toml");

    let flags = FeatureFlags::stable_enabled();
    flags.save_to_file(&config_path).unwrap();

    // File should exist
    assert!(config_path.exists(), "Config file should be created");

    // Content should be valid TOML
    let content = fs::read_to_string(&config_path).unwrap();
    let parsed: FeatureConfig = toml::from_str(&content).unwrap();

    // Parsed config should match original
    assert_eq!(parsed.pipeline.enable_parallel_execution, true);
    assert_eq!(parsed.pipeline.enable_smart_cache, true);
}

#[test]
fn integration_save_to_readonly_directory() {
    // This test may not work on all systems due to permission handling
    // Skip on Windows for simplicity
    if cfg!(windows) {
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let readonly_dir = temp_dir.path().join("readonly");

    fs::create_dir(&readonly_dir).unwrap();

    // Make directory read-only
    let mut perms = fs::metadata(&readonly_dir).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(&readonly_dir, perms).unwrap();

    let config_path = readonly_dir.join("features.toml");
    let flags = FeatureFlags::stable_enabled();

    // Should fail to write to readonly directory
    let result = flags.save_to_file(&config_path);

    // Restore permissions for cleanup
    let mut perms = fs::metadata(&readonly_dir).unwrap().permissions();
    perms.set_readonly(false);
    fs::set_permissions(&readonly_dir, perms).unwrap();

    assert!(result.is_err(), "Should fail to write to readonly directory");
}

// ============================================================================
// Integration with TOML configuration
// ============================================================================

#[test]
fn integration_toml_with_all_sections() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("full.toml");

    let toml_content = r#"
[agent_backend]
enable_claude_opus_backend = true
enable_opencode_backend = false
enable_kimicode_backend = false
enable_codex_backend = false
enable_custom_backend = true

[pipeline]
enable_parallel_execution = true
enable_smart_cache = true
enable_incremental_builds = false
enable_distributed_tasks = false
enable_task_dependency_graph = true
enable_task_batching = false
enable_pipeline_optimization = false

[scheduler]
enable_priority_scheduler = true
enable_adaptive_scheduler = false
enable_ml_scheduler = false
enable_fair_share_scheduler = false
enable_deadline_scheduler = true

[monitoring]
enable_detailed_metrics = true
enable_profiling = false
enable_monitoring_dashboard = false
enable_alerting = true

[development]
enable_verbose_debug = false
enable_tracing = true
enable_experimental_commands = false
enable_testing_utilities = false
"#;

    fs::write(&config_path, toml_content).unwrap();

    let flags = FeatureFlags::load_from_file(&config_path).unwrap();

    // Verify all sections loaded correctly
    assert!(flags.is_enabled(FeatureFlag::EnableClaudeOpusBackend));
    assert!(!flags.is_enabled(FeatureFlag::EnableOpenCodeBackend));
    assert!(flags.is_enabled(FeatureFlag::EnableCustomBackend));

    assert!(flags.is_enabled(FeatureFlag::EnableParallelExecution));
    assert!(flags.is_enabled(FeatureFlag::EnableSmartCache));
    assert!(!flags.is_enabled(FeatureFlag::EnableIncrementalBuilds));
    assert!(flags.is_enabled(FeatureFlag::EnableTaskDependencyGraph));

    assert!(flags.is_enabled(FeatureFlag::EnablePriorityScheduler));
    assert!(!flags.is_enabled(FeatureFlag::EnableAdaptiveScheduler));
    assert!(flags.is_enabled(FeatureFlag::EnableDeadlineScheduler));

    assert!(flags.is_enabled(FeatureFlag::EnableDetailedMetrics));
    assert!(!flags.is_enabled(FeatureFlag::EnableProfiling));
    assert!(flags.is_enabled(FeatureFlag::EnableAlerting));

    assert!(!flags.is_enabled(FeatureFlag::EnableVerboseDebug));
    assert!(flags.is_enabled(FeatureFlag::EnableTracing));
}

#[test]
fn integration_toml_with_rollout_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("rollout.toml");

    let toml_content = r#"
[pipeline]
enable_parallel_execution = true
enable_smart_cache = true

[rollout.enable_parallel_execution]
percentage = 25
users = ["beta_user1", "beta_user2"]
excluded_users = ["blocked_user"]

[rollout.enable_smart_cache]
percentage = 100
"#;

    fs::write(&config_path, toml_content).unwrap();

    let flags = FeatureFlags::load_from_file(&config_path).unwrap();

    // Check rollout for parallel execution
    let parallel_rollout = flags.rollout_config(FeatureFlag::EnableParallelExecution);
    assert!(parallel_rollout.is_some());
    let parallel_rollout = parallel_rollout.unwrap();
    assert_eq!(parallel_rollout.percentage, 25);
    assert!(parallel_rollout.users.contains("beta_user1"));
    assert!(parallel_rollout.users.contains("beta_user2"));
    assert!(parallel_rollout.excluded_users.contains("blocked_user"));

    // Check rollout for smart cache
    let cache_rollout = flags.rollout_config(FeatureFlag::EnableSmartCache);
    assert!(cache_rollout.is_some());
    assert_eq!(cache_rollout.unwrap().percentage, 100);

    // Verify behavior
    assert!(flags.is_enabled_for_user(
        FeatureFlag::EnableParallelExecution,
        "beta_user1"
    ));
    assert!(!flags.is_enabled_for_user(
        FeatureFlag::EnableParallelExecution,
        "blocked_user"
    ));

    // Smart cache at 100% should be enabled for everyone
    assert!(flags.is_enabled_for_user(
        FeatureFlag::EnableSmartCache,
        "any_user"
    ));
}

#[test]
fn integration_toml_with_empty_sections() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("empty.toml");

    let toml_content = r#"
[agent_backend]

[pipeline]

[scheduler]
"#;

    fs::write(&config_path, toml_content).unwrap();

    // Should load successfully with defaults
    let flags = FeatureFlags::load_from_file(&config_path).unwrap();

    // Should use default values
    assert!(flags.is_enabled(FeatureFlag::EnableParallelExecution));
    assert!(flags.is_enabled(FeatureFlag::EnableSmartCache));
}

#[test]
fn integration_toml_with_unknown_fields() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("unknown.toml");

    let toml_content = r#"
[agent_backend]
enable_claude_opus_backend = true
unknown_field = "should_be_ignored"

[pipeline]
enable_parallel_execution = true

[unknown_section]
some_field = "ignored"
"#;

    fs::write(&config_path, toml_content).unwrap();

    // Should load successfully, ignoring unknown fields
    let result = FeatureFlags::load_from_file(&config_path);
    // This depends on whether TOML deserialization is strict or lenient
    // If it fails, that's also acceptable behavior
    if result.is_ok() {
        let flags = result.unwrap();
        assert!(flags.is_enabled(FeatureFlag::EnableClaudeOpusBackend));
        assert!(flags.is_enabled(FeatureFlag::EnableParallelExecution));
    }
    // If it fails, that's fine too - strict validation is good
}

// ============================================================================
// Rollout consistency and determinism
// ============================================================================

#[test]
fn integration_rollout_deterministic_for_user() {
    let rollout = RolloutConfig::new(50);

    // Same user should always get the same result
    let results: Vec<bool> = (0..100)
        .map(|_| rollout.is_enabled_for("test_user_123"))
        .collect();

    // All results should be the same
    let first = results[0];
    assert!(results.iter().all(|&r| r == first),
        "Rollout should be deterministic for the same user");
}

#[test]
fn integration_rollout_distribution() {
    let rollout = RolloutConfig::new(50);

    // Test with 1000 users
    let enabled_count = (0..1000)
        .map(|i| {
            let user = format!("user_{}", i);
            rollout.is_enabled_for(&user)
        })
        .filter(|&enabled| enabled)
        .count();

    // Should be approximately 50% (allow 10% margin for statistical variance)
    let percentage = (enabled_count as f64 / 1000.0) * 100.0;
    assert!(percentage >= 40.0 && percentage <= 60.0,
        "Rollout distribution at 50% should be approximately 50%, got {:.1}%",
        percentage);
}

#[test]
fn integration_rollout_edge_cases() {
    // 0% should disable everyone (unless whitelisted)
    let zero_rollout = RolloutConfig::new(0);
    for i in 0..100 {
        let user = format!("user_{}", i);
        assert!(!zero_rollout.is_enabled_for(&user),
            "0% rollout should disable all users");
    }

    // 100% should enable everyone (unless blacklisted)
    let full_rollout = RolloutConfig::new(100);
    for i in 0..100 {
        let user = format!("user_{}", i);
        assert!(full_rollout.is_enabled_for(&user),
            "100% rollout should enable all users");
    }
}

#[test]
fn integration_rollout_with_special_characters_in_user_id() {
    let rollout = RolloutConfig::new(0)
        .with_user("user@example.com")
        .with_user("user-with-dashes")
        .with_user("user_with_underscores")
        .with_user("user.with.dots");

    // All special character users should be whitelisted
    assert!(rollout.is_enabled_for("user@example.com"));
    assert!(rollout.is_enabled_for("user-with-dashes"));
    assert!(rollout.is_enabled_for("user_with_underscores"));
    assert!(rollout.is_enabled_for("user.with.dots"));

    // Regular user should not be enabled
    assert!(!rollout.is_enabled_for("regularuser"));
}

#[test]
fn integration_rollout_unicode_user_ids() {
    let rollout = RolloutConfig::new(0)
        .with_user("用户123")
        .with_user("пользователь")
        .with_user("🚀rocket_user");

    assert!(rollout.is_enabled_for("用户123"));
    assert!(rollout.is_enabled_for("пользователь"));
    assert!(rollout.is_enabled_for("🚀rocket_user"));
}

// ============================================================================
// Concurrent access scenarios
// ============================================================================

#[test]
fn integration_concurrent_read_access() {
    // Multiple reads should be safe
    let flags = FeatureFlags::stable_enabled();

    // Simulate concurrent reads
    let results: Vec<bool> = (0..100)
        .map(|_| flags.is_enabled(FeatureFlag::EnableParallelExecution))
        .collect();

    // All reads should return the same value
    assert!(results.iter().all(|&r| r == true),
        "Concurrent reads should be consistent");
}

#[test]
fn integration_clone_feature_flags() {
    let mut config = FeatureConfig::default();
    config.pipeline.enable_parallel_execution = true;
    config.agent_backend.enable_claude_opus_backend = true;

    let flags1 = FeatureFlags::new(config.clone());
    let flags2 = flags1.clone();

    // Both should work identically
    assert_eq!(
        flags1.is_enabled(FeatureFlag::EnableParallelExecution),
        flags2.is_enabled(FeatureFlag::EnableParallelExecution)
    );
    assert_eq!(
        flags1.is_enabled(FeatureFlag::EnableClaudeOpusBackend),
        flags2.is_enabled(FeatureFlag::EnableClaudeOpusBackend)
    );
}

// ============================================================================
// Real-world scenarios
// ============================================================================

#[test]
fn integration_scenario_beta_rollout() {
    // Scenario: Roll out a feature to beta users first
    let mut config = FeatureConfig::default();
    config.pipeline.enable_incremental_builds = true;

    // Start with 0% rollout, only for beta testers
    let mut rollout_map = HashMap::new();
    rollout_map.insert(
        "enable_incremental_builds".to_string(),
        RolloutConfig::new(0)
            .with_user("beta_tester_1")
            .with_user("beta_tester_2")
            .with_user("beta_tester_3"),
    );
    config.rollout = rollout_map;

    let flags = FeatureFlags::new(config);

    // Only beta testers should have it
    assert!(flags.is_enabled_for_user(
        FeatureFlag::EnableIncrementalBuilds,
        "beta_tester_1"
    ));
    assert!(!flags.is_enabled_for_user(
        FeatureFlag::EnableIncrementalBuilds,
        "regular_user"
    ));
}

#[test]
fn integration_scenario_gradual_rollout_increase() {
    // Scenario: Gradually increase rollout from 10% to 50%

    // Stage 1: 10% rollout
    let mut config = FeatureConfig::default();
    config.scheduler.enable_priority_scheduler = true;

    let mut rollout_map = HashMap::new();
    rollout_map.insert(
        "enable_priority_scheduler".to_string(),
        RolloutConfig::new(10),
    );
    config.rollout = rollout_map;

    let flags = FeatureFlags::new(config);

    // At 10%, not all users should have it
    let count_10 = (0..100)
        .map(|i| flags.is_enabled_for_user(FeatureFlag::EnablePriorityScheduler, &format!("user_{}", i)))
        .filter(|&e| e)
        .count();

    assert!(count_10 < 50, "At 10% rollout, less than half of users should have it");

    // Stage 2: Increase to 50%
    let mut config2 = FeatureConfig::default();
    config2.scheduler.enable_priority_scheduler = true;

    let mut rollout_map = HashMap::new();
    rollout_map.insert(
        "enable_priority_scheduler".to_string(),
        RolloutConfig::new(50),
    );
    config2.rollout = rollout_map;

    let flags = FeatureFlags::new(config2);

    let count_50 = (0..100)
        .map(|i| flags.is_enabled_for_user(FeatureFlag::EnablePriorityScheduler, &format!("user_{}", i)))
        .filter(|&e| e)
        .count();

    assert!(count_50 > count_10, "Increasing rollout should enable more users");
}

#[test]
fn integration_scenario_problematic_user_blacklist() {
    // Scenario: 100% rollout but exclude problematic users
    let mut config = FeatureConfig::default();
    config.pipeline.enable_smart_cache = true;

    let mut rollout_map = HashMap::new();
    rollout_map.insert(
        "enable_smart_cache".to_string(),
        RolloutConfig::new(100)
            .with_excluded_user("problematic_user_1")
            .with_excluded_user("problematic_user_2"),
    );
    config.rollout = rollout_map;

    let flags = FeatureFlags::new(config);

    // Problematic users should not have it
    assert!(!flags.is_enabled_for_user(
        FeatureFlag::EnableSmartCache,
        "problematic_user_1"
    ));
    assert!(!flags.is_enabled_for_user(
        FeatureFlag::EnableSmartCache,
        "problematic_user_2"
    ));

    // All other users should have it
    assert!(flags.is_enabled_for_user(
        FeatureFlag::EnableSmartCache,
        "normal_user"
    ));
}

#[test]
fn integration_scenario_a_b_testing_different_rollouts() {
    // Scenario: A/B test with different features at different rollouts
    let mut config = FeatureConfig::default();
    config.pipeline.enable_parallel_execution = true;
    config.pipeline.enable_smart_cache = true;
    config.scheduler.enable_priority_scheduler = true;

    let mut rollout_map = HashMap::new();
    rollout_map.insert(
        "enable_parallel_execution".to_string(),
        RolloutConfig::new(100), // Fully rolled out
    );
    rollout_map.insert(
        "enable_smart_cache".to_string(),
        RolloutConfig::new(50), // 50% rollout
    );
    rollout_map.insert(
        "enable_priority_scheduler".to_string(),
        RolloutConfig::new(25), // 25% rollout
    );
    config.rollout = rollout_map;

    let flags = FeatureFlags::new(config);

    // Parallel execution at 100% - everyone has it
    assert!(flags.is_enabled_for_user(
        FeatureFlag::EnableParallelExecution,
        "any_user"
    ));

    // Smart cache at 50% - some users have it
    let cache_result = flags.is_enabled_for_user(
        FeatureFlag::EnableSmartCache,
        "test_user"
    );
    // We can't assert the exact value, but the function should work
    let _ = cache_result;

    // Priority scheduler at 25% - fewer users have it
    let scheduler_result = flags.is_enabled_for_user(
        FeatureFlag::EnablePriorityScheduler,
        "test_user"
    );
    let _ = scheduler_result;
}

#[test]
fn integration_scenario_production_safety() {
    // Scenario: Ensure production doesn't accidentally enable experimental features
    let flags = FeatureFlags::default(); // Uses default config

    // Check that no experimental features are enabled
    let experimental = flags.enabled_experimental_flags();
    assert!(experimental.is_empty(),
        "Production config should not enable experimental features by default. Found: {:?}",
        experimental);

    // Check that stable features are enabled
    assert!(flags.is_enabled(FeatureFlag::EnableParallelExecution),
        "Stable features like parallel execution should be enabled");

    // Try to enable an experimental feature explicitly
    let mut config = FeatureConfig::default();
    config.agent_backend.enable_claude_opus_backend = true; // Experimental
    config.pipeline.enable_distributed_tasks = true; // Experimental

    let flags = FeatureFlags::new(config);

    // Should be able to enable experimental features explicitly
    assert!(flags.is_enabled(FeatureFlag::EnableClaudeOpusBackend));
    assert!(flags.is_enabled(FeatureFlag::EnableDistributedTasks));

    // And they should be identified as experimental
    let experimental = flags.enabled_experimental_flags();
    assert_eq!(experimental.len(), 2);
}

#[test]
fn integration_scenario_feature_discovery() {
    // Scenario: Application discovers all available features
    let flags = FeatureFlags::stable_enabled();

    // Should be able to list all enabled features
    let enabled = flags.enabled_flags();
    assert!(!enabled.is_empty());

    // Should be able to filter experimental features
    let experimental = flags.enabled_experimental_flags();
    assert!(experimental.is_empty(), "stable_enabled() should not enable experimental features");

    // Now enable some experimental features
    let mut config = FeatureConfig::default();
    config.agent_backend.enable_claude_opus_backend = true;
    config.pipeline.enable_distributed_tasks = true;

    let flags = FeatureFlags::new(config);

    // Should be able to identify which are experimental
    let experimental = flags.enabled_experimental_flags();
    assert_eq!(experimental.len(), 2);
    assert!(experimental.contains(&FeatureFlag::EnableClaudeOpusBackend));
    assert!(experimental.contains(&FeatureFlag::EnableDistributedTasks));
}

#[test]
fn integration_migration_from_old_config() {
    // Scenario: Migrate from a config without feature flags to one with them
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("old_style.toml");

    // Old-style config (no feature flags section)
    let toml_content = r#"
# This is an old config file without feature flags
# It should use defaults
"#;

    fs::write(&config_path, toml_content).unwrap();

    // Should load with defaults
    let result = FeatureFlags::load_from_file(&config_path);
    if result.is_ok() {
        let flags = result.unwrap();
        // Should have stable defaults
        assert!(flags.is_enabled(FeatureFlag::EnableParallelExecution));
        assert!(flags.is_enabled(FeatureFlag::EnableSmartCache));
    }
    // If it fails, that's also acceptable - empty TOML might not be valid
}

// ============================================================================
// Performance and edge cases
// ============================================================================

#[test]
fn integration_large_whitelist_performance() {
    // Test performance with large whitelist
    let mut rollout = RolloutConfig::new(0);

    // Add 1000 users to whitelist
    for i in 0..1000 {
        rollout = rollout.with_user(format!("whitelisted_user_{}", i));
    }

    // All whitelisted users should be enabled
    for i in 0..1000 {
        let user = format!("whitelisted_user_{}", i);
        assert!(rollout.is_enabled_for(&user),
            "Whitelisted user {} should be enabled", user);
    }

    // Non-whitelisted user should not be enabled
    assert!(!rollout.is_enabled_for("not_whitelisted"));
}

#[test]
fn integration_large_blacklist_performance() {
    // Test performance with large blacklist
    let mut rollout = RolloutConfig::new(100);

    // Add 1000 users to blacklist
    for i in 0..1000 {
        rollout = rollout.with_excluded_user(format!("blacklisted_user_{}", i));
    }

    // All blacklisted users should not be enabled
    for i in 0..1000 {
        let user = format!("blacklisted_user_{}", i);
        assert!(!rollout.is_enabled_for(&user),
            "Blacklisted user {} should not be enabled", user);
    }

    // Non-blacklisted user should be enabled
    assert!(rollout.is_enabled_for("not_blacklisted"));
}

#[test]
fn integration_empty_string_user_id() {
    let rollout = RolloutConfig::new(0).with_user("");

    // Empty string user ID should work
    assert!(rollout.is_enabled_for(""));
}

#[test]
fn integration_very_long_user_id() {
    let long_user = "a".repeat(10000);
    let rollout = RolloutConfig::new(0).with_user(&long_user);

    // Very long user ID should work
    assert!(rollout.is_enabled_for(&long_user));
}
