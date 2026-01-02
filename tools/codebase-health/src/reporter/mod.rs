//! Report generation module
//!
//! Generates health reports in various formats.

mod markdown;
mod json;

pub use markdown::MarkdownReporter;
pub use json::JsonReporter;

use crate::analyzer::CodebaseAnalysis;
use anyhow::Result;

/// Trait for report generators
pub trait Reporter {
    fn generate(analysis: &CodebaseAnalysis) -> Result<String>;
}
