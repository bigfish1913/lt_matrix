//! Edge case tests for dry-run mode
//!
//! These tests cover edge cases and boundary conditions that may not be
//! covered in the main integration tests.

use ltmatrix::dryrun::{run_dry_run, DryRunConfig};
use ltmatrix::models::{ExecutionMode, Task, TaskComplexity, TaskStatus};
use ltmatrix::pipeline::assess::{assess_tasks, AssessConfig};
use ltmatrix::tasks::scheduler::schedule_tasks;

#[tokio::test]
async fn test_dry_run_with_empty_goal() {
    let goal = "";
    let config = DryRunConfig::default();

    // Should handle empty goal gracefully
    let result = run_dry_run(goal, &config).await;

    // The implementation should still generate placeholder tasks
    // even with an empty goal
    assert!(result.is_ok());
    let result = result.unwrap();
    assert!(!result.tasks.is_empty());
}

#[tokio::test]
async fn test_dry_run_with_very_long_goal() {
    let goal = "implement a very long and complex goal that contains many details and specifications: \
                we need to build a complete microservices architecture with service mesh, \
                implement event sourcing pattern, add caching layer with Redis, \
                create API gateway with rate limiting, implement OAuth2 authentication, \
                add monitoring and observability with Prometheus and Grafana, \
                set up distributed tracing with Jaeger, implement circuit breakers, \
                add retry logic with exponential backoff, create comprehensive logging, \
                set up automated testing pipeline, implement blue-green deployment, \
                add feature flags system, create admin dashboard, implement WebSocket support, \
                add background job processing with Celery, implement message queue with RabbitMQ, \
                create database migration system, add full-text search with Elasticsearch, \
                implement caching strategies, set up CDN, add internationalization, \
                create email notification system, implement webhook handling, \
                add rate limiting per user, implement data archiving, create backup system, \
                set up disaster recovery, implement security auditing, add penetration testing, \
                create compliance reporting, implement GDPR compliance, add data encryption, \
                set up secure key management, implement certificate rotation, create security headers, \
                add CORS configuration, implement CSP policies, create security monitoring, \
                add intrusion detection, implement DDoS protection, create firewall rules, \
                set up network segmentation, implement zero trust architecture, add VPN support, \
                create secure communication channels, implement data loss prevention, \
                add security training, create incident response plan, implement security patches, \
                add vulnerability scanning, create security metrics, implement risk assessment, \
                add threat modeling, create security documentation, implement compliance checks, \
                add security dashboards, create alerting system, implement automated security testing, \
                add security policies, create access controls, implement privilege management, \
                add session management, create password policies, implement multi-factor authentication, \
                add biometric authentication, create token management, implement session fixation protection, \
                add CSRF protection, create XSS prevention, implement SQL injection prevention, \
                add command injection prevention, create path traversal prevention, implement secure file uploads, \
                add secure file storage, implement secure communications, add secure logging, \
                create secure error handling, implement secure configuration, add secure deployment, \
                create secure operations, implement secure maintenance, add secure updates, \
                create secure backups, implement secure recovery, add secure monitoring, \
                create secure incident response, implement secure forensics, add secure remediation";

    let config = DryRunConfig::default();

    // Should handle very long goals without issues
    let result = run_dry_run(goal, &config).await;

    assert!(result.is_ok());
    let result = result.unwrap();
    assert!(!result.tasks.is_empty());
}

#[tokio::test]
async fn test_dry_run_with_special_characters() {
    let goal = "Implement features: authentication 🔐, database 🗄️, API 🌐, and testing 🧪";
    let config = DryRunConfig::default();

    let result = run_dry_run(goal, &config).await;

    assert!(result.is_ok());
    let result = result.unwrap();
    assert!(!result.tasks.is_empty());
}

#[tokio::test]
async fn test_dry_run_preserves_task_structure() {
    let goal = "build a web application";

    // Create a custom task with specific structure
    let mut tasks = vec![
        {
            let mut t = Task::new("task-1", "First Task", "Description 1");
            t.complexity = TaskComplexity::Simple;
            t.status = TaskStatus::Pending;
            t
        },
        {
            let mut t = Task::new("task-2", "Second Task", "Description 2");
            t.complexity = TaskComplexity::Moderate;
            t.depends_on = vec!["task-1".to_string()];
            t.status = TaskStatus::Pending;
            t
        },
    ];

    // Assess tasks
    let config = AssessConfig::default();
    let assessed_tasks = assess_tasks(tasks.clone(), &config).await.unwrap();

    // Verify structure is preserved
    assert_eq!(assessed_tasks.len(), 2);
    assert_eq!(assessed_tasks[0].id, "task-1");
    assert_eq!(assessed_tasks[1].id, "task-2");
    assert_eq!(assessed_tasks[1].depends_on, vec!["task-1".to_string()]);
}

#[tokio::test]
async fn test_dry_run_statistics_are_consistent() {
    let goal = "implement a feature with multiple components";
    let config = DryRunConfig::default();

    let result = run_dry_run(goal, &config).await.unwrap();

    // Verify statistics are internally consistent
    assert_eq!(
        result.statistics.total_tasks,
        result.statistics.simple_tasks +
        result.statistics.moderate_tasks +
        result.statistics.complex_tasks
    );

    // Verify execution depth matches plan
    assert_eq!(
        result.statistics.execution_depth,
        result.execution_plan.max_depth
    );

    // Verify critical path length
    assert_eq!(
        result.statistics.critical_path_length,
        result.execution_plan.critical_path.len()
    );

    // Verify total tasks in plan matches result
    assert_eq!(
        result.execution_plan.total_tasks,
        result.tasks.len()
    );
}

