//! Command execution security utilities
//!
//! This module provides utilities for safely executing external commands,
//! preventing command injection, and managing allowed commands.

use anyhow::{anyhow, Result};
use std::collections::HashSet;

/// Default allowed commands for the ltmatrix application
const DEFAULT_ALLOWED_COMMANDS: &[&str] = &[
    // Version control
    "git",
    // AI agents
    "claude",
    "opencode",
    "kimi-code",
    "codex",
    // Build tools
    "cargo",
    "npm",
    "yarn",
    "pnpm",
    "pip",
    "python",
    "python3",
    // Test runners
    "pytest",
    "go",
    // System utilities (read-only)
    "which",
    "echo",
];

/// Command allowlist for security
#[derive(Debug, Clone)]
pub struct CommandAllowlist {
    allowed: HashSet<String>,
}

impl Default for CommandAllowlist {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandAllowlist {
    /// Creates a new allowlist with default allowed commands
    pub fn new() -> Self {
        let allowed: HashSet<String> = DEFAULT_ALLOWED_COMMANDS
            .iter()
            .map(|s| s.to_string())
            .collect();
        CommandAllowlist { allowed }
    }

    /// Creates an empty allowlist
    pub fn empty() -> Self {
        CommandAllowlist {
            allowed: HashSet::new(),
        }
    }

    /// Adds a command to the allowlist
    pub fn allow(&mut self, command: &str) -> &mut Self {
        self.allowed.insert(command.to_lowercase());
        self
    }

    /// Removes a command from the allowlist
    pub fn deny(&mut self, command: &str) -> &mut Self {
        self.allowed.remove(&command.to_lowercase());
        self
    }

    /// Checks if a command is allowed
    pub fn is_allowed(&self, command: &str) -> bool {
        self.allowed.contains(&command.to_lowercase())
    }

