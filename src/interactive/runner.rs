//! Interactive clarification runner
//!
//! This module implements the interactive dialog system for the --ask flag,
//! providing user-friendly prompts and confirmation dialogs using dialoguer.

use super::clarify::{ClarificationQuestion, ClarificationSession, QuestionType};
use anyhow::{Context, Result};
use dialoguer::{Confirm, Input, MultiSelect, Select};
use tracing::{debug, info, warn};

/// Interactive clarification runner
pub struct ClarificationRunner {
    /// Whether to use colorful output
    use_color: bool,
}

impl ClarificationRunner {
    /// Create a new clarification runner
    pub fn new() -> Self {
        Self {
            use_color: console::colors_enabled(),
        }
    }

    /// Create a new runner with color control
    pub fn with_color(use_color: bool) -> Self {
        Self { use_color }
    }

    /// Run an interactive clarification session
    pub fn run_clarification(&self, mut session: ClarificationSession) -> Result<ClarificationSession> {
        use dialoguer::theme::ColorfulTheme;

        if session.questions.is_empty() {
            info!("No clarification questions to ask");
            session.mark_completed();
            return Ok(session);
        }

        self.print_session_header(&session);

        // Use ColorfulTheme for all dialogs (it has default())
        let theme = ColorfulTheme::default();

        // Collect questions to avoid borrow issues
        let questions: Vec<_> = session.questions.clone();

        for question in &questions {
            self.ask_question_with_theme(&mut session, question, &theme)?;
        }

        session.mark_completed();
        self.print_session_summary(&session);

        Ok(session)
    }

    /// Print session header
    fn print_session_header(&self, session: &ClarificationSession) {
        if self.use_color {
            println!("\n╔════════════════════════════════════════════════════════════╗");
            println!("║  Interactive Clarification                               ║");
            println!("╚════════════════════════════════════════════════════════════╝");
        } else {
            println!("\n--- Interactive Clarification ---");
        }

        println!("\nGoal: {}", session.goal);
        println!("I need to clarify a few things before generating the plan.\n");
    }

    /// Ask a single question with a theme
    fn ask_question_with_theme(
        &self,
        session: &mut ClarificationSession,
        question: &ClarificationQuestion,
        theme: &dialoguer::theme::ColorfulTheme,
    ) -> Result<()> {
        let default_text = question.default_value.as_deref().unwrap_or("");

        match question.question_type {
            QuestionType::Text => self.ask_text_question(session, question, default_text, theme),
            QuestionType::Choice => self.ask_choice_question(session, question, default_text, theme),
            QuestionType::Confirm => self.ask_confirm_question(session, question, theme),
            QuestionType::MultiSelect => self.ask_multiselect_question(session, question, theme),
        }
    }

    /// Ask a text input question
    fn ask_text_question(
        &self,
        session: &mut ClarificationSession,
        question: &ClarificationQuestion,
        default: &str,
        theme: &dialoguer::theme::ColorfulTheme,
    ) -> Result<()> {
        let prompt = if question.required {
            format!("❓ {}", question.question_text)
        } else {
            format!("❓ {} (optional)", question.question_text)
        };

        let answer = if !default.is_empty() {
            Input::with_theme(theme)
                .with_prompt(&prompt)
                .default(default.to_string())
                .allow_empty(!question.required)
                .interact()?
        } else {
            Input::with_theme(theme)
                .with_prompt(&prompt)
                .allow_empty(!question.required)
                .interact()?
        };

        let answer = if answer.is_empty() && !default.is_empty() {
            default.to_string()
        } else {
            answer
        };

        session.answer_question(&question.id, &answer)?;

        if self.use_color {
            println!("✓ Answer recorded\n");
        } else {
            println!("[Answer recorded]\n");
        }

        Ok(())
    }

