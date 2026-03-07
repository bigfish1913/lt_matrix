// Copyright (c) 2026 bigfish
// SPDX-License-Identifier: MIT
// This file is part of ltmatrix under the MIT License.


//! Semantic versioning implementation
//!
//! This module provides version parsing, comparison, and bumping following
//! Semantic Versioning 2.0.0 (https://semver.org/).

use anyhow::Result;
use std::fmt;
use std::path::Path;
use std::str::FromStr;

/// Semantic version representation
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub pre_release: Option<String>,
    pub build: Option<String>,
}

impl Default for Version {
    fn default() -> Self {
        Self {
            major: 0,
            minor: 1,
            patch: 0,
            pre_release: None,
            build: None,
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;

        if let Some(ref pre) = self.pre_release {
            write!(f, "-{}", pre)?;
        }

        if let Some(ref build) = self.build {
            write!(f, "+{}", build)?;
        }

        Ok(())
    }
}

impl FromStr for Version {
    type Err = BumpError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// Version bump type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionBump {
    /// Major version bump (X.0.0) - Breaking changes
    Major,
    /// Minor version bump (0.X.0) - New features
    Minor,
    /// Patch version bump (0.0.X) - Bug fixes
    Patch,
    /// Pre-release bump
    PreRelease(String),
}

impl fmt::Display for VersionBump {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VersionBump::Major => write!(f, "major"),
            VersionBump::Minor => write!(f, "minor"),
            VersionBump::Patch => write!(f, "patch"),
            VersionBump::PreRelease(pre) => write!(f, "pre-release ({})", pre),
        }
    }
}

/// Version-related errors
#[derive(Debug, thiserror::Error)]
pub enum BumpError {
    #[error("Invalid version format: {0}")]
    InvalidFormat(String),

    #[error("Failed to read Cargo.toml: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Failed to parse Cargo.toml: {0}")]
    ParseError(String),

    #[error("Version not found in Cargo.toml")]
    VersionNotFound,
}

impl Version {
    /// Parse version from string
    pub fn parse(s: &str) -> Result<Self, BumpError> {
        let s = s.trim();

        // Handle 'v' prefix
        let s = s.strip_prefix('v').unwrap_or(s);

        // Split off build metadata
        let (version_part, build) = if let Some(idx) = s.find('+') {
            (&s[..idx], Some(s[idx + 1..].to_string()))
        } else {
            (s, None)
        };

        // Split off pre-release
        let (main_part, pre_release) = if let Some(idx) = version_part.find('-') {
            (version_part[..idx].to_string(), Some(version_part[idx + 1..].to_string()))
        } else {
            (version_part.to_string(), None)
        };

        // Parse main version parts
        let parts: Vec<&str> = main_part.split('.').collect();
        if parts.len() != 3 {
            return Err(BumpError::InvalidFormat(format!(
                "Expected MAJOR.MINOR.PATCH, got: {}",
                s
            )));
        }

        let major = parts[0]
            .parse()
            .map_err(|_| BumpError::InvalidFormat(format!("Invalid major version: {}", parts[0])))?;

        let minor = parts[1]
            .parse()
            .map_err(|_| BumpError::InvalidFormat(format!("Invalid minor version: {}", parts[1])))?;

        let patch = parts[2]
            .parse()
            .map_err(|_| BumpError::InvalidFormat(format!("Invalid patch version: {}", parts[2])))?;

        Ok(Self {
            major,
            minor,
            patch,
            pre_release,
            build,
        })
    }

    /// Read current version from Cargo.toml
    pub fn current(project_root: impl AsRef<Path>) -> Result<Self, BumpError> {
        let cargo_toml = project_root.as_ref().join("Cargo.toml");
        let content = std::fs::read_to_string(&cargo_toml)?;

        // Find version line in [package] section
        let in_package = false;
        let lines: Vec<&str> = content.lines().collect();

        for i in 0..lines.len() {
            let line = lines[i].trim();

            if line == "[package]" {
                // Look for version in next few lines
                for j in (i + 1)..lines.len().min(i + 20) {
                    let inner_line = lines[j].trim();

                    // Stop at next section
                    if inner_line.starts_with('[') && !inner_line.starts_with("[[") {
                        break;
                    }

                    if inner_line.starts_with("version") {
                        if let Some(eq_idx) = inner_line.find('=') {
                            let version_str = inner_line[eq_idx + 1..].trim();
                            // Remove quotes
                            let version_str = version_str.trim_matches('"');
                            return Self::parse(version_str);
                        }
                    }
                }
                break;
            }
        }

        Err(BumpError::VersionNotFound)
    }

    /// Bump version by specified amount
    pub fn bump(&self, bump: VersionBump) -> Result<Self, BumpError> {
        match bump {
            VersionBump::Major => Ok(Self {
                major: self.major + 1,
                minor: 0,
                patch: 0,
                pre_release: None,
                build: None,
            }),
            VersionBump::Minor => Ok(Self {
                major: self.major,
                minor: self.minor + 1,
                patch: 0,
                pre_release: None,
                build: None,
            }),
            VersionBump::Patch => Ok(Self {
                major: self.major,
                minor: self.minor,
                patch: self.patch + 1,
                pre_release: None,
                build: None,
            }),
            VersionBump::PreRelease(pre) => {
                let current_pre = self.pre_release.clone().unwrap_or_default();
                let new_pre = increment_pre_release(&current_pre, &pre)?;
                Ok(Self {
                    major: self.major,
                    minor: self.minor,
                    patch: self.patch,
                    pre_release: Some(new_pre),
                    build: self.build.clone(),
                })
            }
        }
    }

