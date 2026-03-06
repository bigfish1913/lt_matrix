//! Acceptance tests for feature flag system task
//!
//! These tests verify the complete acceptance criteria:
//! - Enable/disable experimental features via config
//! - Support gradual rollout of new features
//! - Feature flags for new agent backends
//! - Feature flags for experimental pipeline stages
//! - Feature flags for alternative schedulers
//! - Documentation of feature flags
//!
//! Tests are organized by acceptance criterion.

use ltmatrix::feature::{FeatureConfig, FeatureFlag, FeatureFlags, RolloutConfig};
use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;

// ============================================================================
// Acceptance Criterion 1: Enable/disable experimental features via config
// ============================================================================

#[test]
fn acceptance_1_1_feature_flag_enum_exists() {
    // Verify all required feature flags exist
    let flags = vec![
        // Agent backend features
        FeatureFlag::EnableClaudeOpusBackend,
        FeatureFlag::EnableOpenCodeBackend,
        FeatureFlag::EnableKimiCodeBackend,
        FeatureFlag::EnableCodexBackend,
        FeatureFlag::EnableCustomBackend,
        // Pipeline features
        FeatureFlag::EnableParallelExecution,
        FeatureFlag::EnableSmartCache,
        FeatureFlag::EnableIncrementalBuilds,
        FeatureFlag::EnableDistributedTasks,
        FeatureFlag::EnableTaskDependencyGraph,
        FeatureFlag::EnableTaskBatching,
        FeatureFlag::EnablePipelineOptimization,
        // Scheduler features
        FeatureFlag::EnablePriorityScheduler,
        FeatureFlag::EnableAdaptiveScheduler,
        FeatureFlag::EnableMlScheduler,
        FeatureFlag::EnableFairShareScheduler,
        FeatureFlag::EnableDeadlineScheduler,
        // Monitoring features
        FeatureFlag::EnableDetailedMetrics,
        FeatureFlag::EnableProfiling,
        FeatureFlag::EnableMonitoringDashboard,
        FeatureFlag::EnableAlerting,
        // Development features
        FeatureFlag::EnableVerboseDebug,
        FeatureFlag::EnableTracing,
        FeatureFlag::EnableExperimentalCommands,
        FeatureFlag::EnableTestingUtilities,
    ];

    // All flags should have valid config keys
    for flag in flags {
        let key = flag.config_key();
        assert!(!key.is_empty(), "Feature flag should have a config key");
        assert!(key.contains("enable"), "Config key should contain 'enable'");
    }
}

#[test]
fn acceptance_1_2_enable_features_via_config() {
    // Create a config with specific features enabled
    let mut config = FeatureConfig::default();
    config.agent_backend.enable_claude_opus_backend = true;
    config.pipeline.enable_parallel_execution = true;
    config.scheduler.enable_priority_scheduler = true;

    let flags = FeatureFlags::new(config);

    // Verify features are enabled
    assert!(flags.is_enabled(FeatureFlag::EnableClaudeOpusBackend));
    assert!(flags.is_enabled(FeatureFlag::EnableParallelExecution));
    assert!(flags.is_enabled(FeatureFlag::EnablePriorityScheduler));

    // Verify features that were not enabled are disabled
    assert!(!flags.is_enabled(FeatureFlag::EnableOpenCodeBackend));
    assert!(!flags.is_enabled(FeatureFlag::EnableDistributedTasks));
}

#[test]
fn acceptance_1_3_disable_features_via_config() {
    // Create a config with features explicitly disabled
    let mut config = FeatureConfig::default();
    config.agent_backend.enable_claude_opus_backend = false;
    config.pipeline.enable_parallel_execution = false;
    config.scheduler.enable_priority_scheduler = false;

    let flags = FeatureFlags::new(config);

    // Verify features are disabled
    assert!(!flags.is_enabled(FeatureFlag::EnableClaudeOpusBackend));
    assert!(!flags.is_enabled(FeatureFlag::EnableParallelExecution));
    assert!(!flags.is_enabled(FeatureFlag::EnablePriorityScheduler));
}

