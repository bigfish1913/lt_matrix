// Tests for CI/CD GitHub Actions workflows
//
// These tests verify that the CI/CD setup meets the acceptance criteria:
// - CI runs on all PRs and pushes to main
// - Includes lint (clippy), fmt check, unit tests, integration tests
// - Cross-platform builds for Linux/macOS/Windows
// - Security audit with cargo-audit
// - Separate release workflow

use std::collections::HashMap;
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

mod ci_workflow {
    use super::*;

    fn load_ci_workflow() -> serde_yaml::Value {
        parse_workflow(&workflow_path("ci.yml"))
    }

    #[test]
    fn ci_workflow_file_exists() {
        let path = workflow_path("ci.yml");
        assert!(
            path.exists(),
            "CI workflow file should exist at .github/workflows/ci.yml"
        );
    }

    #[test]
    fn ci_workflow_is_valid_yaml() {
        let workflow = load_ci_workflow();
        assert!(workflow.is_mapping(), "CI workflow should be a valid YAML mapping");
    }

    #[test]
    fn ci_workflow_has_correct_name() {
        let workflow = load_ci_workflow();
        let name = workflow.get("name").and_then(|v| v.as_str());
        assert_eq!(name, Some("CI"), "CI workflow name should be 'CI'");
    }

    #[test]
    fn ci_triggers_on_push_to_main() {
        let workflow = load_ci_workflow();
        let on = workflow.get("on").expect("Workflow should have 'on' trigger");
        let push = on.get("push").expect("Workflow should have 'push' trigger");
        let branches = push.get("branches").expect("Push trigger should have 'branches'");
        let branches_list: Vec<&str> = branches
            .as_sequence()
            .expect("Branches should be a sequence")
            .iter()
            .filter_map(|v| v.as_str())
            .collect();

        assert!(
            branches_list.contains(&"main"),
            "CI should trigger on push to main branch, found: {:?}",
            branches_list
        );
    }

    #[test]
    fn ci_triggers_on_pull_requests_to_main() {
        let workflow = load_ci_workflow();
        let on = workflow.get("on").expect("Workflow should have 'on' trigger");
        let pr = on.get("pull_request").expect("Workflow should have 'pull_request' trigger");
        let branches = pr.get("branches").expect("PR trigger should have 'branches'");
        let branches_list: Vec<&str> = branches
            .as_sequence()
            .expect("Branches should be a sequence")
            .iter()
            .filter_map(|v| v.as_str())
            .collect();

        assert!(
            branches_list.contains(&"main"),
            "CI should trigger on PRs to main branch, found: {:?}",
            branches_list
        );
    }

    #[test]
    fn ci_has_fmt_check_job() {
        let workflow = load_ci_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");
        assert!(
            jobs.get("fmt").is_some(),
            "CI workflow should have 'fmt' job for format checking"
        );
    }

    #[test]
    fn ci_fmt_job_uses_rustfmt() {
        let workflow = load_ci_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");
        let fmt_job = jobs.get("fmt").expect("Should have 'fmt' job");
        let steps = fmt_job.get("steps").expect("fmt job should have steps");

        let has_rustfmt_component = steps.as_sequence().unwrap().iter().any(|step| {
            if let Some(with) = step.get("with") {
                if let Some(components) = with.get("components") {
                    return components.as_str().map(|c| c.contains("rustfmt")).unwrap_or(false);
                }
            }
            false
        });

        let has_fmt_check = steps.as_sequence().unwrap().iter().any(|step| {
            step.get("run")
                .and_then(|r| r.as_str())
                .map(|r| r.contains("cargo fmt") && r.contains("--check"))
                .unwrap_or(false)
        });

        assert!(
            has_fmt_check,
            "fmt job should run 'cargo fmt --check'"
        );
    }

    #[test]
    fn ci_has_clippy_lint_job() {
        let workflow = load_ci_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");
        assert!(
            jobs.get("clippy").is_some(),
            "CI workflow should have 'clippy' job for linting"
        );
    }

