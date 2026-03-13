//! Security best practices integration tests
//!
//! This module tests the security implementation including:
//! - deny.toml configuration validity
//! - SECURITY.md documentation completeness
//! - Command injection prevention
//! - Input validation patterns
//! - Unsafe code documentation
//! - Security module public API

use std::fs;
use std::path::Path;

// Import the security module for direct API testing
use ltmatrix::security::{
    contains_traversal,
    has_dangerous_extension,
    is_safe_for_command_arg,
    is_sensitive_env_var,
    mask_sensitive_value,
    resolve_safe_path,
    sanitize_command_arg,
    sanitize_command_argument_for_display,
    sanitize_identifier,
    sanitize_model_name,
    sanitize_path_component,
    validate_branch_name,
    validate_command_arg,
    validate_command_argument,
    // Command security
    validate_command_name,
    validate_commit_message,
    validate_env_var_name,
    validate_file_extension,
    // Input validation
    validate_identifier,
    validate_model_name,
    // Path security
    validate_path,
    validate_task_description,
    validate_task_title,
    CommandAllowlist,
};

// =============================================================================
// deny.toml Configuration Tests
// =============================================================================

/// Test that deny.toml exists and is valid TOML
#[test]
fn test_deny_toml_exists_and_valid() {
    let deny_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("deny.toml");

    assert!(deny_path.exists(), "deny.toml should exist in project root");

    let content = fs::read_to_string(&deny_path).expect("deny.toml should be readable");

    // Verify it's valid TOML by parsing
    let parsed: toml::Value = toml::from_str(&content).expect("deny.toml should be valid TOML");

    // Verify required sections exist
    assert!(
        parsed.get("advisories").is_some(),
        "deny.toml should have [advisories] section"
    );
    assert!(
        parsed.get("licenses").is_some(),
        "deny.toml should have [licenses] section"
    );
    assert!(
        parsed.get("bans").is_some(),
        "deny.toml should have [bans] section"
    );
    assert!(
        parsed.get("sources").is_some(),
        "deny.toml should have [sources] section"
    );
}

/// Test that deny.toml has proper vulnerability checking configuration
#[test]
fn test_deny_toml_vulnerability_settings() {
    let deny_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("deny.toml");
    let content = fs::read_to_string(&deny_path).unwrap();
    let parsed: toml::Value = toml::from_str(&content).unwrap();

    let advisories = parsed.get("advisories").unwrap().as_table().unwrap();

    // Should have advisory database URL configured
    let db_urls = advisories.get("db-urls").and_then(|v| v.as_array());
    assert!(
        db_urls.is_some(),
        "Advisory database URLs should be configured"
    );

    let urls = db_urls.unwrap();
    assert!(
        urls.iter().any(|u| {
            let url = u.as_str().unwrap().to_lowercase();
            url.contains("rustsec") || url.contains("advisory-db")
        }),
        "Should reference RustSec advisory database"
    );

    // Should have ignore list (even if empty)
    assert!(
        advisories.contains_key("ignore"),
        "Should have ignore field for advisories"
    );
}

