use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "claude-context",
    version,
    about = "Intelligent code context for Claude — full-text search, semantic search, git history, and dependency analysis",
    long_about = "claude-context is an MCP server that gives Claude deep understanding of your codebase.\n\n\
                  It indexes your code, extracts symbols, analyzes git history, and builds dependency\n\
                  graphs so Claude can search, explain, trace decisions, and find related code."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Start the MCP server (JSON-RPC over stdin/stdout)
    Serve {
        /// Project root directory to serve context for
        #[arg(short, long, default_value = ".")]
        root: PathBuf,
    },

    /// Initialize claude-context in a project directory
    Init {
        /// Directory to initialize (default: current directory)
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Rebuild the code index for a project
    Reindex {
        /// Project root directory
        #[arg(short, long, default_value = ".")]
        root: PathBuf,

        /// Force full reindex (ignore cache)
        #[arg(short, long)]
        force: bool,
    },

    /// Search code from the command line (for testing)
    Search {
        /// Search query
        query: String,

        /// Maximum results
        #[arg(short, long, default_value = "10")]
        limit: usize,

        /// Filter by language
        #[arg(short = 'L', long)]
        language: Option<String>,

        /// Project root directory
        #[arg(short, long, default_value = ".")]
        root: PathBuf,
    },

    /// Show index statistics
    Stats {
        /// Project root directory
        #[arg(short, long, default_value = ".")]
        root: PathBuf,
    },

    /// Export knowledge base
    Export {
        /// Output file path
        #[arg(short, long, default_value = "claude-context-export.json")]
        output: PathBuf,

        /// Project root directory
        #[arg(short, long, default_value = ".")]
        root: PathBuf,
    },

    /// Import knowledge base
    Import {
        /// Input file path
        input: PathBuf,

        /// Project root directory
        #[arg(short, long, default_value = ".")]
        root: PathBuf,
    },

    /// Analyze Claude Code sessions and extract patterns
    Sessions {
        /// Show top N patterns (default: 10)
        #[arg(short = 'n', long, default_value = "10")]
        top: usize,

        /// Minimum confidence threshold (0.0-1.0)
        #[arg(short, long, default_value = "0.0")]
        min_confidence: f32,

        /// Output format (text or json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Start web UI server (Pro-only feature)
    Web {
        /// Port to listen on
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },
}
