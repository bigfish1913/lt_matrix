//! Interactive clarification system
//!
//! This module implements the --ask functionality which prompts users
//! for clarification before generating plans, using Claude to generate
//! questions based on goal ambiguity and dialoguer for interactive prompts.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Type of clarification question
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum QuestionType {
    /// Free-form text input
    Text,
    /// Multiple choice (single select)
    Choice,
    /// Yes/No confirmation
    Confirm,
    /// Multiple choice (multi-select)
    MultiSelect,
}

impl std::fmt::Display for QuestionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QuestionType::Text => write!(f, "text input"),
            QuestionType::Choice => write!(f, "multiple choice"),
            QuestionType::Confirm => write!(f, "confirmation"),
            QuestionType::MultiSelect => write!(f, "multi-select"),
        }
    }
}

/// A clarification question to ask the user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClarificationQuestion {
    /// Unique identifier for the question
    pub id: String,
    /// The question text to display
    pub question_text: String,
    /// Type of question
    pub question_type: QuestionType,
    /// Available options (for Choice/MultiSelect/Confirm types)
    pub options: Option<Vec<String>>,
    /// Default value if skipped
    pub default_value: Option<String>,
    /// Whether the question must be answered
    pub required: bool,
    /// Whether multiple selections are allowed
    pub multi_select: bool,
}

/// A clarification session with the user
#[derive(Debug, Clone)]
pub struct ClarificationSession {
    /// The original goal provided by the user
    pub goal: String,
    /// Questions to ask
    pub questions: Vec<ClarificationQuestion>,
    /// Answers provided by the user
    pub answers: HashMap<String, String>,
    /// Whether the session is completed
    pub completed: bool,
}

impl ClarificationSession {
    /// Create a new clarification session for a goal
    pub fn new(goal: &str) -> Self {
        Self {
            goal: goal.to_string(),
            questions: Vec::new(),
            answers: HashMap::new(),
            completed: false,
        }
    }

    /// Add a question to the session
    pub fn add_question(&mut self, question: ClarificationQuestion) {
        // Check for duplicate question IDs
        if let Some(pos) = self.questions.iter().position(|q| q.id == question.id) {
            // Replace existing question with same ID
            self.questions[pos] = question;
        } else {
            self.questions.push(question);
        }
    }

    /// Answer a question
    pub fn answer_question(&mut self, question_id: &str, answer: &str) -> Result<()> {
        // Verify question exists
        if !self.questions.iter().any(|q| q.id == question_id) {
            anyhow::bail!("Question '{}' does not exist", question_id);
        }

        self.answers
            .insert(question_id.to_string(), answer.to_string());
        Ok(())
    }

    /// Skip a question (uses default value if available)
    pub fn skip_question(&mut self, question_id: &str) -> Result<()> {
        let question = self
            .questions
            .iter()
            .find(|q| q.id == question_id)
            .context(format!("Question '{}' not found", question_id))?;

        if question.required && question.default_value.is_none() {
            anyhow::bail!("Cannot skip required question '{}'", question_id);
        }

        if let Some(default) = &question.default_value {
            self.answers
                .insert(question_id.to_string(), default.clone());
        }

        Ok(())
    }

    /// Check if all required questions have been answered
    pub fn all_required_answered(&self) -> bool {
        self.questions
            .iter()
            .filter(|q| q.required)
            .all(|q| self.answers.contains_key(&q.id))
    }

    /// Generate prompt injection for planning
    pub fn generate_prompt_injection(&self) -> String {
        if self.answers.is_empty() {
            return String::new();
        }

        let mut injection = String::from("\n\n=== User Clarifications ===\n");

        for question in &self.questions {
            if let Some(answer) = self.answers.get(&question.id) {
                injection.push_str(&format!("\nQ: {}\nA: {}\n", question.question_text, answer));
            }
        }

        injection.push_str("=== End Clarifications ===\n");
        injection
    }

    /// Mark the session as completed
    pub fn mark_completed(&mut self) {
        self.completed = true;
    }

    /// Get unanswered required questions
    pub fn unanswered_required(&self) -> Vec<&ClarificationQuestion> {
        self.questions
            .iter()
            .filter(|q| q.required && !self.answers.contains_key(&q.id))
            .collect()
    }
}

/// Generate clarification questions based on goal ambiguity
///
/// This function analyzes the goal to determine if clarification is needed
/// and generates appropriate questions using the Claude API.
pub async fn generate_clarification_questions(
    goal: &str,
    claude_client: &Option<Box<dyn ClaudeClient>>,
) -> Result<Vec<ClarificationQuestion>> {
    // Basic local analysis for common ambiguous patterns
    let mut questions = analyze_goal_ambiguity(goal);

    // If we have a Claude client, use it to generate more sophisticated questions
    if let Some(client) = claude_client {
        let ai_questions = client.generate_questions(goal).await?;
        questions.extend(ai_questions);
    }

    Ok(questions)
}