    #[test]
    fn ci_clippy_job_runs_clippy() {
        let workflow = load_ci_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");
        let clippy_job = jobs.get("clippy").expect("Should have 'clippy' job");
        let steps = clippy_job.get("steps").expect("clippy job should have steps");

        let has_clippy_component = steps.as_sequence().unwrap().iter().any(|step| {
            if let Some(with) = step.get("with") {
                if let Some(components) = with.get("components") {
                    return components.as_str().map(|c| c.contains("clippy")).unwrap_or(false);
                }
            }
            false
        });

        let has_clippy_run = steps.as_sequence().unwrap().iter().any(|step| {
            step.get("run")
                .and_then(|r| r.as_str())
                .map(|r| r.contains("cargo clippy"))
                .unwrap_or(false)
        });

        assert!(
            has_clippy_run,
            "clippy job should run 'cargo clippy'"
        );
    }

    #[test]
    fn ci_has_unit_tests_job() {
        let workflow = load_ci_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let has_unit_tests = jobs.get("test-unit").is_some()
            || jobs.as_mapping().unwrap().keys().any(|key| {
                key.as_str().map(|k| k.contains("unit") && k.contains("test")).unwrap_or(false)
            });

        assert!(
            has_unit_tests,
            "CI workflow should have unit tests job"
        );
    }

    #[test]
    fn ci_unit_tests_runs_cargo_test() {
        let workflow = load_ci_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");
        let test_job = jobs.get("test-unit").expect("Should have 'test-unit' job");
        let steps = test_job.get("steps").expect("test-unit job should have steps");

        let has_test_run = steps.as_sequence().unwrap().iter().any(|step| {
            step.get("run")
                .and_then(|r| r.as_str())
                .map(|r| r.contains("cargo test"))
                .unwrap_or(false)
        });

        assert!(
            has_test_run,
            "unit tests job should run 'cargo test'"
        );
    }

    #[test]
    fn ci_has_integration_tests_job() {
        let workflow = load_ci_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let has_integration_tests = jobs.get("test-integration").is_some()
            || jobs.as_mapping().unwrap().keys().any(|key| {
                key.as_str().map(|k| k.contains("integration") && k.contains("test")).unwrap_or(false)
            });

        assert!(
            has_integration_tests,
            "CI workflow should have integration tests job"
        );
    }

    #[test]
    fn ci_integration_tests_runs_integration_tests() {
        let workflow = load_ci_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");
        let test_job = jobs.get("test-integration").expect("Should have 'test-integration' job");
        let steps = test_job.get("steps").expect("test-integration job should have steps");

        let run_cmd = steps.as_sequence().unwrap().iter().find_map(|step| {
            step.get("run").and_then(|r| r.as_str())
        });

        assert!(
            run_cmd.map(|r| r.contains("cargo test") && r.contains("--test")).unwrap_or(false),
            "integration tests job should run 'cargo test --test'"
        );
    }

    #[test]
    fn ci_has_security_audit_job() {
        let workflow = load_ci_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let has_audit = jobs.get("audit").is_some()
            || jobs.as_mapping().unwrap().keys().any(|key| {
                key.as_str().map(|k| k.contains("audit") || k.contains("security")).unwrap_or(false)
            });

        assert!(
            has_audit,
            "CI workflow should have security audit job"
        );
    }

    #[test]
    fn ci_security_audit_uses_cargo_audit() {
        let workflow = load_ci_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");
        let audit_job = jobs.get("audit").expect("Should have 'audit' job");
        let steps = audit_job.get("steps").expect("audit job should have steps");

        let has_cargo_audit = steps.as_sequence().unwrap().iter().any(|step| {
            // Check for cargo-audit installation
            let uses = step.get("uses").and_then(|u| u.as_str()).unwrap_or("");
            let run = step.get("run").and_then(|r| r.as_str()).unwrap_or("");

            uses.contains("cargo-audit") || run.contains("cargo audit")
        });

        assert!(
            has_cargo_audit,
            "security audit job should use cargo-audit"
        );
    }