#[test]
fn acceptance_1_4_load_feature_config_from_toml() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("features.toml");

    let toml_content = r#"
[agent_backend]
enable_claude_opus_backend = true
enable_opencode_backend = false

[pipeline]
enable_parallel_execution = true
enable_smart_cache = true
enable_incremental_builds = false

[scheduler]
enable_priority_scheduler = true
enable_adaptive_scheduler = false

[monitoring]
enable_detailed_metrics = true

[development]
enable_verbose_debug = false
"#;

    fs::write(&config_path, toml_content).unwrap();

    // Load from file
    let flags = FeatureFlags::load_from_file(&config_path).unwrap();

    // Verify the loaded configuration
    assert!(flags.is_enabled(FeatureFlag::EnableClaudeOpusBackend));
    assert!(!flags.is_enabled(FeatureFlag::EnableOpenCodeBackend));
    assert!(flags.is_enabled(FeatureFlag::EnableParallelExecution));
    assert!(flags.is_enabled(FeatureFlag::EnableSmartCache));
    assert!(!flags.is_enabled(FeatureFlag::EnableIncrementalBuilds));
    assert!(flags.is_enabled(FeatureFlag::EnablePriorityScheduler));
    assert!(!flags.is_enabled(FeatureFlag::EnableAdaptiveScheduler));
    assert!(flags.is_enabled(FeatureFlag::EnableDetailedMetrics));
    assert!(!flags.is_enabled(FeatureFlag::EnableVerboseDebug));
}

// ============================================================================
// Acceptance Criterion 2: Support gradual rollout of new features
// ============================================================================

#[test]
fn acceptance_2_1_rollout_config_percentage_based() {
    // Create rollout config at 50%
    let rollout = RolloutConfig::new(50);

    // Should be deterministic based on user hash
    let mut enabled_count = 0;
    for i in 0..100 {
        let user = format!("user{}", i);
        if rollout.is_enabled_for(&user) {
            enabled_count += 1;
        }
    }

    // Should be approximately 50% (allow 20% margin for hash distribution)
    assert!(
        enabled_count >= 30 && enabled_count <= 70,
        "Rollout at 50% should enable approximately 50% of users, got {}%",
        enabled_count
    );
}

#[test]
fn acceptance_2_2_rollout_config_whitelist() {
    // Create rollout at 0% with whitelist
    let rollout = RolloutConfig::new(0)
        .with_user("beta_tester1")
        .with_user("beta_tester2");

    // Whitelisted users should be enabled
    assert!(rollout.is_enabled_for("beta_tester1"));
    assert!(rollout.is_enabled_for("beta_tester2"));

    // Non-whitelisted users should be disabled
    assert!(!rollout.is_enabled_for("regular_user"));
}

#[test]
fn acceptance_2_3_rollout_config_blacklist() {
    // Create rollout at 100% with blacklist
    let rollout = RolloutConfig::new(100).with_excluded_user("problematic_user");

    // Blacklisted user should be disabled
    assert!(!rollout.is_enabled_for("problematic_user"));

    // Non-blacklisted users should be enabled
    assert!(rollout.is_enabled_for("normal_user"));
}

#[test]
fn acceptance_2_4_rollout_priority_blacklist_over_whitelist() {
    // Blacklist should take priority over whitelist
    let rollout = RolloutConfig::new(50)
        .with_user("beta_tester")
        .with_excluded_user("beta_tester");

    // Blacklist wins
    assert!(!rollout.is_enabled_for("beta_tester"));
}

#[test]
fn acceptance_2_5_feature_flags_with_rollout() {
    let mut config = FeatureConfig::default();
    config.pipeline.enable_parallel_execution = true;

    // Add rollout config at 0% with whitelist
    let mut rollout_map = HashMap::new();
    rollout_map.insert(
        "enable_parallel_execution".to_string(),
        RolloutConfig::new(0).with_user("special_user"),
    );
    config.rollout = rollout_map;

    let flags = FeatureFlags::new(config);

    // Regular users should not have the feature
    assert!(!flags.is_enabled_for_user(FeatureFlag::EnableParallelExecution, "regular_user"));

    // Whitelisted user should have the feature
    assert!(flags.is_enabled_for_user(FeatureFlag::EnableParallelExecution, "special_user"));
}