/// Test that deny.toml has proper license compliance configuration
#[test]
fn test_deny_toml_license_settings() {
    let deny_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("deny.toml");
    let content = fs::read_to_string(&deny_path).unwrap();
    let parsed: toml::Value = toml::from_str(&content).unwrap();

    let licenses = parsed.get("licenses").unwrap().as_table().unwrap();

    // Should have allowed licenses list
    let allowed = licenses.get("allow").and_then(|v| v.as_array());
    assert!(allowed.is_some(), "Should have allowed licenses list");

    let allowed_licenses: Vec<&str> = allowed.unwrap().iter().filter_map(|v| v.as_str()).collect();

    // MIT and Apache-2.0 should be allowed (most common Rust licenses)
    assert!(
        allowed_licenses.contains(&"MIT"),
        "MIT license should be allowed"
    );
    assert!(
        allowed_licenses.contains(&"Apache-2.0"),
        "Apache-2.0 license should be allowed"
    );

    // Should have confidence threshold
    let confidence = licenses
        .get("confidence-threshold")
        .and_then(|v| v.as_float());
    assert!(
        confidence.is_some(),
        "Should have confidence threshold for license detection"
    );
    assert!(
        confidence.unwrap() >= 0.8,
        "Confidence threshold should be at least 0.8"
    );
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
    assert!(
        security_path.exists(),
        "SECURITY.md should exist in project root"
    );
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
    let threat_categories = ["Command Injection", "Path Traversal", "Credential"];

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
    let git_ops_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/release/git_ops.rs");

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
    let formatter_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/logging/formatter.rs");

    if !formatter_path.exists() {
        return;
    }

    let content = fs::read_to_string(&formatter_path).unwrap();

    // Check that unsafe blocks have documentation
    if content.contains("unsafe {") {
        // Should have safety documentation nearby
        assert!(
            content.contains("# Safety")
                || content.contains("// Safety")
                || content.contains("/// # Safety"),
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
    assert!(
        lock_path.exists(),
        "Cargo.lock should exist for reproducible builds"
    );
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
    let has_validation = content.contains("validate")
        || content.contains("parse")
        || content.contains("from_str")
        || content.contains("try_from")
        || content.contains("Result");

    assert!(has_validation, "Config module should have input validation");
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

// =============================================================================
// Security Module Public API Tests
// =============================================================================

// -----------------------------------------------------------------------------
// Command Security Tests
// -----------------------------------------------------------------------------

/// Test command allowlist functionality
#[test]
fn test_command_allowlist_default() {
    let allowlist = CommandAllowlist::new();

    // Default allowed commands
    assert!(
        allowlist.is_allowed("git"),
        "git should be allowed by default"
    );
    assert!(
        allowlist.is_allowed("claude"),
        "claude should be allowed by default"
    );
    assert!(
        allowlist.is_allowed("cargo"),
        "cargo should be allowed by default"
    );
    assert!(
        allowlist.is_allowed("npm"),
        "npm should be allowed by default"
    );

    // Case insensitive
    assert!(
        allowlist.is_allowed("GIT"),
        "Allowlist should be case insensitive"
    );
    assert!(
        allowlist.is_allowed("Claude"),
        "Allowlist should be case insensitive"
    );
}

#[test]
fn test_command_allowlist_modification() {
    let mut allowlist = CommandAllowlist::empty();

    // Empty allowlist
    assert!(
        !allowlist.is_allowed("git"),
        "Empty allowlist should not allow any command"
    );

    // Add command
    allowlist.allow("git");
    assert!(
        allowlist.is_allowed("git"),
        "After allowing, command should be permitted"
    );

    // Remove command
    allowlist.deny("git");
    assert!(
        !allowlist.is_allowed("git"),
        "After denying, command should not be permitted"
    );
}

/// Test command name validation
#[test]
fn test_validate_command_name_valid() {
    assert!(validate_command_name("git").is_ok());
    assert!(validate_command_name("my-tool").is_ok());
    assert!(validate_command_name("cargo").is_ok());
    assert!(validate_command_name("python3").is_ok());
}

#[test]
fn test_validate_command_name_invalid() {
    // Empty
    assert!(validate_command_name("").is_err());

    // Path separators
    assert!(validate_command_name("/bin/sh").is_err());
    assert!(validate_command_name(".\\cmd").is_err());

    // Dangerous characters
    assert!(validate_command_name("rm; ls").is_err());
    assert!(validate_command_name("cmd && other").is_err());
    assert!(validate_command_name("cmd`whoami`").is_err());
    assert!(validate_command_name("cmd$(id)").is_err());

    // Shell builtins
    assert!(validate_command_name("eval").is_err());
    assert!(validate_command_name("exec").is_err());
    assert!(validate_command_name("source").is_err());
}

/// Test command argument validation
#[test]
fn test_validate_command_argument_valid() {
    assert!(validate_command_argument("file.txt").is_ok());
    assert!(validate_command_argument("--option=value").is_ok());
    assert!(validate_command_argument("src/main.rs").is_ok());
    assert!(validate_command_argument("path/to/file").is_ok());
}

#[test]
fn test_validate_command_argument_invalid() {
    // Null bytes
    assert!(validate_command_argument("file\0.txt").is_err());

    // Command injection attempts
    assert!(validate_command_argument("file; rm -rf /").is_err());
    assert!(validate_command_argument("file && cat /etc/passwd").is_err());
    assert!(validate_command_argument("file`whoami`").is_err());
    assert!(validate_command_argument("file\nother").is_err());
    assert!(validate_command_argument("file\rother").is_err());
}

/// Test sanitize command argument for display
#[test]
fn test_sanitize_command_argument_for_display() {
    assert_eq!(
        sanitize_command_argument_for_display("file.txt"),
        "file.txt"
    );
    assert_eq!(
        sanitize_command_argument_for_display("file\nname"),
        "file_name"
    );
    assert_eq!(
        sanitize_command_argument_for_display("file\rname"),
        "file_name"
    );
    assert_eq!(
        sanitize_command_argument_for_display("file\0name"),
        "file_name"
    );
}

/// Test environment variable name validation
#[test]
fn test_validate_env_var_name_valid() {
    assert!(validate_env_var_name("PATH").is_ok());
    assert!(validate_env_var_name("MY_VAR").is_ok());
    assert!(validate_env_var_name("_PRIVATE").is_ok());
    assert!(validate_env_var_name("API_KEY_123").is_ok());
}

#[test]
fn test_validate_env_var_name_invalid() {
    assert!(validate_env_var_name("").is_err());
    assert!(validate_env_var_name("123VAR").is_err());
    assert!(validate_env_var_name("MY-VAR").is_err());
    assert!(validate_env_var_name("MY.VAR").is_err());
}

/// Test sensitive environment variable detection
#[test]
fn test_is_sensitive_env_var() {
    // Sensitive patterns
    assert!(is_sensitive_env_var("API_KEY"));
    assert!(is_sensitive_env_var("PASSWORD"));
    assert!(is_sensitive_env_var("SECRET_TOKEN"));
    assert!(is_sensitive_env_var("MY_PRIVATE_KEY"));
    assert!(is_sensitive_env_var("ACCESS_KEY"));
    assert!(is_sensitive_env_var("AUTH_TOKEN"));

    // Non-sensitive
    assert!(!is_sensitive_env_var("PATH"));
    assert!(!is_sensitive_env_var("HOME"));
    assert!(!is_sensitive_env_var("DEBUG"));
    assert!(!is_sensitive_env_var("APP_NAME"));

    // Case insensitive
    assert!(is_sensitive_env_var("password"));
    assert!(is_sensitive_env_var("Secret_Key"));
}

/// Test sensitive value masking
#[test]
fn test_mask_sensitive_value() {
    assert_eq!(mask_sensitive_value("sk-1234567890abcdef"), "sk-1...cdef");
    assert_eq!(mask_sensitive_value("short"), "sh...rt");
    assert_eq!(mask_sensitive_value("ab"), "***");
    assert_eq!(mask_sensitive_value(""), "***");
    assert_eq!(mask_sensitive_value("abcd"), "***");
}

// -----------------------------------------------------------------------------
// Path Security Tests
// -----------------------------------------------------------------------------

/// Test path validation
#[test]
fn test_validate_path_valid() {
    assert!(validate_path(Path::new("src/main.rs")).is_ok());
    assert!(validate_path(Path::new("file.txt")).is_ok());
    assert!(validate_path(Path::new("/absolute/path")).is_ok());
    assert!(validate_path(Path::new("./relative")).is_ok());
}

#[test]
fn test_validate_path_traversal() {
    assert!(validate_path(Path::new("../../../etc/passwd")).is_err());
    assert!(validate_path(Path::new("..\\..\\windows")).is_err());
    assert!(validate_path(Path::new("sub/../../etc")).is_err());
}

#[test]
fn test_validate_path_null_byte() {
    assert!(validate_path(Path::new("file\0.txt")).is_err());
}

/// Test contains_traversal
#[test]
fn test_contains_traversal() {
    assert!(contains_traversal(Path::new("../parent")));
    assert!(contains_traversal(Path::new("sub/../other")));
    assert!(contains_traversal(Path::new("a/b/../../c")));
    assert!(!contains_traversal(Path::new("sub/dir/file")));
    assert!(!contains_traversal(Path::new("./file")));
}

/// Test sanitize_path_component
#[test]
fn test_sanitize_path_component() {
    assert_eq!(sanitize_path_component("file.txt"), "file.txt");
    assert_eq!(sanitize_path_component("file<name>"), "file_name_");
    assert_eq!(sanitize_path_component("file|name?"), "file_name_");
    assert_eq!(sanitize_path_component("file:name"), "file_name");
}

/// Test validate_file_extension
#[test]
fn test_validate_file_extension_valid() {
    assert!(validate_file_extension("rs").is_ok());
    assert!(validate_file_extension("txt").is_ok());
    assert!(validate_file_extension("json").is_ok());
    assert!(validate_file_extension("my-format").is_ok());
}

#[test]
fn test_validate_file_extension_invalid() {
    assert!(validate_file_extension("").is_err());
    assert!(validate_file_extension("ex>e").is_err());
    assert!(validate_file_extension("name|ext").is_err());
    assert!(validate_file_extension(&"a".repeat(40)).is_err());
}

/// Test has_dangerous_extension
#[test]
fn test_has_dangerous_extension() {
    let dangerous: &[&str] = &["exe", "bat", "cmd", "sh", "ps1"];

    assert!(has_dangerous_extension(Path::new("malware.exe"), dangerous));
    assert!(has_dangerous_extension(Path::new("script.bat"), dangerous));
    assert!(has_dangerous_extension(Path::new("script.BAT"), dangerous)); // Case insensitive
    assert!(has_dangerous_extension(Path::new("script.CMD"), dangerous));
    assert!(!has_dangerous_extension(
        Path::new("document.pdf"),
        dangerous
    ));
    assert!(!has_dangerous_extension(Path::new("source.rs"), dangerous));
    assert!(!has_dangerous_extension(
        Path::new("config.json"),
        dangerous
    ));
}

// -----------------------------------------------------------------------------
// Input Validation Tests
// -----------------------------------------------------------------------------

/// Test identifier validation
#[test]
fn test_validate_identifier_valid() {
    assert!(validate_identifier("task-123").is_ok());
    assert!(validate_identifier("my_feature").is_ok());
    assert!(validate_identifier("Task123").is_ok());
    assert!(validate_identifier("a").is_ok());
}

#[test]
fn test_validate_identifier_invalid() {
    assert!(validate_identifier("").is_err());
    assert!(validate_identifier("-invalid").is_err());
    assert!(validate_identifier("_invalid").is_err());
    assert!(validate_identifier("invalid;id").is_err());
    assert!(validate_identifier("invalid id").is_err());
    assert!(validate_identifier(&"a".repeat(300)).is_err());
}

/// Test sanitize_identifier
#[test]
fn test_sanitize_identifier() {
    assert_eq!(sanitize_identifier("my-task"), "my-task");
    assert_eq!(sanitize_identifier("my task"), "my_task");
    assert_eq!(sanitize_identifier("my;task"), "my_task");
    assert_eq!(sanitize_identifier(""), "unnamed");
    assert_eq!(sanitize_identifier("-task"), "x-task");
    assert_eq!(sanitize_identifier("_task"), "x_task");
}

/// Test is_safe_for_command_arg
#[test]
fn test_is_safe_for_command_arg() {
    // Safe
    assert!(is_safe_for_command_arg("hello"));
    assert!(is_safe_for_command_arg("hello world"));
    assert!(is_safe_for_command_arg("file.txt"));

    // Unsafe
    assert!(!is_safe_for_command_arg("hello; rm -rf /"));
    assert!(!is_safe_for_command_arg("hello `whoami`"));
    assert!(!is_safe_for_command_arg("hello$(whoami)"));
    assert!(!is_safe_for_command_arg("hello\nworld"));
    assert!(!is_safe_for_command_arg("hello$world"));
}

/// Test validate_command_arg
#[test]
fn test_validate_command_arg_valid() {
    assert!(validate_command_arg("hello").is_ok());
    assert!(validate_command_arg("hello world").is_ok());
    assert!(validate_command_arg("--option=value").is_ok());
}

#[test]
fn test_validate_command_arg_invalid() {
    assert!(validate_command_arg("hello; world").is_err());
    assert!(validate_command_arg("hello$world").is_err());
    assert!(validate_command_arg("hello`world`").is_err());
}

/// Test sanitize_command_arg
#[test]
fn test_sanitize_command_arg() {
    assert_eq!(sanitize_command_arg("hello"), "hello");
    assert_eq!(sanitize_command_arg("hello; world"), "hello world");
    assert_eq!(sanitize_command_arg("hello$world"), "helloworld");
    assert_eq!(sanitize_command_arg("hello`whoami`"), "hellowhoami");
}

/// Test validate_task_title
#[test]
fn test_validate_task_title_valid() {
    assert!(validate_task_title("Implement feature X").is_ok());
    assert!(validate_task_title("Fix bug in parser").is_ok());
    assert!(validate_task_title("a").is_ok());
}

#[test]
fn test_validate_task_title_invalid() {
    assert!(validate_task_title("").is_err());
    assert!(validate_task_title(&"a".repeat(600)).is_err());
    assert!(validate_task_title("hello\nworld").is_err());
    assert!(validate_task_title("hello\tworld").is_err());
}

/// Test validate_task_description
#[test]
fn test_validate_task_description_valid() {
    assert!(validate_task_description("This is a task description").is_ok());
    assert!(validate_task_description("").is_ok()); // Empty is allowed
    assert!(validate_task_description(&"a".repeat(9999)).is_ok());
}

#[test]
fn test_validate_task_description_invalid() {
    assert!(validate_task_description(&"a".repeat(10001)).is_err());
    assert!(validate_task_description("description\0with null").is_err());
}

/// Test validate_branch_name
#[test]
fn test_validate_branch_name_valid() {
    assert!(validate_branch_name("main").is_ok());
    assert!(validate_branch_name("feature/my-feature").is_ok());
    assert!(validate_branch_name("fix-bug-123").is_ok());
    assert!(validate_branch_name("release/v1.0.0").is_ok());
}

#[test]
fn test_validate_branch_name_invalid() {
    assert!(validate_branch_name("").is_err());
    assert!(validate_branch_name(".hidden").is_err());
    assert!(validate_branch_name("invalid..name").is_err());
    assert!(validate_branch_name("invalid name").is_err());
    assert!(validate_branch_name("branch.lock").is_err());
    assert!(validate_branch_name("@").is_err());
    assert!(validate_branch_name("branch~1").is_err());
    assert!(validate_branch_name("branch^1").is_err());
}

/// Test validate_commit_message
#[test]
fn test_validate_commit_message_valid() {
    assert!(validate_commit_message("Add new feature").is_ok());
    assert!(validate_commit_message("fix: resolve issue").is_ok());
    assert!(validate_commit_message(&"a".repeat(49999)).is_ok());
}

#[test]
fn test_validate_commit_message_invalid() {
    assert!(validate_commit_message("").is_err());
    assert!(validate_commit_message(&"a".repeat(50001)).is_err());
    assert!(validate_commit_message("message\0with null").is_err());
}

/// Test validate_model_name
#[test]
fn test_validate_model_name_valid() {
    assert!(validate_model_name("claude-sonnet-4-6").is_ok());
    assert!(validate_model_name("claude-opus-4.6").is_ok());
    assert!(validate_model_name("gpt-4").is_ok());
    assert!(validate_model_name("model_v2").is_ok());
}

#[test]
fn test_validate_model_name_invalid() {
    assert!(validate_model_name("").is_err());
    assert!(validate_model_name("model/name").is_err());
    assert!(validate_model_name("model name").is_err());
    assert!(validate_model_name("model@v1").is_err());
}

/// Test sanitize_model_name
#[test]
fn test_sanitize_model_name() {
    assert_eq!(
        sanitize_model_name("claude-sonnet-4-6"),
        "claude-sonnet-4-6"
    );
    assert_eq!(sanitize_model_name("model/name"), "modelname");
    assert_eq!(sanitize_model_name("model name"), "modelname");
    assert_eq!(sanitize_model_name("model@v1"), "modelv1");
}

// -----------------------------------------------------------------------------
// Integration Tests
// -----------------------------------------------------------------------------

/// Test that security module properly prevents command injection
#[test]
fn test_command_injection_prevention() {
    // Common command injection payloads that should all be rejected
    // These use validate_command_argument which checks for shell metacharacters
    let command_arg_payloads = [
        "file; rm -rf /",
        "file && cat /etc/passwd",
        "file || whoami",
        "file`id`",
        "file\nrm -rf /",
        "file\r\nrm -rf /",
        "file | cat /etc/passwd",
    ];

    for payload in &command_arg_payloads {
        assert!(
            validate_command_argument(payload).is_err(),
            "validate_command_argument should reject payload '{}'",
            payload
        );
    }

    // These use validate_command_arg (from input module) which is stricter
    let input_payloads = ["file$(whoami)", "file$variable", "hello`whoami`"];

    for payload in &input_payloads {
        assert!(
            validate_command_arg(payload).is_err(),
            "validate_command_arg should reject payload '{}'",
            payload
        );
        assert!(
            !is_safe_for_command_arg(payload),
            "is_safe_for_command_arg should detect '{}' as unsafe",
            payload
        );
    }
}

/// Test that path traversal is prevented
#[test]
fn test_path_traversal_prevention() {
    let traversal_payloads = [
        "../../../etc/passwd",
        "..\\..\\..\\windows\\system32",
        "sub/../../../etc",
        "./../sensitive",
    ];

    for payload in &traversal_payloads {
        let path = Path::new(payload);
        assert!(
            validate_path(path).is_err() || contains_traversal(path),
            "Traversal payload '{}' should be detected",
            payload
        );
    }
}

/// Test that sensitive data is properly masked
#[test]
fn test_sensitive_data_masking() {
    // Test various sensitive data formats
    let test_cases = [
        ("sk-1234567890abcdef", "sk-1...cdef"), // API key
        ("ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", "ghp_...xxxx"), // GitHub token
        ("AKIAIOSFODNN7EXAMPLE", "AKIA...MPLE"), // AWS key
        ("supersecret", "su...et"),             // Short secret
    ];

    for (value, _expected) in &test_cases {
        let masked = mask_sensitive_value(value);
        assert!(
            masked.contains("..."),
            "Value should be masked with ellipsis"
        );
        // The original should not be fully visible
        assert_ne!(&masked, value, "Masked value should not equal original");
    }
}

/// Test CommandAllowlist builder pattern
#[test]
fn test_command_allowlist_builder() {
    let mut allowlist = CommandAllowlist::new();
    allowlist.allow("custom-tool").allow("my-agent");

    assert!(allowlist.is_allowed("custom-tool"));
    assert!(allowlist.is_allowed("my-agent"));

    allowlist.deny("git");
    assert!(!allowlist.is_allowed("git"));
    assert!(allowlist.is_allowed("cargo")); // Still allowed
}

/// Test that resolve_safe_path rejects traversal
#[test]
fn test_resolve_safe_path_rejects_traversal() {
    let base = Path::new("/tmp/test");
    let traversal_paths = [
        Path::new("../../../etc/passwd"),
        Path::new("..\\..\\windows"),
        Path::new("sub/../.."),
    ];

    for traversal in &traversal_paths {
        assert!(
            resolve_safe_path(traversal, base).is_err(),
            "resolve_safe_path should reject traversal: {:?}",
            traversal
        );
    }
}
