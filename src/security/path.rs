// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.

//! Path validation and sanitization utilities
//!
//! This module provides functions to prevent path traversal attacks
//! and ensure file system paths are safe to use.

use anyhow::{anyhow, Context, Result};
use std::path::{Component, Path, PathBuf};

/// Maximum path length (platform-dependent, but use conservative limit)
const MAX_PATH_LENGTH: usize = 4096;

/// Validates a file path for safety.
///
/// Checks for:
/// - Path traversal attempts (../)
/// - Null bytes
/// - Reasonable length
///
/// # Arguments
///
/// * `path` - The path to validate
///
/// # Returns
///
/// `Ok(())` if the path is safe, `Err` otherwise
///
/// # Examples
///
/// ```
/// use ltmatrix::security::validate_path;
/// use std::path::Path;
///
/// assert!(validate_path(Path::new("src/main.rs")).is_ok());
/// assert!(validate_path(Path::new("../../../etc/passwd")).is_err());
/// assert!(validate_path(Path::new("/etc/passwd")).is_ok()); // Absolute paths are allowed
/// ```
pub fn validate_path(path: &Path) -> Result<()> {
    let path_str = path.to_string_lossy();

    // Check for null bytes
    if path_str.contains('\0') {
        return Err(anyhow!("Path contains null bytes"));
    }

    // Check length
    if path_str.len() > MAX_PATH_LENGTH {
        return Err(anyhow!(
            "Path too long: {} characters (max {})",
            path_str.len(),
            MAX_PATH_LENGTH
        ));
    }

    // Check for suspicious patterns
    let suspicious = ["../", "..\\", ".."];
    for pattern in &suspicious {
        if path_str.contains(pattern) {
            // This might be intentional, so just warn in the error message
            return Err(anyhow!(
                "Path contains potential traversal pattern: '{}'. \
                 If this is intentional, use canonicalize() to resolve",
                pattern
            ));
        }
    }

    Ok(())
}

/// Validates that a path stays within a base directory.
///
/// This prevents path traversal attacks by ensuring the resolved path
/// is contained within the specified base directory.
///
/// # Arguments
///
/// * `path` - The path to check
/// * `base` - The base directory that paths must stay within
///
/// # Returns
///
/// `Ok(())` if the path is within the base directory, `Err` otherwise
///
/// # Examples
///
/// ```no_run
/// use ltmatrix::security::validate_path_within_base;
/// use std::path::Path;
///
/// // Note: This validation requires the paths to actually exist on disk
/// // because it uses canonicalize() to resolve symlinks and relative paths.
/// // Example with existing directories:
/// // let base = std::env::current_dir().unwrap();
/// // assert!(validate_path_within_base(Path::new("src/main.rs"), &base).is_ok());
/// ```
pub fn validate_path_within_base(path: &Path, base: &Path) -> Result<()> {
    // Canonicalize both paths to resolve symlinks and relative components
    let canonical_path = path
        .canonicalize()
        .with_context(|| format!("Failed to canonicalize path: {:?}", path))?;

    let canonical_base = base
        .canonicalize()
        .with_context(|| format!("Failed to canonicalize base: {:?}", base))?;

    // Check if the path starts with the base
    if !canonical_path.starts_with(&canonical_base) {
        return Err(anyhow!(
            "Path {:?} is outside base directory {:?}",
            path,
            base
        ));
    }

    Ok(())
}

/// Sanitizes a path component for safe use.
///
/// Removes or replaces characters that are problematic in file names.
///
/// # Arguments
///
/// * `component` - The path component to sanitize
///
/// # Returns
///
/// A sanitized path component
///
/// # Examples
///
/// ```
/// use ltmatrix::security::sanitize_path_component;
///
/// assert_eq!(sanitize_path_component("file.txt"), "file.txt");
/// assert_eq!(sanitize_path_component("file<name>.txt"), "file_name_.txt");
/// assert_eq!(sanitize_path_component("..hidden"), "..hidden");
/// ```
pub fn sanitize_path_component(component: &str) -> String {
    // Characters that are problematic in file names on various platforms
    let problematic = ['<', '>', ':', '"', '|', '?', '*', '\0'];

    component
        .chars()
        .map(|c| if problematic.contains(&c) { '_' } else { c })
        .collect()
}

/// Checks if a path contains any directory traversal components.
///
/// # Arguments
///
/// * `path` - The path to check
///
/// # Returns
///
/// `true` if the path contains traversal components, `false` otherwise
///
/// # Examples
///
/// ```
/// use ltmatrix::security::contains_traversal;
/// use std::path::Path;
///
/// assert!(contains_traversal(Path::new("../../../etc/passwd")));
/// assert!(!contains_traversal(Path::new("src/main.rs")));
/// ```
pub fn contains_traversal(path: &Path) -> bool {
    for component in path.components() {
        if matches!(component, Component::ParentDir) {
            return true;
        }
    }
    false
}