    /// Returns the list of allowed commands
    pub fn allowed_commands(&self) -> Vec<&str> {
        self.allowed.iter().map(|s| s.as_str()).collect()
    }
}

/// Validates a command name for execution.
///
/// Checks that the command:
/// - Contains only safe characters
/// - Is not a shell builtin that could be dangerous
/// - Does not contain path separators (must be found via PATH)
///
/// # Arguments
///
/// * `command` - The command name to validate
///
/// # Returns
///
/// `Ok(())` if the command name is valid, `Err` otherwise
///
/// # Examples
///
/// ```
/// use ltmatrix::security::validate_command_name;
///
/// assert!(validate_command_name("git").is_ok());
/// assert!(validate_command_name("my-tool").is_ok());
/// assert!(validate_command_name("/bin/sh").is_err());
/// assert!(validate_command_name("rm; ls").is_err());
/// ```
pub fn validate_command_name(command: &str) -> Result<()> {
    if command.is_empty() {
        return Err(anyhow!("Command name cannot be empty"));
    }

    // Disallow path separators - commands must be found via PATH
    if command.contains('/') || command.contains('\\') {
        return Err(anyhow!(
            "Command name cannot contain path separators: '{}'. \
             Commands must be found via PATH lookup",
            command
        ));
    }

    // Check for dangerous characters
    let dangerous = ['|', '&', ';', '$', '`', '(', ')', '<', '>', '\n', '\r', '\0', ' '];
    for c in command.chars() {
        if dangerous.contains(&c) {
            return Err(anyhow!(
                "Command name contains dangerous character: '{}'",
                c
            ));
        }
    }

    // Check for shell builtins that should not be executed directly
    let shell_builtins = [
        "exec", "eval", "source", ".", "exit", "export", "alias", "unalias",
    ];
    let cmd_lower = command.to_lowercase();
    if shell_builtins.contains(&cmd_lower.as_str()) {
        return Err(anyhow!(
            "Command '{}' is a shell builtin and should not be executed directly",
            command
        ));
    }

    Ok(())
}

/// Validates a command argument.
///
/// Checks that the argument does not contain characters that could
/// be used for command injection when passed to a shell.
///
/// Note: When using Rust's `std::process::Command`, arguments are passed
/// directly to the program and are NOT subject to shell interpolation.
/// This validation is primarily for defense-in-depth and for cases
/// where arguments might be logged or displayed.
///
/// # Arguments
///
/// * `arg` - The argument to validate
///
/// # Returns
///
/// `Ok(())` if the argument is safe, `Err` otherwise
///
/// # Examples
///
/// ```
/// use ltmatrix::security::validate_command_argument;
///
/// assert!(validate_command_argument("file.txt").is_ok());
/// assert!(validate_command_argument("--option=value").is_ok());
/// assert!(validate_command_argument("file; rm -rf /").is_err());
/// ```
pub fn validate_command_argument(arg: &str) -> Result<()> {
    // Null bytes are never allowed
    if arg.contains('\0') {
        return Err(anyhow!("Command argument contains null byte"));
    }

    // Check for shell metacharacters that could be dangerous if the arg
    // is ever passed through a shell (defense in depth)
    let dangerous = ['|', '&', ';', '`', '\n', '\r'];

    // Allow $ for variable references in arguments (common in build tools)
    // Allow () for subexpressions
    // Allow <> for redirections in controlled contexts
    for c in arg.chars() {
        if dangerous.contains(&c) {
            return Err(anyhow!(
                "Command argument contains potentially dangerous character: '{}'. \
                 This could be used for command injection",
                c
            ));
        }
    }

    Ok(())
}

/// Sanitizes a command argument for safe display.
///
/// Replaces potentially dangerous characters with underscores.
/// This should be used for logging or display purposes only.
///
/// # Arguments
///
/// * `arg` - The argument to sanitize
///
/// # Returns
///
/// A sanitized version safe for display
///
/// # Examples
///
/// ```
/// use ltmatrix::security::sanitize_command_argument_for_display;
///
/// assert_eq!(sanitize_command_argument_for_display("file.txt"), "file.txt");
/// assert_eq!(sanitize_command_argument_for_display("secret"), "secret");
/// ```
pub fn sanitize_command_argument_for_display(arg: &str) -> String {
    let dangerous = ['\n', '\r', '\0'];
    arg.chars()
        .map(|c| if dangerous.contains(&c) { '_' } else { c })
        .collect()
}

/// Validates environment variable name.
///
/// Environment variable names must:
/// - Start with a letter or underscore
/// - Contain only alphanumeric characters or underscores
/// - Not be empty
///
/// # Arguments
///
/// * `name` - The environment variable name to validate
///
/// # Returns
///
/// `Ok(())` if valid, `Err` otherwise
///
/// # Examples
///
/// ```
/// use ltmatrix::security::validate_env_var_name;
///
/// assert!(validate_env_var_name("PATH").is_ok());
/// assert!(validate_env_var_name("MY_VAR").is_ok());
/// assert!(validate_env_var_name("_PRIVATE").is_ok());
/// assert!(validate_env_var_name("123VAR").is_err());
/// assert!(validate_env_var_name("").is_err());
/// ```
pub fn validate_env_var_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(anyhow!("Environment variable name cannot be empty"));
    }

    let mut chars = name.chars();
    let first = chars.next().unwrap();

    // First character must be letter or underscore
    if !first.is_ascii_alphabetic() && first != '_' {
        return Err(anyhow!(
            "Environment variable name must start with letter or underscore"
        ));
    }

    // Rest must be alphanumeric or underscore
    for c in chars {
        if !c.is_ascii_alphanumeric() && c != '_' {
            return Err(anyhow!(
                "Environment variable name contains invalid character: '{}'",
                c
            ));
        }
    }

    Ok(())
}

