// Tests for Homebrew formula
//
// These tests verify that the Homebrew formula meets the acceptance criteria:
// - Formula exists at .github/homebrew/ltmatrix.rb
// - Contains proper class definition
// - Supports macOS Intel and Apple Silicon
// - Supports Linux x86_64 and ARM64
// - Includes download URLs for GitHub releases
// - Has proper test block
// - Has proper installation logic

use std::fs;
use std::path::Path;

/// Get the Homebrew formula file path
fn formula_path() -> std::path::PathBuf {
    Path::new(".github/homebrew/ltmatrix.rb").to_path_buf()
}

/// Load the Homebrew formula content
fn load_formula() -> String {
    let path = formula_path();
    assert!(
        path.exists(),
        "Homebrew formula should exist at .github/homebrew/ltmatrix.rb"
    );
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read formula file {:?}: {}", path, e))
}

// ============================================================================
// Basic Structure Tests
// ============================================================================

mod formula_structure_tests {
    use super::*;

    #[test]
    fn formula_file_exists() {
        let path = formula_path();
        assert!(
            path.exists(),
            "Homebrew formula file should exist at .github/homebrew/ltmatrix.rb"
        );
    }

    #[test]
    fn formula_is_valid_ruby_class() {
        let formula = load_formula();
        assert!(
            formula.contains("class Ltmatrix < Formula"),
            "Formula should define Ltmatrix class"
        );
    }

    #[test]
    fn formula_has_desc() {
        let formula = load_formula();
        assert!(formula.contains("desc "), "Formula should have desc");
        assert!(
            formula.to_lowercase().contains("agent"),
            "Description should mention agent"
        );
    }

    #[test]
    fn formula_has_homepage() {
        let formula = load_formula();
        assert!(
            formula.contains("homepage "),
            "Formula should have homepage"
        );
        assert!(formula.contains("github.com"), "Homepage should be GitHub");
    }

    #[test]
    fn formula_has_mit_license() {
        let formula = load_formula();
        assert!(formula.contains("license "), "Formula should have license");
        assert!(
            formula.contains("\"MIT\""),
            "Formula should use MIT license"
        );
    }

    #[test]
    fn formula_has_version() {
        let formula = load_formula();
        assert!(formula.contains("version "), "Formula should have version");
    }
}

// ============================================================================
// Platform Support Tests
// ============================================================================

mod platform_support_tests {
    use super::*;

    #[test]
    fn formula_supports_macos_intel() {
        let formula = load_formula();
        assert!(
            formula.contains("Hardware::CPU.intel?"),
            "Formula should check for Intel CPU"
        );
        assert!(
            formula.contains("OS.mac?"),
            "Formula should check for macOS"
        );
        assert!(
            formula.contains("x86_64-apple-darwin"),
            "Formula should have macOS Intel download"
        );
    }

    #[test]
    fn formula_supports_macos_arm() {
        let formula = load_formula();
        assert!(
            formula.contains("Hardware::CPU.arm?"),
            "Formula should check for ARM CPU"
        );
        assert!(
            formula.contains("aarch64-apple-darwin"),
            "Formula should have macOS ARM download"
        );
    }

    #[test]
    fn formula_supports_linux_x86_64() {
        let formula = load_formula();
        assert!(
            formula.contains("OS.linux?"),
            "Formula should check for Linux"
        );
        assert!(
            formula.contains("x86_64") && formula.contains("linux"),
            "Formula should have Linux x86_64 download"
        );
    }

    #[test]
    fn formula_supports_linux_arm64() {
        let formula = load_formula();
        assert!(
            formula.contains("aarch64") && formula.contains("linux"),
            "Formula should have Linux ARM64 download"
        );
    }

    #[test]
    fn formula_uses_musl_for_linux() {
        let formula = load_formula();
        // Musl builds are preferred for better compatibility
        assert!(
            formula.contains("musl"),
            "Formula should use musl builds for Linux (static linking)"
        );
    }
}

// ============================================================================
// Download URL Tests
// ============================================================================

mod download_url_tests {
    use super::*;

    #[test]
    fn formula_uses_github_releases() {
        let formula = load_formula();
        assert!(
            formula.contains("github.com/bigfish/ltmatrix/releases"),
            "Formula should use GitHub releases"
        );
    }

    #[test]
    fn formula_urls_include_version() {
        let formula = load_formula();
        assert!(
            formula.contains("v#{version}"),
            "Formula should use version in download URL"
        );
    }

    #[test]
    fn formula_has_sha256_checksums() {
        let formula = load_formula();
        // Count sha256 occurrences - should have one per platform
        let sha_count = formula.matches("sha256 ").count();
        assert!(
            sha_count >= 4,
            "Formula should have sha256 checksums for all platforms (found {})",
            sha_count
        );
    }

    #[test]
    fn formula_uses_tar_gz_archives() {
        let formula = load_formula();
        assert!(
            formula.contains(".tar.gz"),
            "Formula should use tar.gz archives"
        );
    }

    #[test]
    fn formula_download_urls_are_valid_format() {
        let formula = load_formula();
        // Check that URLs follow the expected pattern
        let url_pattern = "https://github.com/bigfish/ltmatrix/releases/download";
        assert!(
            formula.contains(url_pattern),
            "Formula should have valid GitHub release URL pattern"
        );
    }
}

// ============================================================================
// Installation Tests
// ============================================================================

mod installation_tests {
    use super::*;

    #[test]
    fn formula_has_install_method() {
        let formula = load_formula();
        assert!(
            formula.contains("def install"),
            "Formula should have install method"
        );
    }

