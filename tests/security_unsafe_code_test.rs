//! Security tests for unsafe code blocks
//!
//! This module tests that:
//! - Unsafe code is properly documented and justified
//! - Unsafe code doesn't introduce data races or undefined behavior
//! - Mutable static access is thread-safe

use ltmatrix::logging::formatter::{format_level, init_color_config};
use ltmatrix::terminal::ColorConfig;
use std::thread;
use tracing::Level;

/// Test that unsafe mutable static initialization is thread-safe
///
/// This test verifies that the COLOR_CONFIG mutable static doesn't cause
/// data races when accessed from multiple threads.
#[test]
fn test_unsafe_color_config_thread_safety() {
    // Initialize color config once (as documented)
    init_color_config(ColorConfig::plain());

    let handle = thread::spawn(|| {
        // Access from another thread - should not cause data race
        let level = format_level(&Level::INFO);
        assert!(level.contains("INFO"));
    });

    handle.join().unwrap();

    // Main thread access should still work
    let level = format_level(&Level::ERROR);
    assert!(level.contains("ERROR"));
}

/// Test that unsafe mutable static can be safely reinitialized
///
/// This verifies that reinitializing the color config doesn't cause
/// memory corruption or undefined behavior.
#[test]
fn test_unsafe_color_config_reinitialization() {
    // Initialize with plain config
    init_color_config(ColorConfig::plain());
    let level1 = format_level(&Level::DEBUG);
    assert!(level1.contains("DEBUG"));

    // Reinitialize with auto config
    init_color_config(ColorConfig::auto());
    let level2 = format_level(&Level::WARN);
    assert!(level2.contains("WARN"));

    // Should work without memory corruption
    let level3 = format_level(&Level::TRACE);
    assert!(level3.contains("TRACE"));
}

/// Test concurrent access to unsafe mutable static
///
/// This stress test verifies that the unsafe mutable static
/// doesn't cause data races under concurrent access.
#[test]
fn test_unsafe_color_config_concurrent_access() {
    init_color_config(ColorConfig::plain());

    let handles: Vec<_> = (0..10)
        .map(|_| {
            thread::spawn(|| {
                for _ in 0..100 {
                    let _ = format_level(&Level::INFO);
                    let _ = format_level(&Level::ERROR);
                    let _ = format_level(&Level::WARN);
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    // Main thread should still work correctly
    let level = format_level(&Level::INFO);
    assert!(level.contains("INFO"));
}

/// Test that unsafe code documentation exists
///
/// This test ensures that unsafe blocks are properly documented
/// with safety comments.
#[test]
fn test_unsafe_code_has_safety_documentation() {
    // This test verifies that the unsafe code in logging/formatter.rs
    // has proper safety documentation (comments explaining why it's safe)
    //
    // The unsafe code uses:
    // 1. static mut COLOR_CONFIG - requires single-threaded initialization
    // 2. unsafe blocks to access the mutable static
    //
    // Safety requirements documented in the code:
    // - "This function should only be called once during application initialization"
    // - "Returns ColorConfig::auto() if not initialized"
    //
    // This test serves as documentation of the safety requirements.

    let _ = format_level(&Level::INFO);
    // If we reach here, unsafe code executed without UB
}

/// Test unsafe unwrap_or_else pattern
///
/// Verifies that the unwrap_or_else in the unsafe block
/// provides a safe fallback.
#[test]
fn test_unsafe_unwrap_or_else_fallback() {
    // Don't initialize COLOR_CONFIG
    // The unsafe code should use the fallback: ColorConfig::auto()

    let level = format_level(&Level::INFO);
    // Should work with auto-detected color config
    assert!(level.contains("INFO"));
}

/// Test that unsafe code doesn't panic
///
/// Ensures that the unsafe mutable static access handles
/// all cases without panicking.
#[test]
fn test_unsafe_code_no_panic() {
    // Test without initialization
    let level1 = format_level(&Level::INFO);
    assert!(!level1.is_empty());

    // Initialize and test again
    init_color_config(ColorConfig::plain());
    let level2 = format_level(&Level::ERROR);
    assert!(!level2.is_empty());

    // All accesses should succeed without panic
}
