//! Code analysis module
//!
//! Provides functionality to analyze codebase structure, complexity, and issues.

mod file_stats;
mod complexity;
mod issues;

pub use file_stats::*;
pub use complexity::*;
pub use issues::*;

use anyhow::Result;
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Complete analysis result for a codebase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodebaseAnalysis {
    /// Root path of the analyzed project
    pub root_path: PathBuf,
    /// Timestamp of the analysis
    pub analyzed_at: chrono::DateTime<chrono::Utc>,
    /// File statistics by language
    pub file_stats: HashMap<String, LanguageStats>,
    /// Total statistics
    pub total_stats: TotalStats,
    /// Detected issues
    pub issues: Vec<Issue>,
    /// Complexity metrics
    pub complexity: ComplexityReport,
    /// Health score (0-100)
    pub health_score: u8,
}

impl CodebaseAnalysis {
    /// Generate a human-readable summary
    pub fn summary(&self) -> String {
        let mut output = String::new();

        output.push_str(&format!("📊 Codebase Health Summary\n"));
        output.push_str(&format!("========================\n\n"));
        output.push_str(&format!("📁 Project: {}\n", self.root_path.display()));
        output.push_str(&format!("📅 Analyzed: {}\n\n", self.analyzed_at.format("%Y-%m-%d %H:%M:%S UTC")));

        output.push_str(&format!("📈 Health Score: {}/100 {}\n\n",
            self.health_score,
            Self::score_emoji(self.health_score)
        ));

        output.push_str(&format!("📂 Files: {} ({} lines)\n",
            self.total_stats.total_files,
            self.total_stats.total_lines
        ));
        output.push_str(&format!("   Code: {} lines ({:.1}%)\n",
            self.total_stats.code_lines,
            self.total_stats.code_lines as f64 / self.total_stats.total_lines.max(1) as f64 * 100.0
        ));
        output.push_str(&format!("   Comments: {} lines ({:.1}%)\n",
            self.total_stats.comment_lines,
            self.total_stats.comment_lines as f64 / self.total_stats.total_lines.max(1) as f64 * 100.0
        ));
        output.push_str(&format!("   Blank: {} lines\n\n", self.total_stats.blank_lines));

        // Language breakdown
        output.push_str("📊 Languages:\n");
        let mut langs: Vec<_> = self.file_stats.iter().collect();
        langs.sort_by(|a, b| b.1.code_lines.cmp(&a.1.code_lines));
        for (lang, stats) in langs.iter().take(5) {
            output.push_str(&format!("   {}: {} files, {} lines\n", lang, stats.file_count, stats.code_lines));
        }
        output.push('\n');

        // Issues summary
        let critical = self.issues.iter().filter(|i| i.severity == Severity::Critical).count();
        let high = self.issues.iter().filter(|i| i.severity == Severity::High).count();
        let medium = self.issues.iter().filter(|i| i.severity == Severity::Medium).count();
        let low = self.issues.iter().filter(|i| i.severity == Severity::Low).count();

        output.push_str(&format!("⚠️  Issues: {} total\n", self.issues.len()));
        if critical > 0 { output.push_str(&format!("   🔴 Critical: {}\n", critical)); }
        if high > 0 { output.push_str(&format!("   🟠 High: {}\n", high)); }
        if medium > 0 { output.push_str(&format!("   🟡 Medium: {}\n", medium)); }
        if low > 0 { output.push_str(&format!("   🟢 Low: {}\n", low)); }

        output
    }

    fn score_emoji(score: u8) -> &'static str {
        match score {
            90..=100 => "🌟",
            70..=89 => "✅",
            50..=69 => "⚠️",
            30..=49 => "🟠",
            _ => "🔴",
        }
    }
}

/// Total statistics across all files
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TotalStats {
    pub total_files: usize,
    pub total_lines: usize,
    pub code_lines: usize,
    pub comment_lines: usize,
    pub blank_lines: usize,
    pub test_files: usize,
    pub doc_files: usize,
}

/// Statistics for a specific language
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LanguageStats {
    pub file_count: usize,
    pub total_lines: usize,
    pub code_lines: usize,
    pub comment_lines: usize,
    pub blank_lines: usize,
}

/// Codebase analyzer
pub struct CodebaseAnalyzer {
    root_path: PathBuf,
    extensions: Vec<String>,
    include_hidden: bool,
}

impl CodebaseAnalyzer {
    /// Create a new analyzer
    pub fn new(root_path: &Path, extensions: &[&str], include_hidden: bool) -> Result<Self> {
        Ok(Self {
            root_path: root_path.canonicalize().unwrap_or_else(|_| root_path.to_path_buf()),
            extensions: extensions.iter().map(|s| s.to_string()).collect(),
            include_hidden,
        })
    }

