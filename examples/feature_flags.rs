//! Example demonstrating feature flag system usage
//!
//! Run with: cargo run --example feature_flags

use ltmatrix::feature::{FeatureFlag, FeatureFlags, RolloutConfig};
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Feature Flag System Demo ===\n");

    // Example 1: Check if features are enabled
    println!("1. Basic feature flag checking:");
    let flags = FeatureFlags::stable_enabled();

    println!(
        "   Parallel execution: {}",
        if flags.is_enabled(FeatureFlag::EnableParallelExecution) {
            "✓ Enabled"
        } else {
            "✗ Disabled"
        }
    );
    println!(
        "   Smart cache: {}",
        if flags.is_enabled(FeatureFlag::EnableSmartCache) {
            "✓ Enabled"
        } else {
            "✗ Disabled"
        }
    );
    println!(
        "   Claude Opus backend: {}",
        if flags.is_enabled(FeatureFlag::EnableClaudeOpusBackend) {
            "✓ Enabled"
        } else {
            "✗ Disabled"
        }
    );
    println!(
        "   Incremental builds: {}",
        if flags.is_enabled(FeatureFlag::EnableIncrementalBuilds) {
            "✓ Enabled"
        } else {
            "✗ Disabled"
        }
    );

    // Example 2: Get feature descriptions
    println!("\n2. Feature flag descriptions:");
    for flag in &[
        FeatureFlag::EnableParallelExecution,
        FeatureFlag::EnableSmartCache,
        FeatureFlag::EnableClaudeOpusBackend,
    ] {
        println!("   - {}: {}", flag.config_key(), flag.description());
    }

    // Example 3: Check if features are experimental
    println!("\n3. Feature stability:");
    for flag in &[
        FeatureFlag::EnableParallelExecution,
        FeatureFlag::EnableSmartCache,
        FeatureFlag::EnableClaudeOpusBackend,
        FeatureFlag::EnableDistributedTasks,
        FeatureFlag::EnableMlScheduler,
    ] {
        let status = if flag.is_experimental() {
            "⚠ Experimental"
        } else {
            "✓ Stable"
        };
        println!("   - {}: {}", flag.config_key(), status);
    }

    // Example 4: List all enabled flags
    println!("\n4. All enabled feature flags:");
    let enabled = flags.enabled_flags();
    println!("   Total enabled: {}", enabled.len());
    for flag in &enabled {
        println!("   - {}", flag.config_key());
    }

    // Example 5: List experimental flags that are enabled
    println!("\n5. Enabled experimental flags:");
    let experimental = flags.enabled_experimental_flags();
    if experimental.is_empty() {
        println!("   None (good for production!)");
    } else {
        for flag in &experimental {
            println!("   - {} (⚠ Experimental)", flag.config_key());
        }
    }

    // Example 6: Gradual rollout with percentage
    println!("\n6. Gradual rollout (percentage-based):");

    // Create a rollout config at 50%
    let rollout_config = RolloutConfig::new(50);

    // Simulate checking for different users
    let test_users = &["alice", "bob", "charlie", "dave", "eve"];
    println!("   Rollout at 50%:");

    for user in test_users {
        let enabled = rollout_config.is_enabled_for(user);
        println!(
            "   - {}: {}",
            user,
            if enabled {
                "✓ Enabled"
            } else {
                "✗ Disabled"
            }
        );
    }

    // Example 7: Gradual rollout with whitelist
    println!("\n7. Gradual rollout (whitelist):");

    let rollout_whitelist = RolloutConfig::new(0)
        .with_user("beta_tester1")
        .with_user("beta_tester2");

    println!("   Rollout at 0% with whitelist:");
    for user in &["alice", "beta_tester1", "bob", "beta_tester2", "charlie"] {
        let enabled = rollout_whitelist.is_enabled_for(user);
        println!(
            "   - {}: {}",
            user,
            if enabled {
                "✓ Enabled (whitelisted)"
            } else {
                "✗ Disabled"
            }
        );
    }

    // Example 8: Gradual rollout with blacklist
    println!("\n8. Gradual rollout (blacklist):");

    let rollout_blacklist = RolloutConfig::new(100).with_excluded_user("problematic_user");

    println!("   Rollout at 100% with blacklist:");
    for user in &["alice", "bob", "problematic_user", "charlie"] {
        let enabled = rollout_blacklist.is_enabled_for(user);
        println!(
            "   - {}: {}",
            user,
            if enabled {
                "✓ Enabled"
            } else {
                "✗ Disabled (blacklisted)"
            }
        );
    }

    // Example 9: Feature flags with user-specific rollout
    println!("\n9. Feature flags with user-specific checks:");

    // Create a custom feature config with rollout
    use ltmatrix::feature::FeatureConfig;

    let mut feature_config = FeatureConfig::default();
    feature_config.pipeline.enable_parallel_execution = true;

    // Add rollout config for parallel execution at 25%
    let mut rollout_map = HashMap::new();
    rollout_map.insert(
        "enable_parallel_execution".to_string(),
        RolloutConfig::new(25),
    );

    feature_config.rollout = rollout_map;

    let custom_flags = FeatureFlags::new(feature_config);

    // Check for different users
    for user in &["user1", "user2", "user3", "user4", "user5"] {
        let enabled = custom_flags.is_enabled_for_user(FeatureFlag::EnableParallelExecution, user);
        println!(
            "   - {}: {}",
            user,
            if enabled {
                "✓ Enabled"
            } else {
                "✗ Disabled"
            }
        );
    }

    // Example 10: Load feature flags from TOML (example only)
    println!("\n10. Loading feature flags from TOML configuration:");
    println!("   Example TOML configuration:");
    println!("   ```toml");
    println!("   [features.agent_backend]");
    println!("   enable_claude_opus_backend = true");
    println!("   enable_opencode_backend = false");
    println!("");
    println!("   [features.pipeline]");
    println!("   enable_parallel_execution = true");
    println!("   enable_smart_cache = true");
    println!("   enable_incremental_builds = false");
    println!("");
    println!("   [features.scheduler]");
    println!("   enable_priority_scheduler = true");
    println!("");
    println!("   [features.rollout.enable_priority_scheduler]");
    println!("   percentage = 10");
    println!("   users = [\"beta_tester1\", \"beta_tester2\"]");
    println!("   ```");

    println!("\n=== Feature Flag Categories ===");

    // Agent Backend Features
    println!("\n📦 Agent Backend Features:");
    for flag in &[
        FeatureFlag::EnableClaudeOpusBackend,
        FeatureFlag::EnableOpenCodeBackend,
        FeatureFlag::EnableKimiCodeBackend,
        FeatureFlag::EnableCodexBackend,
        FeatureFlag::EnableCustomBackend,
    ] {
        println!("   - {}: {}", flag.config_key(), flag.description());
    }

    // Pipeline Features
    println!("\n⚙️  Pipeline Features:");
    for flag in &[
        FeatureFlag::EnableParallelExecution,
        FeatureFlag::EnableSmartCache,
        FeatureFlag::EnableIncrementalBuilds,
        FeatureFlag::EnableDistributedTasks,
        FeatureFlag::EnableTaskDependencyGraph,
        FeatureFlag::EnableTaskBatching,
        FeatureFlag::EnablePipelineOptimization,
    ] {
        println!("   - {}: {}", flag.config_key(), flag.description());
    }

    // Scheduler Features
    println!("\n📊 Scheduler Features:");
    for flag in &[
        FeatureFlag::EnablePriorityScheduler,
        FeatureFlag::EnableAdaptiveScheduler,
        FeatureFlag::EnableMlScheduler,
        FeatureFlag::EnableFairShareScheduler,
        FeatureFlag::EnableDeadlineScheduler,
    ] {
        println!("   - {}: {}", flag.config_key(), flag.description());
    }

    // Monitoring Features
    println!("\n📈 Monitoring & Observability Features:");
    for flag in &[
        FeatureFlag::EnableDetailedMetrics,
        FeatureFlag::EnableProfiling,
        FeatureFlag::EnableMonitoringDashboard,
        FeatureFlag::EnableAlerting,
    ] {
        println!("   - {}: {}", flag.config_key(), flag.description());
    }

    // Development Features
    println!("\n🔧 Development & Debugging Features:");
    for flag in &[
        FeatureFlag::EnableVerboseDebug,
        FeatureFlag::EnableTracing,
        FeatureFlag::EnableExperimentalCommands,
        FeatureFlag::EnableTestingUtilities,
    ] {
        println!("   - {}: {}", flag.config_key(), flag.description());
    }

    println!("\n=== Demo Complete ===");
    println!("\nKey Capabilities:");
    println!("✓ Enable/disable features via configuration");
    println!("✓ Gradual rollout based on percentage");
    println!("✓ User whitelist/blacklist support");
    println!("✓ Feature stability indicators");
    println!("✓ TOML-based configuration");
    println!("✓ Production-ready defaults");

    Ok(())
}
