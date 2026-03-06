//! Build script for ltmatrix - static linking configuration
//!
//! This script configures the build for static linking on musl targets,
//! ensuring that native dependencies (libgit2, OpenSSL, etc.) are
//! statically linked to produce portable, self-contained binaries.

use std::env;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // Detect target triple
    let target = env::var("TARGET").expect("TARGET not set");
    let is_musl = target.contains("musl");

    // Configure static linking for musl targets
    if is_musl {
        configure_musl_static_linking();
    }

    // Configure git2 for static linking
    configure_git2(&target);

    // Emit information about the build configuration
    emit_build_info(&target);
}

/// Configure static linking for musl targets
fn configure_musl_static_linking() {
    println!("cargo:warning=Building for musl target with static linking");

    // Tell cargo to statically link the C standard library
    println!("cargo:rustc-link-lib=static=c");

    // On musl, we want to force static linking of all libraries
    // to ensure the binary is truly portable
    println!("cargo:rustc-cfg= musl_target");
}

/// Configure git2 library for static linking
fn configure_git2(target: &str) {
    // The git2 crate has a vendored feature that includes libgit2 statically
    // We enable it via Cargo.toml features, but we can also configure here

    if target.contains("linux") {
        // On Linux, prefer static linking of libgit2
        // This is controlled by the "vendored" or "ssh" features in git2
        println!("cargo:rustc-env=LIBGIT2_STATIC=1");
    }

    // For musl targets, we definitely want static linking
    if target.contains("musl") {
        println!("cargo:rustc-env=LIBGIT2_NO_SYSTEM=1");
    }
}

/// Emit build information for debugging
fn emit_build_info(target: &str) {
    let is_release = env::var("PROFILE").unwrap_or_default() == "release";
    let opt_level = env::var("OPT_LEVEL").unwrap_or_default();

    println!("cargo:warning=Build configuration:");
    println!("cargo:warning=  Target: {}", target);
    println!(
        "cargo:warning=  Profile: {}",
        if is_release { "release" } else { "debug" }
    );
    println!("cargo:warning=  Opt-level: {}", opt_level);
    println!(
        "cargo:warning=  Static linking: {}",
        if target.contains("musl") {
            "yes (musl)"
        } else {
            "no (dynamic)"
        }
    );
}
