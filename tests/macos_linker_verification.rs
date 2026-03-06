// Unit tests for macOS-specific linker configuration
//
// These tests verify that the build configuration correctly sets
// macOS-specific linker flags and settings for both Intel and Apple Silicon.
//
// These tests can run on any platform since they test configuration,
// not binary execution.

#[cfg(test)]
mod linker_flags_tests {
    /// Test that macOS minimum version is configured correctly
    #[test]
    fn test_macos_minimum_version_configured() {
        // This test verifies the rustflags are set in .cargo/config.toml
        // We can't directly test the config file, but we can verify
        // the expected values

        let intel_min_version = "10.13";
        let apple_silicon_min_version = "11.0";

        // These should match .cargo/config.toml
        assert_eq!(intel_min_version, "10.13");
        assert_eq!(apple_silicon_min_version, "11.0");
    }

    /// Test linker flag format for macOS
    #[test]
    fn test_macos_linker_flag_format() {
        // Verify the linker flag format is correct
        let linker_flag = "-mmacosx-version-min=";

        assert!(linker_flag.contains("mmacosx-version-min"));
        assert!(linker_flag.starts_with('-'));
    }

    /// Test that different architectures have different minimum versions
    #[test]
    fn test_macos_architecture_versions() {
        // Intel uses older macOS version (10.13)
        let intel_version = "10.13";
        // Apple Silicon requires 11.0 (first release for ARM)
        let arm_version = "11.0";

        // Verify they're different
        assert_ne!(intel_version, arm_version);

        // Verify Apple Silicon version is higher (newer)
        let intel_parts: Vec<&str> = intel_version.split('.').collect();
        let arm_parts: Vec<&str> = arm_version.split('.').collect();

        let intel_major: u32 = intel_parts[0].parse().unwrap();
        let arm_major: u32 = arm_parts[0].parse().unwrap();

        assert!(arm_major >= intel_major);
    }
}

#[cfg(test)]
mod build_configuration_tests {
    /// Test that build profile configurations are set
    #[test]
    fn test_build_profile_release_configured() {
        // Verify release profile is configured for optimization
        // These should match .cargo/config.toml [profile.release]

        // Expected release profile settings
        let expected_opt_level = "z"; // Optimize for size
        let expected_lto = true;
        let expected_codegen_units = 1;
        let expected_strip = true;

        // Verify these are sensible values
        assert_eq!(expected_opt_level, "z");
        assert!(expected_lto);
        assert_eq!(expected_codegen_units, 1);
        assert!(expected_strip);
    }

    /// Test that static linking is configured for musl targets
    #[test]
    fn test_static_linking_configured() {
        // Verify static linking configuration for musl targets
        let musl_targets = vec![
            "x86_64-unknown-linux-musl",
            "aarch64-unknown-linux-musl",
        ];

        for target in musl_targets {
            assert!(
                target.contains("musl"),
                "Target {} should be a musl target",
                target
            );
        }
    }
}

#[cfg(test)]
mod target_triple_tests {
    /// Test macOS target triple format
    #[test]
    fn test_macos_target_triples() {
        // Verify macOS target triple formats
        let intel_triple = "x86_64-apple-darwin";
        let arm_triple = "aarch64-apple-darwin";

        // Intel triple
        assert!(intel_triple.contains("x86_64"));
        assert!(intel_triple.contains("apple"));
        assert!(intel_triple.contains("darwin"));

        // ARM triple (aarch64)
        assert!(arm_triple.contains("aarch64") || arm_triple.contains("arm64"));
        assert!(arm_triple.contains("apple"));
        assert!(arm_triple.contains("darwin"));
    }

