// Tests for release automation workflow
//
// These tests verify the release automation setup meets acceptance criteria:
// - Trigger on git tags (v*.*.*)
// - Build release binaries for all target platforms using cargo-zigbuild
// - Create GitHub Release with binaries attached
// - Generate and upload changelog
// - Support draft releases for manual review

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

/// Load the release workflow
fn load_release_workflow() -> serde_yaml::Value {
    let path = workflow_path("release.yml");
    assert!(
        path.exists(),
        "Release workflow file should exist at .github/workflows/release.yml"
    );
    parse_workflow(&path)
}

// =============================================================================
// Git Tag Trigger Tests
// =============================================================================

mod tag_trigger_tests {
    use super::*;

    #[test]
    fn release_triggers_on_semver_tags() {
        let workflow = load_release_workflow();
        let on = workflow.get("on").expect("Workflow should have 'on' trigger");

        // Check push trigger with version tags
        let push = on.get("push").expect("Release should have push trigger");
        let tags = push.get("tags")
            .and_then(|t| t.as_sequence())
            .expect("Push trigger should have tags");

        let tag_patterns: Vec<&str> = tags.iter()
            .filter_map(|t| t.as_str())
            .collect();

        // Should trigger on version tags pattern
        assert!(
            tag_patterns.iter().any(|p| p.contains("v") && p.contains("*")),
            "Release should trigger on version tag pattern (e.g., 'v*.*.*')"
        );
    }

    #[test]
    fn release_supports_manual_dispatch() {
        let workflow = load_release_workflow();
        let on = workflow.get("on").expect("Workflow should have 'on' trigger");

        // Check for workflow_dispatch for manual releases
        let dispatch = on.get("workflow_dispatch");
        assert!(
            dispatch.is_some(),
            "Release should support manual workflow_dispatch"
        );
    }

    #[test]
    fn release_dispatch_has_version_input() {
        let workflow = load_release_workflow();
        let on = workflow.get("on").expect("Workflow should have 'on' trigger");
        let dispatch = on.get("workflow_dispatch")
            .expect("Release should support workflow_dispatch");
        let inputs = dispatch.get("inputs")
            .expect("workflow_dispatch should have inputs");

        // Should have version input for manual releases
        let version_input = inputs.get("version");
        assert!(version_input.is_some(), "workflow_dispatch should have 'version' input");
    }
}

// =============================================================================
// Cross-Platform Build Tests
// =============================================================================

mod cross_platform_build_tests {
    use super::*;

    #[test]
    fn release_has_linux_build_job() {
        let workflow = load_release_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let linux_job = jobs.as_mapping().unwrap().iter()
            .find(|(k, _)| {
                k.as_str()
                    .map(|s| s.contains("build") && s.contains("linux"))
                    .unwrap_or(false)
            });

        assert!(linux_job.is_some(), "Release should have Linux build job");
    }

    #[test]
    fn release_has_macos_build_job() {
        let workflow = load_release_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let macos_job = jobs.as_mapping().unwrap().iter()
            .find(|(k, _)| {
                k.as_str()
                    .map(|s| s.contains("build") && s.contains("macos"))
                    .unwrap_or(false)
            });

        assert!(macos_job.is_some(), "Release should have macOS build job");
    }

    #[test]
    fn release_has_windows_build_job() {
        let workflow = load_release_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let windows_job = jobs.as_mapping().unwrap().iter()
            .find(|(k, _)| {
                k.as_str()
                    .map(|s| s.contains("build") && s.contains("windows"))
                    .unwrap_or(false)
            });

        assert!(windows_job.is_some(), "Release should have Windows build job");
    }

