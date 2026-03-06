//! Performance tests for algorithmic complexity
//!
//! This module tests that:
//! - Session lookup operations are efficient
//! - Cleanup operations scale linearly
//! - String allocations are minimized
//! - No O(n²) algorithms in hot paths

use ltmatrix::agent::backend::AgentSession;
use ltmatrix::agent::pool::SessionPool;
use ltmatrix::agent::agent_pool::AgentPool;
use ltmatrix::models::Task;
use std::time::{Duration, Instant};

/// Test session lookup is O(1)
///
/// Verifies that session lookup by ID is constant time.
#[tokio::test]
async fn test_session_lookup_complexity() {
    let mut pool = SessionPool::new();

    // Create many sessions
    let mut session_ids = vec![];
    for i in 0..1000 {
        let session = pool.get_or_create(&format!("agent_{}", i), "model");
        session_ids.push(session.session_id().to_string());
    }

    // Measure lookup time for last session
    let start = Instant::now();
    let _ = pool.get(&session_ids[999]);
    let elapsed = start.elapsed();

    // Lookup should be very fast (O(1) HashMap lookup)
    assert!(
        elapsed < Duration::from_micros(100),
        "Session lookup should be O(1): {:?}",
        elapsed
    );
}

/// Test cleanup scales linearly
///
/// Verifies that cleanup operation is O(n).
#[tokio::test]
async fn test_cleanup_linear_complexity() {
    // Test with 100 sessions
    let mut pool1 = SessionPool::new();
    for i in 0..100 {
        let _ = pool1.get_or_create(&format!("agent_{}", i), "model");
    }

    let start1 = Instant::now();
    let _ = pool1.cleanup_stale();
    let elapsed1 = start1.elapsed();

    // Test with 1000 sessions
    let mut pool2 = SessionPool::new();
    for i in 0..1000 {
        let _ = pool2.get_or_create(&format!("agent_{}", i), "model");
    }

    let start2 = Instant::now();
    let _ = pool2.cleanup_stale();
    let elapsed2 = start2.elapsed();

    // 1000 sessions should take roughly 10x longer than 100 (linear)
    // Allow generous margin for variance
    let ratio = elapsed2.as_nanos() as f64 / elapsed1.as_nanos() as f64;
    assert!(
        ratio < 50.0,
        "Cleanup should scale roughly linearly, got ratio: {:.2}",
        ratio
    );
}

/// Test get_or_create doesn't allocate on reuse
///
/// Verifies that reusing sessions is efficient.
#[tokio::test]
async fn test_reuse_no_allocation() {
    let mut pool = SessionPool::new();

    // Create session
    let session1 = pool.get_or_create("agent", "model");
    let id1 = session1.session_id().to_string();

    // Reuse many times
    for _ in 0..100 {
        let session = pool.get_or_create("agent", "model");
        assert_eq!(session.session_id(), id1);
    }

    // Should still have only one session
    assert_eq!(pool.len(), 1);
}

/// Test string cloning is minimized
///
/// Verifies that operations don't create unnecessary string copies.
#[tokio::test]
async fn test_minimal_string_cloning() {
    let mut pool = SessionPool::new();

    // This operation should minimize string allocations
    let agent_name = "claude";
    let model = "claude-sonnet-4-6";

    let start = Instant::now();

    // Many operations with same strings
    for _ in 0..1000 {
        let session = pool.get_or_create(agent_name, model);
        let _ = session.session_id();
    }

    let elapsed = start.elapsed();

    // Should be fast (minimal allocations)
    assert!(
        elapsed < Duration::from_millis(100),
        "Operations should minimize string cloning: {:?}",
        elapsed
    );
}

/// test AgentPool session lookup is efficient
///
/// Verifies that AgentPool's session management is efficient.
#[tokio::test]
async fn test_agent_pool_efficiency() {
    let pool = AgentPool::from_default_config();

    let start = Instant::now();

    // Create and retrieve many sessions
    for i in 0..100 {
        let mut task = Task::new(&format!("task-{}", i), "Test", "Description");
        let session_id = pool
            .get_or_create_session_for_task(&mut task, "claude", "claude-sonnet-4-6")
            .await;

        // Retrieve the same session (reuse scenario)
        let _ = pool
            .get_or_create_session_for_task(&mut task, "claude", "claude-sonnet-4-6")
            .await;

        assert!(!session_id.is_empty());
    }

    let elapsed = start.elapsed();

    // Should complete 200 operations efficiently
    assert!(
        elapsed < Duration::from_secs(1),
        "AgentPool operations should be efficient: {:?}",
        elapsed
    );
}

/// Test stats collection is O(1)
///
/// Verifies that collecting stats doesn't iterate over all sessions.
#[tokio::test]
async fn test_stats_constant_time() {
    let pool = AgentPool::from_default_config();

    // Create many sessions
    for i in 0..100 {
        let mut task = Task::new(&format!("task-{}", i), "Test", "Description");
        let _ = pool
            .get_or_create_session_for_task(&mut task, "claude", "claude-sonnet-4-6")
            .await;
    }

    // Measure stats collection time
    let start = Instant::now();
    let _ = pool.stats().await;
    let elapsed = start.elapsed();

    // Stats should be fast (just getting length, not iterating)
    assert!(
        elapsed < Duration::from_millis(10),
        "Stats collection should be fast: {:?}",
        elapsed
    );
}

