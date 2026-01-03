//! Claude task instruction generator
//!
//! Generates Markdown task files optimized for Claude AI agents.

use crate::analyzer::{CodebaseAnalysis, Issue, IssueCategory, Severity};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Task priority level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TaskPriority {
    P1 = 1,  // Critical - must fix immediately
    P2 = 2,  // High - should fix soon
    P3 = 3,  // Medium - fix when possible
    P4 = 4,  // Low - nice to have
    P5 = 5,  // Info - consider for future
}

impl From<Severity> for TaskPriority {
    fn from(severity: Severity) -> Self {
        match severity {
            Severity::Critical => TaskPriority::P1,
            Severity::High => TaskPriority::P2,
            Severity::Medium => TaskPriority::P3,
            Severity::Low => TaskPriority::P4,
            Severity::Info => TaskPriority::P5,
        }
    }
}

/// A Claude task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeTask {
    pub id: String,
    pub title: String,
    pub priority: TaskPriority,
    pub category: String,
    pub estimated_complexity: String,
    pub files: Vec<String>,
    pub description: String,
    pub context: String,
    pub acceptance_criteria: Vec<String>,
    pub hints: Vec<String>,
}

/// Claude task generator
pub struct ClaudeTaskGenerator {
    max_tasks_per_file: usize,
    priority_threshold: u8,
}

impl ClaudeTaskGenerator {
    pub fn new(max_tasks_per_file: usize, priority_threshold: u8) -> Self {
        Self {
            max_tasks_per_file,
            priority_threshold,
        }
    }

    /// Generate task files from analysis
    pub fn generate(&self, analysis: &CodebaseAnalysis, output_dir: &Path) -> Result<()> {
        // Create output directory
        fs::create_dir_all(output_dir)?;

        // Group issues by category and file
        let mut tasks = self.create_tasks(analysis);

        // Filter by priority
        tasks.retain(|t| (t.priority as u8) <= self.priority_threshold);

        // Sort by priority
        tasks.sort_by_key(|t| t.priority);

        // Generate index file
        self.generate_index(&tasks, output_dir)?;

        // Generate individual task files
        for task in &tasks {
            self.generate_task_file(task, output_dir)?;
        }

        // Generate batch assignment file (for parallel Claude instances)
        self.generate_batch_file(&tasks, output_dir)?;

        Ok(())
    }

