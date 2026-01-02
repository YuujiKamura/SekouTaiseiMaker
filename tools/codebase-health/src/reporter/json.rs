//! JSON report generator

use crate::analyzer::CodebaseAnalysis;
use crate::reporter::Reporter;
use anyhow::Result;

pub struct JsonReporter;

impl Reporter for JsonReporter {
    fn generate(analysis: &CodebaseAnalysis) -> Result<String> {
        serde_json::to_string_pretty(analysis).map_err(|e| e.into())
    }
}
