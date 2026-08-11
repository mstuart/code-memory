use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

use code_memory::cli::{Cli, Command};
use code_memory::mcp::server::McpServer;
use code_memory::sessions::{SessionEvent, SessionEventType, SessionTracker};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging to stderr (never stdout — that's for MCP JSON-RPC)
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    match cli.command {
        // Default (no subcommand) or explicit `serve`: run MCP server
        None => {
            let root = std::env::current_dir()?;
            let server = McpServer::new(root);
            server.run().await
        }
        Some(Command::Serve { root }) => {
            let root = canonicalize_root(root)?;
            let server = McpServer::new(root);
            server.run().await
        }
        Some(Command::Init { path }) => cmd_init(path).await,
        Some(Command::Reindex { root, force }) => cmd_reindex(root, force).await,
        Some(Command::Search {
            query,
            limit,
            language,
            root,
        }) => cmd_search(root, &query, limit, language.as_deref()).await,
        Some(Command::Stats { root }) => cmd_stats(root).await,
        Some(Command::Export { output, root }) => cmd_export(root, output).await,
        Some(Command::Import { input, root }) => cmd_import(root, input).await,
        Some(Command::Sessions {
            top,
            min_confidence,
            format,
        }) => cmd_sessions(top, min_confidence, &format).await,
        Some(Command::Web { port }) => cmd_web(port).await,
    }
}

fn canonicalize_root(root: PathBuf) -> Result<PathBuf> {
    let root = if root.is_relative() {
        std::env::current_dir()?.join(root)
    } else {
        root
    };
    Ok(std::fs::canonicalize(&root).unwrap_or(root))
}

fn config_dir(root: &std::path::Path) -> PathBuf {
    let current = root.join(".code-memory");
    let legacy = root.join(".claude-context");

    if current.exists() || !legacy.exists() {
        current
    } else {
        legacy
    }
}

fn writable_config_dir(root: &std::path::Path) -> PathBuf {
    root.join(".code-memory")
}

async fn cmd_init(path: PathBuf) -> Result<()> {
    let path = canonicalize_root(path)?;
    let config_dir = writable_config_dir(&path);

    if config_dir.exists() {
        eprintln!("Already initialized: {}", config_dir.display());
        return Ok(());
    }

    std::fs::create_dir_all(&config_dir)?;

    // Write default config
    let config = r#"# code-memory configuration
# See https://github.com/mstuart/code-memory for documentation

[index]
# Languages to index (empty = all detected)
languages = []

# Patterns to exclude (in addition to .gitignore)
exclude = ["node_modules", "target", "dist", ".git", "*.min.js", "*.map"]

# Maximum file size to index (bytes)
max_file_size = 1048576  # 1 MB

[search]
# Default max results
default_limit = 10

# Enable semantic search (requires model download on first use)
semantic = true

[git]
# Maximum commits to analyze per file
max_commits = 500

# Keywords that indicate architectural decisions
decision_keywords = ["decision:", "chose", "rationale:", "why:", "trade-off", "alternative:"]
"#;

    std::fs::write(config_dir.join("config.toml"), config)?;
    eprintln!("Initialized code-memory in {}", path.display());
    eprintln!("Config: {}", config_dir.join("config.toml").display());
    eprintln!("\nNext steps:");
    eprintln!("  code-memory reindex    # Build the code index");
    eprintln!("  code-memory serve      # Start the MCP server");
    Ok(())
}

async fn cmd_reindex(root: PathBuf, force: bool) -> Result<()> {
    let root = canonicalize_root(root)?;
    eprintln!("Reindexing {} (force={})", root.display(), force);

    let index_path = code_memory::indexer::walker::index_storage_path(&root);
    if force && index_path.exists() {
        std::fs::remove_dir_all(&index_path)?;
    }

    let code_index = code_memory::indexer::code_index::CodeIndex::open_or_create(&index_path)?;
    let stats = code_memory::indexer::walker::index_project(&root, &code_index)?;

    eprintln!(
        "Indexed {} files ({} skipped) with {} symbols in {}ms",
        stats.files_indexed, stats.files_skipped, stats.symbols_found, stats.elapsed_ms
    );
    eprintln!("Index stored at: {}", index_path.display());
    Ok(())
}

