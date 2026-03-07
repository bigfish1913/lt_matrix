//! Input validation and sanitization utilities
//!
//! This module provides functions to validate and sanitize user inputs
//! to prevent security vulnerabilities like command injection and
//! path traversal attacks.

use anyhow::{anyhow, Result};

/// Characters that are potentially dangerous in shell contexts
const SHELL_DANGEROUS_CHARS: &[char] = &[
    '|', '&', ';', '$', '`', '(', ')', '<', '>', '\n', '\r', '\0',
];

/// Characters allowed in identifiers (task IDs, branch names, etc.)
const IDENTIFIER_ALLOWED_CHARS: &[char] = &['-', '_'];

/// Maximum length for identifiers
const MAX_IDENTIFIER_LENGTH: usize = 256;

/// Minimum length for identifiers
const MIN_IDENTIFIER_LENGTH: usize = 1;

/// Validates that a string is a safe identifier.
///
/// Identifiers can contain alphanumeric characters, dashes, and underscores.
/// This is used for task IDs, branch names, and other user-provided identifiers.
///
/// # Arguments
///
/// * `id` - The identifier string to validate
///
/// # Returns
///
/// `Ok(())` if the identifier is valid, `Err` otherwise
///
/// # Examples
///
/// ```
/// use ltmatrix::security::validate_identifier;
///
/// assert!(validate_identifier("task-123").is_ok());
/// assert!(validate_identifier("my_feature").is_ok());
/// assert!(validate_identifier("invalid;id").is_err());
/// ```
pub fn validate_identifier(id: &str) -> Result<()> {
    if id.is_empty() {
        return Err(anyhow!("Identifier cannot be empty"));
    }

    if id.len() > MAX_IDENTIFIER_LENGTH {
        return Err(anyhow!(
            "Identifier too long: {} characters (max {})",
            id.len(),
            MAX_IDENTIFIER_LENGTH
        ));
    }

    if id.len() < MIN_IDENTIFIER_LENGTH {
        return Err(anyhow!(
            "Identifier too short: {} characters (min {})",
            id.len(),
            MIN_IDENTIFIER_LENGTH
        ));
    }

    // First character must be alphanumeric
    if !id.chars().next().map(|c| c.is_alphanumeric()).unwrap_or(false) {
        return Err(anyhow!(
            "Identifier must start with alphanumeric character"
        ));
    }

    // Check all characters
    for (i, c) in id.chars().enumerate() {
        if !c.is_alphanumeric() && !IDENTIFIER_ALLOWED_CHARS.contains(&c) {
            return Err(anyhow!(
                "Invalid character '{}' at position {} in identifier. \
                 Only alphanumeric, dash, and underscore allowed",
                c, i
            ));
        }
    }

    Ok(())
}

/// Sanitizes a string for safe use as an identifier.
///
/// Replaces invalid characters with underscores and truncates to max length.
///
/// # Arguments
///
/// * `input` - The input string to sanitize
///
/// # Returns
///
/// A sanitized string safe for use as an identifier
///
/// # Examples
///
/// ```
/// use ltmatrix::security::sanitize_identifier;
///
/// assert_eq!(sanitize_identifier("my-task"), "my-task");
/// assert_eq!(sanitize_identifier("my task!"), "my_task_");
/// assert_eq!(sanitize_identifier(""), "unnamed");
/// ```
pub fn sanitize_identifier(input: &str) -> String {
    if input.is_empty() {
        return "unnamed".to_string();
    }

    let mut result = String::with_capacity(input.len().min(MAX_IDENTIFIER_LENGTH));

    for (i, c) in input.chars().take(MAX_IDENTIFIER_LENGTH).enumerate() {
        if i == 0 && !c.is_alphanumeric() {
            // First char must be alphanumeric, prefix with 'x'
            result.push('x');
        }

        if c.is_alphanumeric() || IDENTIFIER_ALLOWED_CHARS.contains(&c) {
            result.push(c);
        } else {
            result.push('_');
        }
    }

    if result.is_empty() {
        "unnamed".to_string()
    } else {
        result
    }
}

/// Checks if a string contains characters that could be used for command injection.
///
/// # Arguments
///
/// * `input` - The input string to check
///
/// # Returns
///
/// `true` if the string is safe, `false` if it contains dangerous characters
///
/// # Examples
///
/// ```
/// use ltmatrix::security::is_safe_for_command_arg;
///
/// assert!(is_safe_for_command_arg("hello world"));
/// assert!(!is_safe_for_command_arg("hello; rm -rf /"));
/// assert!(!is_safe_for_command_arg("hello `whoami`"));
/// ```
pub fn is_safe_for_command_arg(input: &str) -> bool {
    !input.chars().any(|c| SHELL_DANGEROUS_CHARS.contains(&c))
}