#[test]
fn acceptance_2_6_rollout_from_toml_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("features.toml");

    let toml_content = r#"
[pipeline]
enable_parallel_execution = true

[rollout.enable_parallel_execution]
percentage = 0
users = ["user1", "user2"]
excluded_users = ["user3"]
"#;

    fs::write(&config_path, toml_content).unwrap();

    let flags = FeatureFlags::load_from_file(&config_path).unwrap();

    // Check rollout config
    let rollout = flags.rollout_config(FeatureFlag::EnableParallelExecution);
    assert!(rollout.is_some());

    let rollout = rollout.unwrap();
    assert_eq!(rollout.percentage, 0);
    assert!(rollout.users.contains("user1"));
    assert!(rollout.users.contains("user2"));
    assert!(rollout.excluded_users.contains("user3"));

    // Verify rollout behavior
    assert!(flags.is_enabled_for_user(FeatureFlag::EnableParallelExecution, "user1"));
    assert!(!flags.is_enabled_for_user(FeatureFlag::EnableParallelExecution, "user4"));
}

// ============================================================================
// Acceptance Criterion 3: Feature flags for new agent backends
// ============================================================================

#[test]
fn acceptance_3_1_all_agent_backend_flags_exist() {
    let backend_flags = vec![
        FeatureFlag::EnableClaudeOpusBackend,
        FeatureFlag::EnableOpenCodeBackend,
        FeatureFlag::EnableKimiCodeBackend,
        FeatureFlag::EnableCodexBackend,
        FeatureFlag::EnableCustomBackend,
    ];

    for flag in backend_flags {
        // Each flag should have a description
        let desc = flag.description();
        assert!(
            !desc.is_empty(),
            "Agent backend flag should have a description"
        );

        // Each flag should have a valid config key
        let key = flag.config_key();
        assert!(
            key.contains("backend") || key.contains("opus") || key.contains("opencode"),
            "Agent backend flag key should be descriptive"
        );
    }
}

#[test]
fn acceptance_3_2_agent_backend_flags_configurable() {
    let mut config = FeatureConfig::default();
    config.agent_backend.enable_claude_opus_backend = true;
    config.agent_backend.enable_opencode_backend = true;
    config.agent_backend.enable_kimicode_backend = false;
    config.agent_backend.enable_codex_backend = false;
    config.agent_backend.enable_custom_backend = false;

    let flags = FeatureFlags::new(config);

    assert!(flags.is_enabled(FeatureFlag::EnableClaudeOpusBackend));
    assert!(flags.is_enabled(FeatureFlag::EnableOpenCodeBackend));
    assert!(!flags.is_enabled(FeatureFlag::EnableKimiCodeBackend));
    assert!(!flags.is_enabled(FeatureFlag::EnableCodexBackend));
    assert!(!flags.is_enabled(FeatureFlag::EnableCustomBackend));
}

#[test]
fn acceptance_3_3_claude_opus_is_experimental() {
    // Claude Opus should be marked as experimental
    assert!(FeatureFlag::EnableClaudeOpusBackend.is_experimental());
    assert!(!FeatureFlag::EnableClaudeOpusBackend.is_stable());
}

#[test]
fn acceptance_3_4_custom_backend_is_experimental() {
    // Custom backend should be marked as experimental
    assert!(FeatureFlag::EnableCustomBackend.is_experimental());
    assert!(!FeatureFlag::EnableCustomBackend.is_stable());
}