    /// Run the analysis
    pub fn analyze(&self) -> Result<CodebaseAnalysis> {
        let mut file_stats: HashMap<String, LanguageStats> = HashMap::new();
        let mut total_stats = TotalStats::default();
        let mut all_issues = Vec::new();
        let mut complexity_data = Vec::new();

        // Walk the directory tree
        let walker = WalkBuilder::new(&self.root_path)
            .hidden(!self.include_hidden)
            .git_ignore(true)
            .git_exclude(true)
            .build();

        for entry in walker.filter_map(|e| e.ok()) {
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            // Check extension
            let ext = path.extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_lowercase());

            let ext = match ext {
                Some(e) if self.extensions.contains(&e) => e,
                _ => continue,
            };

            // Read and analyze file
            let content = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            // Calculate file stats
            let stats = FileStats::calculate(&content, &ext);

            // Update language stats
            let lang_stats = file_stats.entry(ext.clone()).or_default();
            lang_stats.file_count += 1;
            lang_stats.total_lines += stats.total_lines;
            lang_stats.code_lines += stats.code_lines;
            lang_stats.comment_lines += stats.comment_lines;
            lang_stats.blank_lines += stats.blank_lines;

            // Update total stats
            total_stats.total_files += 1;
            total_stats.total_lines += stats.total_lines;
            total_stats.code_lines += stats.code_lines;
            total_stats.comment_lines += stats.comment_lines;
            total_stats.blank_lines += stats.blank_lines;

            // Check if test file
            let path_str = path.to_string_lossy().to_lowercase();
            if path_str.contains("test") || path_str.contains("spec") {
                total_stats.test_files += 1;
            }

            // Check if documentation
            if ext == "md" || path_str.contains("doc") {
                total_stats.doc_files += 1;
            }

            // Detect issues
            let file_issues = IssueDetector::detect(path, &content, &ext);
            all_issues.extend(file_issues);

            // Calculate complexity
            let file_complexity = ComplexityAnalyzer::analyze(path, &content, &ext);
            complexity_data.push(file_complexity);
        }

        // Aggregate complexity
        let complexity = ComplexityReport::aggregate(&complexity_data);

        // Calculate health score
        let health_score = Self::calculate_health_score(&total_stats, &all_issues, &complexity);

        Ok(CodebaseAnalysis {
            root_path: self.root_path.clone(),
            analyzed_at: chrono::Utc::now(),
            file_stats,
            total_stats,
            issues: all_issues,
            complexity,
            health_score,
        })
    }

    fn calculate_health_score(
        stats: &TotalStats,
        issues: &[Issue],
        complexity: &ComplexityReport,
    ) -> u8 {
        let mut score: f64 = 100.0;

        // Issues減点（ファイル数で正規化して、過剰な減点を防ぐ）
        let critical_count = issues.iter().filter(|i| i.severity == Severity::Critical).count();
        let high_count = issues.iter().filter(|i| i.severity == Severity::High).count();
        let medium_count = issues.iter().filter(|i| i.severity == Severity::Medium).count();
        let low_count = issues.iter().filter(|i| i.severity == Severity::Low).count();
        
        // ファイル数で正規化（1ファイルあたりのissues数で評価）
        if stats.total_files > 0 {
            let files = stats.total_files as f64;
            // Critical: 1ファイルあたり0.1件以上で減点
            if critical_count as f64 / files > 0.1 {
                score -= 20.0;
            } else if critical_count > 0 {
                score -= (critical_count as f64 / files * 200.0).min(20.0);
            }
            
            // High: 1ファイルあたり0.2件以上で減点
            if high_count as f64 / files > 0.2 {
                score -= 15.0;
            } else if high_count > 0 {
                score -= (high_count as f64 / files * 75.0).min(15.0);
            }
            
            // Medium: 1ファイルあたり1件以上で減点
            if medium_count as f64 / files > 1.0 {
                score -= 10.0;
            } else if medium_count > 0 {
                score -= (medium_count as f64 / files * 10.0).min(10.0);
            }
            
            // Low: 1ファイルあたり5件以上で減点
            if low_count as f64 / files > 5.0 {
                score -= 5.0;
            } else if low_count > 0 {
                score -= (low_count as f64 / files * 1.0).min(5.0);
            }
        }

        // Deduct for poor comment ratio (less than 10%)
        if stats.total_lines > 0 {
            let comment_ratio = stats.comment_lines as f64 / stats.total_lines as f64;
            if comment_ratio < 0.05 {
                score -= 10.0;
            } else if comment_ratio < 0.10 {
                score -= 5.0;
            }
        }

        // Deduct for high complexity
        if complexity.avg_complexity > 15.0 {
            score -= 15.0;
        } else if complexity.avg_complexity > 10.0 {
            score -= 10.0;
        } else if complexity.avg_complexity > 5.0 {
            score -= 5.0;
        }

        // Deduct for lack of tests
        if stats.total_files > 0 {
            let test_ratio = stats.test_files as f64 / stats.total_files as f64;
            if test_ratio < 0.05 {
                score -= 15.0;
            } else if test_ratio < 0.10 {
                score -= 10.0;
            } else if test_ratio < 0.20 {
                score -= 5.0;
            }
        }

        score.clamp(0.0, 100.0) as u8
    }
}