    #[test]
    fn linux_build_uses_cargo_zigbuild() {
        let workflow = load_release_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let linux_job = jobs.as_mapping().unwrap().iter()
            .find(|(k, _)| {
                k.as_str()
                    .map(|s| s.contains("build") && s.contains("linux"))
                    .unwrap_or(false)
            })
            .map(|(_, v)| v);

        assert!(linux_job.is_some(), "Should have Linux build job");

        if let Some(job) = linux_job {
            let steps = job.get("steps")
                .and_then(|s| s.as_sequence())
                .expect("Linux build job should have steps");

            // Check for cargo-zigbuild installation or usage
            let uses_zigbuild = steps.iter().any(|step| {
                let run = step.get("run").and_then(|r| r.as_str()).unwrap_or("");
                let name = step.get("name").and_then(|n| n.as_str()).unwrap_or("");
                run.contains("cargo-zigbuild") ||
                run.contains("cargo zigbuild") ||
                name.to_lowercase().contains("zigbuild")
            });

            assert!(
                uses_zigbuild,
                "Linux build job should use cargo-zigbuild for cross-compilation"
            );
        }
    }

    #[test]
    fn linux_build_includes_arm64_target() {
        let workflow = load_release_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let linux_job = jobs.as_mapping().unwrap().iter()
            .find(|(k, _)| {
                k.as_str()
                    .map(|s| s.contains("build") && s.contains("linux"))
                    .unwrap_or(false)
            })
            .map(|(_, v)| v);

        if let Some(job) = linux_job {
            let matrix = job.get("strategy")
                .and_then(|s| s.get("matrix"));

            if let Some(matrix) = matrix {
                // Check include array for targets
                if let Some(includes) = matrix.get("include").and_then(|i| i.as_sequence()) {
                    let targets: Vec<&str> = includes.iter()
                        .filter_map(|item| item.get("target").and_then(|t| t.as_str()))
                        .collect();

                    assert!(
                        targets.iter().any(|t| t.contains("aarch64")),
                        "Linux build should include ARM64 (aarch64) target"
                    );
                } else if let Some(targets) = matrix.get("target").and_then(|t| t.as_sequence()) {
                    assert!(
                        targets.iter().any(|t| t.as_str().map(|s| s.contains("aarch64")).unwrap_or(false)),
                        "Linux build should include ARM64 (aarch64) target"
                    );
                }
            }
        }
    }

    #[test]
    fn linux_build_includes_musl_target() {
        let workflow = load_release_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let linux_job = jobs.as_mapping().unwrap().iter()
            .find(|(k, _)| {
                k.as_str()
                    .map(|s| s.contains("build") && s.contains("linux"))
                    .unwrap_or(false)
            })
            .map(|(_, v)| v);

        if let Some(job) = linux_job {
            let matrix = job.get("strategy")
                .and_then(|s| s.get("matrix"));

            if let Some(matrix) = matrix {
                if let Some(includes) = matrix.get("include").and_then(|i| i.as_sequence()) {
                    let targets: Vec<&str> = includes.iter()
                        .filter_map(|item| item.get("target").and_then(|t| t.as_str()))
                        .collect();

                    assert!(
                        targets.iter().any(|t| t.contains("musl")),
                        "Linux build should include musl (static) target"
                    );
                } else if let Some(targets) = matrix.get("target").and_then(|t| t.as_sequence()) {
                    assert!(
                        targets.iter().any(|t| t.as_str().map(|s| s.contains("musl")).unwrap_or(false)),
                        "Linux build should include musl (static) target"
                    );
                }
            }
        }
    }

    #[test]
    fn macos_build_includes_arm64_target() {
        let workflow = load_release_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let macos_job = jobs.as_mapping().unwrap().iter()
            .find(|(k, _)| {
                k.as_str()
                    .map(|s| s.contains("build") && s.contains("macos") && !s.contains("universal"))
                    .unwrap_or(false)
            })
            .map(|(_, v)| v);

        assert!(macos_job.is_some(), "Should have macOS build job");

        if let Some(job) = macos_job {
            let matrix = job.get("strategy")
                .and_then(|s| s.get("matrix"));

            if let Some(matrix) = matrix {
                if let Some(targets) = matrix.get("target").and_then(|t| t.as_sequence()) {
                    assert!(
                        targets.iter().any(|t| t.as_str().map(|s| s.contains("aarch64")).unwrap_or(false)),
                        "macOS build should include ARM64 (Apple Silicon) target"
                    );
                }
            }
        }
    }

