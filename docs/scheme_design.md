# 📊 Omniscient Scheme Design

Omniscient uses a dual-database approach for storing research knowledge: SurrealDB for structured entities and relationships, and Tantivy for unstructured full-text search.

## 🧬 Knowledge Graph (SurrealDB)

The knowledge graph stores refined entities extracted during research.

### Table: `entity`
Used to store all identified entities (People, Companies, etc.).

| Field | Type | Description |
|-------|------|-------------|
| `id` | `record` | Unique ID (e.g., `entity:google`) |
| `name` | `string` | Human-readable name |
| `entity_type` | `string` | Category (Person, Company, Organization, etc.) |
| `attributes` | `object` | Key-value pairs of entity metadata |

### Table: `relationship`
An edge between two entities.

| Field | Type | Description |
|-------|------|-------------|
| `in` | `record` | Destination entity |
| `out` | `record` | Source entity |
| `relation_type` | `string` | Type of link (e.g., "FOUNDED_BY", "COMPETES_WITH") |
| `confidence` | `float` | AI confidence score (0.0 - 1.0) |
| `source` | `string` | URL or document title where link was found |

### Table: `document`
References to raw sources.

| Field | Type | Description |
|-------|------|-------------|
| `id` | `record` | Unique ID |
| `title` | `string` | Document title |
| `url` | `string` (opt) | Source URL |
| `summary` | `string` | AI-generated summary |
| `timestamp` | `datetime` | When it was indexed |

## 🔍 Search Index (Tantivy)

The full-text index allows for rapid retrieval of raw content across all research sessions.

| Field | Type | Indexed | Stored | Description |
|-------|------|---------|--------|-------------|
| `id` | `STRING` | Yes | Yes | Unique doc ID |
| `title` | `TEXT` | Yes | Yes | Document title |
| `content` | `TEXT` | Yes | Yes | Full extracted text |
| `url` | `STRING` | No | Yes | Source URL |
| `source` | `STRING` | Yes | Yes | Source identifier |
| `timestamp` | `STRING` | No | Yes | ISO8601 timestamp |

## ⚙️ Configuration Scheme (`omniscient.toml`)

Managed in `omniscient-core/src/config.rs`.

- **`llm`**: Model selections and API keys.
- **`search`**: Enabled engines (Brave, DuckDuckGo, etc.) and keys.
- **`database`**: File paths for SurrealDB and Tantivy index.
- **`agent`**: Default iteration limits and research depths.
