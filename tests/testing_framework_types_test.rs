//! Tests for the Framework enum type
//!
//! This test suite verifies the Framework enum's behavior,
//! including its variants, methods, traits, and serialization.

use ltmatrix::testing::Framework;
use serde_json::{from_str, to_string};

#[test]
fn test_framework_all_variants_exist() {
    // Verify all expected framework variants are present
    let frameworks = vec![
        Framework::Pytest,
        Framework::Npm,
        Framework::Go,
        Framework::Cargo,
        Framework::None,
    ];

    // Just collecting them ensures the variants exist and can be instantiated
    assert_eq!(frameworks.len(), 5);
}

#[test]
fn test_framework_name_method() {
    // Test display names for each framework variant
    assert_eq!(Framework::Pytest.name(), "pytest");
    assert_eq!(Framework::Npm.name(), "npm");
    assert_eq!(Framework::Go.name(), "go test");
    assert_eq!(Framework::Cargo.name(), "cargo test");
    assert_eq!(Framework::None.name(), "none");
}

#[test]
fn test_framework_command_method() {
    // Test that each framework returns the correct command

    // Pytest should return "pytest -v"
    let pytest_cmd = Framework::Pytest.command();
    assert_eq!(pytest_cmd, &["pytest", "-v"]);
    assert_eq!(pytest_cmd.len(), 2);

    // Npm should return "npm test"
    let npm_cmd = Framework::Npm.command();
    assert_eq!(npm_cmd, &["npm", "test"]);
    assert_eq!(npm_cmd.len(), 2);

    // Go should return "go test ./..."
    let go_cmd = Framework::Go.command();
    assert_eq!(go_cmd, &["go", "test", "./..."]);
    assert_eq!(go_cmd.len(), 3);

    // Cargo should return "cargo test"
    let cargo_cmd = Framework::Cargo.command();
    assert_eq!(cargo_cmd, &["cargo", "test"]);
    assert_eq!(cargo_cmd.len(), 2);

    // None should return an empty array
    let none_cmd = Framework::None.command();
    assert_eq!(none_cmd, &[] as &[&str]);
    assert_eq!(none_cmd.len(), 0);
}

#[test]
fn test_framework_display_trait() {
    // Test the Display trait implementation

    assert_eq!(format!("{}", Framework::Pytest), "pytest");
    assert_eq!(format!("{}", Framework::Npm), "npm");
    assert_eq!(format!("{}", Framework::Go), "go test");
    assert_eq!(format!("{}", Framework::Cargo), "cargo test");
    assert_eq!(format!("{}", Framework::None), "none");

    // Test in a formatted string context
    let fw = Framework::Cargo;
    assert_eq!(
        format!("Detected framework: {}", fw),
        "Detected framework: cargo test"
    );
}

#[test]
fn test_framework_equality() {
    // Test PartialEq implementation
    assert_eq!(Framework::Pytest, Framework::Pytest);
    assert_eq!(Framework::Npm, Framework::Npm);
    assert_eq!(Framework::Go, Framework::Go);
    assert_eq!(Framework::Cargo, Framework::Cargo);
    assert_eq!(Framework::None, Framework::None);

    // Test inequality
    assert_ne!(Framework::Pytest, Framework::Npm);
    assert_ne!(Framework::Npm, Framework::Go);
    assert_ne!(Framework::Go, Framework::Cargo);
    assert_ne!(Framework::Cargo, Framework::None);
    assert_ne!(Framework::None, Framework::Pytest);
}

#[test]
fn test_framework_clone() {
    // Test that Framework implements Clone correctly
    let original = Framework::Cargo;
    let cloned = original.clone();

    assert_eq!(original, cloned);

    // Test that the clone is independent
    let mut framework = Framework::Pytest;
    let copy = framework.clone();
    // This just verifies cloning works; Framework has no internal state
    assert_eq!(framework, copy);
}

#[test]
fn test_framework_copy() {
    // Test that Framework implements Copy (it should derive Copy)
    let original = Framework::Go;
    let copied = original; // This should work because of Copy

    // Original should still be usable because of Copy
    assert_eq!(original, Framework::Go);
    assert_eq!(copied, Framework::Go);
}

#[test]
fn test_framework_debug_format() {
    // Test Debug trait implementation
    assert_eq!(format!("{:?}", Framework::Pytest), "Pytest");
    assert_eq!(format!("{:?}", Framework::Npm), "Npm");
    assert_eq!(format!("{:?}", Framework::Go), "Go");
    assert_eq!(format!("{:?}", Framework::Cargo), "Cargo");
    assert_eq!(format!("{:?}", Framework::None), "None");

    // Test alternate debug format
    assert_eq!(format!("{:#?}", Framework::Cargo), "Cargo");
}

#[test]
fn test_framework_serialization() {
    // Test that Framework can be serialized to JSON
    let fw = Framework::Cargo;
    let json = to_string(&fw).expect("Failed to serialize Framework");

    assert_eq!(json, "\"Cargo\"");
}

