//! Interactive clarification module
//!
//! This module provides interactive user clarification functionality
//! for the --ask flag, allowing ltmatrix to ask users questions before
//! generating execution plans.

pub mod clarify;
pub mod runner;

pub use clarify::{
    analyze_goal_ambiguity, ClarificationQuestion, ClarificationSession, QuestionType,
};
pub use runner::{ClarificationRunner, NonInteractiveRunner};