async fn cmd_search(
    root: PathBuf,
    query: &str,
    limit: usize,
    _language: Option<&str>,
) -> Result<()> {
    let root = canonicalize_root(root)?;

    let index_path = code_memory::indexer::walker::index_storage_path(&root);
    if !index_path.exists() {
        eprintln!("No index found. Run `code-memory reindex` first.");
        return Ok(());
    }

    let code_index = code_memory::indexer::code_index::CodeIndex::open_or_create(&index_path)?;
    let search = code_memory::search::fulltext::FullTextSearch::new(
        code_index.index(),
        code_index.schema(),
    )?;

    let results = search.search(query, limit)?;
    if results.is_empty() {
        println!("No results found for '{}'", query);
        return Ok(());
    }

    println!("Found {} results for '{}':\n", results.len(), query);
    for (i, result) in results.iter().enumerate() {
        println!(
            "{}. {} (score: {:.2})\n   Language: {}\n   Symbols: {}\n",
            i + 1,
            result.path,
            result.score,
            result.language,
            if result.symbols.is_empty() {
                "(none)"
            } else {
                &result.symbols
            },
        );
    }
    Ok(())
}

async fn cmd_stats(root: PathBuf) -> Result<()> {
    let root = canonicalize_root(root)?;
    let config_dir = config_dir(&root);
    let index_path = code_memory::indexer::walker::index_storage_path(&root);

    println!("code-memory v{}", env!("CARGO_PKG_VERSION"));
    println!("Project root: {}", root.display());
    println!(
        "Initialized: {}",
        if config_dir.exists() { "yes" } else { "no" }
    );
    println!(
        "Index: {}",
        if index_path.exists() {
            "built"
        } else {
            "not built (run `code-memory reindex`)"
        }
    );
    println!("Index path: {}", index_path.display());

    // Knowledge store
    let knowledge_file = config_dir.join("knowledge.json");
    if knowledge_file.exists() {
        if let Ok(data) = std::fs::read_to_string(&knowledge_file) {
            let entries: Vec<serde_json::Value> = serde_json::from_str(&data).unwrap_or_default();
            println!("Knowledge entries: {}", entries.len());
        }
    } else {
        println!("Knowledge entries: 0");
    }

    // Git info
    match code_memory::git::history::GitHistory::discover(&root) {
        Ok(git) => {
            let commits = git.walk_commits(1).unwrap_or_default();
            if !commits.is_empty() {
                println!("Git: available (latest commit: {})", &commits[0].id[..8]);
            } else {
                println!("Git: available (no commits)");
            }
        }
        Err(_) => println!("Git: not available"),
    }

    Ok(())
}

async fn cmd_export(root: PathBuf, output: PathBuf) -> Result<()> {
    let root = canonicalize_root(root)?;
    let knowledge_file = config_dir(&root).join("knowledge.json");

    if !knowledge_file.exists() {
        eprintln!("No knowledge entries to export.");
        return Ok(());
    }

    let data = std::fs::read_to_string(&knowledge_file)?;
    std::fs::write(&output, &data)?;

    let entries: Vec<serde_json::Value> = serde_json::from_str(&data).unwrap_or_default();
    eprintln!(
        "Exported {} knowledge entries to {}",
        entries.len(),
        output.display()
    );
    Ok(())
}