/// Checks if an environment variable contains sensitive information.
///
/// Returns true if the variable name suggests it contains credentials,
/// API keys, or other sensitive data that should not be logged.
///
/// # Arguments
///
/// * `name` - The environment variable name to check
///
/// # Returns
///
/// `true` if the variable might contain sensitive information
///
/// # Examples
///
/// ```
/// use ltmatrix::security::is_sensitive_env_var;
///
/// assert!(is_sensitive_env_var("API_KEY"));
/// assert!(is_sensitive_env_var("PASSWORD"));
/// assert!(is_sensitive_env_var("SECRET_TOKEN"));
/// assert!(!is_sensitive_env_var("PATH"));
/// assert!(!is_sensitive_env_var("HOME"));
/// ```
pub fn is_sensitive_env_var(name: &str) -> bool {
    let name_upper = name.to_uppercase();

    // Patterns that suggest sensitive data
    let sensitive_patterns = [
        "PASSWORD",
        "SECRET",
        "API_KEY",
        "APIKEY",
        "TOKEN",
        "CREDENTIAL",
        "AUTH",
        "PRIVATE",
        "ACCESS_KEY",
        "ACCESSKEY",
    ];

    for pattern in &sensitive_patterns {
        if name_upper.contains(pattern) {
            return true;
        }
    }

    false
}

/// Masks a potentially sensitive value for logging.
///
/// Shows only the first few characters to help with debugging
/// while protecting the actual value.
///
/// # Arguments
///
/// * `value` - The value to mask
///
/// # Returns
///
/// A masked version safe for logging
///
/// # Examples
///
/// ```
/// use ltmatrix::security::mask_sensitive_value;
///
/// assert_eq!(mask_sensitive_value("sk-1234567890abcdef"), "sk-1...cdef");
/// assert_eq!(mask_sensitive_value("short"), "sh...rt");
/// assert_eq!(mask_sensitive_value(""), "***");
/// ```
pub fn mask_sensitive_value(value: &str) -> String {
    if value.is_empty() {
        return "***".to_string();
    }

    let len = value.len();
    if len <= 4 {
        return "***".to_string();
    }

    if len <= 8 {
        // Short values (5-8 chars): show first 2 and last 2 chars
        format!("{}...{}", &value[..2], &value[len - 2..])
    } else {
        // Longer values: show first 4 and last 4
        format!("{}...{}", &value[..4], &value[len - 4..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_allowlist() {
        let mut list = CommandAllowlist::new();

        assert!(list.is_allowed("git"));
        assert!(list.is_allowed("GIT")); // Case insensitive
        assert!(!list.is_allowed("dangerous"));

        list.allow("dangerous");
        assert!(list.is_allowed("dangerous"));

        list.deny("git");
        assert!(!list.is_allowed("git"));
    }

    #[test]
    fn test_validate_command_name() {
        assert!(validate_command_name("git").is_ok());
        assert!(validate_command_name("my-tool").is_ok());
        assert!(validate_command_name("").is_err());
        assert!(validate_command_name("/bin/sh").is_err());
        assert!(validate_command_name("rm; ls").is_err());
        assert!(validate_command_name("eval").is_err());
    }

    #[test]
    fn test_validate_command_argument() {
        assert!(validate_command_argument("file.txt").is_ok());
        assert!(validate_command_argument("--option=value").is_ok());
        assert!(validate_command_argument("file; rm -rf /").is_err());
        assert!(validate_command_argument("cmd\narg").is_err());
    }

    #[test]
    fn test_validate_env_var_name() {
        assert!(validate_env_var_name("PATH").is_ok());
        assert!(validate_env_var_name("MY_VAR").is_ok());
        assert!(validate_env_var_name("_PRIVATE").is_ok());
        assert!(validate_env_var_name("").is_err());
        assert!(validate_env_var_name("123VAR").is_err());
        assert!(validate_env_var_name("MY-VAR").is_err());
    }

    #[test]
    fn test_is_sensitive_env_var() {
        assert!(is_sensitive_env_var("API_KEY"));
        assert!(is_sensitive_env_var("password"));
        assert!(is_sensitive_env_var("SECRET_TOKEN"));
        assert!(is_sensitive_env_var("MY_PRIVATE_KEY"));
        assert!(!is_sensitive_env_var("PATH"));
        assert!(!is_sensitive_env_var("HOME"));
        assert!(!is_sensitive_env_var("DEBUG"));
    }

    #[test]
    fn test_mask_sensitive_value() {
        assert_eq!(mask_sensitive_value("sk-1234567890abcdef"), "sk-1...cdef");
        assert_eq!(mask_sensitive_value("short"), "sh...rt");
        assert_eq!(mask_sensitive_value("ab"), "***");
        assert_eq!(mask_sensitive_value(""), "***");
    }
}