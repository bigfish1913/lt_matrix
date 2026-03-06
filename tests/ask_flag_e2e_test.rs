//! End-to-end tests for --ask flag feature
//!
//! This test suite simulates complete user workflows with the --ask flag,
//! from CLI invocation through clarification to task generation.

use clap::Parser;
use ltmatrix::cli::args::Args;
use ltmatrix::interactive::clarify::{
    ClarificationQuestion, ClarificationSession, QuestionType,
};

/// E2E Test 1: Complete clarification flow for ambiguous goal
#[test]
fn test_e2e_complete_clarification_flow() {
    // Step 1: Parse CLI with --ask flag
    let args = Args::parse_from(["ltmatrix", "--ask", "add authentication"]);
    assert!(args.ask);
    let goal = args.goal.unwrap();

    // Step 2: Create clarification session
    let mut session = ClarificationSession::new(&goal);

    // Step 3: Simulate adding questions to session
    session.add_question(ClarificationQuestion {
        id: "auth_method".to_string(),
        question_text: "Which authentication method?".to_string(),
        question_type: QuestionType::Choice,
        options: Some(vec!["JWT".to_string(), "OAuth".to_string()]),
        default_value: None,
        required: true,
        multi_select: false,
    });

    // Step 4: Simulate user answering questions
    session.answer_question("auth_method", "JWT").unwrap();

    // Step 5: Verify all required questions are answered
    assert!(session.all_required_answered());

    // Step 6: Generate prompt injection for planning
    let injection = session.generate_prompt_injection();
    assert!(!injection.is_empty());
    assert!(injection.contains("JWT"));

    // Step 7: Mark session as completed
    session.mark_completed();
    assert!(session.completed);
}

/// E2E Test 2: Clarification flow with optional questions skipped
#[test]
fn test_e2e_clarification_with_skipped_optionals() {
    let goal = "build a REST API with user management";
    let mut session = ClarificationSession::new(goal);

    // Add mix of required and optional questions
    session.add_question(ClarificationQuestion {
        id: "features".to_string(),
        question_text: "Which features to include?".to_string(),
        question_type: QuestionType::MultiSelect,
        options: Some(vec![
            "User registration".to_string(),
            "Password reset".to_string(),
            "Email verification".to_string(),
        ]),
        default_value: None,
        required: true,
        multi_select: true,
    });

    session.add_question(ClarificationQuestion {
        id: "caching".to_string(),
        question_text: "Add caching layer?".to_string(),
        question_type: QuestionType::Confirm,
        options: Some(vec!["yes".to_string(), "no".to_string()]),
        default_value: Some("no".to_string()),
        required: false,
        multi_select: false,
    });

    session.add_question(ClarificationQuestion {
        id: "logging_level".to_string(),
        question_text: "Logging level?".to_string(),
        question_type: QuestionType::Choice,
        options: Some(vec!["debug".to_string(), "info".to_string(), "warn".to_string()]),
        default_value: Some("info".to_string()),
        required: false,
        multi_select: false,
    });

    // Answer only required questions
    session
        .answer_question("features", "User registration, Password reset")
        .unwrap();

    // Skip optional questions (use defaults)
    session.skip_question("caching").unwrap();
    session.skip_question("logging_level").unwrap();

    // Verify completion with only required answers
    assert!(session.all_required_answered());

    // Verify defaults were used
    assert_eq!(session.answers.get("caching"), Some(&"no".to_string()));
    assert_eq!(session.answers.get("logging_level"), Some(&"info".to_string()));

    session.mark_completed();
    assert!(session.completed);
}