    fn create_tasks(&self, analysis: &CodebaseAnalysis) -> Vec<ClaudeTask> {
        let mut tasks = Vec::new();
        let mut task_id = 1;

        // Group issues by file
        let mut issues_by_file: HashMap<String, Vec<&Issue>> = HashMap::new();
        for issue in &analysis.issues {
            issues_by_file
                .entry(issue.file.clone())
                .or_default()
                .push(issue);
        }

        // Create tasks from grouped issues
        for (file, issues) in issues_by_file {
            // Group by category within each file
            let mut by_category: HashMap<&IssueCategory, Vec<&Issue>> = HashMap::new();
            for issue in &issues {
                by_category.entry(&issue.category).or_default().push(*issue);
            }

            for (category, cat_issues) in by_category {
                // Take up to max_tasks_per_file issues
                let issues_to_process: Vec<_> = cat_issues
                    .iter()
                    .take(self.max_tasks_per_file)
                    .copied()
                    .collect();

                if issues_to_process.is_empty() {
                    continue;
                }

                // Determine overall priority (highest among issues)
                let priority = issues_to_process
                    .iter()
                    .map(|i| TaskPriority::from(i.severity))
                    .min()
                    .unwrap_or(TaskPriority::P5);

                // Estimate complexity
                let complexity = match issues_to_process.len() {
                    1 => "Low",
                    2..=3 => "Medium",
                    _ => "High",
                };

                let category_name = Self::category_name(category);
                let task = ClaudeTask {
                    id: format!("task-{:04}", task_id),
                    title: format!("{} improvements in {}", category_name, Self::short_path(&file)),
                    priority,
                    category: category_name.to_string(),
                    estimated_complexity: complexity.to_string(),
                    files: vec![file.clone()],
                    description: Self::build_description(&issues_to_process),
                    context: Self::build_context(&file, &issues_to_process),
                    acceptance_criteria: Self::build_acceptance_criteria(&issues_to_process),
                    hints: Self::build_hints(category, &issues_to_process),
                };

                tasks.push(task);
                task_id += 1;
            }
        }

        // Add complexity-based tasks
        for func in &analysis.complexity.long_functions {
            tasks.push(ClaudeTask {
                id: format!("task-{:04}", task_id),
                title: format!("Refactor long function: {}", Self::short_path(func)),
                priority: TaskPriority::P3,
                category: "Refactoring".to_string(),
                estimated_complexity: "Medium".to_string(),
                files: vec![func.split(':').next().unwrap_or(func).to_string()],
                description: format!("This function is too long and should be broken down into smaller, focused functions.\n\nLocation: `{}`", func),
                context: "Long functions are harder to maintain, test, and understand. Breaking them into smaller functions improves code quality.".to_string(),
                acceptance_criteria: vec![
                    "Function is split into logical smaller functions".to_string(),
                    "Each new function has a single responsibility".to_string(),
                    "Existing functionality is preserved".to_string(),
                    "New functions have appropriate names".to_string(),
                ],
                hints: vec![
                    "Identify logical blocks within the function".to_string(),
                    "Look for repeated code that can be extracted".to_string(),
                    "Consider if helper functions would improve readability".to_string(),
                ],
            });
            task_id += 1;
        }

        for func in &analysis.complexity.deeply_nested {
            tasks.push(ClaudeTask {
                id: format!("task-{:04}", task_id),
                title: format!("Reduce nesting in: {}", Self::short_path(func)),
                priority: TaskPriority::P3,
                category: "Refactoring".to_string(),
                estimated_complexity: "Medium".to_string(),
                files: vec![func.split(':').next().unwrap_or(func).to_string()],
                description: format!("This function has deep nesting that should be flattened.\n\nLocation: `{}`", func),
                context: "Deeply nested code is hard to follow and prone to bugs. Reducing nesting improves readability.".to_string(),
                acceptance_criteria: vec![
                    "Nesting depth is reduced to 4 levels or less".to_string(),
                    "Early returns are used where appropriate".to_string(),
                    "Logic is clear and easy to follow".to_string(),
                ],
                hints: vec![
                    "Use early returns to handle edge cases first".to_string(),
                    "Consider guard clauses".to_string(),
                    "Extract nested blocks into separate functions".to_string(),
                ],
            });
            task_id += 1;
        }

        tasks
    }

    fn generate_index(&self, tasks: &[ClaudeTask], output_dir: &Path) -> Result<()> {
        let mut content = String::new();

        content.push_str("# Claude Task Index\n\n");
        content.push_str("This directory contains tasks generated by codebase-health analysis.\n\n");
        content.push_str("## How to Use\n\n");
        content.push_str("1. Assign individual tasks to Claude instances\n");
        content.push_str("2. Or use `batch.md` to assign multiple tasks in parallel\n");
        content.push_str("3. Each task file contains context, acceptance criteria, and hints\n\n");

        content.push_str("## Task Summary\n\n");
        content.push_str("| Priority | Count |\n|----------|-------|\n");

        let p1 = tasks.iter().filter(|t| t.priority == TaskPriority::P1).count();
        let p2 = tasks.iter().filter(|t| t.priority == TaskPriority::P2).count();
        let p3 = tasks.iter().filter(|t| t.priority == TaskPriority::P3).count();
        let p4 = tasks.iter().filter(|t| t.priority == TaskPriority::P4).count();
        let p5 = tasks.iter().filter(|t| t.priority == TaskPriority::P5).count();

        content.push_str(&format!("| P1 (Critical) | {} |\n", p1));
        content.push_str(&format!("| P2 (High) | {} |\n", p2));
        content.push_str(&format!("| P3 (Medium) | {} |\n", p3));
        content.push_str(&format!("| P4 (Low) | {} |\n", p4));
        content.push_str(&format!("| P5 (Info) | {} |\n", p5));
        content.push_str(&format!("| **Total** | **{}** |\n\n", tasks.len()));

        content.push_str("## Task List\n\n");
        content.push_str("| ID | Priority | Category | Title | Files |\n");
        content.push_str("|----|----------|----------|-------|-------|\n");

        for task in tasks {
            let priority_str = format!("P{}", task.priority as u8);
            let files_str = task.files.iter()
                .map(|f| Self::short_path(f))
                .collect::<Vec<_>>()
                .join(", ");

            content.push_str(&format!(
                "| [{}](./{}.md) | {} | {} | {} | {} |\n",
                task.id, task.id, priority_str, task.category, task.title, files_str
            ));
        }

        fs::write(output_dir.join("index.md"), content)?;
        Ok(())
    }