    /// Write version to Cargo.toml
    pub fn write_to_cargo_toml(&self, project_root: impl AsRef<Path>) -> Result<(), BumpError> {
        let cargo_toml = project_root.as_ref().join("Cargo.toml");
        let content = std::fs::read_to_string(&cargo_toml)?;

        // Find and replace version line
        let lines: Vec<&str> = content.lines().collect();
        let mut new_lines = Vec::new();
        let mut in_package = false;
        let mut version_updated = false;

        for line in &lines {
            let trimmed = line.trim();

            if trimmed == "[package]" {
                in_package = true;
            } else if trimmed.starts_with('[') && !trimmed.starts_with("[[") {
                in_package = false;
            }

            if in_package && trimmed.starts_with("version") && !version_updated {
                // Replace version line
                let indent = line.len() - line.trim_start().len();
                let indent_str: String = line.chars().take(indent).collect();
                new_lines.push(format!("{}version = \"{}\"", indent_str, self));
                version_updated = true;
            } else {
                new_lines.push(line.to_string());
            }
        }

        if !version_updated {
            return Err(BumpError::VersionNotFound);
        }

        let new_content = new_lines.join("\n");
        std::fs::write(&cargo_toml, new_content)?;

        Ok(())
    }

    /// Check if this is a pre-release version
    pub fn is_pre_release(&self) -> bool {
        self.pre_release.is_some()
    }

    /// Get the base version (without pre-release or build)
    pub fn base_version(&self) -> Self {
        Self {
            major: self.major,
            minor: self.minor,
            patch: self.patch,
            pre_release: None,
            build: None,
        }
    }
}

/// Increment pre-release version
fn increment_pre_release(current: &str, prefix: &str) -> Result<String, BumpError> {
    // Parse pre-release format: prefix.N
    if current.is_empty() || !current.starts_with(prefix) {
        // Start new pre-release series
        return Ok(format!("{}.1", prefix));
    }

    // Extract number from current pre-release
    let parts: Vec<&str> = current.split('.').collect();
    if parts.len() == 2 && parts[0] == prefix {
        if let Ok(num) = parts[1].parse::<u64>() {
            return Ok(format!("{}.{}", prefix, num + 1));
        }
    }

    // Fallback: increment suffix
    Err(BumpError::InvalidFormat(format!(
        "Cannot increment pre-release: {}",
        current
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_version() {
        let v = Version::parse("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
        assert!(!v.is_pre_release());
    }

    #[test]
    fn test_parse_with_v_prefix() {
        let v = Version::parse("v1.2.3").unwrap();
        assert_eq!(v.to_string(), "1.2.3");
    }

    #[test]
    fn test_parse_pre_release() {
        let v = Version::parse("1.2.3-alpha.1").unwrap();
        assert_eq!(v.pre_release, Some("alpha.1".to_string()));
        assert!(v.is_pre_release());
    }

    #[test]
    fn test_parse_build() {
        let v = Version::parse("1.2.3+build.123").unwrap();
        assert_eq!(v.build, Some("build.123".to_string()));
    }

    #[test]
    fn test_parse_full() {
        let v = Version::parse("1.2.3-alpha.1+build.123").unwrap();
        assert_eq!(v.to_string(), "1.2.3-alpha.1+build.123");
    }

    #[test]
    fn test_bump_major() {
        let v = Version::parse("1.2.3").unwrap();
        let bumped = v.bump(VersionBump::Major).unwrap();
        assert_eq!(bumped.to_string(), "2.0.0");
    }

    #[test]
    fn test_bump_minor() {
        let v = Version::parse("1.2.3").unwrap();
        let bumped = v.bump(VersionBump::Minor).unwrap();
        assert_eq!(bumped.to_string(), "1.3.0");
    }

    #[test]
    fn test_bump_patch() {
        let v = Version::parse("1.2.3").unwrap();
        let bumped = v.bump(VersionBump::Patch).unwrap();
        assert_eq!(bumped.to_string(), "1.2.4");
    }

    #[test]
    fn test_bump_pre_release() {
        let v = Version::parse("1.2.3").unwrap();
        let bumped = v.bump(VersionBump::PreRelease("alpha".to_string())).unwrap();
        assert_eq!(bumped.to_string(), "1.2.3-alpha.1");

        let bumped2 = bumped.bump(VersionBump::PreRelease("alpha".to_string())).unwrap();
        assert_eq!(bumped2.to_string(), "1.2.3-alpha.2");
    }

    #[test]
    fn test_version_comparison() {
        let v1 = Version::parse("1.2.3").unwrap();
        let v2 = Version::parse("1.2.4").unwrap();
        let v3 = Version::parse("1.3.0").unwrap();
        let v4 = Version::parse("2.0.0").unwrap();

        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v3 < v4);
    }

    #[test]
    fn test_invalid_version() {
        assert!(Version::parse("1.2").is_err());
        assert!(Version::parse("1.2.3.4").is_err());
        assert!(Version::parse("abc").is_err());
    }
}