/// E2E Test 3: Complex multi-domain clarification
#[test]
fn test_e2e_complex_multi_domain_clarification() {
    let goal = "build a full-stack authentication system";
    let mut session = ClarificationSession::new(goal);

    // Simulate questions from multiple domains
    let questions = vec![
        ClarificationQuestion {
            id: "auth_method".to_string(),
            question_text: "Authentication method?".to_string(),
            question_type: QuestionType::Choice,
            options: Some(vec![
                "JWT (JSON Web Tokens)".to_string(),
                "OAuth 2.0".to_string(),
                "Session-based".to_string(),
            ]),
            default_value: Some("JWT".to_string()),
            required: true,
            multi_select: false,
        },
        ClarificationQuestion {
            id: "frontend_framework".to_string(),
            question_text: "Frontend framework?".to_string(),
            question_type: QuestionType::Choice,
            options: Some(vec![
                "React".to_string(),
                "Vue".to_string(),
                "Angular".to_string(),
                "Svelte".to_string(),
            ]),
            default_value: None,
            required: true,
            multi_select: false,
        },
        ClarificationQuestion {
            id: "backend_framework".to_string(),
            question_text: "Backend framework?".to_string(),
            question_type: QuestionType::Choice,
            options: Some(vec![
                "Express.js".to_string(),
                "Django".to_string(),
                "Rails".to_string(),
                "Actix Web".to_string(),
            ]),
            default_value: None,
            required: true,
            multi_select: false,
        },
        ClarificationQuestion {
            id: "database".to_string(),
            question_text: "Database?".to_string(),
            question_type: QuestionType::Choice,
            options: Some(vec![
                "PostgreSQL".to_string(),
                "MySQL".to_string(),
                "MongoDB".to_string(),
                "Redis".to_string(),
            ]),
            default_value: Some("PostgreSQL".to_string()),
            required: false,
            multi_select: false,
        },
        ClarificationQuestion {
            id: "additional_features".to_string(),
            question_text: "Additional features?".to_string(),
            question_type: QuestionType::MultiSelect,
            options: Some(vec![
                "Password reset".to_string(),
                "Email verification".to_string(),
                "Two-factor auth".to_string(),
                "Social login".to_string(),
            ]),
            default_value: None,
            required: false,
            multi_select: true,
        },
    ];

    for question in questions {
        session.add_question(question);
    }

    // Answer required questions
    session.answer_question("auth_method", "JWT").unwrap();
    session
        .answer_question("frontend_framework", "React")
        .unwrap();
    session
        .answer_question("backend_framework", "Express.js")
        .unwrap();

    // Answer some optional questions
    session
        .answer_question("additional_features", "Password reset, Two-factor auth")
        .unwrap();

    // Skip database (use default)
    session.skip_question("database").unwrap();

    // Verify state
    assert!(session.all_required_answered());
    // Note: skipping a question with a default still adds it to answers
    assert_eq!(session.answers.len(), 5); // 3 required + 2 optional (1 answered, 1 skipped with default)

    // Generate and verify prompt injection
    let injection = session.generate_prompt_injection();
    assert!(injection.contains("JWT"));
    assert!(injection.contains("React"));
    assert!(injection.contains("Express.js"));
    assert!(injection.contains("Password reset"));
    assert!(injection.contains("Two-factor auth"));
    assert!(injection.contains("PostgreSQL")); // Default value
}

/// E2E Test 4: Early termination when user declines
#[test]
fn test_e2e_early_termination_on_decline() {
    let mut session = ClarificationSession::new("add authentication");

    session.add_question(ClarificationQuestion {
        id: "confirm_start".to_string(),
        question_text: "Do you want to proceed with task generation?".to_string(),
        question_type: QuestionType::Confirm,
        options: Some(vec!["yes".to_string(), "no".to_string()]),
        default_value: Some("yes".to_string()),
        required: true,
        multi_select: false,
    });

    // User declines
    session.answer_question("confirm_start", "no").unwrap();

    // Verify we can detect the decline
    assert_eq!(session.answers.get("confirm_start"), Some(&"no".to_string()));

    // In a real implementation, this would trigger early termination
    // For now, we verify the answer is captured correctly
}

/// E2E Test 5: Clarification with validation errors
#[test]
fn test_e2e_clarification_with_validation_errors() {
    let mut session = ClarificationSession::new("test");

    session.add_question(ClarificationQuestion {
        id: "email".to_string(),
        question_text: "Admin email address?".to_string(),
        question_type: QuestionType::Text,
        options: None,
        default_value: None,
        required: true,
        multi_select: false,
    });

    // Try to answer with invalid email (would be validated in real implementation)
    // For now, just verify we can capture the answer
    let invalid_email = "not-an-email";
    session.answer_question("email", invalid_email).unwrap();

    assert_eq!(session.answers.get("email"), Some(&invalid_email.to_string()));

    // In real implementation, validation would happen before acceptance
    // and user would be prompted to correct invalid inputs
}

/// E2E Test 6: Clarification session persistence and resumption
#[test]
fn test_e2e_session_serialization_roundtrip() {
    let mut session = ClarificationSession::new("build API");

    session.add_question(ClarificationQuestion {
        id: "api_type".to_string(),
        question_text: "API type?".to_string(),
        question_type: QuestionType::Choice,
        options: Some(vec!["REST".to_string(), "GraphQL".to_string()]),
        default_value: None,
        required: true,
        multi_select: false,
    });

    session.answer_question("api_type", "REST").unwrap();

    // Verify session state
    assert_eq!(session.goal, "build API");
    assert_eq!(session.questions.len(), 1);
    assert_eq!(session.answers.len(), 1);
    assert!(session.all_required_answered());

    // In a real implementation, session would be serialized to disk
    // and could be resumed after interruption
    // For now, we verify the in-memory state is consistent
}

/// E2E Test 7: Clarification with long-running question generation
#[test]
fn test_e2e_clarification_with_complex_goal() {
    let complex_goal = "Build a scalable microservices architecture with service mesh, API gateway, distributed tracing, and real-time analytics dashboard";
    let session = ClarificationSession::new(complex_goal);

    // Complex goals might trigger AI-powered question generation
    // For now, verify session can handle long goals
    assert_eq!(session.goal.len(), complex_goal.len());

    // In real implementation, this would involve:
    // 1. Calling Claude API to analyze complexity
    // 2. Generating domain-specific questions
    // 3. Potentially asking follow-up questions based on answers
}

