// Tests for license file and headers implementation
// Verifies that the MIT license is properly added to the project

use std::fs;
use std::path::Path;

/// Check if LICENSE file exists and contains MIT license
#[test]
fn test_license_file_exists() {
    let license_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("LICENSE");
    assert!(
        license_path.exists(),
        "LICENSE file should exist at project root"
    );
}

#[test]
fn test_license_contains_mit_text() {
    let license_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("LICENSE");
    let content = fs::read_to_string(&license_path)
        .expect("Should be able to read LICENSE file");

    // MIT license must contain these key elements
    assert!(
        content.contains("MIT License"),
        "LICENSE should contain 'MIT License'"
    );
    assert!(
        content.contains("Permission is hereby granted, free of charge"),
        "LICENSE should contain standard MIT grant text"
    );
    assert!(
        content.contains("without restriction, including without limitation the rights"),
        "LICENSE should contain rights clause"
    );
    assert!(
        content.contains("THE SOFTWARE IS PROVIDED \"AS IS\""),
        "LICENSE should contain warranty disclaimer"
    );
}

#[test]
fn test_license_has_copyright_year() {
    let license_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("LICENSE");
    let content = fs::read_to_string(&license_path)
        .expect("Should be able to read LICENSE file");

    // Check for copyright year (current year 2026 or a range)
    let has_year = content.contains("2026") || content.contains("2025") || content.contains("2024");
    assert!(
        has_year,
        "LICENSE should contain a copyright year"
    );
}

/// Check if LICENSE-3RDPARTY exists for dependency licensing
#[test]
fn test_third_party_license_file_exists() {
    let third_party_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("LICENSE-3RDPARTY");
    assert!(
        third_party_path.exists(),
        "LICENSE-3RDPARTY file should exist at project root"
    );
}

#[test]
fn test_third_party_license_lists_dependencies() {
    let third_party_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("LICENSE-3RDPARTY");
    let content = fs::read_to_string(&third_party_path)
        .expect("Should be able to read LICENSE-3RDPARTY file");

    // Should list some of the key dependencies
    let key_deps = ["tokio", "serde", "clap", "anyhow"];
    let mut found_count = 0;

    for dep in &key_deps {
        if content.to_lowercase().contains(dep) {
            found_count += 1;
        }
    }

    assert!(
        found_count >= 2,
        "LICENSE-3RDPARTY should list at least 2 key dependencies, found {}",
        found_count
    );
}

/// Check if README documents the license
#[test]
fn test_readme_documents_license() {
    let readme_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md");
    let content = fs::read_to_string(&readme_path)
        .expect("Should be able to read README.md");

    let content_lower = content.to_lowercase();
    assert!(
        content_lower.contains("license"),
        "README should mention 'license'"
    );
    assert!(
        content_lower.contains("mit"),
        "README should specify MIT license"
    );
}

/// Check if main source files have license headers
#[test]
fn test_main_rs_has_license_header() {
    let main_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/main.rs");
    let content = fs::read_to_string(&main_path)
        .expect("Should be able to read src/main.rs");

    assert!(
        content.contains("Copyright") || content.contains("SPDX-License-Identifier") || content.contains("MIT"),
        "src/main.rs should have a license header with Copyright, SPDX identifier, or MIT reference"
    );
}

#[test]
fn test_lib_rs_has_license_header() {
    let lib_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    let content = fs::read_to_string(&lib_path)
        .expect("Should be able to read src/lib.rs");

    assert!(
        content.contains("Copyright") || content.contains("SPDX-License-Identifier") || content.contains("MIT"),
        "src/lib.rs should have a license header with Copyright, SPDX identifier, or MIT reference"
    );
}

/// Verify all .rs files in src/ have license headers
#[test]
fn test_all_source_files_have_license_headers() {
    let src_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

    let mut files_checked = 0;
    let mut files_without_header = Vec::new();

    fn check_rs_files(dir: &Path, files_checked: &mut usize, files_without_header: &mut Vec<String>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    check_rs_files(&path, files_checked, files_without_header);
                } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
                    *files_checked += 1;
                    if let Ok(content) = fs::read_to_string(&path) {
                        let has_header = content.contains("Copyright")
                            || content.contains("SPDX-License-Identifier")
                            || content.contains("MIT License")
                            || content.contains("Licensed under");

                        if !has_header {
                            let relative = path.strip_prefix(env!("CARGO_MANIFEST_DIR"))
                                .unwrap_or(&path);
                            files_without_header.push(relative.display().to_string());
                        }
                    }
                }
            }
        }
    }

    check_rs_files(&src_path, &mut files_checked, &mut files_without_header);

    assert!(
        files_checked > 0,
        "Should have found at least one .rs file to check"
    );

    assert!(
        files_without_header.is_empty(),
        "The following source files are missing license headers: {:?}",
        files_without_header
    );
}

/// Verify Cargo.toml has license field
#[test]
fn test_cargo_toml_has_license_field() {
    let cargo_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let content = fs::read_to_string(&cargo_path)
        .expect("Should be able to read Cargo.toml");

    assert!(
        content.contains("license") && content.contains("MIT"),
        "Cargo.toml should have license = \"MIT\" field"
    );
}