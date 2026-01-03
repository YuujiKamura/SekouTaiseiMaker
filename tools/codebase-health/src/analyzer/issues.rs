//! Issue detection

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Severity level for issues
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

impl Severity {
    pub fn priority(&self) -> u8 {
        match self {
            Severity::Critical => 5,
            Severity::High => 4,
            Severity::Medium => 3,
            Severity::Low => 2,
            Severity::Info => 1,
        }
    }
}

/// Category of issue
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IssueCategory {
    CodeQuality,
    Security,
    Performance,
    Maintainability,
    Documentation,
    Testing,
    BestPractice,
}

/// A detected issue in the codebase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub file: String,
    pub line: Option<usize>,
    pub severity: Severity,
    pub category: IssueCategory,
    pub title: String,
    pub description: String,
    pub suggestion: String,
}

/// Issue detector
pub struct IssueDetector;

impl IssueDetector {
    /// Detect issues in a file
    pub fn detect(path: &Path, content: &str, extension: &str) -> Vec<Issue> {
        let mut issues = Vec::new();
        let path_str = path.display().to_string();

        // Common patterns for all languages
        issues.extend(Self::detect_common_issues(&path_str, content));

        // Language-specific patterns
        match extension {
            "rs" => issues.extend(Self::detect_rust_issues(&path_str, content)),
            "ts" | "tsx" | "js" | "jsx" => issues.extend(Self::detect_js_issues(&path_str, content)),
            "py" => issues.extend(Self::detect_python_issues(&path_str, content)),
            "go" => issues.extend(Self::detect_go_issues(&path_str, content)),
            _ => {}
        }

        issues
    }

