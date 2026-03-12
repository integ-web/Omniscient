# 🔮 Omniscient — The God of All Research Agents

A zero-compromise, Rust-native deep research AI agent designed to run on low-end hardware while outperforming existing research tools.

**Cross-platform** with **Windows-first optimization**. Built entirely in Rust for maximum performance, safety, and minimal resource usage.

## ✨ Features

- 🏗️ **Pure Rust** — Single binary, ~50MB, <100ms startup
- 🧠 **Multi-LLM** — OpenAI, Anthropic, Ollama (local), BitNet (1-bit)
- 🔍 **Multi-Search** — DuckDuckGo, Brave, SearXNG, Serper, Firecrawl
- 📚 **Academic** — arXiv, Semantic Scholar (free, no API keys)
- 🕷️ **Smart Crawling** — Async crawler with rate limiting and content extraction
- 🧬 **Knowledge Graph** — SurrealDB for entities and relationships
- 📊 **Full-Text Search** — Tantivy (Lucene-level) indexing
- 🤖 **SLM Categorization** — Small models classify tasks to reduce load
- 📄 **Report Generation** — Markdown reports with citations
- ⚡ **Low Resource** — Runs on 4GB RAM with quantized models

## 🚀 Quick Start

```bash
# Build the project
cargo build --release

# Initialize configuration
omniscient config init

# Run a research query
omniscient research "AI agent frameworks comparison 2026"

# Deep research
omniscient research "Tesla competitive analysis" --depth deep

# Check status
omniscient status
```

## 📦 Architecture

```
omniscient/
├── crates/
│   ├── omniscient-core/      # Agent traits, orchestrator, memory, tools
│   ├── omniscient-llm/       # LLM inference (Ollama/OpenAI/Anthropic)
│   ├── omniscient-web/       # Crawler, search, academic DBs
│   ├── omniscient-knowledge/ # Tantivy search + SurrealDB graph
│   ├── omniscient-research/  # Research pipelines + report gen
│   └── omniscient-cli/       # CLI interface
├── config/
│   └── omniscient.toml       # Configuration
└── gui/                      # Tauri v2 GUI (coming soon)
```

## 🔧 Configuration

```bash
omniscient config init    # Create default config
omniscient config show    # Show current config
```

Edit `config/omniscient.toml` to configure:
- LLM providers and models
- Search engines and API keys
- Research depth defaults
- Knowledge database settings

## 📋 Commands

| Command | Description |
|---------|-------------|
| `omniscient research "query"` | Conduct deep research |
| `omniscient research "query" --depth deep` | Set research depth |
| `omniscient profile "Tesla" --type company` | Company profile |
| `omniscient profile "Karpathy" --type person` | Person profile |
| `omniscient compare "A,B,C"` | Compare entities |
| `omniscient config init` | Initialize config |
| `omniscient status` | Show system status |

## 🔮 Superiority

| Feature | Omniscient | Python Agents |
|---------|-----------|---------------|
| Memory | ~50MB | 500MB+ |
| Startup | <100ms | 2-5s |
| Concurrency | True async | GIL-limited |
| Safety | Rust memory safety | Runtime crashes |
| Deployment | Single binary | pip install |

## 🛠️ Building

```bash
# Debug build
cargo build --workspace

# Release build (optimized)
cargo build --release

# Run tests
cargo test --workspace
```

## License

MIT
