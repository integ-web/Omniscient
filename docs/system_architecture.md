# 🏗️ Omniscient System Architecture

Omniscient is built with a modular, crate-based architecture designed for high performance and low resource usage.

## 📦 Component Overview

The system is divided into several specialized crates:

- **`omniscient-core`**: The heart of the system. Defines traits like `Agent`, the `Orchestrator`, and internal memory systems.
- **`omniscient-research`**: Implements high-level research agents and pipelines (e.g., `ResearchAgent`, `DeepResearch`).
- **`omniscient-knowledge`**: Manages the knowledge graph (SurrealDB) and full-text search index (Tantivy).
- **`omniscient-web`**: Handles web searching, crawling, and content extraction.
- **`omniscient-llm`**: Unified interface for various LLM providers (Ollama, OpenAI, Anthropic).
- **`omniscient-cli`**: The user-facing command-line interface.

## 🔄 The Research Loop (Orchestrator)

The `Orchestrator` manages the lifecycle of a `ResearchTask` through a continuous **Plan → Execute → Synthesize** loop.

1.  **Planning**: The `Agent` analyzes the query and current context to generate an `AgentPlan` (a series of steps with specific tools).
2.  **Execution**: The `Orchestrator` iterates through plan steps, invoking tools via the `ToolRegistry`. Results are stored in `Memory`.
3.  **Synthesis**: The `Agent` combines findings from the execution phase into a `Synthesis` report.
4.  **Feedback**: The system checks if the research goals are met. If gaps remain, it loops back to step 1, using the `Synthesis` as new context.

## 🧠 Core Systems

### Agent System
Agents implement the `Agent` trait, allowing them to:
- `plan`: Create a sequence of actions.
- `execute_step`: Run a specific step using tools.
- `synthesize`: Combine raw data into insights.

### Memory & Knowledge
- **Working Memory**: In-memory ephemeral storage for active research steps.
- **Long-term Knowledge**: Persistent storage using **SurrealDB** for entity-relationship graphs and **Tantivy** for full-text indexing.

### Tool Registry
A central registry for all capabilities:
- `web_search`: Multi-engine search.
- `web_crawl`: Content extraction.
- `academic_search`: arXiv and Semantic Scholar.
- `analyze`: Text analysis using SLMs/LLMs.
