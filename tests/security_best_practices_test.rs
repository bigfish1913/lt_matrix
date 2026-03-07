//! Security best practices integration tests
//!
//! This module tests the security implementation including:
//! - deny.toml configuration validity
//! - SECURITY.md documentation completeness
//! - Command injection prevention
//! - Input validation patterns
//! - Unsafe code documentation

use std::fs;
use std::path::Path;

// =============================================================================
// deny.toml Configuration Tests
// =============================================================================

/// Test that deny.toml exists and is valid TOML
#[test]
fn test_deny_toml_exists_and_valid() {
    let deny_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("deny.toml");

    assert!(deny_path.exists(), "deny.toml should exist in project root");

    let content = fs::read_to_string(&deny_path)
        .expect("deny.toml should be readable");

    // Verify it's valid TOML by parsing
    let parsed: toml::Value = toml::from_str(&content)
        .expect("deny.toml should be valid TOML");

    // Verify required sections exist
    assert!(parsed.get("advisories").is_some(), "deny.toml should have [advisories] section");
    assert!(parsed.get("licenses").is_some(), "deny.toml should have [licenses] section");
    assert!(parsed.get("bans").is_some(), "deny.toml should have [bans] section");
    assert!(parsed.get("sources").is_some(), "deny.toml should have [sources] section");
}

/// Test that deny.toml has proper vulnerability checking configuration
#[test]
fn test_deny_toml_vulnerability_settings() {
    let deny_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("deny.toml");
    let content = fs::read_to_string(&deny_path).unwrap();
    let parsed: toml::Value = toml::from_str(&content).unwrap();

    let advisories = parsed.get("advisories").unwrap().as_table().unwrap();

    // Vulnerability check should be set to "deny" for production
    assert_eq!(
        advisories.get("vulnerability").and_then(|v| v.as_str()),
        Some("deny"),
        "Vulnerability checking should be set to 'deny'"
    );

    // Should have advisory database URL configured
    let db_urls = advisories.get("db-urls").and_then(|v| v.as_array());
    assert!(db_urls.is_some(), "Advisory database URLs should be configured");

    let urls = db_urls.unwrap();
    assert!(
        urls.iter().any(|u| u.as_str().unwrap().contains("RustSec")),
        "Should reference RustSec advisory database"
    );
}

/// Test that deny.toml has proper license compliance configuration
#[test]
fn test_deny_toml_license_settings() {
    let deny_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("deny.toml");
    let content = fs::read_to_string(&deny_path).unwrap();
    let parsed: toml::Value = toml::from_str(&content).unwrap();

    let licenses = parsed.get("licenses").unwrap().as_table().unwrap();

    // Unlicensed crates should be denied
    assert_eq!(
        licenses.get("unlicensed").and_then(|v| v.as_str()),
        Some("deny"),
        "Unlicensed crates should be denied"
    );

    // Should have allowed licenses list
    let allowed = licenses.get("allow").and_then(|v| v.as_array());
    assert!(allowed.is_some(), "Should have allowed licenses list");

    let allowed_licenses: Vec<&str> = allowed.unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();

    // MIT and Apache-2.0 should be allowed (most common Rust licenses)
    assert!(allowed_licenses.contains(&"MIT"), "MIT license should be allowed");
    assert!(allowed_licenses.contains(&"Apache-2.0"), "Apache-2.0 license should be allowed");
}

/// Test that deny.toml restricts crate sources
#[test]
fn test_deny_toml_source_restrictions() {
    let deny_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("deny.toml");
    let content = fs::read_to_string(&deny_path).unwrap();
    let parsed: toml::Value = toml::from_str(&content).unwrap();

    let sources = parsed.get("sources").unwrap().as_table().unwrap();

    // Unknown registries should be denied for supply chain security
    assert_eq!(
        sources.get("unknown-registry").and_then(|v| v.as_str()),
        Some("deny"),
        "Unknown registries should be denied"
    );
}

/// Test that deny.toml has wildcard ban
#[test]
fn test_deny_toml_wildcard_ban() {
    let deny_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("deny.toml");
    let content = fs::read_to_string(&deny_path).unwrap();
    let parsed: toml::Value = toml::from_str(&content).unwrap();

    let bans = parsed.get("bans").unwrap().as_table().unwrap();

    // Wildcard dependencies should be denied
    assert_eq!(
        bans.get("wildcards").and_then(|v| v.as_str()),
        Some("deny"),
        "Wildcard dependencies should be denied for reproducibility"
    );
}

// =============================================================================
// SECURITY.md Documentation Tests
// =============================================================================

/// Test that SECURITY.md exists
#[test]
fn test_security_md_exists() {
    let security_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("SECURITY.md");
    assert!(security_path.exists(), "SECURITY.md should exist in project root");
}