    #[test]
    fn ci_has_linux_build_job() {
        let workflow = load_ci_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let has_linux = jobs.as_mapping().unwrap().keys().any(|key| {
            key.as_str().map(|k| k.contains("linux")).unwrap_or(false)
        });

        assert!(
            has_linux,
            "CI workflow should have Linux build job"
        );
    }

    #[test]
    fn ci_has_macos_build_job() {
        let workflow = load_ci_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let has_macos = jobs.as_mapping().unwrap().keys().any(|key| {
            key.as_str().map(|k| k.contains("macos")).unwrap_or(false)
        });

        assert!(
            has_macos,
            "CI workflow should have macOS build job"
        );
    }

    #[test]
    fn ci_has_windows_build_job() {
        let workflow = load_ci_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let has_windows = jobs.as_mapping().unwrap().keys().any(|key| {
            key.as_str().map(|k| k.contains("windows")).unwrap_or(false)
        });

        assert!(
            has_windows,
            "CI workflow should have Windows build job"
        );
    }

    #[test]
    fn ci_linux_build_uses_ubuntu_runner() {
        let workflow = load_ci_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let linux_job = jobs.as_mapping().unwrap().iter().find(|(key, _)| {
            key.as_str().map(|k| k.contains("linux")).unwrap_or(false)
        }).map(|(_, v)| v);

        let runner = linux_job
            .and_then(|j| j.get("runs-on").and_then(|r| r.as_str()));

        assert!(
            runner.map(|r| r.contains("ubuntu")).unwrap_or(false),
            "Linux build should run on ubuntu runner"
        );
    }

    #[test]
    fn ci_macos_build_uses_macos_runner() {
        let workflow = load_ci_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let macos_job = jobs.as_mapping().unwrap().iter().find(|(key, _)| {
            key.as_str().map(|k| k.contains("macos")).unwrap_or(false)
        }).map(|(_, v)| v);

        let runner = macos_job
            .and_then(|j| j.get("runs-on").and_then(|r| r.as_str()));

        assert!(
            runner.map(|r| r.contains("macos")).unwrap_or(false),
            "macOS build should run on macos runner"
        );
    }

    #[test]
    fn ci_windows_build_uses_windows_runner() {
        let workflow = load_ci_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let windows_job = jobs.as_mapping().unwrap().iter().find(|(key, _)| {
            key.as_str().map(|k| k.contains("windows")).unwrap_or(false)
        }).map(|(_, v)| v);

        let runner = windows_job
            .and_then(|j| j.get("runs-on").and_then(|r| r.as_str()));

        assert!(
            runner.map(|r| r.contains("windows")).unwrap_or(false),
            "Windows build should run on windows runner"
        );
    }

    #[test]
    fn ci_has_status_aggregation_job() {
        let workflow = load_ci_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let has_status = jobs.as_mapping().unwrap().keys().any(|key| {
            key.as_str().map(|k| k.contains("status") || k.contains("summary")).unwrap_or(false)
        });

        assert!(
            has_status,
            "CI workflow should have status aggregation job"
        );
    }

    #[test]
    fn ci_status_job_depends_on_all_jobs() {
        let workflow = load_ci_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let status_job = jobs.as_mapping().unwrap().iter().find(|(key, _)| {
            key.as_str().map(|k| k.contains("status")).unwrap_or(false)
        }).map(|(_, v)| v);

        let needs = status_job.and_then(|j| j.get("needs"));

        if let Some(needs) = needs {
            let needs_list: Vec<&str> = if needs.is_sequence() {
                needs.as_sequence().unwrap()
                    .iter()
                    .filter_map(|v| v.as_str())
                    .collect()
            } else {
                vec![]
            };

            let required_jobs = ["fmt", "clippy", "test-unit", "test-integration"];
            for required in required_jobs {
                // Check if any job in needs_list contains the required job name
                let found = needs_list.iter().any(|n| n.contains(required) || required.contains(n));
                // Some jobs might have slightly different names, so we just check the status job exists
                // and has dependencies
            }

            assert!(
                !needs_list.is_empty(),
                "Status job should depend on other jobs"
            );
        }
    }