async fn cmd_import(root: PathBuf, input: PathBuf) -> Result<()> {
    let root = canonicalize_root(root)?;
    let config_dir = config_dir(&root);
    let knowledge_file = config_dir.join("knowledge.json");

    if !input.exists() {
        eprintln!("Input file not found: {}", input.display());
        return Ok(());
    }

    let import_data = std::fs::read_to_string(&input)?;
    let import_entries: Vec<serde_json::Value> = serde_json::from_str(&import_data)?;

    // Merge with existing
    let mut entries: Vec<serde_json::Value> = if knowledge_file.exists() {
        let data = std::fs::read_to_string(&knowledge_file)?;
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        std::fs::create_dir_all(&config_dir)?;
        Vec::new()
    };

    let before = entries.len();
    for entry in import_entries {
        let topic = entry.get("topic").and_then(|v| v.as_str()).unwrap_or("");
        let exists = entries
            .iter()
            .any(|e| e.get("topic").and_then(|v| v.as_str()) == Some(topic));
        if !exists {
            entries.push(entry);
        }
    }

    std::fs::write(&knowledge_file, serde_json::to_string_pretty(&entries)?)?;

    eprintln!(
        "Imported {} new entries (total: {}) from {}",
        entries.len() - before,
        entries.len(),
        input.display()
    );
    Ok(())
}

async fn cmd_sessions(top: usize, min_confidence: f32, format: &str) -> Result<()> {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let history_path = home.join(".claude");

    if !history_path.exists() {
        eprintln!("No Claude Code history found at {}", history_path.display());
        eprintln!("Claude Code stores session data in ~/.claude/");
        return Ok(());
    }

    eprintln!(
        "Analyzing Claude Code sessions in {}...",
        history_path.display()
    );

    let mut tracker = SessionTracker::new(history_path);
    let events = tracker.analyze_all_sessions();

    if format == "json" {
        let output = serde_json::json!({
            "events": events,
            "event_count": events.len(),
            "patterns": tracker.patterns().to_json(),
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // Text output
    println!("code-memory: Session Analysis");
    println!("================================\n");

    // Events summary
    let decisions: Vec<&SessionEvent> = events
        .iter()
        .filter(|e| e.event_type == SessionEventType::ArchitecturalDecision)
        .collect();
    let errors: Vec<&SessionEvent> = events
        .iter()
        .filter(|e| e.event_type == SessionEventType::ErrorAndFix)
        .collect();
    let tests: Vec<&SessionEvent> = events
        .iter()
        .filter(|e| e.event_type == SessionEventType::TestWritten)
        .collect();
    let refactors: Vec<&SessionEvent> = events
        .iter()
        .filter(|e| e.event_type == SessionEventType::Refactoring)
        .collect();

    println!("Events found: {}", events.len());
    println!("  Architectural decisions: {}", decisions.len());
    println!("  Error-and-fix pairs:     {}", errors.len());
    println!("  Tests written:           {}", tests.len());
    println!("  Refactorings:            {}", refactors.len());
    println!();

    // Show recent decisions
    if !decisions.is_empty() {
        println!("Recent Decisions:");
        for decision in decisions.iter().rev().take(5) {
            let desc = if decision.description.len() > 80 {
                format!("{}...", &decision.description[..80])
            } else {
                decision.description.clone()
            };
            println!("  - {}", desc);
        }
        println!();
    }

    // Show patterns
    let patterns = if min_confidence > 0.0 {
        tracker.patterns().confident_patterns(min_confidence)
    } else {
        tracker.patterns().top_patterns(top)
    };

    if !patterns.is_empty() {
        println!("Learned Patterns:");
        for pattern in &patterns {
            println!(
                "  [{:.0}%] {} (observed {} times)",
                pattern.confidence * 100.0,
                pattern.description,
                pattern.observation_count
            );
        }
    } else {
        println!("No patterns learned yet. More session data needed.");
    }

    Ok(())
}

async fn cmd_web(port: u16) -> Result<()> {
    let addr = format!("127.0.0.1:{}", port);
    eprintln!("Starting web UI on http://{}", addr);
    eprintln!("Note: This is a Pro-only feature");

    code_memory::web::server::start_server(&addr)?;
    Ok(())
}
