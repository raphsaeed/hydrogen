//! Curriculum compiler subsystem: deterministic Quality Gate (`gate`) and (in Phase 3) Gemini 4 agentic loop.

pub mod gate;

pub use gate::{
    GateCategory, GateIssue, GateReport, verify_blueprints, verify_question_package,
};