    #[test]
    fn macos_build_includes_universal_binary() {
        let workflow = load_release_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let universal_job = jobs.as_mapping().unwrap().iter()
            .find(|(k, _)| {
                k.as_str()
                    .map(|s| s.contains("macos") && s.contains("universal"))
                    .unwrap_or(false)
            });

        assert!(
            universal_job.is_some(),
            "Release should have macOS universal binary build job"
        );
    }

    #[test]
    fn windows_build_includes_arm64_target() {
        let workflow = load_release_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let windows_job = jobs.as_mapping().unwrap().iter()
            .find(|(k, _)| {
                k.as_str()
                    .map(|s| s.contains("build") && s.contains("windows"))
                    .unwrap_or(false)
            })
            .map(|(_, v)| v);

        assert!(windows_job.is_some(), "Should have Windows build job");

        if let Some(job) = windows_job {
            let matrix = job.get("strategy")
                .and_then(|s| s.get("matrix"));

            if let Some(matrix) = matrix {
                if let Some(targets) = matrix.get("target").and_then(|t| t.as_sequence()) {
                    assert!(
                        targets.iter().any(|t| t.as_str().map(|s| s.contains("aarch64")).unwrap_or(false)),
                        "Windows build should include ARM64 target"
                    );
                }
            }
        }
    }

    #[test]
    fn build_jobs_use_correct_runners() {
        let workflow = load_release_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        // Linux should use ubuntu runner
        let linux_job = jobs.as_mapping().unwrap().iter()
            .find(|(k, _)| k.as_str().map(|s| s.contains("build") && s.contains("linux")).unwrap_or(false))
            .map(|(_, v)| v);
        if let Some(job) = linux_job {
            let runner = job.get("runs-on").and_then(|r| r.as_str());
            assert!(
                runner.map(|r| r.contains("ubuntu")).unwrap_or(false),
                "Linux build should run on ubuntu runner"
            );
        }

        // macOS should use macos runner
        let macos_job = jobs.as_mapping().unwrap().iter()
            .find(|(k, _)| k.as_str().map(|s| s.contains("build") && s.contains("macos")).unwrap_or(false))
            .map(|(_, v)| v);
        if let Some(job) = macos_job {
            let runner = job.get("runs-on").and_then(|r| r.as_str());
            assert!(
                runner.map(|r| r.contains("macos")).unwrap_or(false),
                "macOS build should run on macos runner"
            );
        }

        // Windows should use windows runner
        let windows_job = jobs.as_mapping().unwrap().iter()
            .find(|(k, _)| k.as_str().map(|s| s.contains("build") && s.contains("windows")).unwrap_or(false))
            .map(|(_, v)| v);
        if let Some(job) = windows_job {
            let runner = job.get("runs-on").and_then(|r| r.as_str());
            assert!(
                runner.map(|r| r.contains("windows")).unwrap_or(false),
                "Windows build should run on windows runner"
            );
        }
    }
}

// =============================================================================
// GitHub Release Tests
// =============================================================================

mod github_release_tests {
    use super::*;

    #[test]
    fn release_has_create_release_job() {
        let workflow = load_release_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let create_release = jobs.get("create-release");
        assert!(
            create_release.is_some(),
            "Release workflow should have 'create-release' job"
        );
    }

    #[test]
    fn create_release_uses_gh_release_action() {
        let workflow = load_release_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let create_release = jobs.get("create-release")
            .expect("Should have create-release job");

        let steps = create_release.get("steps")
            .and_then(|s| s.as_sequence())
            .expect("create-release should have steps");

        let uses_release_action = steps.iter().any(|step| {
            step.get("uses")
                .and_then(|u| u.as_str())
                .map(|u| u.contains("release"))
                .unwrap_or(false)
        });

        assert!(
            uses_release_action,
            "create-release job should use a GitHub release action"
        );
    }