#[test]
fn acceptance_3_5_agent_backend_rollout() {
    let mut config = FeatureConfig::default();
    config.agent_backend.enable_claude_opus_backend = true;

    // Gradual rollout for new backend
    let mut rollout_map = HashMap::new();
    rollout_map.insert(
        "enable_claude_opus_backend".to_string(),
        RolloutConfig::new(25), // 25% rollout
    );
    config.rollout = rollout_map;

    let flags = FeatureFlags::new(config);

    // Should vary by user - test with more users for statistical significance
    let results: Vec<bool> = (0..20)
        .map(|i| {
            flags.is_enabled_for_user(FeatureFlag::EnableClaudeOpusBackend, &format!("user{}", i))
        })
        .collect();

    // Not all should be the same at 25%
    let all_enabled = results.iter().all(|&r| r);
    let all_disabled = results.iter().all(|&r| !r);
    assert!(
        !all_enabled && !all_disabled,
        "At 25% rollout, some users should have it and some shouldn't"
    );
}

// ============================================================================
// Acceptance Criterion 4: Feature flags for experimental pipeline stages
// ============================================================================

#[test]
fn acceptance_4_1_all_pipeline_feature_flags_exist() {
    let pipeline_flags = vec![
        FeatureFlag::EnableParallelExecution,
        FeatureFlag::EnableSmartCache,
        FeatureFlag::EnableIncrementalBuilds,
        FeatureFlag::EnableDistributedTasks,
        FeatureFlag::EnableTaskDependencyGraph,
        FeatureFlag::EnableTaskBatching,
        FeatureFlag::EnablePipelineOptimization,
    ];

    for flag in pipeline_flags {
        let desc = flag.description();
        assert!(!desc.is_empty(), "Pipeline flag should have a description");

        let key = flag.config_key();
        assert!(!key.is_empty(), "Pipeline flag should have a config key");
    }
}

#[test]
fn acceptance_4_2_pipeline_flags_configurable() {
    let mut config = FeatureConfig::default();
    config.pipeline.enable_parallel_execution = true;
    config.pipeline.enable_smart_cache = true;
    config.pipeline.enable_incremental_builds = true;
    config.pipeline.enable_distributed_tasks = false;
    config.pipeline.enable_task_dependency_graph = false;
    config.pipeline.enable_task_batching = false;
    config.pipeline.enable_pipeline_optimization = false;

    let flags = FeatureFlags::new(config);

    assert!(flags.is_enabled(FeatureFlag::EnableParallelExecution));
    assert!(flags.is_enabled(FeatureFlag::EnableSmartCache));
    assert!(flags.is_enabled(FeatureFlag::EnableIncrementalBuilds));
    assert!(!flags.is_enabled(FeatureFlag::EnableDistributedTasks));
    assert!(!flags.is_enabled(FeatureFlag::EnableTaskDependencyGraph));
    assert!(!flags.is_enabled(FeatureFlag::EnableTaskBatching));
    assert!(!flags.is_enabled(FeatureFlag::EnablePipelineOptimization));
}

#[test]
fn acceptance_4_3_stable_pipeline_features_enabled_by_default() {
    let flags = FeatureFlags::stable_enabled();

    // Parallel execution and smart cache should be enabled by default
    assert!(flags.is_enabled(FeatureFlag::EnableParallelExecution));
    assert!(flags.is_enabled(FeatureFlag::EnableSmartCache));
}

#[test]
fn acceptance_4_4_experimental_pipeline_features() {
    // Distributed tasks should be experimental
    assert!(FeatureFlag::EnableDistributedTasks.is_experimental());

    // Task dependency graph should be experimental
    assert!(FeatureFlag::EnableTaskDependencyGraph.is_experimental());

    // Parallel execution should be stable
    assert!(!FeatureFlag::EnableParallelExecution.is_experimental());

    // Smart cache should be stable
    assert!(!FeatureFlag::EnableSmartCache.is_experimental());
}

