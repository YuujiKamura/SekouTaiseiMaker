//! Markdown report generator

use crate::analyzer::{CodebaseAnalysis, IssueCategory, Severity};
use crate::reporter::Reporter;
use anyhow::Result;

pub struct MarkdownReporter;

impl Reporter for MarkdownReporter {
    fn generate(analysis: &CodebaseAnalysis) -> Result<String> {
        let mut output = String::new();

        // Header
        output.push_str("# Codebase Health Report\n\n");
        output.push_str(&format!("**Project:** `{}`\n\n", analysis.root_path.display()));
        output.push_str(&format!("**Analyzed:** {}\n\n", analysis.analyzed_at.format("%Y-%m-%d %H:%M:%S UTC")));

        // Health Score Badge
        let score_color = match analysis.health_score {
            90..=100 => "brightgreen",
            70..=89 => "green",
            50..=69 => "yellow",
            30..=49 => "orange",
            _ => "red",
        };
        output.push_str(&format!(
            "![Health Score](https://img.shields.io/badge/Health%20Score-{}%25-{})\n\n",
            analysis.health_score, score_color
        ));

        // Summary Section
        output.push_str("## Summary\n\n");
        output.push_str("| Metric | Value |\n");
        output.push_str("|--------|-------|\n");
        output.push_str(&format!("| Total Files | {} |\n", analysis.total_stats.total_files));
        output.push_str(&format!("| Total Lines | {} |\n", analysis.total_stats.total_lines));
        output.push_str(&format!("| Code Lines | {} |\n", analysis.total_stats.code_lines));
        output.push_str(&format!("| Comment Lines | {} |\n", analysis.total_stats.comment_lines));
        output.push_str(&format!("| Test Files | {} |\n", analysis.total_stats.test_files));
        output.push_str(&format!("| Health Score | {}/100 |\n", analysis.health_score));
        output.push('\n');

        // Language Breakdown
        output.push_str("## Language Breakdown\n\n");
        output.push_str("| Language | Files | Lines | Code | Comments |\n");
        output.push_str("|----------|-------|-------|------|----------|\n");

        let mut langs: Vec<_> = analysis.file_stats.iter().collect();
        langs.sort_by(|a, b| b.1.code_lines.cmp(&a.1.code_lines));

        for (lang, stats) in langs {
            output.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                lang, stats.file_count, stats.total_lines, stats.code_lines, stats.comment_lines
            ));
        }
        output.push('\n');

        // Complexity Section
        output.push_str("## Complexity Analysis\n\n");
        output.push_str(&format!("- **Functions Analyzed:** {}\n", analysis.complexity.total_functions));
        output.push_str(&format!("- **Average Complexity:** {:.2}\n", analysis.complexity.avg_complexity));
        output.push_str(&format!("- **Max Complexity:** {}\n", analysis.complexity.max_complexity));

        if let Some(ref func) = analysis.complexity.max_complexity_function {
            output.push_str(&format!("  - Location: `{}`\n", func));
        }
        output.push('\n');

        if !analysis.complexity.long_functions.is_empty() {
            output.push_str("### Long Functions (>50 lines)\n\n");
            for func in &analysis.complexity.long_functions {
                output.push_str(&format!("- `{}`\n", func));
            }
            output.push('\n');
        }

        if !analysis.complexity.deeply_nested.is_empty() {
            output.push_str("### Deeply Nested Functions (>4 levels)\n\n");
            for func in &analysis.complexity.deeply_nested {
                output.push_str(&format!("- `{}`\n", func));
            }
            output.push('\n');
        }

        // Issues Section
        output.push_str("## Issues\n\n");

        let critical: Vec<_> = analysis.issues.iter().filter(|i| i.severity == Severity::Critical).collect();
        let high: Vec<_> = analysis.issues.iter().filter(|i| i.severity == Severity::High).collect();
        let medium: Vec<_> = analysis.issues.iter().filter(|i| i.severity == Severity::Medium).collect();
        let low: Vec<_> = analysis.issues.iter().filter(|i| i.severity == Severity::Low).collect();

        output.push_str(&format!(
            "| Severity | Count |\n|----------|-------|\n| Critical | {} |\n| High | {} |\n| Medium | {} |\n| Low | {} |\n\n",
            critical.len(), high.len(), medium.len(), low.len()
        ));

        if !critical.is_empty() {
            output.push_str("### Critical Issues\n\n");
            for issue in &critical {
                output.push_str(&Self::format_issue(issue));
            }
        }

        if !high.is_empty() {
            output.push_str("### High Priority Issues\n\n");
            for issue in &high {
                output.push_str(&Self::format_issue(issue));
            }
        }

        if !medium.is_empty() {
            output.push_str("### Medium Priority Issues\n\n");
            for issue in medium.iter().take(10) {
                output.push_str(&Self::format_issue(issue));
            }
            if medium.len() > 10 {
                output.push_str(&format!("\n*...and {} more medium priority issues*\n\n", medium.len() - 10));
            }
        }

        if !low.is_empty() {
            output.push_str("### Low Priority Issues\n\n");
            output.push_str(&format!("*{} low priority issues found. Run with `--format json` for full details.*\n\n", low.len()));
        }

        // Issue Categories
        output.push_str("### Issues by Category\n\n");
        let categories = [
            (IssueCategory::Security, "Security"),
            (IssueCategory::CodeQuality, "Code Quality"),
            (IssueCategory::Performance, "Performance"),
            (IssueCategory::Maintainability, "Maintainability"),
            (IssueCategory::Documentation, "Documentation"),
            (IssueCategory::Testing, "Testing"),
            (IssueCategory::BestPractice, "Best Practice"),
        ];

        output.push_str("| Category | Count |\n|----------|-------|\n");
        for (cat, name) in &categories {
            let count = analysis.issues.iter().filter(|i| &i.category == cat).count();
            if count > 0 {
                output.push_str(&format!("| {} | {} |\n", name, count));
            }
        }
        output.push('\n');

        // Recommendations
        output.push_str("## Recommendations\n\n");

        if analysis.health_score >= 90 {
            output.push_str("Excellent codebase health! Keep up the good work.\n\n");
        } else {
            if critical.len() > 0 {
                output.push_str("1. **Address critical issues immediately** - Security vulnerabilities and critical bugs should be fixed first.\n");
            }
            if analysis.complexity.avg_complexity > 10.0 {
                output.push_str("2. **Reduce code complexity** - Consider refactoring functions with high cyclomatic complexity.\n");
            }
            if (analysis.total_stats.comment_lines as f64 / analysis.total_stats.total_lines.max(1) as f64) < 0.10 {
                output.push_str("3. **Add documentation** - Comment coverage is below 10%. Add documentation for complex logic.\n");
            }
            if (analysis.total_stats.test_files as f64 / analysis.total_stats.total_files.max(1) as f64) < 0.10 {
                output.push_str("4. **Increase test coverage** - Test file ratio is low. Add more unit tests.\n");
            }
            if !analysis.complexity.long_functions.is_empty() {
                output.push_str("5. **Break down long functions** - Split functions longer than 50 lines into smaller, focused functions.\n");
            }
        }

        output.push_str("\n---\n\n");
        output.push_str("*Generated by codebase-health*\n");

        Ok(output)
    }
}

impl MarkdownReporter {
    fn format_issue(issue: &crate::analyzer::Issue) -> String {
        let mut s = String::new();
        s.push_str(&format!("#### {}\n\n", issue.title));
        s.push_str(&format!("- **File:** `{}`", issue.file));
        if let Some(line) = issue.line {
            s.push_str(&format!(":L{}", line));
        }
        s.push('\n');
        if !issue.description.is_empty() {
            s.push_str(&format!("- **Description:** {}\n", issue.description));
        }
        s.push_str(&format!("- **Suggestion:** {}\n\n", issue.suggestion));
        s
    }
}