    #[test]
    fn create_release_has_tag_name() {
        let workflow = load_release_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let create_release = jobs.get("create-release")
            .expect("Should have create-release job");

        let steps = create_release.get("steps")
            .and_then(|s| s.as_sequence())
            .expect("create-release should have steps");

        let release_step = steps.iter().find(|step| {
            step.get("uses")
                .and_then(|u| u.as_str())
                .map(|u| u.contains("release"))
                .unwrap_or(false)
        });

        assert!(release_step.is_some(), "Should have release step");

        if let Some(step) = release_step {
            let with = step.get("with");
            assert!(with.is_some(), "Release action should have 'with' config");

            if let Some(config) = with {
                assert!(
                    config.get("tag_name").is_some(),
                    "Release action should specify tag_name"
                );
            }
        }
    }

    #[test]
    fn create_release_has_body_for_changelog() {
        let workflow = load_release_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let create_release = jobs.get("create-release")
            .expect("Should have create-release job");

        let steps = create_release.get("steps")
            .and_then(|s| s.as_sequence())
            .expect("create-release should have steps");

        let release_step = steps.iter().find(|step| {
            step.get("uses")
                .and_then(|u| u.as_str())
                .map(|u| u.contains("release"))
                .unwrap_or(false)
        });

        if let Some(step) = release_step {
            if let Some(with) = step.get("with") {
                assert!(
                    with.get("body_path").is_some() || with.get("body").is_some(),
                    "Release action should include changelog (body or body_path)"
                );
            }
        }
    }

    #[test]
    fn build_jobs_upload_to_release() {
        let workflow = load_release_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let build_jobs: Vec<_> = jobs.as_mapping().unwrap().iter()
            .filter(|(k, _)| k.as_str().map(|s| s.contains("build")).unwrap_or(false))
            .collect();

        for (job_name, job) in build_jobs {
            let steps = job.get("steps")
                .and_then(|s| s.as_sequence());

            let has_upload = steps.map(|s| s.iter().any(|step| {
                let uses = step.get("uses").and_then(|u| u.as_str()).unwrap_or("");
                uses.contains("upload") || uses.contains("release")
            })).unwrap_or(false);

            assert!(
                has_upload,
                "Build job '{}' should upload release artifacts",
                job_name.as_str().unwrap_or("unknown")
            );
        }
    }
}

// =============================================================================
// Changelog Tests
// =============================================================================

mod changelog_tests {
    use super::*;

    #[test]
    fn release_has_changelog_job() {
        let workflow = load_release_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let changelog_job = jobs.get("changelog");
        assert!(
            changelog_job.is_some(),
            "Release workflow should have 'changelog' job"
        );
    }

    #[test]
    fn changelog_job_generates_changelog() {
        let workflow = load_release_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let changelog_job = jobs.get("changelog")
            .expect("Should have changelog job");

        let steps = changelog_job.get("steps")
            .and_then(|s| s.as_sequence())
            .expect("changelog job should have steps");

        let has_generate_step = steps.iter().any(|step| {
            let name = step.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let run = step.get("run").and_then(|r| r.as_str()).unwrap_or("");
            name.to_lowercase().contains("changelog") ||
            run.to_lowercase().contains("changelog")
        });

        assert!(
            has_generate_step,
            "changelog job should have changelog generation step"
        );
    }

    #[test]
    fn changelog_job_uploads_artifact() {
        let workflow = load_release_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let changelog_job = jobs.get("changelog")
            .expect("Should have changelog job");

        let steps = changelog_job.get("steps")
            .and_then(|s| s.as_sequence())
            .expect("changelog job should have steps");

        let has_artifact_upload = steps.iter().any(|step| {
            step.get("uses")
                .and_then(|u| u.as_str())
                .map(|u| u.contains("upload-artifact"))
                .unwrap_or(false)
        });

        assert!(
            has_artifact_upload,
            "changelog job should upload artifact for use by create-release"
        );
    }

    #[test]
    fn changelog_job_has_output() {
        let workflow = load_release_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let changelog_job = jobs.get("changelog")
            .expect("Should have changelog job");

        let outputs = changelog_job.get("outputs");
        assert!(
            outputs.is_some(),
            "changelog job should have outputs for passing changelog path"
        );
    }
}

