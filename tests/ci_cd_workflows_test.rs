// Tests for CI/CD GitHub Actions workflows
//
// These tests verify that the CI/CD setup meets the acceptance criteria:
// - CI runs on all PRs and pushes to main
// - Includes lint (clippy), fmt check, unit tests, integration tests
// - Cross-platform builds for Linux/macOS/Windows
// - Security audit with cargo-audit
// - Separate release workflow
//
// Optimized for fast execution with shared workflow loading.

use std::fs;
use std::path::Path;

/// Parse YAML workflow file into a generic structure
fn parse_workflow(path: &Path) -> serde_yaml::Value {
    let content = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to read workflow file {:?}: {}", path, e));
    serde_yaml::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse YAML in {:?}: {}", path, e))
}

/// Get workflow file path
fn workflow_path(filename: &str) -> std::path::PathBuf {
    Path::new(".github/workflows").join(filename)
}

/// Load both workflows once and share across tests
struct Workflows {
    ci: serde_yaml::Value,
    release: serde_yaml::Value,
}

impl Workflows {
    fn load() -> Self {
        let ci_path = workflow_path("ci.yml");
        let release_path = workflow_path("release.yml");

        assert!(
            ci_path.exists(),
            "CI workflow file should exist at .github/workflows/ci.yml"
        );
        assert!(
            release_path.exists(),
            "Release workflow file should exist at .github/workflows/release.yml"
        );

        Self {
            ci: parse_workflow(&ci_path),
            release: parse_workflow(&release_path),
        }
    }
}

// ============================================================================
// CI Workflow Tests
// ============================================================================

mod ci_workflow_tests {
    use super::*;

    /// Test all CI workflow requirements in a single test to reduce overhead
    #[test]
    fn ci_workflow_comprehensive() {
        let workflows = Workflows::load();
        let workflow = &workflows.ci;

        // Basic structure
        assert!(workflow.is_mapping(), "CI workflow should be a valid YAML mapping");
        assert_eq!(
            workflow.get("name").and_then(|v| v.as_str()),
            Some("CI"),
            "CI workflow name should be 'CI'"
        );

        // Triggers
        let on = workflow.get("on").expect("Workflow should have 'on' trigger");

        // Push trigger
        let push = on.get("push").expect("Workflow should have 'push' trigger");
        let push_branches = push.get("branches").expect("Push trigger should have 'branches'");
        let push_branches_list: Vec<&str> = push_branches
            .as_sequence()
            .expect("Branches should be a sequence")
            .iter()
            .filter_map(|v| v.as_str())
            .collect();
        assert!(
            push_branches_list.contains(&"main"),
            "CI should trigger on push to main branch"
        );

        // Pull request trigger
        let pr = on.get("pull_request").expect("Workflow should have 'pull_request' trigger");
        let pr_branches = pr.get("branches").expect("PR trigger should have 'branches'");
        let pr_branches_list: Vec<&str> = pr_branches
            .as_sequence()
            .expect("Branches should be a sequence")
            .iter()
            .filter_map(|v| v.as_str())
            .collect();
        assert!(
            pr_branches_list.contains(&"main"),
            "CI should trigger on PRs to main branch"
        );

        // Jobs
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");
        let job_names: Vec<&str> = jobs
            .as_mapping()
            .unwrap()
            .keys()
            .filter_map(|k| k.as_str())
            .collect();

        // Required jobs
        let required_jobs = ["fmt", "clippy", "test-unit", "test-integration", "audit"];
        for required in required_jobs {
            assert!(
                job_names.iter().any(|j| j.contains(required)),
                "CI workflow should have '{}' job",
                required
            );
        }

        // Cross-platform builds
        assert!(
            job_names.iter().any(|j| j.contains("linux")),
            "CI workflow should have Linux build job"
        );
        assert!(
            job_names.iter().any(|j| j.contains("macos")),
            "CI workflow should have macOS build job"
        );
        assert!(
            job_names.iter().any(|j| j.contains("windows")),
            "CI workflow should have Windows build job"
        );
    }

