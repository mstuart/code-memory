<div align="center">
  <img src="docs/assets/logo.svg" alt="code-memory — Persistent memory for AI coding - semantic search, git history, and intelligent context preservation" width="720">
</div>

<p align="center"><strong>Persistent memory for AI coding - semantic search, git history, and intelligent context preservation</strong></p>

<p align="center">
  <a href="https://github.com/mstuart/code-memory/actions/workflows/ci.yml"><img src="https://github.com/mstuart/code-memory/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <img src="https://img.shields.io/badge/rust-stable-orange.svg" alt="Rust">
  <a href="https://deepwiki.com/mstuart/code-memory"><img src="https://deepwiki.com/badge.svg" alt="Ask DeepWiki"></a>
</p>

---
[![npm version](https://badge.fury.io/js/code-memory.svg)](https://www.npmjs.com/package/code-memory)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Socket](https://socket.dev/api/badge/npm/package/code-memory)](https://socket.dev/npm/package/code-memory)

Code Memory is an MCP (Model Context Protocol) server that gives AI coding assistants long-term memory of your codebase through semantic search, git history analysis, and intelligent context preservation.

## The Problem

AI coding assistants forget:
- 🤔 **Context between sessions** - "Why did we use JWT instead of sessions?"
- 🔍 **Architectural decisions** - "What was the rationale for this design?"
- 📚 **Historical discussions** - "We already tried that approach last month"
- 🕸️ **Code relationships** - "What else depends on this module?"

## The Solution

Code Memory provides:
- 🧠 **Semantic search** - Find code by meaning, not just keywords
- 📚 **Git history analysis** - Extract decisions from commit messages and PRs
- 🕸️ **Dependency graphs** - Understand code relationships across 16 languages
- 📝 **Session learning** - Learn patterns from your coding sessions
- 💾 **Persistent knowledge** - Remember important facts across sessions

## Quick Start

### Installation

```bash
npm install -g code-memory
```

### Basic Usage

```bash
# Initialize in your project
cd your-project
code-memory init

# Index your codebase
code-memory reindex

# Start the MCP server
code-memory serve

# Search from CLI
code-memory search "authentication flow"
```

### Configure with Claude Code

Add to your `~/.claude/config.json`:

```json
{
  "mcpServers": {
    "code-memory": {
      "command": "code-memory",
      "args": ["serve"]
    }
  }
}
```

## Features

### 🔍 Semantic Search

Find code by meaning, not just keywords:

```bash
code-memory search "user authentication logic"
# Finds auth code even if it doesn't contain the word "user"
```

**How it works:**
- Uses `fastembed` for local embeddings (all-MiniLM-L6-v2)
- No API calls, completely offline
- Understands intent, not just text matching

### 📝 Full-Text Search

Fast keyword-based search powered by tantivy:

```bash
code-memory search --fulltext "async function"
```

### 📚 Git History Analysis

Extract architectural decisions from your git history:

```bash
code-memory trace-decision "why microservices"
```

**Finds:**
- Explicit decisions in commit messages
- Architectural choices from PR descriptions
- Rationale and "why" statements
- Refactoring decisions
- Breaking changes

### 🕸️ Dependency Graphs

Understand what depends on what:

```bash
code-memory find-related "UserService"
```

**Analyzes:**
- Import relationships
- Co-change patterns (files changed together)
- Dependency chains
- Most coupled modules

**Supported languages:** Rust, TypeScript, JavaScript, Python, Go, Java, C++, C#, Ruby, PHP, Swift, Kotlin, Scala, Haskell, Elixir, Clojure

### 📊 Session Tracking

Learn from your coding sessions:

```bash
code-memory sessions
```

**Extracts:**
- Architectural decisions made in sessions
- Error-and-fix patterns
- Refactoring approaches
- Testing strategies

### 💾 Persistent Knowledge

Remember important facts:

```bash
# Via MCP server
remember("We use JWT for auth because of scalability requirements")
```

## MCP Tools

Code Memory provides 7 MCP tools for AI assistants:

### 1. `search_code`

Search codebase with semantic + full-text search:

```typescript
search_code({
  query: "authentication flow",
  limit: 10
})
```

### 2. `explain_code`

Get detailed explanation of a symbol:

```typescript
explain_code({
  symbol: "UserService",
  context_lines: 20
})
```

### 3. `trace_decision`

Find why a decision was made:

```typescript
trace_decision({
  topic: "why microservices",
  max_results: 5
})
```

### 4. `find_related`

Find related code and dependencies:

```typescript
find_related({
  symbol: "AuthController",
  relationship_type: "both" // "depends_on" | "depended_by" | "both"
})
```

### 5. `remember`

Store persistent knowledge:

```typescript
remember({
  key: "auth-strategy",
  value: "We use JWT for stateless auth across microservices"
})
```

### 6. `index_project`

Manually trigger reindexing:

```typescript
index_project({
  force: true
})
```

### 7. `get_session_patterns`

Retrieve learned patterns:

```typescript
get_session_patterns({
  pattern_type: "architecture", // or "errors", "refactoring", "testing"
  min_confidence: 0.7
})
```

## CLI Commands

### `init`

Initialize Code Memory in a project:

```bash
code-memory init
```

Creates `.code-memory/` directory with:
- `config.toml` - Configuration
- `index/` - Search index
- `knowledge.json` - Persistent knowledge

### `serve`

Start the MCP server:

```bash
code-memory serve
```

### `reindex`

Rebuild the code index:

```bash
code-memory reindex           # Incremental
code-memory reindex --force   # Full rebuild
```

### `search`

Search from the command line:

```bash
code-memory search "query"
code-memory search "query" --lang rust
code-memory search "query" --limit 20
```

### `stats`

Show index statistics:

```bash
code-memory stats
```

### `sessions`

View session patterns:

```bash
code-memory sessions
code-memory sessions -n 10            # Top 10
code-memory sessions -c 0.8           # Min confidence 0.8
code-memory sessions -f json          # JSON output
```

### `export` / `import`

Backup and restore knowledge:

```bash
code-memory export knowledge.json
code-memory import knowledge.json
```

## Configuration

Configuration is stored in `.code-memory/config.toml`:

```toml
[indexing]
# File patterns to index
include = ["**/*.rs", "**/*.ts", "**/*.js", "**/*.py"]

# File patterns to ignore
exclude = ["**/node_modules/**", "**/target/**", "**/.git/**"]

# Maximum file size (in bytes)
max_file_size = 1048576  # 1MB

[search]
# Number of results to return by default
default_limit = 10

# Minimum relevance score (0.0-1.0)
min_score = 0.5

[git]
# Analyze git history for decisions
analyze_history = true

# How far back to look (in days)
history_depth = 365

[embedding]
# Embedding model (local, no API calls)
model = "all-MiniLM-L6-v2"

# Embedding dimension
dimension = 384
```

## Architecture

```
code-memory/
├── src/
│   ├── indexer/         # Code indexing with tantivy
│   │   ├── walker.rs    # File system traversal
│   │   ├── parser.rs    # Symbol extraction
│   │   └── code_index.rs
│   ├── search/          # Search engines
│   │   ├── fulltext.rs  # Tantivy full-text search
│   │   ├── semantic.rs  # Fastembed semantic search
│   │   └── hybrid.rs    # Combined ranking
│   ├── git/             # Git history analysis
│   │   ├── history.rs   # Commit parsing
│   │   └── decisions.rs # Decision extraction
│   ├── graph/           # Dependency graphs
│   │   ├── imports.rs   # Import parsing
│   │   └── analyzer.rs  # Graph analysis
│   ├── sessions/        # Session tracking
│   │   ├── tracker.rs   # Event extraction
│   │   └── patterns.rs  # Pattern learning
│   ├── mcp/             # MCP server
│   │   ├── server.rs    # JSON-RPC server
│   │   ├── tools.rs     # Tool handlers
│   │   └── protocol.rs  # MCP protocol
│   └── cli.rs           # CLI interface
└── .code-memory/
    ├── config.toml      # Configuration
    ├── index/           # Search index
    └── knowledge.json   # Persistent facts
```

## Performance

- **Indexing speed:** ~10,000 files/minute
- **Search latency:** <100ms (full-text), <200ms (semantic)
- **Memory usage:** ~100MB for 50k files
- **Binary size:** 8MB (including embedding model)

## Pricing

- **Free:** Up to 5,000 files, basic search
- **Pro ($20/month):**
  - Unlimited files
  - Advanced query optimization
  - Team knowledge sharing
  - Priority support

## Supported Languages

Full support (symbol extraction + imports):
- Rust, TypeScript, JavaScript, Python, Go
- Java, C++, C#, Ruby, PHP
- Swift, Kotlin, Scala, Haskell, Elixir, Clojure

Additional languages supported for full-text search only.

## Comparison

| Feature | Code Memory | grep/ripgrep | GitHub Copilot |
|---------|------------|--------------|----------------|
| Semantic search | ✅ | ❌ | ✅ (API) |
| Offline | ✅ | ✅ | ❌ |
| Git history | ✅ | ❌ | ❌ |
| Dependency graphs | ✅ | ❌ | ❌ |
| Session learning | ✅ | ❌ | ✅ |
| MCP integration | ✅ | ❌ | ❌ |
| Cost | Free/$20 | Free | $10+/mo |

## FAQ

### How is this different from grep/ripgrep?

Code Memory understands **meaning**, not just text:
- "user auth" finds authentication code even without those exact words
- Extracts architectural decisions from git history
- Understands code relationships across files
- Learns from your coding sessions

### Does it send my code to an API?

**No.** Everything runs locally:
- Embeddings generated on your machine (fastembed)
- Search index stored locally (tantivy)
- No network calls during normal operation
- Your code never leaves your computer

### How much disk space does it use?

Approximately:
- ~50MB per 10,000 files indexed
- Embedding model: 90MB (downloaded once)
- Total: ~150-300MB for a typical project

### Can I use it with VS Code / other editors?

Yes! Code Memory is an MCP server, so it works with any MCP-compatible tool:
- Claude Code CLI
- Any editor with MCP support
- Custom integrations via MCP protocol

### What about private repositories?

Code Memory only accesses files you explicitly index. It never:
- Uploads code to remote servers
- Shares data with third parties
- Requires authentication or accounts (for free tier)

## Troubleshooting

### Index not building

```bash
# Check for errors
code-memory reindex --verbose

# Force rebuild
code-memory reindex --force

# Check config
cat .code-memory/config.toml
```

### Search returns no results

```bash
# Verify index exists
code-memory stats

# Rebuild index
code-memory reindex --force

# Check file patterns in config
```

### MCP server not starting

```bash
# Check if port is in use
lsof -i :8080

# Start with verbose logging
code-memory serve --verbose
```

## Development

### Building from source

```bash
git clone https://github.com/mstuart/code-memory.git
cd code-memory
cargo build --release
```

### Running tests

```bash
cargo test                    # Run all tests
cargo test --lib             # Unit tests only
cargo test --test mcp_tools  # Integration tests
```

## Contributing

Contributions welcome! Please:
1. Fork the repository
2. Create a feature branch
3. Add tests for new features
4. Ensure all tests pass
5. Submit a pull request

## License

MIT License - see [LICENSE](LICENSE) for details

## Support

- 📖 [Documentation](https://github.com/mstuart/code-memory)
- 🐛 [Issues](https://github.com/mstuart/code-memory/issues)
- 💬 [Discussions](https://github.com/mstuart/code-memory/discussions)

## Acknowledgments

Built with:
- [tantivy](https://github.com/quickwit-oss/tantivy) - Full-text search
- [fastembed](https://github.com/Anush008/fastembed-rs) - Local embeddings
- [git2](https://github.com/rust-lang/git2-rs) - Git integration
- [petgraph](https://github.com/petgraph/petgraph) - Dependency graphs
- [tree-sitter](https://tree-sitter.github.io/) - Code parsing

---

**Give your AI assistant a memory. Never lose context again.** 🧠

```bash
npm install -g code-memory
```