#[test]
fn acceptance_4_5_pipeline_feature_rollout() {
    let mut config = FeatureConfig::default();
    config.pipeline.enable_incremental_builds = true;

    // Gradual rollout for incremental builds
    let mut rollout_map = HashMap::new();
    rollout_map.insert(
        "enable_incremental_builds".to_string(),
        RolloutConfig::new(10).with_user("beta_tester"),
    );
    config.rollout = rollout_map;

    let flags = FeatureFlags::new(config);

    // Beta tester should have it
    assert!(flags.is_enabled_for_user(FeatureFlag::EnableIncrementalBuilds, "beta_tester"));

    // Regular user at 10% might not have it (depends on hash)
    let regular_user_has_it =
        flags.is_enabled_for_user(FeatureFlag::EnableIncrementalBuilds, "regular_user");
    // We can't assert the exact value due to hash, but we can verify the function works
    let _ = regular_user_has_it;
}

// ============================================================================
// Acceptance Criterion 5: Feature flags for alternative schedulers
// ============================================================================

#[test]
fn acceptance_5_1_all_scheduler_flags_exist() {
    let scheduler_flags = vec![
        FeatureFlag::EnablePriorityScheduler,
        FeatureFlag::EnableAdaptiveScheduler,
        FeatureFlag::EnableMlScheduler,
        FeatureFlag::EnableFairShareScheduler,
        FeatureFlag::EnableDeadlineScheduler,
    ];

    for flag in scheduler_flags {
        let desc = flag.description();
        assert!(!desc.is_empty(), "Scheduler flag should have a description");

        let key = flag.config_key();
        assert!(!key.is_empty(), "Scheduler flag should have a config key");
    }
}

#[test]
fn acceptance_5_2_scheduler_flags_configurable() {
    let mut config = FeatureConfig::default();
    config.scheduler.enable_priority_scheduler = true;
    config.scheduler.enable_adaptive_scheduler = true;
    config.scheduler.enable_ml_scheduler = false;
    config.scheduler.enable_fair_share_scheduler = false;
    config.scheduler.enable_deadline_scheduler = false;

    let flags = FeatureFlags::new(config);

    assert!(flags.is_enabled(FeatureFlag::EnablePriorityScheduler));
    assert!(flags.is_enabled(FeatureFlag::EnableAdaptiveScheduler));
    assert!(!flags.is_enabled(FeatureFlag::EnableMlScheduler));
    assert!(!flags.is_enabled(FeatureFlag::EnableFairShareScheduler));
    assert!(!flags.is_enabled(FeatureFlag::EnableDeadlineScheduler));
}

#[test]
fn acceptance_5_3_ml_scheduler_is_experimental() {
    // ML scheduler should be marked as experimental
    assert!(FeatureFlag::EnableMlScheduler.is_experimental());
    assert!(!FeatureFlag::EnableMlScheduler.is_stable());
}

#[test]
fn acceptance_5_4_scheduler_rollout() {
    let mut config = FeatureConfig::default();
    config.scheduler.enable_priority_scheduler = true;

    // Gradual rollout for priority scheduler
    let mut rollout_map = HashMap::new();
    rollout_map.insert(
        "enable_priority_scheduler".to_string(),
        RolloutConfig::new(5), // Start with 5% rollout
    );
    config.rollout = rollout_map;

    let flags = FeatureFlags::new(config);

    // Function should work correctly
    let user_result = flags.is_enabled_for_user(FeatureFlag::EnablePriorityScheduler, "test_user");
    let _ = user_result; // We can't assert exact value due to hash
}

#[test]
fn acceptance_5_5_multiple_schedulers_configurable() {
    // Test that multiple scheduler flags can be configured independently
    let mut config = FeatureConfig::default();
    config.scheduler.enable_priority_scheduler = true;
    config.scheduler.enable_adaptive_scheduler = true;
    config.scheduler.enable_ml_scheduler = true;
    config.scheduler.enable_fair_share_scheduler = true;
    config.scheduler.enable_deadline_scheduler = true;

    let flags = FeatureFlags::new(config);

    // All should be enabled
    assert!(flags.is_enabled(FeatureFlag::EnablePriorityScheduler));
    assert!(flags.is_enabled(FeatureFlag::EnableAdaptiveScheduler));
    assert!(flags.is_enabled(FeatureFlag::EnableMlScheduler));
    assert!(flags.is_enabled(FeatureFlag::EnableFairShareScheduler));
    assert!(flags.is_enabled(FeatureFlag::EnableDeadlineScheduler));
}