/// Analyze goal ambiguity locally (without AI)
pub fn analyze_goal_ambiguity(goal: &str) -> Vec<ClarificationQuestion> {
    let mut questions = Vec::new();
    let goal_lower = goal.to_lowercase();

    // Check for authentication-related ambiguity
    if goal_lower.contains("auth") || goal_lower.contains("login") || goal_lower.contains("user") {
        if !goal_lower.contains("jwt")
            && !goal_lower.contains("oauth")
            && !goal_lower.contains("session")
        {
            questions.push(ClarificationQuestion {
                id: "auth_method".to_string(),
                question_text: "Which authentication method should be implemented?".to_string(),
                question_type: QuestionType::Choice,
                options: Some(vec![
                    "JWT (JSON Web Tokens)".to_string(),
                    "OAuth 2.0".to_string(),
                    "Session-based".to_string(),
                    "Basic Auth".to_string(),
                ]),
                default_value: Some("JWT (JSON Web Tokens)".to_string()),
                required: true,
                multi_select: false,
            });
        }
    }

    // Check for API type ambiguity
    if goal_lower.contains("api") {
        if !goal_lower.contains("rest")
            && !goal_lower.contains("graphql")
            && !goal_lower.contains("grpc")
        {
            questions.push(ClarificationQuestion {
                id: "api_type".to_string(),
                question_text: "What type of API should be built?".to_string(),
                question_type: QuestionType::Choice,
                options: Some(vec![
                    "REST API".to_string(),
                    "GraphQL API".to_string(),
                    "gRPC".to_string(),
                ]),
                default_value: Some("REST API".to_string()),
                required: true,
                multi_select: false,
            });
        }
    }

    // Check for database ambiguity
    if goal_lower.contains("database")
        || goal_lower.contains("db")
        || goal_lower.contains("storage")
    {
        if !goal_lower.contains("sql")
            && !goal_lower.contains("nosql")
            && !goal_lower.contains("postgres")
            && !goal_lower.contains("mysql")
            && !goal_lower.contains("mongodb")
        {
            questions.push(ClarificationQuestion {
                id: "database_type".to_string(),
                question_text: "Which database technology should be used?".to_string(),
                question_type: QuestionType::Choice,
                options: Some(vec![
                    "PostgreSQL".to_string(),
                    "MySQL".to_string(),
                    "MongoDB".to_string(),
                    "SQLite".to_string(),
                    "Redis".to_string(),
                ]),
                default_value: Some("PostgreSQL".to_string()),
                required: true,
                multi_select: false,
            });
        }
    }

    // Check for testing ambiguity
    if goal_lower.contains("test") {
        if !goal_lower.contains("unit")
            && !goal_lower.contains("integration")
            && !goal_lower.contains("e2e")
        {
            questions.push(ClarificationQuestion {
                id: "test_type".to_string(),
                question_text: "What type of tests are needed?".to_string(),
                question_type: QuestionType::MultiSelect,
                options: Some(vec![
                    "Unit tests".to_string(),
                    "Integration tests".to_string(),
                    "End-to-end tests".to_string(),
                    "Property-based tests".to_string(),
                ]),
                default_value: Some("Unit tests".to_string()),
                required: true,
                multi_select: true,
            });
        }
    }

    // Very short goals are always ambiguous
    if goal.len() < 30 && !goal.contains(".") {
        questions.push(ClarificationQuestion {
            id: "more_details".to_string(),
            question_text: "Could you provide more details about what you'd like to accomplish?"
                .to_string(),
            question_type: QuestionType::Text,
            options: None,
            default_value: None,
            required: true,
            multi_select: false,
        });
    }

    questions
}

/// Trait for Claude client to generate questions
#[async_trait::async_trait]
pub trait ClaudeClient: Send + Sync {
    async fn generate_questions(&self, goal: &str) -> Result<Vec<ClarificationQuestion>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_goal_ambiguity_auth() {
        let questions = analyze_goal_ambiguity("add authentication");
        assert!(!questions.is_empty());
        assert!(questions.iter().any(|q| q.id == "auth_method"));
    }

    #[test]
    fn test_analyze_goal_ambiguity_api() {
        let questions = analyze_goal_ambiguity("build an api");
        assert!(!questions.is_empty());
        assert!(questions.iter().any(|q| q.id == "api_type"));
    }

    #[test]
    fn test_analyze_goal_ambiguity_specific() {
        let questions = analyze_goal_ambiguity("add JWT authentication to the user service");
        // Should not ask about auth type since JWT is specified
        assert!(!questions.iter().any(|q| q.id == "auth_method"));
    }

    #[test]
    fn test_analyze_goal_ambiguity_short() {
        let questions = analyze_goal_ambiguity("add tests");
        assert!(!questions.is_empty());
        assert!(questions.iter().any(|q| q.id == "more_details"));
    }