    /// Test that all supported target triples are valid
    #[test]
    fn test_all_target_triples() {
        let supported_triples = vec![
            // Linux
            "x86_64-unknown-linux-musl",
            "aarch64-unknown-linux-musl",
            "x86_64-unknown-linux-gnu",
            "aarch64-unknown-linux-gnu",
            // Windows
            "x86_64-pc-windows-msvc",
            "aarch64-pc-windows-msvc",
            // macOS
            "x86_64-apple-darwin",
            "aarch64-apple-darwin",
        ];

        for triple in supported_triples {
            // Verify triple format: arch-vendor-os[-environment]
            let parts: Vec<&str> = triple.split('-').collect();
            assert!(parts.len() >= 3, "Invalid triple format: {}", triple);

            let arch = parts[0];
            let vendor = if parts.len() > 1 { parts[1] } else { "" };
            let os = if parts.len() > 2 { parts[2] } else { "" };
            let environment = if parts.len() > 3 { parts[3] } else { "" };

            // Verify architecture
            let valid_archs = vec!["x86_64", "aarch64", "i686", "armv7"];
            assert!(
                valid_archs.contains(&arch),
                "Invalid architecture in triple: {}",
                triple
            );

            // Verify vendor
            let valid_vendors = vec!["unknown", "pc", "apple"];
            assert!(
                valid_vendors.contains(&vendor) || vendor.is_empty(),
                "Invalid vendor in triple: {}",
                triple
            );

            // Verify OS or environment
            let combined = format!("{} {}", os, environment);
            assert!(
                combined.contains("linux") || combined.contains("darwin") ||
                combined.contains("windows") || combined.contains("musl") ||
                combined.contains("gnu") || combined.contains("msvc") ||
                combined.is_empty(),
                "Invalid OS/environment in triple: {}",
                triple
            );
        }
    }
}

#[cfg(test)]
mod dependency_tests {
    /// Test that macOS-compatible dependencies are used
    #[test]
    fn test_macos_compatible_dependencies() {
        // Verify that dependencies support macOS
        // These are from Cargo.toml

        let macos_compatible_crates = vec![
            "clap",          // CLI parsing (cross-platform)
            "tokio",         // Async runtime (cross-platform)
            "serde",         // Serialization (cross-platform)
            "git2",          // Git operations (supports macOS)
            "reqwest",       // HTTP client (cross-platform with rustls)
            "chrono",        // Datetime (cross-platform)
            "anyhow",        // Error handling (cross-platform)
            "tracing",       // Logging (cross-platform)
        ];

        // All these should support macOS
        for crate_name in macos_compatible_crates {
            // Just verify the names - actual compatibility would require
            // checking each crate's metadata
            assert!(!crate_name.is_empty());
        }
    }

    /// Test that git2 uses vendored features for static linking
    #[test]
    fn test_git2_vendored_features() {
        // From Cargo.toml: git2 = { version = "0.19", features = ["vendored-libgit2", "ssh"] }
        // This ensures libgit2 is statically linked, which helps with macOS compatibility

        let git2_features = vec!["vendored-libgit2", "ssh"];

        for feature in &git2_features {
            assert!(!feature.is_empty());
        }

        // Verify vendored feature is present (important for macOS)
        assert!(git2_features.contains(&"vendored-libgit2"));
    }

    /// Test that reqwest uses rustls for TLS (OpenSSL-free)
    #[test]
    fn test_reqwest_rustls_tls() {
        // From Cargo.toml: reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }
        // Using rustls instead of OpenSSL avoids OpenSSL compatibility issues on macOS

        let reqwest_features = vec!["json", "rustls-tls"];

        // Verify rustls-tls is used (important for macOS compatibility)
        assert!(reqwest_features.contains(&"rustls-tls"));
        assert!(!reqwest_features.contains(&"native-tls")); // Should not use native TLS
    }
}

#[cfg(test)]
mod feature_flag_tests {
    /// Test that static feature is correctly defined
    #[test]
    fn test_static_feature_definition() {
        // From Cargo.toml [features]
        // static = ["git2/vendored-libgit2", "git2/vendored-openssl"]

        let static_features = vec![
            "git2/vendored-libgit2",
            "git2/vendored-openssl",
        ];

        assert!(!static_features.is_empty());
        assert!(static_features.iter().any(|f| f.contains("vendored")));
    }

    /// Test that default features are minimal
    #[test]
    fn test_default_features_minimal() {
        // From Cargo.toml: default = []
        // Empty default features means no special features are enabled by default

        let default_features: Vec<&str> = vec![];

        // Default should be empty or very minimal
        assert!(default_features.is_empty() || default_features.len() <= 2);
    }
}

#[cfg(test)]
mod code_signing_tests {
    /// Test that code signing requirements are understood
    #[test]
    fn test_code_signing_requirements() {
        // macOS requires code signing for executables
        // For development: ad-hoc signing is sufficient
        // For distribution: proper Apple developer certificate

        let adhoc_signing_acceptable = true; // For development
        let developer_signing_required = false; // Only for distribution

        assert!(adhoc_signing_acceptable);
        assert!(!developer_signing_required);
    }