/// Validates that a string is safe for use as a command argument.
///
/// # Arguments
///
/// * `input` - The input string to validate
///
/// # Returns
///
/// `Ok(())` if safe, `Err` with details about dangerous characters
///
/// # Examples
///
/// ```
/// use ltmatrix::security::validate_command_arg;
///
/// assert!(validate_command_arg("hello").is_ok());
/// assert!(validate_command_arg("hello; world").is_err());
/// ```
pub fn validate_command_arg(input: &str) -> Result<()> {
    let dangerous: Vec<char> = input
        .chars()
        .filter(|c| SHELL_DANGEROUS_CHARS.contains(c))
        .collect();

    if dangerous.is_empty() {
        Ok(())
    } else {
        let dangerous_str: String = dangerous.iter().collect();
        Err(anyhow!(
            "Input contains dangerous characters for shell: '{}'. \
             These characters could be used for command injection: {}",
            dangerous_str,
            dangerous_str
        ))
    }
}

/// Sanitizes a string for safe use in command arguments.
///
/// Removes or escapes dangerous characters. Note: For best security,
/// prefer using array-style command arguments instead of shell interpolation.
///
/// # Arguments
///
/// * `input` - The input string to sanitize
///
/// # Returns
///
/// A sanitized string with dangerous characters removed
///
/// # Examples
///
/// ```
/// use ltmatrix::security::sanitize_command_arg;
///
/// assert_eq!(sanitize_command_arg("hello"), "hello");
/// assert_eq!(sanitize_command_arg("hello; world"), "hello world");
/// ```
pub fn sanitize_command_arg(input: &str) -> String {
    input
        .chars()
        .filter(|c| !SHELL_DANGEROUS_CHARS.contains(c))
        .collect()
}

/// Validates a task title.
///
/// Task titles should be non-empty, reasonably short, and not contain
/// dangerous characters.
///
/// # Arguments
///
/// * `title` - The task title to validate
///
/// # Returns
///
/// `Ok(())` if valid, `Err` otherwise
pub fn validate_task_title(title: &str) -> Result<()> {
    if title.is_empty() {
        return Err(anyhow!("Task title cannot be empty"));
    }

    if title.len() > 500 {
        return Err(anyhow!("Task title too long: {} characters (max 500)", title.len()));
    }

    // Check for control characters
    if title.chars().any(|c| c.is_control()) {
        return Err(anyhow!("Task title contains control characters"));
    }

    Ok(())
}

/// Validates a task description.
///
/// Task descriptions can be longer than titles but still have limits.
///
/// # Arguments
///
/// * `description` - The task description to validate
///
/// # Returns
///
/// `Ok(())` if valid, `Err` otherwise
pub fn validate_task_description(description: &str) -> Result<()> {
    if description.len() > 10000 {
        return Err(anyhow!(
            "Task description too long: {} characters (max 10000)",
            description.len()
        ));
    }

    // Check for null bytes (potential for truncation attacks)
    if description.contains('\0') {
        return Err(anyhow!("Task description contains null bytes"));
    }

    Ok(())
}

/// Validates a git branch name.
///
/// Git branch names have specific restrictions. This validates that
/// the name follows git conventions.
///
/// # Arguments
///
/// * `name` - The branch name to validate
///
/// # Returns
///
/// `Ok(())` if valid, `Err` otherwise
///
/// # Examples
///
/// ```
/// use ltmatrix::security::validate_branch_name;
///
/// assert!(validate_branch_name("feature/my-feature").is_ok());
/// assert!(validate_branch_name("main").is_ok());
/// assert!(validate_branch_name("invalid..name").is_err());
/// ```
pub fn validate_branch_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(anyhow!("Branch name cannot be empty"));
    }

    if name.len() > 250 {
        return Err(anyhow!("Branch name too long: {} characters (max 250)", name.len()));
    }

    // Git branch name rules
    // Cannot start with '.'
    if name.starts_with('.') {
        return Err(anyhow!("Branch name cannot start with '.'"));
    }

    // Cannot contain '..'
    if name.contains("..") {
        return Err(anyhow!("Branch name cannot contain '..'"));
    }

    // Cannot contain '~', '^', ':', ' ', '\t', '\n', '\r', or control chars
    let forbidden = ['~', '^', ':', ' ', '\t', '\n', '\r', '\\', '*'];
    for c in name.chars() {
        if forbidden.contains(&c) || c.is_control() {
            return Err(anyhow!(
                "Branch name contains forbidden character: '{}'",
                c
            ));
        }
    }

    // Cannot end with '.lock'
    if name.ends_with(".lock") {
        return Err(anyhow!("Branch name cannot end with '.lock'"));
    }

    // Cannot be just '@'
    if name == "@" {
        return Err(anyhow!("Branch name cannot be just '@'"));
    }

    Ok(())
}