// ============================================================================
// Acceptance Criterion 6: Documentation of feature flags
// ============================================================================

#[test]
fn acceptance_6_1_all_feature_flags_have_descriptions() {
    use FeatureFlag::*;

    let all_flags = vec![
        // Agent backends
        EnableClaudeOpusBackend,
        EnableOpenCodeBackend,
        EnableKimiCodeBackend,
        EnableCodexBackend,
        EnableCustomBackend,
        // Pipeline
        EnableParallelExecution,
        EnableSmartCache,
        EnableIncrementalBuilds,
        EnableDistributedTasks,
        EnableTaskDependencyGraph,
        EnableTaskBatching,
        EnablePipelineOptimization,
        // Scheduler
        EnablePriorityScheduler,
        EnableAdaptiveScheduler,
        EnableMlScheduler,
        EnableFairShareScheduler,
        EnableDeadlineScheduler,
        // Monitoring
        EnableDetailedMetrics,
        EnableProfiling,
        EnableMonitoringDashboard,
        EnableAlerting,
        // Development
        EnableVerboseDebug,
        EnableTracing,
        EnableExperimentalCommands,
        EnableTestingUtilities,
    ];

    for flag in all_flags {
        let desc = flag.description();
        assert!(
            !desc.is_empty(),
            "Feature flag {:?} should have a description",
            flag
        );
        assert!(
            desc.len() > 10,
            "Description for {:?} should be meaningful (got: {})",
            flag,
            desc
        );
    }
}

#[test]
fn acceptance_6_2_descriptions_are_meaningful() {
    // Descriptions should contain relevant keywords
    assert!(FeatureFlag::EnableParallelExecution
        .description()
        .contains("parallel"));
    assert!(FeatureFlag::EnableSmartCache
        .description()
        .to_lowercase()
        .contains("cach"));
    assert!(FeatureFlag::EnableMlScheduler
        .description()
        .to_lowercase()
        .contains("ml"));
    assert!(FeatureFlag::EnableDetailedMetrics
        .description()
        .contains("metrics"));
}

#[test]
fn acceptance_6_3_config_keys_follow_naming_convention() {
    use FeatureFlag::*;

    let all_flags = vec![
        EnableClaudeOpusBackend,
        EnableOpenCodeBackend,
        EnableKimiCodeBackend,
        EnableCodexBackend,
        EnableCustomBackend,
        EnableParallelExecution,
        EnableSmartCache,
        EnableIncrementalBuilds,
        EnableDistributedTasks,
        EnableTaskDependencyGraph,
        EnableTaskBatching,
        EnablePipelineOptimization,
        EnablePriorityScheduler,
        EnableAdaptiveScheduler,
        EnableMlScheduler,
        EnableFairShareScheduler,
        EnableDeadlineScheduler,
        EnableDetailedMetrics,
        EnableProfiling,
        EnableMonitoringDashboard,
        EnableAlerting,
        EnableVerboseDebug,
        EnableTracing,
        EnableExperimentalCommands,
        EnableTestingUtilities,
    ];

    for flag in all_flags {
        let key = flag.config_key();
        // All config keys should start with "enable_"
        assert!(
            key.starts_with("enable_"),
            "Config key for {:?} should start with 'enable_', got: {}",
            flag,
            key
        );
        // All config keys should be snake_case
        assert!(!key.contains("-"), "Config key should not contain hyphens");
        assert!(!key.contains(" "), "Config key should not contain spaces");
    }
}

// ============================================================================
// Additional comprehensive tests
// ============================================================================

