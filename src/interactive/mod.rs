//! Interactive clarification module
//!
//! This module provides interactive user clarification functionality
//! for the --ask flag, allowing ltmatrix to ask users questions before
//! generating execution plans.

pub mod clarify;

pub use clarify::{ClarificationQuestion, ClarificationSession, QuestionType};