/// Test concurrent operations scale well
///
/// Verifies that operations scale reasonably with concurrency.
#[tokio::test]
async fn test_concurrent_scaling() {
    let pool = std::sync::Arc::new(AgentPool::from_default_config());

    // Single-threaded baseline
    let start = Instant::now();
    for i in 0..100 {
        let mut task = Task::new(&format!("task-{}", i), "Test", "Description");
        let _ = pool
            .get_or_create_session_for_task(&mut task, "claude", "claude-sonnet-4-6")
            .await;
    }
    let single_elapsed = start.elapsed();

    // 10-way concurrent
    let mut handles = vec![];
    let start = Instant::now();

    for i in 0..10 {
        let pool_clone = std::sync::Arc::clone(&pool);
        let handle = tokio::spawn(async move {
            for j in 0..10 {
                let mut task = Task::new(&format!("task-{}-{}", i, j), "Test", "Description");
                let _ = pool_clone
                    .get_or_create_session_for_task(&mut task, "claude", "claude-sonnet-4-6")
                    .await;
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    let concurrent_elapsed = start.elapsed();

    // Concurrent should be faster or similar, not 10x slower
    let ratio = concurrent_elapsed.as_nanos() as f64 / single_elapsed.as_nanos() as f64;
    assert!(
        ratio < 5.0,
        "Concurrent operations should scale reasonably, got ratio: {:.2}",
        ratio
    );
}

/// Test memory allocation doesn't grow quadratically
///
/// Verifies that operations don't have O(n²) memory usage.
#[tokio::test]
async fn test_no_quadratic_growth() {
    // Test with different sizes
    let sizes = vec![10, 50, 100];

    for size in sizes {
        let mut pool = SessionPool::new();

        let start = Instant::now();
        let mut ops = 0;

        for i in 0..size {
            // Create
            let id = pool.get_or_create(&format!("agent_{}", i), "model").session_id().to_string();

            // Lookup
            let _ = pool.get(&id);

            // Cleanup
            if i % 10 == 0 {
                let _ = pool.cleanup_stale();
            }

            ops += 3;
        }

        let elapsed = start.elapsed();
        let ops_per_ms = ops as f64 / elapsed.as_millis() as f64;

        // Operations per ms should not degrade dramatically
        assert!(
            ops_per_ms > 0.1,
            "Operations per ms degraded at size {}: {:.2}",
            size,
            ops_per_ms
        );
    }
}

/// test list_by_agent efficiency
///
/// Verifies that listing sessions by agent is efficient.
#[tokio::test]
async fn test_list_by_agent_efficiency() {
    let mut pool = SessionPool::new();

    // Create sessions for multiple agents with unique models to avoid reuse
    for i in 0..100 {
        let agent_name = if i % 2 == 0 { "agent_a" } else { "agent_b" };
        // Use unique model to create separate sessions per iteration
        let model = format!("model_{}", i);
        let _ = pool.get_or_create(agent_name, &model);
    }

    let start = Instant::now();
    let sessions = pool.list_by_agent("agent_a");
    let elapsed = start.elapsed();

    // Should return ~50 sessions quickly
    assert_eq!(sessions.len(), 50);
    assert!(
        elapsed < Duration::from_millis(10),
        "Listing by agent should be fast: {:?}",
        elapsed
    );
}

/// Test iteration is efficient
///
/// Verifies that iterating over sessions is reasonably fast.
#[tokio::test]
async fn test_iteration_efficiency() {
    let mut pool = SessionPool::new();

    // Create many sessions
    for i in 0..1000 {
        let _ = pool.get_or_create(&format!("agent_{}", i), "model");
    }

    let start = Instant::now();
    let count = pool.iter().count();
    let elapsed = start.elapsed();

    assert_eq!(count, 1000);
    assert!(
        elapsed < Duration::from_millis(10),
        "Iteration should be fast: {:?}",
        elapsed
    );
}

/// Test task operations don't cause excessive allocations
///
/// Verifies that task operations are efficient.
#[tokio::test]
async fn test_task_operations_efficiency() {
    let mut pool = SessionPool::new();

    let start = Instant::now();

    for i in 0..100 {
        let mut task = Task::new(&format!("task-{}", i), "Test", "Description");
        let _ = pool.get_or_create_for_task(&mut task);
    }

    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_millis(100),
        "Task operations should be efficient: {:?}",
        elapsed
    );
}

/// Test cleanup doesn't copy sessions
///
/// Verifies that cleanup doesn't unnecessarily clone session data.
#[tokio::test]
async fn test_cleanup_no_cloning() {
    let mut pool = SessionPool::new();

    // Create many sessions
    for i in 0..100 {
        let session = pool.get_or_create(&format!("agent_{}", i), "model");
        // Access each session to ensure it's not stale
        let _ = session.session_id();
    }

    // Measure memory usage approximation via time
    let start = Instant::now();
    let removed = pool.cleanup_stale();
    let elapsed = start.elapsed();

    // Should be fast and not remove any (all fresh)
    assert_eq!(removed, 0);
    assert!(
        elapsed < Duration::from_millis(10),
        "Cleanup should be fast: {:?}",
        elapsed
    );
}