// =============================================================================
// Draft Release Tests
// =============================================================================

mod draft_release_tests {
    use super::*;

    #[test]
    fn workflow_dispatch_has_draft_input() {
        let workflow = load_release_workflow();
        let on = workflow.get("on").expect("Workflow should have 'on' trigger");
        let dispatch = on.get("workflow_dispatch")
            .expect("Release should support workflow_dispatch");
        let inputs = dispatch.get("inputs")
            .expect("workflow_dispatch should have inputs");

        let draft_input = inputs.get("draft");
        assert!(
            draft_input.is_some(),
            "workflow_dispatch should have 'draft' input for draft releases"
        );
    }

    #[test]
    fn draft_input_is_boolean() {
        let workflow = load_release_workflow();
        let on = workflow.get("on").expect("Workflow should have 'on' trigger");
        let dispatch = on.get("workflow_dispatch")
            .expect("Release should support workflow_dispatch");
        let inputs = dispatch.get("inputs")
            .expect("workflow_dispatch should have inputs");

        let draft_input = inputs.get("draft")
            .expect("Should have draft input");

        let draft_type = draft_input.get("type").and_then(|t| t.as_str());
        assert_eq!(
            draft_type,
            Some("boolean"),
            "draft input should be boolean type"
        );
    }

    #[test]
    fn draft_default_is_true() {
        let workflow = load_release_workflow();
        let on = workflow.get("on").expect("Workflow should have 'on' trigger");
        let dispatch = on.get("workflow_dispatch")
            .expect("Release should support workflow_dispatch");
        let inputs = dispatch.get("inputs")
            .expect("workflow_dispatch should have inputs");

        let draft_input = inputs.get("draft")
            .expect("Should have draft input");

        let default = draft_input.get("default").and_then(|d| d.as_str());
        assert_eq!(
            default,
            Some("true"),
            "draft input should default to 'true' for manual review"
        );
    }

    #[test]
    fn create_release_uses_draft_setting() {
        let workflow = load_release_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let create_release = jobs.get("create-release")
            .expect("Should have create-release job");

        let steps = create_release.get("steps")
            .and_then(|s| s.as_sequence())
            .expect("create-release should have steps");

        let release_step = steps.iter().find(|step| {
            step.get("uses")
                .and_then(|u| u.as_str())
                .map(|u| u.contains("release"))
                .unwrap_or(false)
        });

        if let Some(step) = release_step {
            let with = step.get("with");
            if let Some(config) = with {
                assert!(
                    config.get("draft").is_some(),
                    "Release action should use draft setting from prepare job"
                );
            }
        }
    }

    #[test]
    fn prepare_job_outputs_draft_status() {
        let workflow = load_release_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let prepare = jobs.get("prepare")
            .expect("Should have prepare job");

        let outputs = prepare.get("outputs")
            .expect("prepare job should have outputs");

        assert!(
            outputs.get("is_draft").is_some(),
            "prepare job should output 'is_draft' for use by create-release"
        );
    }
}

// =============================================================================
// Prerelease Detection Tests
// =============================================================================

mod prerelease_tests {
    use super::*;

    #[test]
    fn prepare_job_detects_prerelease() {
        let workflow = load_release_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let prepare = jobs.get("prepare")
            .expect("Should have prepare job");

        let outputs = prepare.get("outputs")
            .expect("prepare job should have outputs");

        assert!(
            outputs.get("is_prerelease").is_some(),
            "prepare job should output 'is_prerelease' for prerelease detection"
        );
    }

    #[test]
    fn create_release_uses_prerelease_setting() {
        let workflow = load_release_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let create_release = jobs.get("create-release")
            .expect("Should have create-release job");

        let steps = create_release.get("steps")
            .and_then(|s| s.as_sequence())
            .expect("create-release should have steps");

        let release_step = steps.iter().find(|step| {
            step.get("uses")
                .and_then(|u| u.as_str())
                .map(|u| u.contains("release"))
                .unwrap_or(false)
        });

        if let Some(step) = release_step {
            let with = step.get("with");
            if let Some(config) = with {
                assert!(
                    config.get("prerelease").is_some(),
                    "Release action should use prerelease setting from prepare job"
                );
            }
        }
    }
}