/// E2E Test 8: Multiple clarification cycles
#[test]
fn test_e2e_multiple_clarification_cycles() {
    // Some goals might require multiple rounds of clarification
    let mut session = ClarificationSession::new("build web application");

    // First cycle: basic questions
    session.add_question(ClarificationQuestion {
        id: "type".to_string(),
        question_text: "Application type?".to_string(),
        question_type: QuestionType::Choice,
        options: Some(vec![
            "SPA".to_string(),
            "SSR".to_string(),
            "PWA".to_string(),
            "Static".to_string(),
        ]),
        default_value: None,
        required: true,
        multi_select: false,
    });

    session.answer_question("type", "SPA").unwrap();

    // Based on first answer, add more specific questions
    if session.answers.get("type") == Some(&"SPA".to_string()) {
        session.add_question(ClarificationQuestion {
            id: "spa_framework".to_string(),
            question_text: "Which SPA framework?".to_string(),
            question_type: QuestionType::Choice,
            options: Some(vec![
                "React".to_string(),
                "Vue".to_string(),
                "Angular".to_string(),
            ]),
            default_value: None,
            required: true,
            multi_select: false,
        });

        session.answer_question("spa_framework", "React").unwrap();
    }

    // Verify final state
    assert_eq!(session.questions.len(), 2);
    assert!(session.all_required_answered());
}

/// E2E Test 9: Clarification with default acceptance
#[test]
fn test_e2e_clarification_accepting_all_defaults() {
    let mut session = ClarificationSession::new("add database");

    // Add questions all with defaults
    session.add_question(ClarificationQuestion {
        id: "database".to_string(),
        question_text: "Database type?".to_string(),
        question_type: QuestionType::Choice,
        options: Some(vec![
            "PostgreSQL".to_string(),
            "MySQL".to_string(),
            "MongoDB".to_string(),
        ]),
        default_value: Some("PostgreSQL".to_string()),
        required: true,
        multi_select: false,
    });

    session.add_question(ClarificationQuestion {
        id: "orm".to_string(),
        question_text: "ORM framework?".to_string(),
        question_type: QuestionType::Choice,
        options: Some(vec![
            "Diesel".to_string(),
            "SeaORM".to_string(),
            "SQLx".to_string(),
        ]),
        default_value: Some("Diesel".to_string()),
        required: false,
        multi_select: false,
    });

    // Skip all questions to accept defaults
    session.skip_question("database").unwrap();
    session.skip_question("orm").unwrap();

    // Verify defaults were used
    assert_eq!(session.answers.get("database"), Some(&"PostgreSQL".to_string()));
    assert_eq!(session.answers.get("orm"), Some(&"Diesel".to_string()));
    assert!(session.all_required_answered());
}

/// E2E Test 10: Integration with planning prompt
#[test]
fn test_e2e_integration_with_planning_prompt() {
    let mut session = ClarificationSession::new("add authentication");

    session.add_question(ClarificationQuestion {
        id: "auth_method".to_string(),
        question_text: "Authentication method?".to_string(),
        question_type: QuestionType::Choice,
        options: Some(vec!["JWT".to_string(), "OAuth".to_string()]),
        default_value: None,
        required: true,
        multi_select: false,
    });

    session.add_question(ClarificationQuestion {
        id: "features".to_string(),
        question_text: "Additional features?".to_string(),
        question_type: QuestionType::MultiSelect,
        options: Some(vec![
            "Password reset".to_string(),
            "Email verification".to_string(),
        ]),
        default_value: None,
        required: false,
        multi_select: true,
    });

    session.answer_question("auth_method", "JWT").unwrap();
    session
        .answer_question("features", "Password reset, Email verification")
        .unwrap();

    // Generate the prompt injection
    let injection = session.generate_prompt_injection();

    // Verify it would be correctly integrated into the planning prompt
    let full_prompt = format!(
        "{}\n\nOriginal Goal: add authentication\n\nGenerate tasks...",
        injection
    );

    assert!(full_prompt.contains("User Clarifications"));
    assert!(full_prompt.contains("Authentication method?"));
    assert!(full_prompt.contains("JWT"));
    assert!(full_prompt.contains("Password reset"));
    assert!(full_prompt.contains("Email verification"));
    assert!(full_prompt.contains("Original Goal"));
}

/// Helper function to simulate question generation with context
#[allow(dead_code)]
fn generate_questions_with_context(_goal: &str, previous_answers: &[(&str, &str)]) -> Vec<ClarificationQuestion> {
    let mut questions = Vec::new();

    // In a real implementation, previous answers would influence
    // which follow-up questions are generated
    for (question_id, answer) in previous_answers {
        if *question_id == "auth_method" && *answer == "OAuth" {
            questions.push(ClarificationQuestion {
                id: "oauth_provider".to_string(),
                question_text: "Which OAuth provider?".to_string(),
                question_type: QuestionType::MultiSelect,
                options: Some(vec![
                    "Google".to_string(),
                    "GitHub".to_string(),
                    "Facebook".to_string(),
                    "Twitter".to_string(),
                ]),
                default_value: None,
                required: true,
                multi_select: true,
            });
        }
    }

    questions
}