    #[test]
    fn ci_fmt_job_valid() {
        let workflows = Workflows::load();
        let jobs = workflows.ci.get("jobs").expect("Workflow should have 'jobs'");
        let fmt_job = jobs.get("fmt").expect("Should have 'fmt' job");
        let steps = fmt_job.get("steps").expect("fmt job should have steps");

        // Check for fmt check command
        let has_fmt_check = steps.as_sequence().unwrap().iter().any(|step| {
            step.get("run")
                .and_then(|r| r.as_str())
                .map(|r| r.contains("cargo fmt") && r.contains("--check"))
                .unwrap_or(false)
        });

        assert!(has_fmt_check, "fmt job should run 'cargo fmt --check'");
    }

    #[test]
    fn ci_clippy_job_valid() {
        let workflows = Workflows::load();
        let jobs = workflows.ci.get("jobs").expect("Workflow should have 'jobs'");
        let clippy_job = jobs.get("clippy").expect("Should have 'clippy' job");
        let steps = clippy_job.get("steps").expect("clippy job should have steps");

        // Check for clippy command with proper flags
        let has_clippy = steps.as_sequence().unwrap().iter().any(|step| {
            let run = step.get("run").and_then(|r| r.as_str()).unwrap_or("");
            run.contains("cargo clippy") && run.contains("--all-targets") && run.contains("-D warnings")
        });

        assert!(has_clippy, "clippy job should run 'cargo clippy --all-targets -- -D warnings'");
    }

    #[test]
    fn ci_tests_jobs_valid() {
        let workflows = Workflows::load();
        let jobs = workflows.ci.get("jobs").expect("Workflow should have 'jobs'");

        // Unit tests
        let unit_job = jobs.get("test-unit").expect("Should have 'test-unit' job");
        let unit_steps = unit_job.get("steps").expect("test-unit job should have steps");
        let has_unit_test = unit_steps.as_sequence().unwrap().iter().any(|step| {
            step.get("run")
                .and_then(|r| r.as_str())
                .map(|r| r.contains("cargo test") && r.contains("--lib"))
                .unwrap_or(false)
        });
        assert!(has_unit_test, "unit tests job should run 'cargo test --lib'");

        // Integration tests
        let int_job = jobs.get("test-integration").expect("Should have 'test-integration' job");
        let int_steps = int_job.get("steps").expect("test-integration job should have steps");
        let has_int_test = int_steps.as_sequence().unwrap().iter().any(|step| {
            let run = step.get("run").and_then(|r| r.as_str()).unwrap_or("");
            run.contains("cargo test") && run.contains("--test")
        });
        assert!(has_int_test, "integration tests job should run 'cargo test --test'");
    }

    #[test]
    fn ci_security_audit_valid() {
        let workflows = Workflows::load();
        let jobs = workflows.ci.get("jobs").expect("Workflow should have 'jobs'");
        let audit_job = jobs.get("audit").expect("Should have 'audit' job");
        let steps = audit_job.get("steps").expect("audit job should have steps");

        // Check for cargo-audit
        let has_audit = steps.as_sequence().unwrap().iter().any(|step| {
            let uses = step.get("uses").and_then(|u| u.as_str()).unwrap_or("");
            let run = step.get("run").and_then(|r| r.as_str()).unwrap_or("");
            uses.contains("cargo-audit") || run.contains("cargo audit")
        });

        assert!(has_audit, "security audit job should use cargo-audit");
    }

    #[test]
    fn ci_cross_platform_builds_valid() {
        let workflows = Workflows::load();
        let jobs = workflows.ci.get("jobs").expect("Workflow should have 'jobs'");

        // Linux build
        let linux_job = jobs.as_mapping().unwrap().iter()
            .find(|(k, _)| k.as_str().map(|s| s.contains("linux")).unwrap_or(false))
            .map(|(_, v)| v);
        let linux_runner = linux_job.and_then(|j| j.get("runs-on").and_then(|r| r.as_str()));
        assert!(
            linux_runner.map(|r| r.contains("ubuntu")).unwrap_or(false),
            "Linux build should run on ubuntu runner"
        );

        // macOS build
        let macos_job = jobs.as_mapping().unwrap().iter()
            .find(|(k, _)| k.as_str().map(|s| s.contains("macos")).unwrap_or(false))
            .map(|(_, v)| v);
        let macos_runner = macos_job.and_then(|j| j.get("runs-on").and_then(|r| r.as_str()));
        assert!(
            macos_runner.map(|r| r.contains("macos")).unwrap_or(false),
            "macOS build should run on macos runner"
        );

        // Windows build
        let windows_job = jobs.as_mapping().unwrap().iter()
            .find(|(k, _)| k.as_str().map(|s| s.contains("windows")).unwrap_or(false))
            .map(|(_, v)| v);
        let windows_runner = windows_job.and_then(|j| j.get("runs-on").and_then(|r| r.as_str()));
        assert!(
            windows_runner.map(|r| r.contains("windows")).unwrap_or(false),
            "Windows build should run on windows runner"
        );
    }