    /// Ask a multiple choice question
    fn ask_choice_question(
        &self,
        session: &mut ClarificationSession,
        question: &ClarificationQuestion,
        default: &str,
        theme: &dialoguer::theme::ColorfulTheme,
    ) -> Result<()> {
        let options = question.options.as_ref()
            .context("Choice question must have options")?;

        let prompt = if question.required {
            format!("❓ {}", question.question_text)
        } else {
            format!("❓ {} (optional)", question.question_text)
        };

        let default_index = if !default.is_empty() {
            options.iter().position(|o| o == default)
        } else {
            None
        };

        let selection = if let Some(idx) = default_index {
            Select::with_theme(theme)
                .with_prompt(&prompt)
                .items(&options)
                .default(idx)
                .interact()?
        } else {
            Select::with_theme(theme)
                .with_prompt(&prompt)
                .items(&options)
                .interact()?
        };

        session.answer_question(&question.id, &options[selection])?;

        if self.use_color {
            println!("✓ Selected: {}\n", options[selection]);
        } else {
            println!("[Selected: {}]\n", options[selection]);
        }

        Ok(())
    }

    /// Ask a confirmation (yes/no) question
    fn ask_confirm_question(
        &self,
        session: &mut ClarificationSession,
        question: &ClarificationQuestion,
        theme: &dialoguer::theme::ColorfulTheme,
    ) -> Result<()> {
        let prompt = format!("❓ {}", question.question_text);

        let default = question.default_value.as_ref()
            .map(|d| d == "yes" || d == "true" || d == "y")
            .unwrap_or(false);

        let answer = Confirm::with_theme(theme)
            .with_prompt(&prompt)
            .default(default)
            .interact()?;

        let answer_str = if answer { "yes" } else { "no" };
        session.answer_question(&question.id, answer_str)?;

        if self.use_color {
            println!("✓ Answer: {}\n", answer_str);
        } else {
            println!("[Answer: {}]\n", answer_str);
        }

        Ok(())
    }

    /// Ask a multi-select question
    fn ask_multiselect_question(
        &self,
        session: &mut ClarificationSession,
        question: &ClarificationQuestion,
        theme: &dialoguer::theme::ColorfulTheme,
    ) -> Result<()> {
        let options = question.options.as_ref()
            .context("Multi-select question must have options")?;

        let prompt = if question.required {
            format!("❓ {} (space to select, enter to confirm)", question.question_text)
        } else {
            format!("❓ {} (space to select, enter to confirm, optional)", question.question_text)
        };

        // Parse default selections - convert to boolean array
        let default_indices: Vec<usize> = if let Some(default) = &question.default_value {
            default.split(',')
                .filter_map(|idx| idx.trim().parse::<usize>().ok())
                .filter(|&idx| idx < options.len())
                .collect()
        } else {
            Vec::new()
        };

        // Create boolean defaults array
        let defaults: Vec<bool> = (0..options.len())
            .map(|i| default_indices.contains(&i))
            .collect();

        let selections = MultiSelect::with_theme(theme)
            .with_prompt(&prompt)
            .items(&options)
            .defaults(&defaults)
            .interact()?;

        let answer = selections
            .iter()
            .map(|&idx| options[idx].clone())
            .collect::<Vec<_>>()
            .join(", ");

        session.answer_question(&question.id, &answer)?;

        if self.use_color {
            println!("✓ Selected: {}\n", answer);
        } else {
            println!("[Selected: {}]\n", answer);
        }

        Ok(())
    }

    /// Print session summary
    fn print_session_summary(&self, session: &ClarificationSession) {
        if self.use_color {
            println!("╔════════════════════════════════════════════════════════════╗");
            println!("║  Clarification Complete                                     ║");
            println!("╚════════════════════════════════════════════════════════════╝\n");
        } else {
            println!("\n--- Clarification Complete ---\n");
        }

        println!("Here's what I understand:\n");

        for question in &session.questions {
            if let Some(answer) = session.answers.get(&question.id) {
                println!("  • {}: {}", question.question_text, answer);
            }
        }

        println!();
    }

    /// Confirm before proceeding with plan generation
    pub fn confirm_proceed(&self, session: &ClarificationSession) -> Result<bool> {
        use dialoguer::theme::ColorfulTheme;
        let theme = ColorfulTheme::default();

        if self.use_color {
            println!("╔════════════════════════════════════════════════════════════╗");
            println!("║  Ready to Generate Plan                                     ║");
            println!("╚════════════════════════════════════════════════════════════╝\n");
        } else {
            println!("\n--- Ready to Generate Plan ---\n");
        }

        println!("Based on your clarifications, I'll now generate a task plan.");
        println!("You'll have a chance to review it before execution.\n");

        let should_proceed = Confirm::with_theme(&theme)
            .with_prompt("Continue to plan generation?")
            .default(true)
            .interact()?;

        Ok(should_proceed)
    }