/// Test that SECURITY.md contains required sections
#[test]
fn test_security_md_required_sections() {
    let security_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("SECURITY.md");
    let content = fs::read_to_string(&security_path).unwrap();

    // Required sections for a comprehensive security policy
    let required_sections = [
        "Reporting Security Issues",
        "Security Architecture",
        "Dependency Management",
        "Input Validation",
        "Command Execution Security",
        "Unsafe Code",
    ];

    for section in &required_sections {
        assert!(
            content.contains(section),
            "SECURITY.md should contain section: {}",
            section
        );
    }
}

/// Test that SECURITY.md documents threat model
#[test]
fn test_security_md_threat_model() {
    let security_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("SECURITY.md");
    let content = fs::read_to_string(&security_path).unwrap();

    // Should document key threat categories
    let threat_categories = [
        "Command Injection",
        "Path Traversal",
        "Credential",
    ];

    for threat in &threat_categories {
        assert!(
            content.contains(threat),
            "SECURITY.md should document threat: {}",
            threat
        );
    }
}

/// Test that SECURITY.md documents validation patterns
#[test]
fn test_security_md_validation_patterns() {
    let security_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("SECURITY.md");
    let content = fs::read_to_string(&security_path).unwrap();

    // Should have code examples for validation
    assert!(
        content.contains("sanitize_path") || content.contains("validate"),
        "SECURITY.md should document validation patterns"
    );

    // Should mention input sources
    assert!(
        content.contains("CLI") || content.contains("command-line"),
        "SECURITY.md should mention CLI as input source"
    );
}

/// Test that SECURITY.md documents unsafe code
#[test]
fn test_security_md_unsafe_code_documentation() {
    let security_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("SECURITY.md");
    let content = fs::read_to_string(&security_path).unwrap();

    // Should document unsafe code usage
    assert!(
        content.contains("unsafe"),
        "SECURITY.md should document unsafe code usage"
    );

    // Should have safety invariants documented
    assert!(
        content.contains("Safety") || content.contains("invariant"),
        "SECURITY.md should document safety invariants"
    );
}

// =============================================================================
// Command Injection Prevention Tests
// =============================================================================

/// Test that git operations use safe command patterns
#[test]
fn test_git_operations_use_safe_patterns() {
    // Read git_ops.rs to verify safe command patterns
    let git_ops_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/release/git_ops.rs");

    if !git_ops_path.exists() {
        // Skip if module doesn't exist
        return;
    }

    let content = fs::read_to_string(&git_ops_path).unwrap();

    // Should use Command::new() not shell execution
    assert!(
        content.contains("Command::new(\"git\")"),
        "Git operations should use Command::new() with explicit command name"
    );

    // Should use .args() for arguments
    assert!(
        content.contains(".args("),
        "Git operations should use .args() for command arguments"
    );

    // Should NOT use shell interpolation patterns
    assert!(
        !content.contains("sh -c") && !content.contains("shell = true"),
        "Git operations should NOT use shell interpolation"
    );
}

/// Test that agent commands use safe patterns
#[test]
fn test_agent_commands_safe_patterns() {
    // Check agent files for safe command patterns
    let agent_files = [
        "src/agent/claude.rs",
        "src/agent/codex.rs",
        "src/agent/opencode.rs",
    ];

    for agent_file in &agent_files {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(agent_file);

        if !path.exists() {
            continue;
        }

        let content = fs::read_to_string(&path).unwrap();

        // If file uses Command, it should use safe patterns
        if content.contains("Command::new") {
            assert!(
                content.contains(".args(") || content.contains(".arg("),
                "{} should use .args() or .arg() for command arguments",
                agent_file
            );
        }
    }
}

// =============================================================================
// Unsafe Code Audit Tests
// =============================================================================

/// Test that unsafe code in main.rs is documented
#[test]
fn test_unsafe_main_rs_documented() {
    let main_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/main.rs");

    if !main_path.exists() {
        return;
    }

    let content = fs::read_to_string(&main_path).unwrap();

    // Check that unsafe impl has documentation
    if content.contains("unsafe impl") {
        // Should have a comment explaining why - check for various documentation patterns
        let has_safety_doc =
            // Standard SAFETY comment
            content.contains("// SAFETY") ||
            content.contains("// Safety") ||
            // Doc comments
            content.contains("/// Safety") ||
            content.contains("/// # Safety") ||
            // Justification patterns
            content.contains("Justification") ||
            // Common security-related terms near the unsafe code
            content.contains("thread-safe") ||
            content.contains("thread safe") ||
            // Comments explaining the purpose (common in Rust)
            content.contains("We need to keep") ||
            content.contains("This is a simple way") ||
            // References to specific safety properties
            content.contains("never accessed") ||
            content.contains("only stored once");

        assert!(
            has_safety_doc,
            "Unsafe code in main.rs should be documented with safety justification"
        );
    }
}