    #[test]
    fn ci_uses_modern_actions() {
        let workflows = Workflows::load();
        let jobs = workflows.ci.get("jobs").expect("Workflow should have 'jobs'");

        for (_, job) in jobs.as_mapping().unwrap() {
            if let Some(steps) = job.get("steps") {
                for step in steps.as_sequence().unwrap() {
                    if let Some(uses) = step.get("uses").and_then(|u| u.as_str()) {
                        if uses.starts_with("actions/checkout@") {
                            assert!(
                                uses.contains("@v4") || uses.contains("@main") || uses.contains("@master"),
                                "actions/checkout should use v4 or latest, found: {}",
                                uses
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn ci_has_caching() {
        let workflows = Workflows::load();
        let jobs = workflows.ci.get("jobs").expect("Workflow should have 'jobs'");

        let jobs_with_cache: usize = jobs.as_mapping().unwrap().values()
            .filter(|job| {
                job.get("steps")
                    .and_then(|steps| steps.as_sequence())
                    .map(|steps| steps.iter().any(|step| {
                        step.get("uses")
                            .and_then(|u| u.as_str())
                            .map(|u| u.contains("cache"))
                            .unwrap_or(false)
                    }))
                    .unwrap_or(false)
            })
            .count();

        let total_jobs = jobs.as_mapping().unwrap().len();

        assert!(
            jobs_with_cache >= total_jobs / 2,
            "At least half of CI jobs should have caching for efficiency"
        );
    }
}

// ============================================================================
// Release Workflow Tests
// ============================================================================

mod release_workflow_tests {
    use super::*;

    #[test]
    fn release_workflow_comprehensive() {
        let workflows = Workflows::load();
        let workflow = &workflows.release;

        // Basic structure
        assert!(workflow.is_mapping(), "Release workflow should be a valid YAML mapping");
        assert_eq!(
            workflow.get("name").and_then(|v| v.as_str()),
            Some("Release"),
            "Release workflow name should be 'Release'"
        );

        // Triggers on version tags
        let on = workflow.get("on").expect("Workflow should have 'on' trigger");
        let push = on.get("push").expect("Release should trigger on push");
        let tags = push.get("tags").expect("Release should trigger on tags");
        let tags_list: Vec<&str> = tags
            .as_sequence()
            .expect("Tags should be a sequence")
            .iter()
            .filter_map(|v| v.as_str())
            .collect();
        assert!(
            tags_list.iter().any(|t| t.starts_with('v')),
            "Release workflow should trigger on version tags (v*.*.*)"
        );

        // Jobs
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");
        let job_names: Vec<&str> = jobs
            .as_mapping()
            .unwrap()
            .keys()
            .filter_map(|k| k.as_str())
            .collect();

        // Required jobs
        assert!(
            job_names.iter().any(|j| j.contains("release") && j.contains("create")),
            "Release workflow should have create-release job"
        );
        assert!(
            job_names.iter().any(|j| j.contains("linux")),
            "Release workflow should have Linux build job"
        );
        assert!(
            job_names.iter().any(|j| j.contains("macos")),
            "Release workflow should have macOS build job"
        );
        assert!(
            job_names.iter().any(|j| j.contains("windows")),
            "Release workflow should have Windows build job"
        );
    }

    #[test]
    fn release_uses_gh_release_action() {
        let workflows = Workflows::load();
        let jobs = workflows.release.get("jobs").expect("Workflow should have 'jobs'");

        let mut found_release_action = false;
        for (_, job) in jobs.as_mapping().unwrap() {
            if let Some(steps) = job.get("steps").and_then(|s| s.as_sequence()) {
                for step in steps {
                    if let Some(uses) = step.get("uses").and_then(|u| u.as_str()) {
                        if uses.contains("release") {
                            found_release_action = true;
                        }
                    }
                }
            }
        }

        assert!(
            found_release_action,
            "Release workflow should use a GitHub release action"
        );
    }

    #[test]
    fn release_builds_upload_artifacts() {
        let workflows = Workflows::load();
        let jobs = workflows.release.get("jobs").expect("Workflow should have 'jobs'");

        let build_jobs: Vec<_> = jobs.as_mapping().unwrap().iter()
            .filter(|(k, _)| k.as_str().map(|s| s.contains("build")).unwrap_or(false))
            .collect();

        for (job_name, job) in build_jobs {
            let has_upload = job.get("steps")
                .and_then(|steps| steps.as_sequence())
                .map(|steps| steps.iter().any(|step| {
                    let uses = step.get("uses").and_then(|u| u.as_str()).unwrap_or("");
                    uses.contains("upload") || uses.contains("release")
                }))
                .unwrap_or(false);

            assert!(
                has_upload,
                "Build job '{}' should upload release artifacts",
                job_name.as_str().unwrap_or("unknown")
            );
        }
    }

    #[test]
    fn release_has_caching() {
        let workflows = Workflows::load();
        let jobs = workflows.release.get("jobs").expect("Workflow should have 'jobs'");

        let build_jobs_with_cache: usize = jobs.as_mapping().unwrap().iter()
            .filter(|(k, _)| k.as_str().map(|s| s.contains("build")).unwrap_or(false))
            .filter(|(_, job)| {
                job.get("steps")
                    .and_then(|steps| steps.as_sequence())
                    .map(|steps| steps.iter().any(|step| {
                        step.get("uses")
                            .and_then(|u| u.as_str())
                            .map(|u| u.contains("cache"))
                            .unwrap_or(false)
                    }))
                    .unwrap_or(false)
            })
            .count();

        let total_build_jobs = jobs.as_mapping().unwrap().keys()
            .filter(|k| k.as_str().map(|s| s.contains("build")).unwrap_or(false))
            .count();

        assert!(
            build_jobs_with_cache >= total_build_jobs / 2 || total_build_jobs == 0,
            "At least half of release build jobs should have caching"
        );
    }
}

// ============================================================================
// Best Practices Tests
// ============================================================================

mod best_practices_tests {
    use super::*;

    #[test]
    fn workflows_use_dtolnay_rust_toolchain() {
        let workflows = Workflows::load();

        fn check_toolchain(workflow: &serde_yaml::Value) -> bool {
            workflow.get("jobs")
                .and_then(|jobs| jobs.as_mapping())
                .map(|jobs| jobs.values().any(|job| {
                    job.get("steps")
                        .and_then(|steps| steps.as_sequence())
                        .map(|steps| steps.iter().any(|step| {
                            step.get("uses")
                                .and_then(|u| u.as_str())
                                .map(|u| u.contains("dtolnay/rust-toolchain"))
                                .unwrap_or(false)
                        }))
                        .unwrap_or(false)
                }))
                .unwrap_or(false)
        }

        assert!(
            check_toolchain(&workflows.ci) && check_toolchain(&workflows.release),
            "Both workflows should use dtolnay/rust-toolchain action"
        );
    }

    #[test]
    fn ci_status_aggregation() {
        let workflows = Workflows::load();
        let jobs = workflows.ci.get("jobs").expect("Workflow should have 'jobs'");

        // Check for status aggregation job
        let status_job = jobs.as_mapping().unwrap().iter()
            .find(|(k, _)| {
                k.as_str()
                    .map(|s| s.contains("status") || s.contains("summary"))
                    .unwrap_or(false)
            });

        assert!(
            status_job.is_some(),
            "CI workflow should have status aggregation job"
        );

        // Check it depends on other jobs
        if let Some((_, job)) = status_job {
            let needs = job.get("needs");
            assert!(
                needs.is_some(),
                "Status job should have dependencies"
            );
        }
    }
}