    fn generate_task_file(&self, task: &ClaudeTask, output_dir: &Path) -> Result<()> {
        let mut content = String::new();

        content.push_str(&format!("# Task: {}\n\n", task.id));
        content.push_str(&format!("**Title:** {}\n\n", task.title));
        content.push_str(&format!("**Priority:** P{} ({})\n\n", task.priority as u8, Self::priority_name(task.priority)));
        content.push_str(&format!("**Category:** {}\n\n", task.category));
        content.push_str(&format!("**Estimated Complexity:** {}\n\n", task.estimated_complexity));

        content.push_str("## Files to Modify\n\n");
        for file in &task.files {
            content.push_str(&format!("- `{}`\n", file));
        }
        content.push('\n');

        content.push_str("## Description\n\n");
        content.push_str(&task.description);
        content.push_str("\n\n");

        content.push_str("## Context\n\n");
        content.push_str(&task.context);
        content.push_str("\n\n");

        content.push_str("## Acceptance Criteria\n\n");
        for criterion in &task.acceptance_criteria {
            content.push_str(&format!("- [ ] {}\n", criterion));
        }
        content.push('\n');

        content.push_str("## Hints\n\n");
        for hint in &task.hints {
            content.push_str(&format!("- {}\n", hint));
        }
        content.push('\n');

        content.push_str("---\n\n");
        content.push_str("## Claude Instructions\n\n");
        content.push_str("When working on this task:\n\n");
        content.push_str("1. Read the affected files first to understand the current implementation\n");
        content.push_str("2. Make minimal, focused changes that address the specific issues\n");
        content.push_str("3. Ensure all acceptance criteria are met\n");
        content.push_str("4. Run any relevant tests to verify the changes\n");
        content.push_str("5. Commit with a clear message referencing this task ID\n\n");

        content.push_str("```\n");
        content.push_str(&format!("git commit -m \"fix({}): {}\"\n", task.id, task.title));
        content.push_str("```\n");

        fs::write(output_dir.join(format!("{}.md", task.id)), content)?;
        Ok(())
    }

    fn generate_batch_file(&self, tasks: &[ClaudeTask], output_dir: &Path) -> Result<()> {
        let mut content = String::new();

        content.push_str("# Batch Task Assignment\n\n");
        content.push_str("This file is designed for parallel Claude instance assignment.\n\n");
        content.push_str("## Assignment Strategy\n\n");
        content.push_str("Tasks are organized by priority and independence. Tasks that affect different files can be worked on in parallel.\n\n");

        // Group by priority
        let mut by_priority: HashMap<TaskPriority, Vec<&ClaudeTask>> = HashMap::new();
        for task in tasks {
            by_priority.entry(task.priority).or_default().push(task);
        }

        for priority in [TaskPriority::P1, TaskPriority::P2, TaskPriority::P3, TaskPriority::P4, TaskPriority::P5] {
            if let Some(priority_tasks) = by_priority.get(&priority) {
                content.push_str(&format!("## Priority {} Tasks\n\n", priority as u8));

                // Group by file for parallel assignment
                let mut file_groups: HashMap<&str, Vec<&&ClaudeTask>> = HashMap::new();
                for task in priority_tasks {
                    for file in &task.files {
                        file_groups.entry(file.as_str()).or_default().push(task);
                    }
                }

                // Find independent task sets (tasks that don't share files)
                content.push_str("### Parallel Assignment Groups\n\n");
                content.push_str("Tasks in different groups can be assigned to different Claude instances simultaneously:\n\n");

                let mut assigned: std::collections::HashSet<&str> = std::collections::HashSet::new();
                let mut group_num = 1;

                for task in priority_tasks {
                    if assigned.contains(task.id.as_str()) {
                        continue;
                    }

                    content.push_str(&format!("**Group {}:**\n", group_num));
                    content.push_str(&format!("- [ ] [{}](./{}.md) - {}\n", task.id, task.id, task.title));
                    assigned.insert(&task.id);

                    // Find other tasks that don't conflict
                    for other in priority_tasks {
                        if assigned.contains(other.id.as_str()) {
                            continue;
                        }
                        let conflicts = other.files.iter().any(|f| task.files.contains(f));
                        if !conflicts {
                            content.push_str(&format!("- [ ] [{}](./{}.md) - {}\n", other.id, other.id, other.title));
                            assigned.insert(&other.id);
                        }
                    }

                    content.push('\n');
                    group_num += 1;
                }
            }
        }

        content.push_str("---\n\n");
        content.push_str("## Usage Example\n\n");
        content.push_str("```bash\n");
        content.push_str("# Assign to Claude Code (Terminal 1)\n");
        content.push_str("claude-code \"Complete task task-0001 following the instructions in .claude/tasks/task-0001.md\"\n\n");
        content.push_str("# Assign to Claude Code (Terminal 2) - runs in parallel\n");
        content.push_str("claude-code \"Complete task task-0002 following the instructions in .claude/tasks/task-0002.md\"\n");
        content.push_str("```\n");

        fs::write(output_dir.join("batch.md"), content)?;
        Ok(())
    }