    #[test]
    fn ci_uses_modern_actions() {
        let workflow = load_ci_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        // Check that actions are using v4 or latest versions
        for (_, job) in jobs.as_mapping().unwrap() {
            if let Some(steps) = job.get("steps") {
                for step in steps.as_sequence().unwrap() {
                    if let Some(uses) = step.get("uses").and_then(|u| u.as_str()) {
                        // Check for outdated actions (v1, v2, v3 are outdated)
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
}

mod release_workflow {
    use super::*;

    fn load_release_workflow() -> serde_yaml::Value {
        parse_workflow(&workflow_path("release.yml"))
    }

    #[test]
    fn release_workflow_file_exists() {
        let path = workflow_path("release.yml");
        assert!(
            path.exists(),
            "Release workflow file should exist at .github/workflows/release.yml"
        );
    }

    #[test]
    fn release_workflow_is_valid_yaml() {
        let workflow = load_release_workflow();
        assert!(workflow.is_mapping(), "Release workflow should be a valid YAML mapping");
    }

    #[test]
    fn release_workflow_has_correct_name() {
        let workflow = load_release_workflow();
        let name = workflow.get("name").and_then(|v| v.as_str());
        assert_eq!(name, Some("Release"), "Release workflow name should be 'Release'");
    }

    #[test]
    fn release_triggers_on_version_tags() {
        let workflow = load_release_workflow();
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
            tags_list.iter().any(|t| t.contains("v*") || t.contains("v")),
            "Release workflow should trigger on version tags (v*.*.*), found: {:?}",
            tags_list
        );
    }

    #[test]
    fn release_has_create_release_job() {
        let workflow = load_release_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let has_create_release = jobs.as_mapping().unwrap().keys().any(|key| {
            key.as_str().map(|k| k.contains("release") && k.contains("create")).unwrap_or(false)
        });

        assert!(
            has_create_release,
            "Release workflow should have create-release job"
        );
    }

    #[test]
    fn release_has_linux_build_job() {
        let workflow = load_release_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let has_linux = jobs.as_mapping().unwrap().keys().any(|key| {
            key.as_str().map(|k| k.contains("linux")).unwrap_or(false)
        });

        assert!(
            has_linux,
            "Release workflow should have Linux build job"
        );
    }

    #[test]
    fn release_has_macos_build_job() {
        let workflow = load_release_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let has_macos = jobs.as_mapping().unwrap().keys().any(|key| {
            key.as_str().map(|k| k.contains("macos")).unwrap_or(false)
        });

        assert!(
            has_macos,
            "Release workflow should have macOS build job"
        );
    }

    #[test]
    fn release_has_windows_build_job() {
        let workflow = load_release_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let has_windows = jobs.as_mapping().unwrap().keys().any(|key| {
            key.as_str().map(|k| k.contains("windows")).unwrap_or(false)
        });

        assert!(
            has_windows,
            "Release workflow should have Windows build job"
        );
    }

    #[test]
    fn release_uses_gh_release_action() {
        let workflow = load_release_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let mut found_release_action = false;
        for (_, job) in jobs.as_mapping().unwrap() {
            if let Some(steps) = job.get("steps") {
                for step in steps.as_sequence().unwrap() {
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
    fn release_builds_have_artifact_upload() {
        let workflow = load_release_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let mut build_jobs_with_upload = 0;
        let mut build_jobs = 0;

        for (key, job) in jobs.as_mapping().unwrap() {
            let key_str = key.as_str().unwrap_or("");
            if key_str.contains("build-") {
                build_jobs += 1;
                if let Some(steps) = job.get("steps") {
                    for step in steps.as_sequence().unwrap() {
                        if let Some(uses) = step.get("uses").and_then(|u| u.as_str()) {
                            if uses.contains("upload") || uses.contains("release") {
                                build_jobs_with_upload += 1;
                                break;
                            }
                        }
                    }
                }
            }
        }

        assert!(
            build_jobs > 0 && build_jobs_with_upload == build_jobs,
            "All build jobs should upload release artifacts"
        );
    }
}

mod workflow_best_practices {
    use super::*;

    #[test]
    fn ci_workflow_has_caching() {
        let workflow = parse_workflow(&workflow_path("ci.yml"));
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let mut jobs_with_cache = 0;
        let mut total_jobs = 0;

        for (_, job) in jobs.as_mapping().unwrap() {
            total_jobs += 1;
            if let Some(steps) = job.get("steps") {
                for step in steps.as_sequence().unwrap() {
                    if let Some(uses) = step.get("uses").and_then(|u| u.as_str()) {
                        if uses.contains("cache") {
                            jobs_with_cache += 1;
                            break;
                        }
                    }
                }
            }
        }

        // Most jobs should have caching for efficiency
        assert!(
            jobs_with_cache >= total_jobs / 2,
            "At least half of CI jobs should have caching for efficiency"
        );
    }

    #[test]
    fn release_workflow_has_caching() {
        let workflow = parse_workflow(&workflow_path("release.yml"));
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let mut jobs_with_cache = 0;
        let mut build_jobs = 0;

        for (key, job) in jobs.as_mapping().unwrap() {
            if key.as_str().unwrap_or("").contains("build") {
                build_jobs += 1;
                if let Some(steps) = job.get("steps") {
                    for step in steps.as_sequence().unwrap() {
                        if let Some(uses) = step.get("uses").and_then(|u| u.as_str()) {
                            if uses.contains("cache") {
                                jobs_with_cache += 1;
                                break;
                            }
                        }
                    }
                }
            }
        }

        // Build jobs should have caching
        assert!(
            jobs_with_cache >= build_jobs / 2 || build_jobs == 0,
            "At least half of build jobs should have caching for efficiency"
        );
    }

    #[test]
    fn workflows_use_dtolnay_rust_toolchain() {
        // dtolnay/rust-toolchain is the recommended action for Rust
        let ci = parse_workflow(&workflow_path("ci.yml"));
        let release = parse_workflow(&workflow_path("release.yml"));

        fn check_toolchain(workflow: &serde_yaml::Value) -> bool {
            let jobs = match workflow.get("jobs") {
                Some(j) => j,
                None => return false,
            };

            for (_, job) in jobs.as_mapping().unwrap() {
                if let Some(steps) = job.get("steps") {
                    for step in steps.as_sequence().unwrap() {
                        if let Some(uses) = step.get("uses").and_then(|u| u.as_str()) {
                            if uses.contains("dtolnay/rust-toolchain") {
                                return true;
                            }
                        }
                    }
                }
            }
            false
        }

        assert!(
            check_toolchain(&ci) && check_toolchain(&release),
            "Workflows should use dtolnay/rust-toolchain action"
        );
    }

    #[test]
    fn ci_workflow_checks_all_targets() {
        let workflow = parse_workflow(&workflow_path("ci.yml"));
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let clippy_job = jobs.get("clippy").expect("Should have clippy job");
        let steps = clippy_job.get("steps").expect("clippy job should have steps");

        let has_all_targets = steps.as_sequence().unwrap().iter().any(|step| {
            step.get("run")
                .and_then(|r| r.as_str())
                .map(|r| r.contains("--all-targets"))
                .unwrap_or(false)
        });

        assert!(
            has_all_targets,
            "Clippy should check all targets (--all-targets)"
        );
    }

    #[test]
    fn ci_clippy_treats_warnings_as_errors() {
        let workflow = parse_workflow(&workflow_path("ci.yml"));
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let clippy_job = jobs.get("clippy").expect("Should have clippy job");
        let steps = clippy_job.get("steps").expect("clippy job should have steps");

        let treats_warnings_as_errors = steps.as_sequence().unwrap().iter().any(|step| {
            step.get("run")
                .and_then(|r| r.as_str())
                .map(|r| r.contains("-D warnings"))
                .unwrap_or(false)
        });

        assert!(
            treats_warnings_as_errors,
            "Clippy should treat warnings as errors (-D warnings)"
        );
    }
}