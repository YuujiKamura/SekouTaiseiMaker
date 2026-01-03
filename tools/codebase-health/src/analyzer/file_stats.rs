//! File statistics calculation

use serde::{Deserialize, Serialize};

/// Statistics for a single file
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileStats {
    pub total_lines: usize,
    pub code_lines: usize,
    pub comment_lines: usize,
    pub blank_lines: usize,
}

impl FileStats {
    /// Calculate statistics for file content
    pub fn calculate(content: &str, extension: &str) -> Self {
        let mut stats = FileStats::default();
        let mut in_block_comment = false;

        let (line_comment, block_start, block_end) = Self::comment_markers(extension);

        for line in content.lines() {
            stats.total_lines += 1;
            let trimmed = line.trim();

            if trimmed.is_empty() {
                stats.blank_lines += 1;
                continue;
            }

            // Handle block comments
            if let (Some(start), Some(end)) = (&block_start, &block_end) {
                if in_block_comment {
                    stats.comment_lines += 1;
                    if trimmed.contains(end.as_str()) {
                        in_block_comment = false;
                    }
                    continue;
                }

                if trimmed.starts_with(start.as_str()) {
                    in_block_comment = true;
                    stats.comment_lines += 1;
                    if trimmed.contains(end.as_str()) {
                        in_block_comment = false;
                    }
                    continue;
                }
            }

            // Handle line comments
            if let Some(marker) = &line_comment {
                if trimmed.starts_with(marker.as_str()) {
                    stats.comment_lines += 1;
                    continue;
                }
            }

            // Code line
            stats.code_lines += 1;
        }

        stats
    }

    /// Get comment markers for a language extension
    fn comment_markers(extension: &str) -> (Option<String>, Option<String>, Option<String>) {
        match extension {
            "rs" | "go" | "java" | "ts" | "tsx" | "js" | "jsx" | "c" | "cpp" | "h" | "hpp" => {
                (Some("//".to_string()), Some("/*".to_string()), Some("*/".to_string()))
            }
            "py" => {
                (Some("#".to_string()), Some(r#"""""#.to_string()), Some(r#"""""#.to_string()))
            }
            "rb" => {
                (Some("#".to_string()), Some("=begin".to_string()), Some("=end".to_string()))
            }
            "sh" | "bash" | "zsh" => {
                (Some("#".to_string()), None, None)
            }
            "html" | "xml" => {
                (None, Some("<!--".to_string()), Some("-->".to_string()))
            }
            "css" | "scss" | "less" => {
                (None, Some("/*".to_string()), Some("*/".to_string()))
            }
            _ => (Some("//".to_string()), Some("/*".to_string()), Some("*/".to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_stats() {
        let content = r#"
// This is a comment
fn main() {
    /* Block comment */
    println!("Hello");
}

"#;
        let stats = FileStats::calculate(content, "rs");
        assert!(stats.total_lines > 0);
        assert!(stats.comment_lines >= 2);
        assert!(stats.code_lines >= 3);
    }

    #[test]
    fn test_empty_file() {
        let stats = FileStats::calculate("", "rs");
        assert_eq!(stats.total_lines, 0);
        assert_eq!(stats.code_lines, 0);
    }
}
