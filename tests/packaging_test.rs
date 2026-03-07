// Tests for packaging configurations (Scoop and Homebrew)
//
// These tests verify that the packaging manifests are correctly configured:
// - Scoop manifest for Windows
// - Homebrew formula for macOS/Linux

use std::fs;
use std::path::Path;

/// Get packaging file path
fn packaging_path(subdir: &str, filename: &str) -> std::path::PathBuf {
    Path::new(".github").join(subdir).join(filename)
}

// ============================================================================
// Scoop Manifest Tests
// ============================================================================

mod scoop_tests {
    use super::*;

    fn load_scoop_manifest() -> serde_json::Value {
        let path = packaging_path("scoop", "ltmatrix.json");
        assert!(path.exists(), "Scoop manifest should exist at .github/scoop/ltmatrix.json");

        let content = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Failed to read Scoop manifest: {}", e));
        serde_json::from_str(&content)
            .unwrap_or_else(|e| panic!("Failed to parse Scoop manifest JSON: {}", e))
    }

    #[test]
    fn scoop_manifest_exists() {
        let path = packaging_path("scoop", "ltmatrix.json");
        assert!(path.exists(), "Scoop manifest should exist");
    }

    #[test]
    fn scoop_manifest_is_valid_json() {
        let manifest = load_scoop_manifest();
        assert!(manifest.is_object(), "Scoop manifest should be a JSON object");
    }

    #[test]
    fn scoop_manifest_has_required_fields() {
        let manifest = load_scoop_manifest();

        // Required fields for Scoop manifest
        assert!(manifest.get("version").is_some(), "Scoop manifest should have 'version'");
        assert!(manifest.get("description").is_some(), "Scoop manifest should have 'description'");
        assert!(manifest.get("homepage").is_some(), "Scoop manifest should have 'homepage'");
        assert!(manifest.get("license").is_some(), "Scoop manifest should have 'license'");
        assert!(manifest.get("architecture").is_some(), "Scoop manifest should have 'architecture'");
    }

    #[test]
    fn scoop_manifest_has_correct_license() {
        let manifest = load_scoop_manifest();
        let license = manifest.get("license").and_then(|l| l.as_str());
        assert_eq!(license, Some("MIT"), "Scoop manifest should specify MIT license");
    }

    #[test]
    fn scoop_manifest_has_correct_homepage() {
        let manifest = load_scoop_manifest();
        let homepage = manifest.get("homepage").and_then(|h| h.as_str());
        assert!(
            homepage.map(|h| h.contains("github.com/bigfish/ltmatrix")).unwrap_or(false),
            "Scoop manifest homepage should point to GitHub repository"
        );
    }

    #[test]
    fn scoop_manifest_has_64bit_architecture() {
        let manifest = load_scoop_manifest();
        let arch = manifest.get("architecture").expect("Should have architecture");
        let bit64 = arch.get("64bit").expect("Should have 64bit architecture");

        // Check URL pattern
        let url = bit64.get("url").and_then(|u| u.as_str()).expect("64bit should have URL");
        assert!(
            url.contains("x86_64-pc-windows-msvc"),
            "64bit URL should target x86_64-pc-windows-msvc"
        );
        assert!(
            url.contains("github.com/bigfish/ltmatrix/releases"),
            "64bit URL should point to GitHub releases"
        );

        // Check bin configuration
        let bin = bit64.get("bin").expect("64bit should have bin configuration");
        assert!(bin.is_array(), "bin should be an array");
    }

    #[test]
    fn scoop_manifest_has_arm64_architecture() {
        let manifest = load_scoop_manifest();
        let arch = manifest.get("architecture").expect("Should have architecture");
        let arm64 = arch.get("arm64").expect("Should have arm64 architecture");

        // Check URL pattern
        let url = arm64.get("url").and_then(|u| u.as_str()).expect("arm64 should have URL");
        assert!(
            url.contains("aarch64-pc-windows-msvc"),
            "arm64 URL should target aarch64-pc-windows-msvc"
        );
        assert!(
            url.contains("github.com/bigfish/ltmatrix/releases"),
            "arm64 URL should point to GitHub releases"
        );
    }