    fn category_name(category: &IssueCategory) -> &'static str {
        match category {
            IssueCategory::Security => "Security",
            IssueCategory::CodeQuality => "Code Quality",
            IssueCategory::Performance => "Performance",
            IssueCategory::Maintainability => "Maintainability",
            IssueCategory::Documentation => "Documentation",
            IssueCategory::Testing => "Testing",
            IssueCategory::BestPractice => "Best Practice",
        }
    }

    fn priority_name(priority: TaskPriority) -> &'static str {
        match priority {
            TaskPriority::P1 => "Critical",
            TaskPriority::P2 => "High",
            TaskPriority::P3 => "Medium",
            TaskPriority::P4 => "Low",
            TaskPriority::P5 => "Info",
        }
    }

    fn short_path(path: &str) -> String {
        path.split('/').last().unwrap_or(path).to_string()
    }

    fn build_description(issues: &[&Issue]) -> String {
        let mut desc = String::new();
        desc.push_str("The following issues need to be addressed:\n\n");
        for issue in issues {
            desc.push_str(&format!("- **{}**", issue.title));
            if let Some(line) = issue.line {
                desc.push_str(&format!(" (line {})", line));
            }
            desc.push('\n');
            if !issue.description.is_empty() {
                desc.push_str(&format!("  - {}\n", issue.description));
            }
        }
        desc
    }

    fn build_context(file: &str, issues: &[&Issue]) -> String {
        let mut ctx = String::new();
        ctx.push_str(&format!("File: `{}`\n\n", file));
        ctx.push_str("Issues found in this file relate to ");
        let categories: std::collections::HashSet<_> = issues.iter().map(|i| Self::category_name(&i.category)).collect();
        let cats: Vec<_> = categories.into_iter().collect();
        ctx.push_str(&cats.join(", "));
        ctx.push_str(".\n\n");
        ctx.push_str("Review the suggestions for each issue and apply appropriate fixes.");
        ctx
    }

    fn build_acceptance_criteria(issues: &[&Issue]) -> Vec<String> {
        let mut criteria = vec!["All identified issues are resolved".to_string()];

        for issue in issues {
            criteria.push(issue.suggestion.clone());
        }

        criteria.push("Code compiles without errors".to_string());
        criteria.push("Existing tests pass".to_string());

        criteria
    }

    fn build_hints(category: &IssueCategory, issues: &[&Issue]) -> Vec<String> {
        let mut hints = Vec::new();

        match category {
            IssueCategory::Security => {
                hints.push("Check for sensitive data exposure".to_string());
                hints.push("Consider using environment variables for secrets".to_string());
            }
            IssueCategory::CodeQuality => {
                hints.push("Focus on improving readability".to_string());
                hints.push("Consider adding error handling".to_string());
            }
            IssueCategory::Performance => {
                hints.push("Profile the code if needed".to_string());
                hints.push("Consider caching or memoization".to_string());
            }
            IssueCategory::Maintainability => {
                hints.push("Keep changes minimal and focused".to_string());
                hints.push("Add comments for complex logic".to_string());
            }
            IssueCategory::Documentation => {
                hints.push("Add doc comments for public APIs".to_string());
                hints.push("Include examples where helpful".to_string());
            }
            IssueCategory::Testing => {
                hints.push("Add unit tests for new code".to_string());
                hints.push("Consider edge cases".to_string());
            }
            IssueCategory::BestPractice => {
                hints.push("Follow the project's coding conventions".to_string());
                hints.push("Check the project's CONTRIBUTING guide if available".to_string());
            }
        }

        // Add issue-specific hints
        for issue in issues.iter().take(2) {
            if !issue.suggestion.is_empty() && !hints.contains(&issue.suggestion) {
                hints.push(issue.suggestion.clone());
            }
        }

        hints
    }
}
