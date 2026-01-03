//! Report generation module
//!
//! Generates health reports in various formats.

mod markdown;
mod json;
mod html;

pub use markdown::MarkdownReporter;
pub use json::JsonReporter;
pub use html::HtmlReporter;

use crate::analyzer::CodebaseAnalysis;
use anyhow::Result;

/// Trait for report generators
pub trait Reporter {
    fn generate(analysis: &CodebaseAnalysis) -> Result<String>;
}