    #[test]
    fn scoop_manifest_has_autoupdate() {
        let manifest = load_scoop_manifest();
        let autoupdate = manifest.get("autoupdate");
        assert!(autoupdate.is_some(), "Scoop manifest should have autoupdate configuration");

        if let Some(au) = autoupdate {
            let arch = au.get("architecture").expect("autoupdate should have architecture");
            assert!(
                arch.get("64bit").is_some(),
                "autoupdate should have 64bit configuration"
            );
            assert!(
                arch.get("arm64").is_some(),
                "autoupdate should have arm64 configuration"
            );
        }
    }

    #[test]
    fn scoop_manifest_has_checkver() {
        let manifest = load_scoop_manifest();
        let checkver = manifest.get("checkver");
        assert!(checkver.is_some(), "Scoop manifest should have checkver configuration");

        if let Some(cv) = checkver {
            assert!(
                cv.get("github").is_some(),
                "checkver should use GitHub"
            );
        }
    }

    #[test]
    fn scoop_manifest_autoupdate_uses_version_variable() {
        let manifest = load_scoop_manifest();
        let autoupdate = manifest.get("autoupdate").expect("Should have autoupdate");
        let arch = autoupdate.get("architecture").expect("autoupdate should have architecture");

        // Check 64bit URL uses $version
        let bit64 = arch.get("64bit").expect("Should have 64bit");
        let url = bit64.get("url").and_then(|u| u.as_str()).expect("Should have URL");
        assert!(
            url.contains("$version"),
            "autoupdate URL should use $version variable"
        );

        // Check arm64 URL uses $version
        let arm64 = arch.get("arm64").expect("Should have arm64");
        let url = arm64.get("url").and_then(|u| u.as_str()).expect("Should have URL");
        assert!(
            url.contains("$version"),
            "autoupdate URL should use $version variable"
        );
    }

    #[test]
    fn scoop_manifest_has_notes() {
        let manifest = load_scoop_manifest();
        let notes = manifest.get("notes");
        assert!(notes.is_some(), "Scoop manifest should have notes about dependencies");
    }

    #[test]
    fn scoop_manifest_has_suggests() {
        let manifest = load_scoop_manifest();
        let suggest = manifest.get("suggest");
        assert!(suggest.is_some(), "Scoop manifest should have suggest field for dependencies");
    }
}

// ============================================================================
// Homebrew Formula Tests
// ============================================================================

mod homebrew_tests {
    use super::*;

    fn load_homebrew_formula() -> String {
        let path = packaging_path("homebrew", "ltmatrix.rb");
        assert!(path.exists(), "Homebrew formula should exist at .github/homebrew/ltmatrix.rb");

        fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Failed to read Homebrew formula: {}", e))
    }

    #[test]
    fn homebrew_formula_exists() {
        let path = packaging_path("homebrew", "ltmatrix.rb");
        assert!(path.exists(), "Homebrew formula should exist");
    }

    #[test]
    fn homebrew_formula_has_class_definition() {
        let formula = load_homebrew_formula();
        assert!(
            formula.contains("class Ltmatrix < Formula"),
            "Homebrew formula should define Ltmatrix class"
        );
    }

    #[test]
    fn homebrew_formula_has_description() {
        let formula = load_homebrew_formula();
        assert!(
            formula.contains("desc "),
            "Homebrew formula should have desc"
        );
    }

    #[test]
    fn homebrew_formula_has_homepage() {
        let formula = load_homebrew_formula();
        assert!(
            formula.contains("homepage "),
            "Homebrew formula should have homepage"
        );
        assert!(
            formula.contains("github.com/bigfish/ltmatrix"),
            "Homepage should point to GitHub repository"
        );
    }

    #[test]
    fn homebrew_formula_has_license() {
        let formula = load_homebrew_formula();
        assert!(
            formula.contains("license \"MIT\"") || formula.contains("license 'MIT'"),
            "Homebrew formula should specify MIT license"
        );
    }

