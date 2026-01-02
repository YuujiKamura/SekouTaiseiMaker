//! Complexity analysis

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Complexity metrics for a single file
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileComplexity {
    pub path: String,
    pub functions: Vec<FunctionComplexity>,
    pub total_complexity: usize,
    pub max_nesting: usize,
    pub long_functions: usize,
}

/// Complexity metrics for a function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionComplexity {
    pub name: String,
    pub line_start: usize,
    pub line_count: usize,
    pub cyclomatic_complexity: usize,
    pub nesting_depth: usize,
}

/// Aggregated complexity report
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComplexityReport {
    pub files_analyzed: usize,
    pub total_functions: usize,
    pub avg_complexity: f64,
    pub max_complexity: usize,
    pub max_complexity_function: Option<String>,
    pub long_functions: Vec<String>,
    pub deeply_nested: Vec<String>,
}

impl ComplexityReport {
    /// Aggregate complexity data from multiple files
    pub fn aggregate(file_data: &[FileComplexity]) -> Self {
        let mut report = ComplexityReport::default();

        let mut total_complexity = 0usize;
        let mut function_count = 0usize;

        for file in file_data {
            report.files_analyzed += 1;

            for func in &file.functions {
                function_count += 1;
                total_complexity += func.cyclomatic_complexity;

                if func.cyclomatic_complexity > report.max_complexity {
                    report.max_complexity = func.cyclomatic_complexity;
                    report.max_complexity_function = Some(format!(
                        "{}:{} ({})",
                        file.path, func.line_start, func.name
                    ));
                }

                // Long function threshold: 50 lines
                if func.line_count > 50 {
                    report.long_functions.push(format!(
                        "{}:{} ({}) - {} lines",
                        file.path, func.line_start, func.name, func.line_count
                    ));
                }

                // Deep nesting threshold: 4 levels
                if func.nesting_depth > 4 {
                    report.deeply_nested.push(format!(
                        "{}:{} ({}) - {} levels",
                        file.path, func.line_start, func.name, func.nesting_depth
                    ));
                }
            }
        }

        report.total_functions = function_count;
        report.avg_complexity = if function_count > 0 {
            total_complexity as f64 / function_count as f64
        } else {
            0.0
        };

        report
    }
}

/// Analyzer for code complexity
pub struct ComplexityAnalyzer;

impl ComplexityAnalyzer {
    /// Analyze a file's complexity
    pub fn analyze(path: &Path, content: &str, extension: &str) -> FileComplexity {
        let mut complexity = FileComplexity {
            path: path.display().to_string(),
            ..Default::default()
        };

        let functions = Self::extract_functions(content, extension);

        for (name, start_line, func_content) in functions {
            let line_count = func_content.lines().count();
            let cyclomatic = Self::calculate_cyclomatic(&func_content, extension);
            let nesting = Self::calculate_max_nesting(&func_content, extension);

            if line_count > 50 {
                complexity.long_functions += 1;
            }

            complexity.total_complexity += cyclomatic;
            complexity.max_nesting = complexity.max_nesting.max(nesting);

            complexity.functions.push(FunctionComplexity {
                name,
                line_start: start_line,
                line_count,
                cyclomatic_complexity: cyclomatic,
                nesting_depth: nesting,
            });
        }

        complexity
    }

    /// Extract functions from source code (simplified)
    fn extract_functions(content: &str, extension: &str) -> Vec<(String, usize, String)> {
        let mut functions = Vec::new();

        let fn_pattern = match extension {
            "rs" => r"(?m)^\s*(?:pub\s+)?(?:async\s+)?fn\s+(\w+)",
            "ts" | "tsx" | "js" | "jsx" => r"(?m)(?:function\s+(\w+)|(?:const|let|var)\s+(\w+)\s*=\s*(?:async\s*)?\()",
            "py" => r"(?m)^\s*def\s+(\w+)",
            "go" => r"(?m)^func\s+(?:\([^)]+\)\s+)?(\w+)",
            "java" => r"(?m)(?:public|private|protected)?\s*(?:static\s+)?(?:\w+\s+)+(\w+)\s*\(",
            _ => return functions,
        };

        let re = match Regex::new(fn_pattern) {
            Ok(r) => r,
            Err(_) => return functions,
        };

        for cap in re.captures_iter(content) {
            // Get function name from first non-empty capture group
            let name = cap.iter()
                .skip(1)
                .filter_map(|m| m)
                .next()
                .map(|m| m.as_str().to_string())
                .unwrap_or_else(|| "anonymous".to_string());

            // Find line number
            let match_start = cap.get(0).map(|m| m.start()).unwrap_or(0);
            let line_num = content[..match_start].lines().count() + 1;

            // Extract function body (simplified - just count to next function or EOF)
            let remaining = &content[match_start..];
            let body_end = Self::find_function_end(remaining, extension);
            // Ensure we slice at a valid UTF-8 char boundary
            let safe_end = remaining
                .char_indices()
                .take_while(|(i, _)| *i < body_end)
                .last()
                .map(|(i, c)| i + c.len_utf8())
                .unwrap_or(0);
            let body = &remaining[..safe_end];

            functions.push((name, line_num, body.to_string()));
        }

        functions
    }