// =============================================================================
// Job Dependencies Tests
// =============================================================================

mod job_dependency_tests {
    use super::*;

    #[test]
    fn create_release_depends_on_prepare() {
        let workflow = load_release_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let create_release = jobs.get("create-release")
            .expect("Should have create-release job");

        let needs = create_release.get("needs");
        assert!(
            needs.is_some(),
            "create-release job should have dependencies"
        );

        if let Some(needs) = needs {
            let needs_list: Vec<&str> = if let Some(seq) = needs.as_sequence() {
                seq.iter().filter_map(|v| v.as_str()).collect()
            } else if let Some(s) = needs.as_str() {
                vec![s]
            } else {
                vec![]
            };

            assert!(
                needs_list.iter().any(|n| n.contains("prepare")),
                "create-release should depend on prepare job"
            );
        }
    }

    #[test]
    fn create_release_depends_on_changelog() {
        let workflow = load_release_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let create_release = jobs.get("create-release")
            .expect("Should have create-release job");

        let needs = create_release.get("needs");
        if let Some(needs) = needs {
            let needs_list: Vec<&str> = if let Some(seq) = needs.as_sequence() {
                seq.iter().filter_map(|v| v.as_str()).collect()
            } else if let Some(s) = needs.as_str() {
                vec![s]
            } else {
                vec![]
            };

            assert!(
                needs_list.iter().any(|n| n.contains("changelog")),
                "create-release should depend on changelog job"
            );
        }
    }

    #[test]
    fn build_jobs_depend_on_create_release() {
        let workflow = load_release_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let build_jobs: Vec<_> = jobs.as_mapping().unwrap().iter()
            .filter(|(k, _)| k.as_str().map(|s| s.contains("build")).unwrap_or(false))
            .collect();

        for (job_name, job) in build_jobs {
            let needs = job.get("needs");
            if let Some(needs) = needs {
                let needs_list: Vec<&str> = if let Some(seq) = needs.as_sequence() {
                    seq.iter().filter_map(|v| v.as_str()).collect()
                } else if let Some(s) = needs.as_str() {
                    vec![s]
                } else {
                    vec![]
                };

                assert!(
                    needs_list.iter().any(|n| n.contains("release") || n.contains("prepare")),
                    "Build job '{}' should depend on create-release or prepare job",
                    job_name.as_str().unwrap_or("unknown")
                );
            }
        }
    }
}

// =============================================================================
// Caching Tests
// =============================================================================

mod caching_tests {
    use super::*;

    #[test]
    fn build_jobs_have_cargo_cache() {
        let workflow = load_release_workflow();
        let jobs = workflow.get("jobs").expect("Workflow should have 'jobs'");

        let build_jobs: Vec<_> = jobs.as_mapping().unwrap().iter()
            .filter(|(k, _)| k.as_str().map(|s| s.contains("build")).unwrap_or(false))
            .collect();

        let jobs_with_cache: usize = build_jobs.iter()
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

        assert!(
            jobs_with_cache > 0,
            "At least some build jobs should have cargo caching for efficiency"
        );
    }
}

// =============================================================================
// Permissions Tests
// =============================================================================

mod permissions_tests {
    use super::*;

    #[test]
    fn workflow_has_write_permissions() {
        let workflow = load_release_workflow();

        let permissions = workflow.get("permissions");
        assert!(
            permissions.is_some(),
            "Release workflow should specify permissions"
        );

        if let Some(permissions) = permissions {
            let contents = permissions.get("contents");
            if let Some(contents) = contents {
                assert_eq!(
                    contents.as_str(),
                    Some("write"),
                    "Release workflow needs write permissions for contents"
                );
            }
        }
    }
}