    #[test]
    fn formula_installs_binary() {
        let formula = load_formula();
        assert!(
            formula.contains("bin.install"),
            "Formula should install binary to bin"
        );
        assert!(
            formula.contains("\"ltmatrix\""),
            "Formula should install ltmatrix binary"
        );
    }

    #[test]
    fn formula_supports_head_build() {
        let formula = load_formula();
        assert!(
            formula.contains("head do"),
            "Formula should support head builds"
        );
        assert!(
            formula.contains("depends_on \"rust\" => :build"),
            "Formula should depend on rust for head builds"
        );
    }

    #[test]
    fn formula_handles_head_build_from_source() {
        let formula = load_formula();
        assert!(
            formula.contains("build.head?"),
            "Formula should check for head build"
        );
        assert!(
            formula.contains("cargo"),
            "Formula should use cargo for head builds"
        );
    }
}

// ============================================================================
// Shell Completion Tests
// ============================================================================

mod completion_tests {
    use super::*;

    #[test]
    fn formula_generates_completions() {
        let formula = load_formula();
        assert!(
            formula.contains("generate_completions"),
            "Formula should have completion generation method"
        );
    }

    #[test]
    fn formula_supports_bash_completion() {
        let formula = load_formula();
        assert!(
            formula.contains("\"bash\""),
            "Formula should support bash completions"
        );
        assert!(
            formula.contains("bash_completion"),
            "Formula should install bash completions"
        );
    }

    #[test]
    fn formula_supports_fish_completion() {
        let formula = load_formula();
        assert!(
            formula.contains("\"fish\""),
            "Formula should support fish completions"
        );
        assert!(
            formula.contains("fish_completion"),
            "Formula should install fish completions"
        );
    }

    #[test]
    fn formula_supports_zsh_completion() {
        let formula = load_formula();
        assert!(
            formula.contains("\"zsh\""),
            "Formula should support zsh completions"
        );
        assert!(
            formula.contains("zsh_completion"),
            "Formula should install zsh completions"
        );
    }
}

// ============================================================================
// Test Block Tests
// ============================================================================

mod test_block_tests {
    use super::*;

    #[test]
    fn formula_has_test_block() {
        let formula = load_formula();
        assert!(
            formula.contains("test do"),
            "Formula should have test block"
        );
    }

    #[test]
    fn formula_tests_version() {
        let formula = load_formula();
        assert!(
            formula.contains("--version"),
            "Formula test should check version"
        );
    }

    #[test]
    fn formula_tests_help() {
        let formula = load_formula();
        assert!(formula.contains("--help"), "Formula test should check help");
    }

    #[test]
    fn formula_uses_assert_match() {
        let formula = load_formula();
        assert!(
            formula.contains("assert_match"),
            "Formula test should use assertions"
        );
    }
}

// ============================================================================
// Code Quality Tests
// ============================================================================

mod code_quality_tests {
    use super::*;

    #[test]
    fn formula_has_frozen_string_literal() {
        let formula = load_formula();
        assert!(
            formula.contains("# frozen_string_literal: true"),
            "Formula should have frozen string literal comment"
        );
    }

    #[test]
    fn formula_has_typed_sigil() {
        let formula = load_formula();
        assert!(
            formula.contains("# typed:"),
            "Formula should have Sorbet type annotation"
        );
    }

    #[test]
    fn formula_class_name_follows_convention() {
        let formula = load_formula();
        // Homebrew requires capitalized class names
        assert!(
            formula.contains("class Ltmatrix < Formula"),
            "Formula class should be capitalized"
        );
    }

    #[test]
    fn formula_no_runtime_dependencies_for_static_binary() {
        let formula = load_formula();
        // Static binaries shouldn't need runtime dependencies
        // The formula should mention this
        assert!(
            formula.contains("No runtime dependencies") || formula.contains("statically linked"),
            "Formula should note that binary is statically linked"
        );
    }

    #[test]
    fn formula_has_documentation_comments() {
        let formula = load_formula();
        // Formula should have comments explaining installation methods
        assert!(
            formula.contains("# Homebrew formula"),
            "Formula should have descriptive header comment"
        );
        assert!(
            formula.contains("brew install"),
            "Formula should show installation example"
        );
    }
}

// ============================================================================
// README Integration Tests
// ============================================================================

mod readme_integration_tests {
    use super::*;

    fn load_readme() -> String {
        let path = Path::new("README.md");
        assert!(path.exists(), "README.md should exist");
        fs::read_to_string(path).unwrap_or_else(|e| panic!("Failed to read README.md: {}", e))
    }

    #[test]
    fn readme_has_homebrew_installation_section() {
        let readme = load_readme();
        assert!(
            readme.contains("Homebrew"),
            "README should mention Homebrew"
        );
        assert!(
            readme.contains("brew install"),
            "README should show brew install command"
        );
    }

    #[test]
    fn readme_has_tap_command() {
        let readme = load_readme();
        assert!(
            readme.contains("brew tap"),
            "README should show brew tap command"
        );
    }

    #[test]
    fn readme_homebrew_is_recommended() {
        let readme = load_readme();
        // Check that Homebrew is listed as recommended or first option
        let homebrew_pos = readme.find("Homebrew").unwrap_or(usize::MAX);
        let cargo_pos = readme.find("cargo install").unwrap_or(usize::MAX);
        assert!(
            homebrew_pos < cargo_pos,
            "Homebrew should be listed before cargo install as the recommended method"
        );
    }

    #[test]
    fn readme_has_tap_name() {
        let readme = load_readme();
        assert!(
            readme.contains("bigfish/ltmatrix"),
            "README should reference the tap name"
        );
    }
}