    /// Detect common issues across languages
    fn detect_common_issues(path: &str, content: &str) -> Vec<Issue> {
        let mut issues = Vec::new();

        // TODO/FIXME/HACK comments - only match in actual comments, not string literals
        // Match keywords that appear after comment markers (// # /* -- etc.)
        let comment_todo_re = Regex::new(r"(?://|#|/\*|--)\s*(?i)(TODO|FIXME|HACK|XXX|BUG)\b[:\s]*(.*)").unwrap();
        for (line_num, line) in content.lines().enumerate() {
            if let Some(cap) = comment_todo_re.captures(line) {
                let tag = cap.get(1).map(|m| m.as_str()).unwrap_or("TODO");
                let description = cap.get(2).map(|m| m.as_str().trim()).unwrap_or("");

                let severity = match tag.to_uppercase().as_str() {
                    "BUG" | "FIXME" => Severity::High,
                    "HACK" | "XXX" => Severity::Medium,
                    _ => Severity::Low,
                };

                issues.push(Issue {
                    file: path.to_string(),
                    line: Some(line_num + 1),
                    severity,
                    category: IssueCategory::Maintainability,
                    title: format!("{} comment found", tag),
                    description: description.to_string(),
                    suggestion: "Address this issue or remove the comment if resolved".to_string(),
                });
            }
        }

        // Very long lines (> 120 characters)
        let long_line_threshold = 120;
        let mut long_line_count = 0;
        for (line_num, line) in content.lines().enumerate() {
            if line.len() > long_line_threshold {
                long_line_count += 1;
                if long_line_count <= 3 {
                    issues.push(Issue {
                        file: path.to_string(),
                        line: Some(line_num + 1),
                        severity: Severity::Info,
                        category: IssueCategory::CodeQuality,
                        title: "Line too long".to_string(),
                        description: format!("Line has {} characters", line.len()),
                        suggestion: "Consider breaking this line for better readability".to_string(),
                    });
                }
            }
        }

        // Hardcoded credentials patterns (potential security issue)
        let credential_patterns = [
            (r#"(?i)(password|passwd|pwd)\s*[=:]\s*["'][^"']+["']"#, "Hardcoded password"),
            (r#"(?i)(api_?key|apikey)\s*[=:]\s*["'][^"']+["']"#, "Hardcoded API key"),
            (r#"(?i)(secret|token)\s*[=:]\s*["'][^"']+["']"#, "Hardcoded secret/token"),
        ];

        for (pattern, title) in credential_patterns {
            if let Ok(re) = Regex::new(pattern) {
                for (line_num, line) in content.lines().enumerate() {
                    if re.is_match(line) {
                        issues.push(Issue {
                            file: path.to_string(),
                            line: Some(line_num + 1),
                            severity: Severity::Critical,
                            category: IssueCategory::Security,
                            title: title.to_string(),
                            description: "Credentials should not be hardcoded in source code".to_string(),
                            suggestion: "Use environment variables or a secure secrets manager".to_string(),
                        });
                    }
                }
            }
        }

        // Duplicated code patterns (simplified - just check for identical consecutive lines)
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;
        while i < lines.len().saturating_sub(2) {
            let line = lines[i].trim();
            if line.len() > 20 && !line.starts_with("//") && !line.starts_with("#") {
                if lines.get(i + 1).map(|l| l.trim()) == Some(line)
                    && lines.get(i + 2).map(|l| l.trim()) == Some(line)
                {
                    issues.push(Issue {
                        file: path.to_string(),
                        line: Some(i + 1),
                        severity: Severity::Low,
                        category: IssueCategory::Maintainability,
                        title: "Potential code duplication".to_string(),
                        description: "Multiple consecutive identical lines detected".to_string(),
                        suggestion: "Consider refactoring to reduce duplication".to_string(),
                    });
                    i += 3;
                    continue;
                }
            }
            i += 1;
        }

        issues
    }

    /// Detect Rust-specific issues
    fn detect_rust_issues(path: &str, content: &str) -> Vec<Issue> {
        let mut issues = Vec::new();

        // unwrap() usage
        let unwrap_re = Regex::new(r"\.unwrap\(\)").unwrap();
        for (line_num, line) in content.lines().enumerate() {
            if unwrap_re.is_match(line) && !line.contains("test") {
                issues.push(Issue {
                    file: path.to_string(),
                    line: Some(line_num + 1),
                    severity: Severity::Medium,
                    category: IssueCategory::CodeQuality,
                    title: "Usage of unwrap()".to_string(),
                    description: "unwrap() can cause panics if the value is None or Err".to_string(),
                    suggestion: "Consider using ? operator, expect(), or proper error handling".to_string(),
                });
            }
        }

        // expect() without meaningful message
        let expect_re = Regex::new(r#"\.expect\(\s*["'](?:[^"']*panic|failed|error)[^"']*["']\s*\)"#).unwrap();
        for (line_num, line) in content.lines().enumerate() {
            if expect_re.is_match(line) {
                issues.push(Issue {
                    file: path.to_string(),
                    line: Some(line_num + 1),
                    severity: Severity::Info,
                    category: IssueCategory::BestPractice,
                    title: "Generic expect() message".to_string(),
                    description: "expect() message should be descriptive".to_string(),
                    suggestion: "Provide a meaningful error message that explains why this should not happen".to_string(),
                });
            }
        }

        // clone() in a loop (potential performance issue)
        let mut in_loop = false;
        for (line_num, line) in content.lines().enumerate() {
            if line.contains("for ") || line.contains("while ") || line.contains("loop {") {
                in_loop = true;
            }
            if in_loop && line.contains("}") {
                in_loop = false;
            }
            if in_loop && line.contains(".clone()") {
                issues.push(Issue {
                    file: path.to_string(),
                    line: Some(line_num + 1),
                    severity: Severity::Medium,
                    category: IssueCategory::Performance,
                    title: "clone() in loop".to_string(),
                    description: "Cloning inside a loop may impact performance".to_string(),
                    suggestion: "Consider moving the clone outside the loop or using references".to_string(),
                });
            }
        }

        // #[allow(dead_code)] - might indicate unused code
        let dead_code_re = Regex::new(r"#\[allow\(dead_code\)\]").unwrap();
        for (line_num, line) in content.lines().enumerate() {
            if dead_code_re.is_match(line) {
                issues.push(Issue {
                    file: path.to_string(),
                    line: Some(line_num + 1),
                    severity: Severity::Low,
                    category: IssueCategory::Maintainability,
                    title: "Dead code allowed".to_string(),
                    description: "Code marked as dead_code should be reviewed".to_string(),
                    suggestion: "Remove unused code or document why it's needed".to_string(),
                });
            }
        }

        issues
    }

    /// Detect JavaScript/TypeScript-specific issues
    fn detect_js_issues(path: &str, content: &str) -> Vec<Issue> {
        let mut issues = Vec::new();

        // console.log statements
        let console_re = Regex::new(r"console\.(log|warn|error|debug)").unwrap();
        for (line_num, line) in content.lines().enumerate() {
            if console_re.is_match(line) && !path.contains("test") {
                issues.push(Issue {
                    file: path.to_string(),
                    line: Some(line_num + 1),
                    severity: Severity::Low,
                    category: IssueCategory::CodeQuality,
                    title: "Console statement found".to_string(),
                    description: "Console statements should not be in production code".to_string(),
                    suggestion: "Remove or replace with proper logging".to_string(),
                });
            }
        }

        // any type usage (TypeScript)
        if path.ends_with(".ts") || path.ends_with(".tsx") {
            let any_re = Regex::new(r":\s*any\b").unwrap();
            for (line_num, line) in content.lines().enumerate() {
                if any_re.is_match(line) {
                    issues.push(Issue {
                        file: path.to_string(),
                        line: Some(line_num + 1),
                        severity: Severity::Medium,
                        category: IssueCategory::CodeQuality,
                        title: "Usage of 'any' type".to_string(),
                        description: "Using 'any' defeats TypeScript's type safety".to_string(),
                        suggestion: "Define proper types or use 'unknown' if type is truly unknown".to_string(),
                    });
                }
            }
        }

        // Callback hell (nested callbacks)
        let callback_re = Regex::new(r"\)\s*=>\s*\{").unwrap();
        for (line_num, line) in content.lines().enumerate() {
            let count = callback_re.find_iter(line).count();
            if count >= 3 {
                issues.push(Issue {
                    file: path.to_string(),
                    line: Some(line_num + 1),
                    severity: Severity::Medium,
                    category: IssueCategory::Maintainability,
                    title: "Potential callback nesting".to_string(),
                    description: "Deeply nested callbacks reduce readability".to_string(),
                    suggestion: "Consider using async/await or breaking into separate functions".to_string(),
                });
            }
        }

        issues
    }

    /// Detect Python-specific issues
    fn detect_python_issues(path: &str, content: &str) -> Vec<Issue> {
        let mut issues = Vec::new();

        // bare except
        let bare_except_re = Regex::new(r"except\s*:").unwrap();
        for (line_num, line) in content.lines().enumerate() {
            if bare_except_re.is_match(line) {
                issues.push(Issue {
                    file: path.to_string(),
                    line: Some(line_num + 1),
                    severity: Severity::Medium,
                    category: IssueCategory::BestPractice,
                    title: "Bare except clause".to_string(),
                    description: "Catching all exceptions can hide bugs".to_string(),
                    suggestion: "Specify the exception type(s) to catch".to_string(),
                });
            }
        }

        // print statements (should use logging)
        let print_re = Regex::new(r"\bprint\s*\(").unwrap();
        for (line_num, line) in content.lines().enumerate() {
            if print_re.is_match(line) && !path.contains("test") {
                issues.push(Issue {
                    file: path.to_string(),
                    line: Some(line_num + 1),
                    severity: Severity::Low,
                    category: IssueCategory::BestPractice,
                    title: "Print statement found".to_string(),
                    description: "Print statements should be replaced with proper logging".to_string(),
                    suggestion: "Use the logging module instead".to_string(),
                });
            }
        }

        issues
    }

    /// Detect Go-specific issues
    fn detect_go_issues(path: &str, content: &str) -> Vec<Issue> {
        let mut issues = Vec::new();

        // Ignoring errors
        let ignore_err_re = Regex::new(r"_\s*,?\s*=.*\berr\b").unwrap();
        for (line_num, line) in content.lines().enumerate() {
            if ignore_err_re.is_match(line) {
                issues.push(Issue {
                    file: path.to_string(),
                    line: Some(line_num + 1),
                    severity: Severity::High,
                    category: IssueCategory::CodeQuality,
                    title: "Error ignored".to_string(),
                    description: "Errors should be handled, not discarded".to_string(),
                    suggestion: "Handle the error appropriately or use a linter to enforce error handling".to_string(),
                });
            }
        }

        // fmt.Println (should use log)
        let fmt_println_re = Regex::new(r"fmt\.Print").unwrap();
        for (line_num, line) in content.lines().enumerate() {
            if fmt_println_re.is_match(line) && !path.contains("test") {
                issues.push(Issue {
                    file: path.to_string(),
                    line: Some(line_num + 1),
                    severity: Severity::Low,
                    category: IssueCategory::BestPractice,
                    title: "fmt.Print usage".to_string(),
                    description: "Consider using log package for production code".to_string(),
                    suggestion: "Replace with log.Print or a structured logger".to_string(),
                });
            }
        }

        issues
    }
}