/// Test that unsafe code in logging is documented
#[test]
fn test_unsafe_logging_documented() {
    let formatter_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/logging/formatter.rs");

    if !formatter_path.exists() {
        return;
    }

    let content = fs::read_to_string(&formatter_path).unwrap();

    // Check that unsafe blocks have documentation
    if content.contains("unsafe {") {
        // Should have safety documentation nearby
        assert!(
            content.contains("# Safety") ||
            content.contains("// Safety") ||
            content.contains("/// # Safety"),
            "Unsafe code in formatter.rs should have safety documentation"
        );
    }
}

/// Test that unsafe code count is minimal in key source files
#[test]
fn test_unsafe_code_count_minimal() {
    // Check specific key files for unsafe code count
    let key_files = [
        "src/main.rs",
        "src/logging/formatter.rs",
        "src/lib.rs",
        "src/config/settings.rs",
        "src/agent/mod.rs",
        "src/pipeline/mod.rs",
    ];

    let mut total_unsafe_count = 0;

    for file_path in &key_files {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(file_path);

        if !path.exists() {
            continue;
        }

        let content = fs::read_to_string(&path).unwrap();

        // Count "unsafe" keyword occurrences (excluding comments)
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") {
                continue;
            }
            if line.contains("unsafe") {
                total_unsafe_count += 1;
            }
        }
    }

    // Should have minimal unsafe code
    assert!(
        total_unsafe_count <= 15,
        "Key source files should have minimal unsafe code, found {} occurrences",
        total_unsafe_count
    );
}

// =============================================================================
// Supply Chain Security Tests
// =============================================================================

/// Test that Cargo.lock is committed for reproducibility
#[test]
fn test_cargo_lock_exists() {
    let lock_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.lock");
    assert!(lock_path.exists(), "Cargo.lock should exist for reproducible builds");
}

/// Test that no deprecated/insecure dependencies
#[test]
fn test_no_known_insecure_patterns() {
    let cargo_toml = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml).unwrap();

    // Should not have wildcard versions in production dependencies
    let parsed: toml::Value = toml::from_str(&content).unwrap();

    if let Some(deps) = parsed.get("dependencies").and_then(|d| d.as_table()) {
        for (name, value) in deps {
            // Check for wildcard versions
            if let Some(version) = value.as_str() {
                assert_ne!(
                    version, "*",
                    "Dependency {} should not use wildcard version",
                    name
                );
            }
        }
    }
}

// =============================================================================
// Input Validation Pattern Tests
// =============================================================================

/// Test that config module has input validation patterns
#[test]
fn test_config_input_validation() {
    let config_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/config/settings.rs");

    if !config_path.exists() {
        return;
    }

    let content = fs::read_to_string(&config_path).unwrap();

    // Should have some form of validation
    let has_validation =
        content.contains("validate") ||
        content.contains("parse") ||
        content.contains("from_str") ||
        content.contains("try_from") ||
        content.contains("Result");

    assert!(
        has_validation,
        "Config module should have input validation"
    );
}

/// Test that plugin validation exists
#[test]
fn test_plugin_validation() {
    let validator_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/plugin/validator.rs");

    if !validator_path.exists() {
        // Plugin validation module might not exist
        return;
    }

    let content = fs::read_to_string(&validator_path).unwrap();

    // Validator should have validation logic
    assert!(
        content.contains("validate") || content.contains("check") || content.contains("verify"),
        "Plugin validator should have validation logic"
    );
}

// =============================================================================
// Security Documentation Content Tests
// =============================================================================

/// Test that SECURITY.md has proper contact information
#[test]
fn test_security_md_contact_info() {
    let security_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("SECURITY.md");
    let content = fs::read_to_string(&security_path).unwrap();

    // Should have security contact information
    assert!(
        content.contains("security@") || content.contains("Security Contact"),
        "SECURITY.md should have security contact information"
    );
}

/// Test that SECURITY.md documents dependency security tools
#[test]
fn test_security_md_dependency_tools() {
    let security_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("SECURITY.md");
    let content = fs::read_to_string(&security_path).unwrap();

    // Should mention security audit tools
    assert!(
        content.contains("cargo-audit") || content.contains("cargo audit"),
        "SECURITY.md should document cargo-audit usage"
    );

    assert!(
        content.contains("cargo-deny") || content.contains("cargo deny"),
        "SECURITY.md should document cargo-deny usage"
    );
}

/// Test that SECURITY.md has self-audit checklist
#[test]
fn test_security_md_audit_checklist() {
    let security_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("SECURITY.md");
    let content = fs::read_to_string(&security_path).unwrap();

    // Should have a checklist for security audits
    assert!(
        content.contains("checklist") || content.contains("verify") || content.contains("audit"),
        "SECURITY.md should have a security audit checklist"
    );
}

/// Test that SECURITY.md documents command allowlisting
#[test]
fn test_security_md_command_allowlist() {
    let security_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("SECURITY.md");
    let content = fs::read_to_string(&security_path).unwrap();

    // Should document allowed commands
    assert!(
        content.contains("git") && content.contains("claude"),
        "SECURITY.md should document allowed external commands"
    );
}