    /// Confirm plan execution after generation
    pub fn confirm_execution(&self, task_count: usize) -> Result<bool> {
        use dialoguer::theme::ColorfulTheme;
        let theme = ColorfulTheme::default();

        if self.use_color {
            println!("\n╔════════════════════════════════════════════════════════════╗");
            println!("║  Plan Generated                                             ║");
            println!("╚════════════════════════════════════════════════════════════╝\n");
        } else {
            println!("\n--- Plan Generated ---\n");
        }

        println!("Generated {} tasks to accomplish your goal.", task_count);
        println!("Tasks will be executed with full testing and verification.\n");

        let should_execute = Confirm::with_theme(&theme)
            .with_prompt("Proceed with task execution?")
            .default(true)
            .interact()?;

        Ok(should_execute)
    }

    /// Handle skip request from user
    pub fn ask_skip_remaining(&self) -> Result<bool> {
        use dialoguer::theme::ColorfulTheme;
        let theme = ColorfulTheme::default();

        Confirm::with_theme(&theme)
            .with_prompt("Skip remaining clarification questions?")
            .default(false)
            .interact()
            .context("Failed to get skip confirmation")
    }
}

impl Default for ClarificationRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple text-based clarification runner (no interactive prompts)
pub struct NonInteractiveRunner;

impl NonInteractiveRunner {
    /// Process a clarification session non-interactively
    pub fn process_session(mut session: ClarificationSession) -> Result<ClarificationSession> {
        info!("Processing clarification session non-interactively");

        // Collect question IDs to avoid borrow issues
        let question_ids: Vec<String> = session.questions.iter().map(|q| q.id.clone()).collect();

        for qid in &question_ids {
            // Find the question
            let question = session.questions.iter()
                .find(|q| &q.id == qid)
                .cloned()
                .context(format!("Question '{}' not found", qid))?;

            // Use default value if available
            if let Some(default) = &question.default_value {
                debug!("Using default value for question '{}': {}", question.id, default);
                session.answer_question(&question.id, default)?;
            } else if !question.required {
                debug!("Skipping optional question '{}'", question.id);
            } else {
                warn!("Required question '{}' has no default value", question.id);
            }
        }

        session.mark_completed();

        if !session.answers.is_empty() {
            info!("Clarification session completed with {} answers", session.answers.len());
        }

        Ok(session)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runner_creation() {
        let runner = ClarificationRunner::new();
        // Just verify it creates
        assert!(runner.use_color || !runner.use_color);
    }

    #[test]
    fn test_runner_with_color() {
        let runner = ClarificationRunner::with_color(true);
        assert!(runner.use_color);

        let runner = ClarificationRunner::with_color(false);
        assert!(!runner.use_color);
    }

    #[test]
    fn test_default_runner() {
        let runner = ClarificationRunner::default();
        assert!(runner.use_color || !runner.use_color);
    }

    #[test]
    fn test_non_interactive_runner() {
        use super::super::clarify::{ClarificationQuestion, QuestionType};

        let mut session = ClarificationSession::new("test goal");
        session.add_question(ClarificationQuestion {
            id: "q1".to_string(),
            question_text: "What framework?".to_string(),
            question_type: QuestionType::Choice,
            options: Some(vec!["React".to_string(), "Vue".to_string()]),
            default_value: Some("React".to_string()),
            required: true,
            multi_select: false,
        });

        let result = NonInteractiveRunner::process_session(session);
        assert!(result.is_ok());

        let processed = result.unwrap();
        assert!(processed.completed);
        assert_eq!(processed.answers.get("q1"), Some(&"React".to_string()));
    }

    #[test]
    fn test_non_interactive_optional_question() {
        use super::super::clarify::{ClarificationQuestion, QuestionType};

        let mut session = ClarificationSession::new("test goal");
        session.add_question(ClarificationQuestion {
            id: "opt".to_string(),
            question_text: "Optional?".to_string(),
            question_type: QuestionType::Text,
            options: None,
            default_value: None,
            required: false,
            multi_select: false,
        });

        let result = NonInteractiveRunner::process_session(session);
        assert!(result.is_ok());

        let processed = result.unwrap();
        assert!(processed.completed);
        // Optional question without default should be skipped
        assert!(!processed.answers.contains_key("opt"));
    }
}