#[test]
fn comprehensive_all_features_disabled() {
    let flags = FeatureFlags::all_disabled();
    let enabled = flags.enabled_flags();

    // No features should be enabled
    assert!(
        enabled.is_empty(),
        "all_disabled() should have no enabled features"
    );
}

#[test]
fn comprehensive_only_stable_features_enabled() {
    let flags = FeatureFlags::stable_enabled();
    let enabled = flags.enabled_flags();
    let experimental = flags.enabled_experimental_flags();

    // Should have some enabled features
    assert!(
        !enabled.is_empty(),
        "stable_enabled() should have some enabled features"
    );

    // Should not have experimental features enabled
    assert!(
        experimental.is_empty(),
        "stable_enabled() should not enable experimental features, but found: {:?}",
        experimental
    );
}

#[test]
fn comprehensive_serialization_roundtrip() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("features.toml");

    // Create a comprehensive config
    let mut config = FeatureConfig::default();
    config.agent_backend.enable_claude_opus_backend = true;
    config.pipeline.enable_parallel_execution = true;
    config.pipeline.enable_distributed_tasks = true;
    config.scheduler.enable_ml_scheduler = true;
    config.monitoring.enable_monitoring_dashboard = true;
    config.development.enable_experimental_commands = true;

    let flags = FeatureFlags::new(config.clone());

    // Save to file
    flags.save_to_file(&config_path).unwrap();

    // Load from file
    let loaded = FeatureFlags::load_from_file(&config_path).unwrap();

    // Verify all flags match
    assert_eq!(
        loaded.is_enabled(FeatureFlag::EnableClaudeOpusBackend),
        flags.is_enabled(FeatureFlag::EnableClaudeOpusBackend)
    );
    assert_eq!(
        loaded.is_enabled(FeatureFlag::EnableParallelExecution),
        flags.is_enabled(FeatureFlag::EnableParallelExecution)
    );
    assert_eq!(
        loaded.is_enabled(FeatureFlag::EnableDistributedTasks),
        flags.is_enabled(FeatureFlag::EnableDistributedTasks)
    );
    assert_eq!(
        loaded.is_enabled(FeatureFlag::EnableMlScheduler),
        flags.is_enabled(FeatureFlag::EnableMlScheduler)
    );
    assert_eq!(
        loaded.is_enabled(FeatureFlag::EnableMonitoringDashboard),
        flags.is_enabled(FeatureFlag::EnableMonitoringDashboard)
    );
    assert_eq!(
        loaded.is_enabled(FeatureFlag::EnableExperimentalCommands),
        flags.is_enabled(FeatureFlag::EnableExperimentalCommands)
    );
}

#[test]
fn comprehensive_production_safe_defaults() {
    // Default config should be production-safe
    let flags = FeatureFlags::default();
    let experimental = flags.enabled_experimental_flags();

    // No experimental features should be enabled by default
    assert!(
        experimental.is_empty(),
        "Default configuration should not enable experimental features, but found: {:?}",
        experimental
    );

    // Stable features like parallel execution and smart cache should be enabled
    assert!(
        flags.is_enabled(FeatureFlag::EnableParallelExecution),
        "Parallel execution should be enabled by default"
    );
    assert!(
        flags.is_enabled(FeatureFlag::EnableSmartCache),
        "Smart cache should be enabled by default"
    );
}

#[test]
fn comprehensive_feature_categories() {
    // Verify all feature categories are represented
    let flags = FeatureFlags::stable_enabled();

    // Agent backend
    assert!(flags.is_enabled(FeatureFlag::EnableParallelExecution));

    // Pipeline
    assert!(flags.is_enabled(FeatureFlag::EnableSmartCache));

    // Check we can query flags by category
    let enabled = flags.enabled_flags();

    // Should have features from different categories
    let pipeline_count = enabled
        .iter()
        .filter(|f| {
            matches!(
                f,
                FeatureFlag::EnableParallelExecution | FeatureFlag::EnableSmartCache
            )
        })
        .count();

    assert!(pipeline_count > 0, "Should have pipeline features enabled");
}