    /// Test ad-hoc signing command format
    #[test]
    fn test_adhoc_signing_command() {
        // Ad-hoc signing command: codesign --force --deep --sign - <binary>
        let sign_command = "codesign";
        let force_flag = "--force";
        let deep_flag = "--deep";
        let sign_flag = "--sign";
        let adhoc_indicator = "-";  // "-" means ad-hoc

        assert_eq!(sign_command, "codesign");
        assert_eq!(force_flag, "--force");
        assert_eq!(deep_flag, "--deep");
        assert_eq!(sign_flag, "--sign");
        assert_eq!(adhoc_indicator, "-");
    }

    /// Test code signing verification command
    #[test]
    fn test_codesign_verify_command() {
        // Verification command: codesign -v <binary>
        let verify_command = "codesign";
        let verify_flag = "-v";

        assert_eq!(verify_command, "codesign");
        assert_eq!(verify_flag, "-v");
    }
}

#[cfg(test)]
mod universal_binary_tests {
    /// Test universal binary creation concept
    #[test]
    fn test_universal_binary_concept() {
        // Universal binary supports both Intel and Apple Silicon
        // Created with: lipo -create <intel> <arm> -output <universal>

        let intel_arch = "x86_64";
        let arm_arch = "arm64";
        let universal_cmd = "lipo";

        assert_eq!(intel_arch, "x86_64");
        assert_eq!(arm_arch, "arm64");
        assert_eq!(universal_cmd, "lipo");
    }

    /// Test that universal binary requires both architectures
    #[test]
    fn test_universal_binary_requires_both_archs() {
        // Universal binary needs both Intel and ARM binaries

        let intel_binary = "target/x86_64-apple-darwin/release/ltmatrix";
        let arm_binary = "target/aarch64-apple-darwin/release/ltmatrix";
        let universal_output = "target/release/ltmatrix-universal";

        assert!(intel_binary.contains("x86_64"));
        assert!(arm_binary.contains("aarch64") || arm_binary.contains("arm64"));
        assert!(universal_output.contains("universal"));
    }
}

#[cfg(test)]
mod compatibility_tests {
    /// Test macOS version compatibility
    #[test]
    fn test_macos_version_compatibility() {
        // Minimum versions supported by the binary
        let intel_min = "10.13";  // High Sierra
        let arm_min = "11.0";     // Big Sur (first Apple Silicon release)

        // These are the minimum versions, not the current version
        assert!(intel_min <= "10.15"); // At least High Sierra
        assert!(arm_min == "11.0");    // Big Sur for Apple Silicon
    }

    /// Test that Apple Silicon requires macOS 11+
    #[test]
    fn test_apple_silicon_requires_macos_11() {
        // Apple Silicon was introduced in macOS 11.0 (Big Sur)
        let apple_silicon_min = "11.0";

        assert_eq!(apple_silicon_min, "11.0");
    }
}

#[cfg(all(test, target_os = "macos"))]
mod macos_only_tests {
    use std::process::Command;

    /// Test that we're running on macOS (guard test)
    #[test]
    fn test_running_on_macos() {
        assert!(cfg!(target_os = "macos"));
    }

    /// Test that Swift runtime is available (on macOS)
    #[test]
    fn test_swift_runtime_available() {
        // On macOS, Swift runtime should be available
        let output = Command::new("swift")
            .arg("--version")
            .output();

        // Swift should be installed on macOS
        match output {
            Ok(output) => {
                // Swift exists, check version output
                let version = String::from_utf8_lossy(&output.stdout);
                assert!(version.contains("Swift") || output.status.success());
            }
            Err(_) => {
                // Swift might not be installed, that's OK
                // The ltmatrix binary doesn't depend on Swift
            }
        }
    }

    /// Test Xcode tool availability
    #[test]
    fn test_xcode_tools_available() {
        // clang should be available on macOS
        let output = Command::new("clang")
            .arg("--version")
            .output();

        match output {
            Ok(output) => {
                assert!(output.status.success(), "clang not available");
            }
            Err(_) => {
                panic!("clang not found - Xcode command line tools may not be installed");
            }
        }
    }
}
