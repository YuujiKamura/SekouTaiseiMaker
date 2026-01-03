//! Codebase Health Dashboard CLI Tool
//!
//! Analyzes codebase structure and generates Claude task instructions.

mod analyzer;
mod reporter;
mod task_generator;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use analyzer::CodebaseAnalyzer;
use reporter::{MarkdownReporter, JsonReporter, HtmlReporter, Reporter};
use task_generator::ClaudeTaskGenerator;

#[derive(Parser)]
#[command(name = "codebase-health")]
#[command(author = "SekouTaiseiMaker Team")]
#[command(version = "0.1.0")]
#[command(about = "Analyze codebase health and generate Claude task instructions")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze codebase and generate health report
    Analyze {
        /// Path to the project root
        #[arg(short, long, default_value = ".")]
        path: PathBuf,

        /// Output format (markdown, json, html)
        #[arg(short, long, default_value = "markdown")]
        format: String,

        /// Output file path (defaults to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Include hidden files
        #[arg(long)]
        include_hidden: bool,

        /// File extensions to analyze (comma-separated)
        #[arg(short, long, default_value = "rs,ts,tsx,js,jsx,py,go,java")]
        extensions: String,
    },

    /// Generate Claude task instructions for improvements
    Tasks {
        /// Path to the project root
        #[arg(short, long, default_value = ".")]
        path: PathBuf,

        /// Output directory for task files
        #[arg(short, long, default_value = ".claude/tasks")]
        output_dir: PathBuf,

        /// Maximum tasks per file
        #[arg(long, default_value = "5")]
        max_tasks_per_file: usize,

        /// Priority threshold (1-5, only include issues at or above this level)
        #[arg(long, default_value = "3")]
        priority_threshold: u8,

        /// File extensions to analyze
        #[arg(short, long, default_value = "rs,ts,tsx,js,jsx,py,go,java")]
        extensions: String,
    },

    /// Quick summary of codebase health
    Summary {
        /// Path to the project root
        #[arg(short, long, default_value = ".")]
        path: PathBuf,

        /// File extensions to analyze
        #[arg(short, long, default_value = "rs,ts,tsx,js,jsx,py,go,java")]
        extensions: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Analyze {
            path,
            format,
            output,
            include_hidden,
            extensions,
        } => {
            let ext_list: Vec<&str> = extensions.split(',').map(|s| s.trim()).collect();
            let analyzer = CodebaseAnalyzer::new(&path, &ext_list, include_hidden)?;
            let analysis = analyzer.analyze()?;

            let report = match format.as_str() {
                "json" => JsonReporter::generate(&analysis)?,
                "html" => HtmlReporter::generate(&analysis)?,
                _ => MarkdownReporter::generate(&analysis)?,
            };

            match output {
                Some(path) => std::fs::write(path, report)?,
                None => println!("{}", report),
            }
        }

        Commands::Tasks {
            path,
            output_dir,
            max_tasks_per_file,
            priority_threshold,
            extensions,
        } => {
            let ext_list: Vec<&str> = extensions.split(',').map(|s| s.trim()).collect();
            let analyzer = CodebaseAnalyzer::new(&path, &ext_list, false)?;
            let analysis = analyzer.analyze()?;

            let generator = ClaudeTaskGenerator::new(max_tasks_per_file, priority_threshold);
            generator.generate(&analysis, &output_dir)?;

            println!("Task instructions generated in: {}", output_dir.display());
        }

        Commands::Summary {
            path,
            extensions,
        } => {
            let ext_list: Vec<&str> = extensions.split(',').map(|s| s.trim()).collect();
            let analyzer = CodebaseAnalyzer::new(&path, &ext_list, false)?;
            let analysis = analyzer.analyze()?;

            println!("{}", analysis.summary());
        }
    }

    Ok(())
}