    #[test]
    fn test_question_type_display() {
        assert_eq!(format!("{}", QuestionType::Text), "text input");
        assert_eq!(format!("{}", QuestionType::Choice), "multiple choice");
    }

    #[test]
    fn test_clarification_session_new() {
        let session = ClarificationSession::new("test goal");
        assert_eq!(session.goal, "test goal");
        assert!(session.questions.is_empty());
        assert!(session.answers.is_empty());
        assert!(!session.completed);
    }

    #[test]
    fn test_add_question() {
        let mut session = ClarificationSession::new("test");
        let question = ClarificationQuestion {
            id: "q1".to_string(),
            question_text: "Question?".to_string(),
            question_type: QuestionType::Text,
            options: None,
            default_value: None,
            required: true,
            multi_select: false,
        };
        session.add_question(question);
        assert_eq!(session.questions.len(), 1);
    }

    #[test]
    fn test_answer_question() {
        let mut session = ClarificationSession::new("test");
        session.add_question(ClarificationQuestion {
            id: "q1".to_string(),
            question_text: "Question?".to_string(),
            question_type: QuestionType::Text,
            options: None,
            default_value: None,
            required: true,
            multi_select: false,
        });
        assert!(session.answer_question("q1", "answer").is_ok());
        assert_eq!(session.answers.get("q1"), Some(&"answer".to_string()));
    }

    #[test]
    fn test_skip_question_with_default() {
        let mut session = ClarificationSession::new("test");
        session.add_question(ClarificationQuestion {
            id: "q1".to_string(),
            question_text: "Question?".to_string(),
            question_type: QuestionType::Text,
            options: None,
            default_value: Some("default".to_string()),
            required: false,
            multi_select: false,
        });
        assert!(session.skip_question("q1").is_ok());
        assert_eq!(session.answers.get("q1"), Some(&"default".to_string()));
    }

    #[test]
    fn test_all_required_answered() {
        let mut session = ClarificationSession::new("test");
        session.add_question(ClarificationQuestion {
            id: "req".to_string(),
            question_text: "Required?".to_string(),
            question_type: QuestionType::Text,
            options: None,
            default_value: None,
            required: true,
            multi_select: false,
        });
        session.add_question(ClarificationQuestion {
            id: "opt".to_string(),
            question_text: "Optional?".to_string(),
            question_type: QuestionType::Text,
            options: None,
            default_value: Some("default".to_string()),
            required: false,
            multi_select: false,
        });
        assert!(!session.all_required_answered());
        session.answer_question("req", "answer").unwrap();
        assert!(session.all_required_answered());
    }

    #[test]
    fn test_generate_prompt_injection() {
        let mut session = ClarificationSession::new("test");
        session.add_question(ClarificationQuestion {
            id: "q1".to_string(),
            question_text: "What framework?".to_string(),
            question_type: QuestionType::Choice,
            options: Some(vec!["React".to_string(), "Vue".to_string()]),
            default_value: None,
            required: true,
            multi_select: false,
        });
        session.answer_question("q1", "React").unwrap();
        let injection = session.generate_prompt_injection();
        assert!(injection.contains("User Clarifications"));
        assert!(injection.contains("What framework?"));
        assert!(injection.contains("React"));
    }

    #[test]
    fn test_duplicate_question_replacement() {
        let mut session = ClarificationSession::new("test");
        session.add_question(ClarificationQuestion {
            id: "q1".to_string(),
            question_text: "First".to_string(),
            question_type: QuestionType::Text,
            options: None,
            default_value: None,
            required: true,
            multi_select: false,
        });
        session.add_question(ClarificationQuestion {
            id: "q1".to_string(),
            question_text: "Second".to_string(),
            question_type: QuestionType::Text,
            options: None,
            default_value: None,
            required: true,
            multi_select: false,
        });
        assert_eq!(session.questions.len(), 1);
        assert_eq!(session.questions[0].question_text, "Second");
    }

    #[test]
    fn test_unanswered_required() {
        let mut session = ClarificationSession::new("test");
        session.add_question(ClarificationQuestion {
            id: "req1".to_string(),
            question_text: "Required 1".to_string(),
            question_type: QuestionType::Text,
            options: None,
            default_value: None,
            required: true,
            multi_select: false,
        });
        session.add_question(ClarificationQuestion {
            id: "req2".to_string(),
            question_text: "Required 2".to_string(),
            question_type: QuestionType::Text,
            options: None,
            default_value: None,
            required: true,
            multi_select: false,
        });
        session.answer_question("req1", "answer").unwrap();
        let unanswered = session.unanswered_required();
        assert_eq!(unanswered.len(), 1);
        assert_eq!(unanswered[0].id, "req2");
    }
}
