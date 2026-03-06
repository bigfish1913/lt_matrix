//! Interactive clarification system tests
//!
//! This test suite verifies the interactive clarification functionality
//! for the --ask flag, which prompts users for clarification before
//! generating execution plans.

use ltmatrix::cli::args::Args;
use ltmatrix::interactive::clarify::{ClarificationQuestion, ClarificationSession, QuestionType};
use clap::Parser;

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_clarification_question_creation() {
        let question = ClarificationQuestion {
            id: "tech_stack".to_string(),
            question_text: "Which web framework should we use?".to_string(),
            question_type: QuestionType::Choice,
            options: Some(vec![
                "Actix Web".to_string(),
                "Axum".to_string(),
                "Rocket".to_string(),
            ]),
            default_value: Some("Actix Web".to_string()),
            required: true,
            multi_select: false,
        };

        assert_eq!(question.id, "tech_stack");
        assert!(question.required);
        assert!(!question.multi_select);
        assert_eq!(question.options.unwrap().len(), 3);
    }

    #[test]
    fn test_question_type_display() {
        assert_eq!(format!("{}", QuestionType::Text), "text input");
        assert_eq!(format!("{}", QuestionType::Choice), "multiple choice");
        assert_eq!(format!("{}", QuestionType::Confirm), "confirmation");
        assert_eq!(format!("{}", QuestionType::MultiSelect), "multi-select");
    }

    #[test]
    fn test_clarification_session_initialization() {
        let session = ClarificationSession::new("Build a REST API");

        assert_eq!(session.goal, "Build a REST API");
        assert!(session.questions.is_empty());
        assert!(session.answers.is_empty());
        assert!(!session.completed);
    }

    #[test]
    fn test_add_question_to_session() {
        let mut session = ClarificationSession::new("Add authentication");

        let question = ClarificationQuestion {
            id: "auth_method".to_string(),
            question_text: "Which authentication method?".to_string(),
            question_type: QuestionType::Choice,
            options: Some(vec!["JWT".to_string(), "OAuth".to_string(), "Session".to_string()]),
            default_value: Some("JWT".to_string()),
            required: true,
            multi_select: false,
        };

        session.add_question(question);
        assert_eq!(session.questions.len(), 1);
        assert_eq!(session.questions[0].id, "auth_method");
    }

    #[test]
    fn test_answer_question() {
        let mut session = ClarificationSession::new("Test goal");

        session.add_question(ClarificationQuestion {
            id: "q1".to_string(),
            question_text: "Question 1".to_string(),
            question_type: QuestionType::Text,
            options: None,
            default_value: None,
            required: true,
            multi_select: false,
        });

        let result = session.answer_question("q1", "My answer");
        assert!(result.is_ok());
        assert_eq!(session.answers.len(), 1);
        assert_eq!(session.answers.get("q1"), Some(&"My answer".to_string()));
    }

    #[test]
    fn test_answer_nonexistent_question() {
        let mut session = ClarificationSession::new("Test goal");

        let result = session.answer_question("nonexistent", "answer");
        assert!(result.is_err());
    }

    #[test]
    fn test_skip_question_with_default() {
        let mut session = ClarificationSession::new("Test goal");

        session.add_question(ClarificationQuestion {
            id: "skippable".to_string(),
            question_text: "Skippable question".to_string(),
            question_type: QuestionType::Text,
            options: None,
            default_value: Some("default_value".to_string()),
            required: false,
            multi_select: false,
        });

        let result = session.skip_question("skippable");
        assert!(result.is_ok());
        assert_eq!(session.answers.get("skippable"), Some(&"default_value".to_string()));
    }

    #[test]
    fn test_skip_required_question_fails() {
        let mut session = ClarificationSession::new("Test goal");

        session.add_question(ClarificationQuestion {
            id: "required".to_string(),
            question_text: "Required question".to_string(),
            question_type: QuestionType::Text,
            options: None,
            default_value: None,
            required: true,
            multi_select: false,
        });

        let result = session.skip_question("required");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_all_required_answered() {
        let mut session = ClarificationSession::new("Test goal");

        session.add_question(ClarificationQuestion {
            id: "required_q".to_string(),
            question_text: "Required".to_string(),
            question_type: QuestionType::Text,
            options: None,
            default_value: None,
            required: true,
            multi_select: false,
        });

        session.add_question(ClarificationQuestion {
            id: "optional_q".to_string(),
            question_text: "Optional".to_string(),
            question_type: QuestionType::Text,
            options: None,
            default_value: Some("default".to_string()),
            required: false,
            multi_select: false,
        });

        assert!(!session.all_required_answered());

        session.answer_question("required_q", "answer").unwrap();
        assert!(session.all_required_answered());
    }

    #[test]
    fn test_generate_planning_prompt_injection() {
        let mut session = ClarificationSession::new("Build a web app");

        session.add_question(ClarificationQuestion {
            id: "framework".to_string(),
            question_text: "Framework?".to_string(),
            question_type: QuestionType::Choice,
            options: Some(vec!["React".to_string(), "Vue".to_string()]),
            default_value: None,
            required: true,
            multi_select: false,
        });

        session.answer_question("framework", "React").unwrap();

        let injection = session.generate_prompt_injection();
        assert!(injection.contains("User Clarifications"));
        assert!(injection.contains("Framework?"));
        assert!(injection.contains("React"));
    }

    #[test]
    fn test_session_completion() {
        let mut session = ClarificationSession::new("Complete this");

        session.add_question(ClarificationQuestion {
            id: "q1".to_string(),
            question_text: "Q1".to_string(),
            question_type: QuestionType::Text,
            options: None,
            default_value: None,
            required: true,
            multi_select: false,
        });

        session.answer_question("q1", "answer").unwrap();
        session.mark_completed();

        assert!(session.completed);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_ask_flag_parsing() {
        let args = Args::try_parse_from(["ltmatrix", "--ask", "build API"]).unwrap();
        assert!(args.ask);
        assert_eq!(args.goal, Some("build API".to_string()));
    }

    #[test]
    fn test_clarification_flow_with_ambiguous_goal() {
        let goal = "Add authentication";
        let _session = ClarificationSession::new(goal);

        // Simulate generating questions based on ambiguity
        let questions = generate_questions_for_goal(goal);
        assert!(!questions.is_empty());

        // Verify questions are relevant to the goal
        for q in &questions {
            assert!(q.question_text.len() > 0);
            assert!(q.id.len() > 0);
        }
    }

    #[test]
    fn test_clarification_flow_with_clear_goal() {
        let goal = "Add unit tests for the user service login function";
        let _session = ClarificationSession::new(goal);

        let questions = generate_questions_for_goal(goal);

        // Clear goals should generate fewer or no questions
        assert!(questions.len() < 3, "Clear goals should require minimal clarification");
    }

    #[test]
    fn test_multi_select_question_handling() {
        let mut session = ClarificationSession::new("Multi-select test");

        session.add_question(ClarificationQuestion {
            id: "features".to_string(),
            question_text: "Which features?".to_string(),
            question_type: QuestionType::MultiSelect,
            options: Some(vec![
                "Feature A".to_string(),
                "Feature B".to_string(),
                "Feature C".to_string(),
            ]),
            default_value: None,
            required: true,
            multi_select: true,
        });

        // Multi-select answers should be comma-separated
        let result = session.answer_question("features", "Feature A, Feature C");
        assert!(result.is_ok());
        assert_eq!(session.answers.get("features"), Some(&"Feature A, Feature C".to_string()));
    }

    #[test]
    fn test_confirm_question_handling() {
        let mut session = ClarificationSession::new("Confirmation test");

        session.add_question(ClarificationQuestion {
            id: "confirm".to_string(),
            question_text: "Continue?".to_string(),
            question_type: QuestionType::Confirm,
            options: Some(vec!["yes".to_string(), "no".to_string()]),
            default_value: Some("yes".to_string()),
            required: true,
            multi_select: false,
        });

        // Confirm questions should accept yes/no
        let result = session.answer_question("confirm", "yes");
        assert!(result.is_ok());
        assert_eq!(session.answers.get("confirm"), Some(&"yes".to_string()));
    }
}

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_empty_goal() {
        let session = ClarificationSession::new("");
        assert_eq!(session.goal, "");
    }

    #[test]
    fn test_no_questions_needed() {
        let session = ClarificationSession::new("Very clear and specific task");
        assert!(session.questions.is_empty());
        assert!(session.all_required_answered());
    }

    #[test]
    fn test_duplicate_question_ids() {
        let mut session = ClarificationSession::new("Duplicate test");

        let q1 = ClarificationQuestion {
            id: "duplicate".to_string(),
            question_text: "First".to_string(),
            question_type: QuestionType::Text,
            options: None,
            default_value: None,
            required: true,
            multi_select: false,
        };

        let q2 = ClarificationQuestion {
            id: "duplicate".to_string(),
            question_text: "Second".to_string(),
            question_type: QuestionType::Text,
            options: None,
            default_value: None,
            required: true,
            multi_select: false,
        };

        session.add_question(q1);

        // Adding duplicate question should handle gracefully
        // (either replace or return error - implementation choice)
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            session.add_question(q2);
        }));

        // Should not panic
        assert!(result.is_ok());
    }

    #[test]
    fn test_special_characters_in_answers() {
        let mut session = ClarificationSession::new("Special chars");

        session.add_question(ClarificationQuestion {
            id: "special".to_string(),
            question_text: "Special?".to_string(),
            question_type: QuestionType::Text,
            options: None,
            default_value: None,
            required: true,
            multi_select: false,
        });

        let special_answer = "Answer with 'quotes', \"double quotes\", and\n newlines";
        let result = session.answer_question("special", special_answer);
        assert!(result.is_ok());
    }

    #[test]
    fn test_very_long_question_text() {
        let long_text = "A".repeat(10000);
        let question = ClarificationQuestion {
            id: "long".to_string(),
            question_text: long_text.clone(),
            question_type: QuestionType::Text,
            options: None,
            default_value: None,
            required: true,
            multi_select: false,
        };

        assert_eq!(question.question_text.len(), 10000);
    }
}

// Helper function to simulate question generation
fn generate_questions_for_goal(goal: &str) -> Vec<ClarificationQuestion> {
    let mut questions = Vec::new();

    // Simulate analyzing goal ambiguity
    let goal_lower = goal.to_lowercase();

    if goal_lower.contains("authentication") || goal_lower.contains("auth") {
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

    if goal_lower.contains("api") && !goal_lower.contains("rest") && !goal_lower.contains("graphql") {
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

    if goal.len() < 20 {
        // Very short goals are ambiguous
        questions.push(ClarificationQuestion {
            id: "clarification".to_string(),
            question_text: "Could you provide more details about what you'd like to accomplish?".to_string(),
            question_type: QuestionType::Text,
            options: None,
            default_value: None,
            required: true,
            multi_select: false,
        });
    }

    questions
}