#[test]
fn test_framework_deserialization() {
    // Test that Framework can be deserialized from JSON

    // Test each variant
    let pytest: Framework = from_str("\"Pytest\"").expect("Failed to deserialize Pytest");
    assert_eq!(pytest, Framework::Pytest);

    let npm: Framework = from_str("\"Npm\"").expect("Failed to deserialize Npm");
    assert_eq!(npm, Framework::Npm);

    let go: Framework = from_str("\"Go\"").expect("Failed to deserialize Go");
    assert_eq!(go, Framework::Go);

    let cargo: Framework = from_str("\"Cargo\"").expect("Failed to deserialize Cargo");
    assert_eq!(cargo, Framework::Cargo);

    let none: Framework = from_str("\"None\"").expect("Failed to deserialize None");
    assert_eq!(none, Framework::None);
}

#[test]
fn test_framework_roundtrip_serialization() {
    // Test that serialization and deserialization are symmetric
    let frameworks = vec![
        Framework::Pytest,
        Framework::Npm,
        Framework::Go,
        Framework::Cargo,
        Framework::None,
    ];

    for fw in frameworks {
        let json = to_string(&fw).expect("Failed to serialize");
        let deserialized: Framework = from_str(&json).expect("Failed to deserialize");
        assert_eq!(fw, deserialized);
    }
}

#[test]
fn test_framework_hash_trait() {
    // Test that Framework can be used in hash-based collections
    use std::collections::{HashMap, HashSet};

    // Test HashSet
    let mut set = HashSet::new();
    set.insert(Framework::Pytest);
    set.insert(Framework::Npm);
    set.insert(Framework::Go);
    set.insert(Framework::Cargo);
    set.insert(Framework::None);
    assert_eq!(set.len(), 5);

    // Test duplicate detection
    set.insert(Framework::Pytest);
    assert_eq!(set.len(), 5); // Still 5, duplicates ignored

    // Test HashMap
    let mut map = HashMap::new();
    map.insert(Framework::Pytest, "Python");
    map.insert(Framework::Cargo, "Rust");
    assert_eq!(map.get(&Framework::Pytest), Some(&"Python"));
    assert_eq!(map.get(&Framework::Cargo), Some(&"Rust"));
    assert_eq!(map.get(&Framework::Npm), None);
}

#[test]
fn test_framework_command_execution_formatting() {
    // Test that command arrays can be properly formatted for shell execution
    let fw = Framework::Cargo;
    let cmd = fw.command();
    let cmd_string = cmd.join(" ");

    assert_eq!(cmd_string, "cargo test");

    // Go test with path
    let go_fw = Framework::Go;
    let go_cmd = go_fw.command();
    let go_cmd_string = go_cmd.join(" ");
    assert_eq!(go_cmd_string, "go test ./...");

    // Pytest with verbose flag
    let pytest_fw = Framework::Pytest;
    let pytest_cmd = pytest_fw.command();
    let pytest_cmd_string = pytest_cmd.join(" ");
    assert_eq!(pytest_cmd_string, "pytest -v");
}

#[test]
fn test_framework_none_returns_empty_command() {
    // Framework::None should return an empty command array
    let cmd = Framework::None.command();
    assert!(cmd.is_empty());
    assert_eq!(cmd.len(), 0);
}

#[test]
fn test_framework_match_completeness() {
    // Test that all framework variants can be matched exhaustively
    let all_frameworks = vec![
        Framework::Pytest,
        Framework::Npm,
        Framework::Go,
        Framework::Cargo,
        Framework::None,
    ];

    for fw in all_frameworks {
        let _name = match fw {
            Framework::Pytest => "pytest",
            Framework::Npm => "npm",
            Framework::Go => "go test",
            Framework::Cargo => "cargo test",
            Framework::None => "none",
        };

        // Verify name() method matches
        assert_eq!(fw.name(), _name);
    }
}

#[test]
fn test_framework_use_in_match_statement() {
    // Test using Framework in a match statement (common pattern)
    fn get_test_file_pattern(fw: &Framework) -> &'static str {
        match fw {
            Framework::Pytest => "test_*.py",
            Framework::Npm => "*.test.js",
            Framework::Go => "*_test.go",
            Framework::Cargo => "*_test.rs",
            Framework::None => "",
        }
    }

    assert_eq!(get_test_file_pattern(&Framework::Pytest), "test_*.py");
    assert_eq!(get_test_file_pattern(&Framework::Npm), "*.test.js");
    assert_eq!(get_test_file_pattern(&Framework::Go), "*_test.go");
    assert_eq!(get_test_file_pattern(&Framework::Cargo), "*_test.rs");
    assert_eq!(get_test_file_pattern(&Framework::None), "");
}

#[test]
fn test_framework_command_immutability() {
    // Test that command() returns a reference that cannot be used to modify
    // internal state (it returns &[&str])
    let fw = Framework::Cargo;
    let cmd1 = fw.command();
    let cmd2 = fw.command();

    // Should return the same reference each time
    assert_eq!(cmd1.as_ptr(), cmd2.as_ptr());
}

#[test]
fn test_framework_name_returns_static_str() {
    // Test that name() returns a static string with valid lifetime
    let fw = Framework::Pytest;
    let name = fw.name();

    // The returned reference should be valid
    assert!(!name.is_empty());
    assert_eq!(name.len(), 6); // "pytest"
}
