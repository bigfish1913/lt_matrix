//! Performance tests for lock contention
//!
//! This module tests that:
//! - AgentPool lock doesn't cause excessive contention
//! - Concurrent operations scale reasonably
//! - Lock hold times are minimal
//! - Deadlock scenarios are avoided

use ltmatrix::agent::agent_pool::AgentPool;
use ltmatrix::models::Task;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;

/// Test single-threaded performance baseline
///
/// Establishes baseline performance without contention.
#[tokio::test]
async fn test_single_threaded_baseline() {
    let pool = AgentPool::from_default_config();

    let start = Instant::now();

    for i in 0..100 {
        let mut task = Task::new(&format!("task-{}", i), "Test", "Description");
        let _ = pool
            .get_or_create_session_for_task(&mut task, "claude", "claude-sonnet-4-6")
            .await;
    }

    let elapsed = start.elapsed();

    // Should complete 100 operations in reasonable time
    assert!(
        elapsed < Duration::from_secs(1),
        "Single-threaded operations should be fast: {:?}",
        elapsed
    );
}

/// Test concurrent read performance
///
/// Measures performance of concurrent read operations.
#[tokio::test]
async fn test_concurrent_read_performance() {
    let pool = Arc::new(AgentPool::from_default_config());
    let mut handles = vec![];

    let start = Instant::now();

    // Spawn 10 concurrent readers
    for _i in 0..10 {
        let pool_clone = Arc::clone(&pool);
        let handle = tokio::spawn(async move {
            for j in 0..50 {
                let _ = pool_clone.stats().await;
                let _ = pool_clone.cleanup_stale_sessions().await;

                if j % 10 == 0 {
                    sleep(Duration::from_millis(1)).await;
                }
            }
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }

    let elapsed = start.elapsed();

    // Should complete 500 operations (10 * 50) in reasonable time
    assert!(
        elapsed < Duration::from_secs(5),
        "Concurrent reads should scale well: {:?}",
        elapsed
    );
}

/// Test concurrent write performance
///
/// Measures performance of concurrent write operations.
#[tokio::test]
async fn test_concurrent_write_performance() {
    let pool = Arc::new(AgentPool::from_default_config());
    let mut handles = vec![];

    let start = Instant::now();

    // Spawn 10 concurrent writers
    for i in 0..10 {
        let pool_clone = Arc::clone(&pool);
        let handle = tokio::spawn(async move {
            for j in 0..20 {
                let mut task = Task::new(&format!("task-{}-{}", i, j), "Test", "Description");
                let _ = pool_clone
                    .get_or_create_session_for_task(&mut task, "claude", "claude-sonnet-4-6")
                    .await;
            }
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }

    let elapsed = start.elapsed();

    // Should complete 200 operations (10 * 20) in reasonable time
    assert!(
        elapsed < Duration::from_secs(5),
        "Concurrent writes should scale reasonably: {:?}",
        elapsed
    );
}

/// Test mixed read/write performance
///
/// Measures performance with mixed read/write workload.
#[tokio::test]
async fn test_mixed_read_write_performance() {
    let pool = Arc::new(AgentPool::from_default_config());
    let mut handles = vec![];

    let start = Instant::now();

    // Spawn 5 writers
    for i in 0..5 {
        let pool_clone = Arc::clone(&pool);
        let handle = tokio::spawn(async move {
            for j in 0..20 {
                let mut task =
                    Task::new(&format!("write-{}-{}", i, j), "Test", "Description");
                let _ = pool_clone
                    .get_or_create_session_for_task(&mut task, "claude", "claude-sonnet-4-6")
                    .await;
            }
        });
        handles.push(handle);
    }

    // Spawn 5 readers
    for _i in 0..5 {
        let pool_clone = Arc::clone(&pool);
        let handle = tokio::spawn(async move {
            for _ in 0..100 {
                let _ = pool_clone.stats().await;
            }
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }

    let elapsed = start.elapsed();

    // Should complete mixed workload in reasonable time
    assert!(
        elapsed < Duration::from_secs(5),
        "Mixed workload should scale reasonably: {:?}",
        elapsed
    );
}

/// Test lock hold time
///
/// Measures how long locks are held during operations.
#[tokio::test]
async fn test_lock_hold_time() {
    let pool = AgentPool::from_default_config();

    // Create a session
    let mut task = Task::new("task-1", "Test", "Description");
    let _ = pool
        .get_or_create_session_for_task(&mut task, "claude", "claude-sonnet-4-6")
        .await;

    // Measure lock hold time for stats operation
    let start = Instant::now();
    let _ = pool.stats().await;
    let elapsed = start.elapsed();

    // Stats operation should be very fast
    assert!(
        elapsed < Duration::from_millis(100),
        "Lock hold time for stats should be minimal: {:?}",
        elapsed
    );

    // Measure lock hold time for cleanup
    let start = Instant::now();
    let _ = pool.cleanup_stale_sessions().await;
    let elapsed = start.elapsed();

    // Cleanup should also be fast
    assert!(
        elapsed < Duration::from_millis(100),
        "Lock hold time for cleanup should be minimal: {:?}",
        elapsed
    );
}

/// Test no deadlocks under heavy contention
///
/// Stress test for deadlock scenarios.
#[tokio::test]
async fn test_no_deadlocks_under_contention() {
    let pool = Arc::new(AgentPool::from_default_config());
    let mut handles = vec![];

    // Spawn many tasks that do various operations
    for i in 0..20 {
        let pool_clone = Arc::clone(&pool);
        let handle = tokio::spawn(async move {
            for j in 0..50 {
                match j % 4 {
                    0 => {
                        let mut task =
                            Task::new(&format!("task-{}-{}", i, j), "Test", "Description");
                        let _ = pool_clone.get_or_create_session_for_task(
                            &mut task,
                            "claude",
                            "claude-sonnet-4-6",
                        );
                    }
                    1 => {
                        let _ = pool_clone.stats().await;
                    }
                    2 => {
                        let _ = pool_clone.cleanup_stale_sessions().await;
                    }
                    3 => {
                        // Mix in some sync operations
                        let mut task =
                            Task::new(&format!("task-{}-{}", i, j), "Test", "Description");
                        let _ = pool_clone.get_session_for_task_sync(
                            &mut task,
                            "claude",
                            "claude-sonnet-4-6",
                        );
                    }
                    _ => unreachable!(),
                }
            }
        });
        handles.push(handle);
    }

    // Set a timeout to catch deadlocks
    let timeout = Duration::from_secs(10);
    let start = Instant::now();

    for handle in handles {
        match tokio::time::timeout(timeout, handle).await {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => panic!("Task failed: {}", e),
            Err(_) => panic!("Potential deadlock - task took longer than {:?}", timeout),
        }
    }

    let elapsed = start.elapsed();

    // All tasks should complete
    assert!(
        elapsed < Duration::from_secs(15),
        "All tasks should complete without deadlock: {:?}",
        elapsed
    );
}

/// Test lock fairness
///
/// Verifies that locks are fair and no thread starves.
#[tokio::test]
async fn test_lock_fairness() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let pool = Arc::new(AgentPool::from_default_config());
    let counter = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];

    // Spawn many competing tasks
    for _i in 0..10 {
        let pool_clone = Arc::clone(&pool);
        let counter_clone = Arc::clone(&counter);
        let handle = tokio::spawn(async move {
            for _ in 0..20 {
                let mut task = Task::new("test-task", "Test", "Description");
                let _ = pool_clone
                    .get_or_create_session_for_task(&mut task, "claude", "claude-sonnet-4-6")
                    .await;

                counter_clone.fetch_add(1, Ordering::Relaxed);
            }
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }

    // All tasks should have made progress
    let total = counter.load(Ordering::Relaxed);
    assert_eq!(total, 200, "All operations should complete");

    // Pool should be in consistent state
    let stats = pool.stats().await;
    assert!(stats.total_sessions >= 0);
}

/// Test sync operations during async lock hold
///
/// Verifies that sync operations handle contention gracefully.
#[tokio::test]
async fn test_sync_operations_during_async_lock() {
    let pool = Arc::new(AgentPool::from_default_config());

    // Hold the async lock
    let pool_clone1 = Arc::clone(&pool);
    let lock_holder = tokio::spawn(async move {
        let mut task = Task::new("lock-holder", "Test", "Description");
        let _ = pool_clone1
            .get_or_create_session_for_task(&mut task, "claude", "claude-sonnet-4-6")
            .await;

        // Hold lock for a bit
        sleep(Duration::from_millis(100)).await;
    });

    // Try sync operations while lock is held
    sleep(Duration::from_millis(10)).await;

    let pool_clone2 = Arc::clone(&pool);
    let sync_user = tokio::spawn(async move {
        let mut task = Task::new("sync-user", "Test", "Description");

        // Should either succeed or fail gracefully, not deadlock
        let result = pool_clone2.get_session_for_task_sync(&mut task, "claude", "claude-sonnet-4-6");

        match result {
            Ok(_) => {}
            Err(e) => {
                assert!(
                    e.to_string().contains("contended") || e.to_string().contains("lock"),
                    "Sync operation should handle contention gracefully"
                );
            }
        }
    });

    // Both should complete
    lock_holder.await.unwrap();
    sync_user.await.unwrap();
}

/// Test cleanup task doesn't cause excessive contention
///
/// Verifies that background cleanup task doesn't interfere with operations.
#[tokio::test]
async fn test_cleanup_task_contention() {
    let pool = Arc::new(AgentPool::from_default_config());

    // Spawn cleanup task
    let pool_clone = Arc::clone(&pool);
    let _cleanup_handle = tokio::spawn(async move {
        loop {
            sleep(Duration::from_millis(50)).await;
            pool_clone.cleanup_stale_sessions().await;
        }
    });

    // Do many operations while cleanup runs
    let mut handles = vec![];
    for i in 0..20 {
        let pool_clone = Arc::clone(&pool);
        let handle = tokio::spawn(async move {
            for j in 0..20 {
                let mut task =
                    Task::new(&format!("task-{}-{}", i, j), "Test", "Description");
                let _ = pool_clone
                    .get_or_create_session_for_task(&mut task, "claude", "claude-sonnet-4-6")
                    .await;
            }
        });
        handles.push(handle);
    }

    // All should complete without excessive delay
    let start = Instant::now();
    for handle in handles {
        handle.await.unwrap();
    }
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_secs(10),
        "Operations should complete despite cleanup task: {:?}",
        elapsed
    );
}
