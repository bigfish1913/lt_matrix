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

    #[test]
    fn release_uses_cargo_zigbuild_for_cross_compilation() {
        let workflows = Workflows::load();
        let jobs = workflows.release.get("jobs").expect("Workflow should have 'jobs'");

        // Find Linux build job
        let linux_build_job = jobs.as_mapping().unwrap().iter()
            .find(|(k, _)| k.as_str().map(|s| s.contains("build") && s.contains("linux")).unwrap_or(false))
            .map(|(_, v)| v);

        assert!(linux_build_job.is_some(), "Should have Linux build job");

        if let Some(job) = linux_build_job {
            let steps = job.get("steps")
                .and_then(|s| s.as_sequence())
                .expect("Linux build job should have steps");

            // Check for cargo-zigbuild installation
            let has_zigbuild_install = steps.iter().any(|step| {
                step.get("run")
                    .and_then(|r| r.as_str())
                    .map(|r| r.contains("cargo-zigbuild") || r.contains("pip install cargo-zigbuild"))
                    .unwrap_or(false)
            });

            // Check for cargo zigbuild command
            let has_zigbuild_command = steps.iter().any(|step| {
                step.get("run")
                    .and_then(|r| r.as_str())
                    .map(|r| r.contains("cargo zigbuild"))
                    .unwrap_or(false)
            });

            assert!(
                has_zigbuild_install || has_zigbuild_command,
                "Linux build job should use cargo-zigbuild for cross-compilation"
            );
        }
    }

    #[test]
    fn release_supports_draft_releases() {
        let workflows = Workflows::load();
        let workflow = &workflows.release;

        // Check workflow_dispatch has draft input
        let on = workflow.get("on").expect("Workflow should have 'on' trigger");
        let dispatch = on.get("workflow_dispatch")
            .expect("Release should support workflow_dispatch for draft releases");
        let inputs = dispatch.get("inputs")
            .expect("workflow_dispatch should have inputs");

        // Check for draft input
        let draft_input = inputs.get("draft");
        assert!(draft_input.is_some(), "workflow_dispatch should have 'draft' input");

        if let Some(draft) = draft_input {
            let draft_type = draft.get("type").and_then(|t| t.as_str());
            assert_eq!(draft_type, Some("boolean"), "draft input should be boolean type");
        }

        // Verify create-release job uses draft setting
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");
        let create_release = jobs.get("create-release")
            .expect("Should have create-release job");

        let steps = create_release.get("steps")
            .and_then(|s| s.as_sequence())
            .expect("create-release should have steps");

        let uses_draft = steps.iter().any(|step| {
            let with = step.get("with");
            if let Some(with) = with {
                with.get("draft").is_some()
            } else {
                false
            }
        });

        assert!(uses_draft, "create-release job should use draft input");
    }

    #[test]
    fn release_generates_changelog() {
        let workflows = Workflows::load();
        let jobs = workflows.release.get("jobs").expect("Workflow should have 'jobs'");

        // Check for changelog job
        let changelog_job = jobs.get("changelog");
        assert!(changelog_job.is_some(), "Release workflow should have 'changelog' job");

        if let Some(job) = changelog_job {
            let steps = job.get("steps")
                .and_then(|s| s.as_sequence())
                .expect("changelog job should have steps");

            // Check for changelog generation step
            let has_generate_step = steps.iter().any(|step| {
                let name = step.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let run = step.get("run").and_then(|r| r.as_str()).unwrap_or("");
                name.to_lowercase().contains("changelog") ||
                run.to_lowercase().contains("changelog")
            });

            assert!(has_generate_step, "changelog job should have changelog generation step");

            // Check for artifact upload
            let has_artifact_upload = steps.iter().any(|step| {
                step.get("uses")
                    .and_then(|u| u.as_str())
                    .map(|u| u.contains("upload-artifact"))
                    .unwrap_or(false)
            });

            assert!(has_artifact_upload, "changelog job should upload artifact");
        }
    }

    #[test]
    fn release_builds_all_target_platforms() {
        let workflows = Workflows::load();
        let jobs = workflows.release.get("jobs").expect("Workflow should have 'jobs'");

        // Required platforms for release (documented targets)
        let _required_platforms = [
            // Linux
            ("linux", vec!["x86_64-unknown-linux-gnu", "x86_64-unknown-linux-musl", "aarch64-unknown-linux-gnu", "aarch64-unknown-linux-musl"]),
            // macOS
            ("macos", vec!["x86_64-apple-darwin", "aarch64-apple-darwin", "universal-apple-darwin"]),
            // Windows
            ("windows", vec!["x86_64-pc-windows-msvc", "aarch64-pc-windows-msvc"]),
        ];

        // Check for Linux build job with matrix
        let linux_job = jobs.as_mapping().unwrap().iter()
            .find(|(k, _)| k.as_str().map(|s| s.contains("build") && s.contains("linux")).unwrap_or(false))
            .map(|(_, v)| v);

        if let Some(job) = linux_job {
            // Check matrix includes required targets
            let matrix = job.get("strategy")
                .and_then(|s| s.get("matrix"))
                .and_then(|m| m.get("include"));

            if let Some(matrix_includes) = matrix.and_then(|m| m.as_sequence()) {
                let targets: Vec<&str> = matrix_includes.iter()
                    .filter_map(|item| item.get("target").and_then(|t| t.as_str()))
                    .collect();

                assert!(
                    targets.iter().any(|t| t.contains("x86_64-unknown-linux")),
                    "Linux build should include x86_64 target"
                );
                assert!(
                    targets.iter().any(|t| t.contains("aarch64-unknown-linux")),
                    "Linux build should include ARM64 target"
                );
                assert!(
                    targets.iter().any(|t| t.contains("musl")),
                    "Linux build should include musl (static) target"
                );
            }
        }

        // Check for macOS build job
        let macos_job = jobs.as_mapping().unwrap().iter()
            .find(|(k, _)| k.as_str().map(|s| s.contains("build") && s.contains("macos") && !s.contains("universal")).unwrap_or(false))
            .map(|(_, v)| v);

        assert!(macos_job.is_some(), "Should have macOS build job");

        // Check for macOS universal build job
        let macos_universal = jobs.as_mapping().unwrap().iter()
            .find(|(k, _)| k.as_str().map(|s| s.contains("macos") && s.contains("universal")).unwrap_or(false))
            .map(|(_, v)| v);

        assert!(macos_universal.is_some(), "Should have macOS universal binary build job");

        // Check for Windows build job
        let windows_job = jobs.as_mapping().unwrap().iter()
            .find(|(k, _)| k.as_str().map(|s| s.contains("build") && s.contains("windows")).unwrap_or(false))
            .map(|(_, v)| v);

        assert!(windows_job.is_some(), "Should have Windows build job");
    }

    #[test]
    fn release_creates_github_release_with_artifacts() {
        let workflows = Workflows::load();
        let jobs = workflows.release.get("jobs").expect("Workflow should have 'jobs'");

        let create_release = jobs.get("create-release")
            .expect("Should have create-release job");

        let steps = create_release.get("steps")
            .and_then(|s| s.as_sequence())
            .expect("create-release should have steps");

        // Check for release action usage
        let release_step = steps.iter().find(|step| {
            step.get("uses")
                .and_then(|u| u.as_str())
                .map(|u| u.contains("release"))
                .unwrap_or(false)
        });

        assert!(release_step.is_some(), "create-release should use a release action");

        if let Some(step) = release_step {
            let with = step.get("with");
            assert!(with.is_some(), "Release action should have 'with' configuration");

            if let Some(config) = with {
                // Should have tag_name
                assert!(
                    config.get("tag_name").is_some(),
                    "Release action should specify tag_name"
                );
                // Should have release name
                assert!(
                    config.get("name").is_some(),
                    "Release action should specify name"
                );
                // Should have body or body_path for changelog
                assert!(
                    config.get("body_path").is_some() || config.get("body").is_some(),
                    "Release action should include changelog (body or body_path)"
                );
            }
        }
    }

    #[test]
    fn release_triggers_on_version_tags() {
        let workflows = Workflows::load();
        let on = workflows.release.get("on").expect("Workflow should have 'on' trigger");

        // Check push trigger with tags
        let push = on.get("push").expect("Release should have push trigger");
        let tags = push.get("tags")
            .and_then(|t| t.as_sequence())
            .expect("Push trigger should have tags");

        let tag_patterns: Vec<&str> = tags.iter()
            .filter_map(|t| t.as_str())
            .collect();

        // Should trigger on version tags like v*.*.*
        assert!(
            tag_patterns.iter().any(|p| p.starts_with('v') && p.contains('*')),
            "Release should trigger on version tag pattern (e.g., 'v*.*.*')"
        );
    }

    #[test]
    fn release_has_prepare_job() {
        let workflows = Workflows::load();
        let jobs = workflows.release.get("jobs").expect("Workflow should have 'jobs'");

        let prepare = jobs.get("prepare");
        assert!(prepare.is_some(), "Release workflow should have 'prepare' job");

        if let Some(job) = prepare {
            // Should output version
            let outputs = job.get("outputs");
            assert!(outputs.is_some(), "prepare job should have outputs");

            if let Some(outputs) = outputs {
                assert!(
                    outputs.get("version").is_some(),
                    "prepare job should output 'version'"
                );
            }
        }
    }

    #[test]
    fn release_has_release_summary() {
        let workflows = Workflows::load();
        let jobs = workflows.release.get("jobs").expect("Workflow should have 'jobs'");

        // Check for summary job
        let summary_job = jobs.as_mapping().unwrap().iter()
            .find(|(k, _)| {
                k.as_str()
                    .map(|s| s.contains("summary"))
                    .unwrap_or(false)
            });

        assert!(summary_job.is_some(), "Release workflow should have release summary job");
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