#[tokio::test]
async fn test_dry_run_with_fast_execution_mode() {
    let goal = "implement a simple feature";
    let config = DryRunConfig {
        execution_mode: ExecutionMode::Fast,
        ..Default::default()
    };

    let result = run_dry_run(goal, &config).await.unwrap();

    // Should still work with fast mode
    assert!(!result.tasks.is_empty());
    assert!(!result.execution_plan.execution_order.is_empty());
}

#[tokio::test]
async fn test_dry_run_with_expert_execution_mode() {
    let goal = "implement a complex distributed system";
    let config = DryRunConfig {
        execution_mode: ExecutionMode::Expert,
        ..Default::default()
    };

    let result = run_dry_run(goal, &config).await.unwrap();

    // Should work with expert mode
    assert!(!result.tasks.is_empty());
    assert!(!result.execution_plan.execution_order.is_empty());
}

#[tokio::test]
async fn test_dry_run_execution_levels_have_no_internal_dependencies() {
    let goal = "build a system with multiple parallel tracks";

    let result = run_dry_run(goal, &DryRunConfig::default()).await.unwrap();

    // For each execution level, verify no task depends on another in the same level
    for (level_idx, level) in result.execution_plan.execution_levels.iter().enumerate() {
        for (i, task_i) in level.iter().enumerate() {
            for (j, task_j) in level.iter().enumerate() {
                if i != j {
                    assert!(
                        !task_i.depends_on.contains(&task_j.id),
                        "Task {} in level {} should not depend on task {} in the same level",
                        task_i.id,
                        level_idx + 1,
                        task_j.id
                    );
                }
            }
        }
    }
}

#[tokio::test]
async fn test_dry_run_critical_path_is_valid_sequence() {
    let goal = "build a complex system with dependencies";

    let result = run_dry_run(goal, &DryRunConfig::default()).await.unwrap();

    // Verify critical path is a valid sequence (each task exists)
    for task_id in &result.execution_plan.critical_path {
        assert!(
            result.tasks.iter().any(|t| &t.id == task_id),
            "Critical path task {} should exist in task list",
            task_id
        );
    }

    // Verify critical path is in execution order
    for (i, task_id) in result.execution_plan.critical_path.iter().enumerate() {
        if let Some(pos) = result.execution_plan.execution_order.iter().position(|id| id == task_id) {
            // All tasks before this in critical path should come before in execution order
            for prev_task_id in &result.execution_plan.critical_path[..i] {
                if let Some(prev_pos) = result.execution_plan.execution_order.iter().position(|id| id == prev_task_id) {
                    assert!(
                        prev_pos < pos,
                        "Previous critical path task {} should come before {} in execution order",
                        prev_task_id,
                        task_id
                    );
                }
            }
        }
    }
}

#[tokio::test]
async fn test_dry_run_parallelizable_tasks_are_not_on_critical_path() {
    let goal = "build a system with parallelizable components";

    let result = run_dry_run(goal, &DryRunConfig::default()).await.unwrap();

    // Create a set of critical path tasks
    let critical_path_set: std::collections::HashSet<&String> =
        result.execution_plan.critical_path.iter().collect();

    // Verify parallelizable tasks are not on critical path
    for task_id in &result.execution_plan.parallelizable_tasks {
        assert!(
            !critical_path_set.contains(task_id),
            "Parallelizable task {} should not be on critical path",
            task_id
        );
    }
}

#[tokio::test]
async fn test_dry_run_with_unicode_goal() {
    let goal = "实现一个完整的应用程序，包含用户认证、数据库集成和API开发";
    let config = DryRunConfig::default();

    let result = run_dry_run(goal, &config).await;

    // Should handle Unicode characters
    assert!(result.is_ok());
    let result = result.unwrap();
    assert!(!result.tasks.is_empty());
}

#[tokio::test]
async fn test_dry_run_with_newlines_in_goal() {
    let goal = "Implement feature A\nThen implement feature B\nFinally add tests";
    let config = DryRunConfig::default();

    let result = run_dry_run(goal, &config).await;

    // Should handle newlines
    assert!(result.is_ok());
    let result = result.unwrap();
    assert!(!result.tasks.is_empty());
}

#[tokio::test]
async fn test_dry_run_config_variations() {
    let goal = "test goal";

    // Test with json_output = true
    let config = DryRunConfig {
        json_output: true,
        ..Default::default()
    };
    let result = run_dry_run(goal, &config).await;
    assert!(result.is_ok());

    // Test with different execution modes
    for mode in [ExecutionMode::Fast, ExecutionMode::Standard, ExecutionMode::Expert] {
        let config = DryRunConfig {
            execution_mode: mode.clone(),
            ..Default::default()
        };
        let result = run_dry_run(goal, &config).await;
        assert!(result.is_ok(), "Should work with {:?} mode", mode);
    }
}