    #[test]
    fn homebrew_formula_has_version() {
        let formula = load_homebrew_formula();
        assert!(
            formula.contains("version "),
            "Homebrew formula should have version"
        );
    }

    #[test]
    fn homebrew_formula_has_url_for_intel() {
        let formula = load_homebrew_formula();
        assert!(
            formula.contains("x86_64-apple-darwin"),
            "Homebrew formula should have URL for Intel Macs"
        );
    }

    #[test]
    fn homebrew_formula_has_url_for_arm() {
        let formula = load_homebrew_formula();
        assert!(
            formula.contains("aarch64-apple-darwin"),
            "Homebrew formula should have URL for Apple Silicon"
        );
    }

    #[test]
    fn homebrew_formula_has_install_method() {
        let formula = load_homebrew_formula();
        assert!(
            formula.contains("def install"),
            "Homebrew formula should have install method"
        );
        assert!(
            formula.contains("bin.install"),
            "Homebrew formula should install binary"
        );
    }

    #[test]
    fn homebrew_formula_has_test() {
        let formula = load_homebrew_formula();
        assert!(
            formula.contains("test do"),
            "Homebrew formula should have test block"
        );
    }

    #[test]
    fn homebrew_formula_has_caveats() {
        let formula = load_homebrew_formula();
        assert!(
            formula.contains("def caveats"),
            "Homebrew formula should have caveats about dependencies"
        );
    }

    #[test]
    fn homebrew_formula_has_livecheck() {
        let formula = load_homebrew_formula();
        assert!(
            formula.contains("livecheck do"),
            "Homebrew formula should have livecheck for version updates"
        );
    }

    #[test]
    fn homebrew_formula_uses_github_releases() {
        let formula = load_homebrew_formula();
        assert!(
            formula.contains("github.com/bigfish/ltmatrix/releases"),
            "Homebrew formula should download from GitHub releases"
        );
    }

    #[test]
    fn homebrew_formula_has_completions() {
        let formula = load_homebrew_formula();
        assert!(
            formula.contains("completions"),
            "Homebrew formula should install shell completions"
        );
    }

    #[test]
    fn homebrew_formula_specifies_macos_dependency() {
        let formula = load_homebrew_formula();
        assert!(
            formula.contains("depends_on :macos") || formula.contains("depends_on \"macos\""),
            "Homebrew formula should specify macOS dependency"
        );
    }
}

// ============================================================================
// README Installation Instructions Tests
// ============================================================================

mod readme_tests {
    use super::*;

    fn load_readme() -> String {
        let path = Path::new("README.md");
        assert!(path.exists(), "README.md should exist");

        fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("Failed to read README.md: {}", e))
    }

    #[test]
    fn readme_has_installation_section() {
        let readme = load_readme();
        assert!(
            readme.contains("## Installation"),
            "README should have Installation section"
        );
    }

    #[test]
    fn readme_has_homebrew_instructions() {
        let readme = load_readme();
        assert!(
            readme.contains("brew tap") && readme.contains("brew install ltmatrix"),
            "README should have Homebrew installation instructions"
        );
    }

    #[test]
    fn readme_has_scoop_instructions() {
        let readme = load_readme();
        assert!(
            readme.contains("scoop bucket add") || readme.contains("scoop install"),
            "README should have Scoop installation instructions"
        );
    }

    #[test]
    fn readme_has_cargo_instructions() {
        let readme = load_readme();
        assert!(
            readme.contains("cargo install ltmatrix"),
            "README should have Cargo installation instructions"
        );
    }

    #[test]
    fn readme_mentions_supported_platforms() {
        let readme = load_readme();
        assert!(
            readme.contains("Windows") && readme.contains("macOS") && readme.contains("Linux"),
            "README should mention all supported platforms"
        );
    }

    #[test]
    fn readme_mentions_architectures() {
        let readme = load_readme();
        assert!(
            readme.contains("x86_64") || readme.contains("64") || readme.contains("Intel"),
            "README should mention x86_64/64-bit support"
        );
        assert!(
            readme.contains("ARM64") || readme.contains("aarch64") || readme.contains("Apple Silicon"),
            "README should mention ARM64 support"
        );
    }
}