    /// Find the end of a function body (simplified heuristic)
    fn find_function_end(content: &str, extension: &str) -> usize {
        let mut brace_count = 0;
        let mut started = false;
        let mut pos = 0;

        match extension {
            "py" => {
                // Python: count indentation
                let lines: Vec<&str> = content.lines().collect();
                if lines.is_empty() {
                    return content.len();
                }

                let first_line_indent = lines[0].len() - lines[0].trim_start().len();

                for (i, line) in lines.iter().enumerate().skip(1) {
                    if line.trim().is_empty() {
                        continue;
                    }
                    let indent = line.len() - line.trim_start().len();
                    if indent <= first_line_indent && !line.trim().is_empty() {
                        // Found next function or end of block
                        return lines[..i].join("\n").len();
                    }
                }
                content.len()
            }
            _ => {
                // Brace-based languages
                for (i, ch) in content.chars().enumerate() {
                    if ch == '{' {
                        brace_count += 1;
                        started = true;
                    } else if ch == '}' {
                        brace_count -= 1;
                        if started && brace_count == 0 {
                            return i + 1;
                        }
                    }
                    pos = i;
                }
                pos + 1
            }
        }
    }

    /// Calculate cyclomatic complexity
    fn calculate_cyclomatic(content: &str, extension: &str) -> usize {
        let patterns = match extension {
            "rs" => vec![
                r"\bif\b", r"\belse\s+if\b", r"\bwhile\b", r"\bfor\b",
                r"\bloop\b", r"\bmatch\b", r"\b\?\b", r"&&", r"\|\|",
            ],
            "ts" | "tsx" | "js" | "jsx" => vec![
                r"\bif\b", r"\belse\s+if\b", r"\bwhile\b", r"\bfor\b",
                r"\bswitch\b", r"\bcase\b", r"\bcatch\b", r"\?", r"&&", r"\|\|",
            ],
            "py" => vec![
                r"\bif\b", r"\belif\b", r"\bwhile\b", r"\bfor\b",
                r"\bexcept\b", r"\band\b", r"\bor\b",
            ],
            "go" => vec![
                r"\bif\b", r"\belse\s+if\b", r"\bfor\b", r"\bswitch\b",
                r"\bcase\b", r"\bselect\b", r"&&", r"\|\|",
            ],
            "java" => vec![
                r"\bif\b", r"\belse\s+if\b", r"\bwhile\b", r"\bfor\b",
                r"\bswitch\b", r"\bcase\b", r"\bcatch\b", r"\?", r"&&", r"\|\|",
            ],
            _ => vec![r"\bif\b", r"\bwhile\b", r"\bfor\b", r"&&", r"\|\|"],
        };

        let mut complexity = 1; // Base complexity

        for pattern in patterns {
            if let Ok(re) = Regex::new(pattern) {
                complexity += re.find_iter(content).count();
            }
        }

        complexity
    }

    /// Calculate maximum nesting depth
    fn calculate_max_nesting(content: &str, _extension: &str) -> usize {
        let mut max_depth: usize = 0;
        let mut current_depth: usize = 0;

        for ch in content.chars() {
            match ch {
                '{' | '(' => {
                    current_depth += 1;
                    max_depth = max_depth.max(current_depth);
                }
                '}' | ')' => {
                    current_depth = current_depth.saturating_sub(1);
                }
                _ => {}
            }
        }

        max_depth
    }
}