/// Resolves a relative path against a base directory, ensuring it stays within bounds.
///
/// # Arguments
///
/// * `relative_path` - The relative path to resolve
/// * `base` - The base directory
///
/// # Returns
///
/// The resolved absolute path, or an error if the path would escape the base
///
/// # Examples
///
/// ```
/// use ltmatrix::security::resolve_safe_path;
/// use std::path::Path;
///
/// // This would require actual directories to test properly
/// // let base = Path::new("/tmp/test");
/// // let resolved = resolve_safe_path(Path::new("subdir/file.txt"), base);
/// ```
pub fn resolve_safe_path(relative_path: &Path, base: &Path) -> Result<PathBuf> {
    // Check for traversal before joining
    if contains_traversal(relative_path) {
        return Err(anyhow!(
            "Relative path contains traversal components: {:?}",
            relative_path
        ));
    }

    let resolved = base.join(relative_path);

    // Verify the resolved path is within base
    // Note: This requires the paths to exist for canonicalize
    if resolved.exists() && base.exists() {
        validate_path_within_base(&resolved, base)?;
    }

    Ok(resolved)
}

/// Validates a file extension.
///
/// # Arguments
///
/// * `ext` - The extension to validate (without the dot)
///
/// # Returns
///
/// `Ok(())` if the extension is valid, `Err` otherwise
///
/// # Examples
///
/// ```
/// use ltmatrix::security::validate_file_extension;
///
/// assert!(validate_file_extension("rs").is_ok());
/// assert!(validate_file_extension("txt").is_ok());
/// assert!(validate_file_extension("").is_err());
/// assert!(validate_file_extension("ex>e").is_err());
/// ```
pub fn validate_file_extension(ext: &str) -> Result<()> {
    if ext.is_empty() {
        return Err(anyhow!("File extension cannot be empty"));
    }

    if ext.len() > 32 {
        return Err(anyhow!(
            "File extension too long: {} characters (max 32)",
            ext.len()
        ));
    }

    // Extensions should be alphanumeric
    for c in ext.chars() {
        if !c.is_alphanumeric() && c != '-' && c != '_' {
            return Err(anyhow!("Invalid character '{}' in file extension", c));
        }
    }

    Ok(())
}

/// Checks if a file path has a potentially dangerous extension.
///
/// This is a helper for applications that want to restrict certain file types.
///
/// # Arguments
///
/// * `path` - The path to check
/// * `dangerous_extensions` - List of extensions to consider dangerous (without dots)
///
/// # Returns
///
/// `true` if the file has a dangerous extension
///
/// # Examples
///
/// ```
/// use ltmatrix::security::has_dangerous_extension;
/// use std::path::Path;
///
/// let dangerous = &["exe", "bat", "cmd", "sh", "ps1"];
/// assert!(has_dangerous_extension(Path::new("malware.exe"), dangerous));
/// assert!(!has_dangerous_extension(Path::new("document.pdf"), dangerous));
/// ```
pub fn has_dangerous_extension(path: &Path, dangerous_extensions: &[&str]) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            let ext_lower = ext.to_lowercase();
            dangerous_extensions
                .iter()
                .any(|d| d.to_lowercase() == ext_lower)
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_path_valid() {
        assert!(validate_path(Path::new("src/main.rs")).is_ok());
        assert!(validate_path(Path::new("file.txt")).is_ok());
        assert!(validate_path(Path::new("/absolute/path")).is_ok());
    }

    #[test]
    fn test_validate_path_traversal() {
        assert!(validate_path(Path::new("../../../etc/passwd")).is_err());
        assert!(validate_path(Path::new("..\\..\\windows")).is_err());
    }

    #[test]
    fn test_contains_traversal() {
        assert!(contains_traversal(Path::new("../parent")));
        assert!(contains_traversal(Path::new("sub/../other")));
        assert!(!contains_traversal(Path::new("sub/dir/file")));
        assert!(!contains_traversal(Path::new("./file")));
    }

    #[test]
    fn test_sanitize_path_component() {
        assert_eq!(sanitize_path_component("file.txt"), "file.txt");
        assert_eq!(sanitize_path_component("file<name>"), "file_name_");
        assert_eq!(sanitize_path_component("file|name?"), "file_name_");
    }

    #[test]
    fn test_validate_file_extension() {
        assert!(validate_file_extension("rs").is_ok());
        assert!(validate_file_extension("txt").is_ok());
        assert!(validate_file_extension("").is_err());
        assert!(validate_file_extension("ex>e").is_err());
    }

    #[test]
    fn test_has_dangerous_extension() {
        let dangerous: &[&str] = &["exe", "bat", "cmd"];

        assert!(has_dangerous_extension(Path::new("file.exe"), dangerous));
        assert!(has_dangerous_extension(Path::new("file.EXE"), dangerous));
        assert!(has_dangerous_extension(Path::new("file.bat"), dangerous));
        assert!(!has_dangerous_extension(Path::new("file.txt"), dangerous));
        assert!(!has_dangerous_extension(Path::new("file.rs"), dangerous));
    }
}