/// Validates a git commit message.
///
/// Commit messages should be reasonably sized and not contain
/// certain problematic characters.
///
/// # Arguments
///
/// * `message` - The commit message to validate
///
/// # Returns
///
/// `Ok(())` if valid, `Err` otherwise
pub fn validate_commit_message(message: &str) -> Result<()> {
    if message.is_empty() {
        return Err(anyhow!("Commit message cannot be empty"));
    }

    if message.len() > 50000 {
        return Err(anyhow!(
            "Commit message too long: {} characters (max 50000)",
            message.len()
        ));
    }

    // Check for null bytes
    if message.contains('\0') {
        return Err(anyhow!("Commit message contains null bytes"));
    }

    Ok(())
}

/// Validates a model name for Claude or other AI providers.
///
/// Model names should match expected patterns for the provider.
///
/// # Arguments
///
/// * `model` - The model name to validate
///
/// # Returns
///
/// `Ok(())` if valid, `Err` otherwise
pub fn validate_model_name(model: &str) -> Result<()> {
    if model.is_empty() {
        return Err(anyhow!("Model name cannot be empty"));
    }

    // Model names should be alphanumeric with dots and dashes
    for c in model.chars() {
        if !c.is_alphanumeric() && c != '.' && c != '-' && c != '_' {
            return Err(anyhow!(
                "Invalid character '{}' in model name. \
                 Only alphanumeric, '.', '-', and '_' allowed",
                c
            ));
        }
    }

    Ok(())
}

/// Sanitizes a model name by removing invalid characters.
///
/// # Arguments
///
/// * `model` - The model name to sanitize
///
/// # Returns
///
/// A sanitized model name
pub fn sanitize_model_name(model: &str) -> String {
    model
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '.' || *c == '-' || *c == '_')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_sanitize_identifier() {
        assert_eq!(sanitize_identifier("my-task"), "my-task");
        assert_eq!(sanitize_identifier("my task"), "my_task");
        assert_eq!(sanitize_identifier("my;task"), "my_task");
        assert_eq!(sanitize_identifier(""), "unnamed");
        assert_eq!(sanitize_identifier("-task"), "x-task");
    }

    #[test]
    fn test_is_safe_for_command_arg() {
        assert!(is_safe_for_command_arg("hello"));
        assert!(is_safe_for_command_arg("hello world"));
        assert!(!is_safe_for_command_arg("hello; rm -rf /"));
        assert!(!is_safe_for_command_arg("hello `whoami`"));
        assert!(!is_safe_for_command_arg("hello$(whoami)"));
        assert!(!is_safe_for_command_arg("hello\nworld"));
    }

    #[test]
    fn test_validate_command_arg() {
        assert!(validate_command_arg("hello").is_ok());
        assert!(validate_command_arg("hello world").is_ok());
        assert!(validate_command_arg("hello; world").is_err());
        assert!(validate_command_arg("hello$world").is_err());
    }

    #[test]
    fn test_sanitize_command_arg() {
        assert_eq!(sanitize_command_arg("hello"), "hello");
        assert_eq!(sanitize_command_arg("hello; world"), "hello world");
        assert_eq!(sanitize_command_arg("hello$world"), "helloworld");
    }

    #[test]
    fn test_validate_task_title() {
        assert!(validate_task_title("Implement feature X").is_ok());
        assert!(validate_task_title("").is_err());
        assert!(validate_task_title(&"a".repeat(600)).is_err());
        assert!(validate_task_title("hello\nworld").is_err());
    }

    #[test]
    fn test_validate_branch_name() {
        assert!(validate_branch_name("main").is_ok());
        assert!(validate_branch_name("feature/my-feature").is_ok());
        assert!(validate_branch_name("fix-bug-123").is_ok());
        assert!(validate_branch_name("").is_err());
        assert!(validate_branch_name(".hidden").is_err());
        assert!(validate_branch_name("invalid..name").is_err());
        assert!(validate_branch_name("invalid name").is_err());
        assert!(validate_branch_name("branch.lock").is_err());
        assert!(validate_branch_name("@").is_err());
    }

    #[test]
    fn test_validate_model_name() {
        assert!(validate_model_name("claude-sonnet-4-6").is_ok());
        assert!(validate_model_name("claude-opus-4.6").is_ok());
        assert!(validate_model_name("gpt-4").is_ok());
        assert!(validate_model_name("").is_err());
        assert!(validate_model_name("model/name").is_err());
        assert!(validate_model_name("model name").is_err());
    }

    #[test]
    fn test_sanitize_model_name() {
        assert_eq!(sanitize_model_name("claude-sonnet-4-6"), "claude-sonnet-4-6");
        assert_eq!(sanitize_model_name("model/name"), "modelname");
        assert_eq!(sanitize_model_name("model name"), "modelname");
    }
}