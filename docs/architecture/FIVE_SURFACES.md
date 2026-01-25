# Strata: Five API Surfaces

> **Status**: Architecture Decision
> **Date**: 2025-01-24

---

## Executive Summary

Strata is an **embedded database** like SQLite, not a client-server database like PostgreSQL. Users embed Strata directly into their applications - no separate server deployment required.

To reach developers across the modern tech stack, Strata provides **five API surfaces**:

| Surface | Target Audience | Installation | Status |
|---------|-----------------|--------------|--------|
| **Rust** | Systems developers, performance-critical apps | `cargo add strata` | ✅ Done |
| **Python** | AI/ML developers, data scientists | `pip install strata` | Planned |
| **Node.js** | Web developers, serverless | `npm install strata` | Planned |
| **CLI** | DevOps, debugging, exploration | `brew install strata` | Planned |
| **MCP** | AI agents (Claude, etc.) | Configure in AI client | Planned |

All five surfaces wrap the same embedded Rust core. No network calls, no server processes (except MCP which runs as a subprocess).

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                      User Applications                       │
├───────────┬───────────┬───────────┬───────────┬─────────────┤
│   Rust    │  Python   │  Node.js  │    CLI    │     MCP     │
│   App     │   App     │    App    │   REPL    │   Server    │
├───────────┼───────────┼───────────┼───────────┼─────────────┤
│  Native   │   PyO3    │  napi-rs  │  Native   │   Native    │
│           │ Bindings  │ Bindings  │   Rust    │    Rust     │
├───────────┴───────────┴───────────┴───────────┴─────────────┤
│                                                              │
│                    strata (Rust Core)                        │
│                                                              │
│  ┌─────────┬─────────┬─────────┬─────────┬─────────┐       │
│  │   KV    │  JSON   │ Events  │  State  │ Vectors │       │
│  └─────────┴─────────┴─────────┴─────────┴─────────┘       │
│                                                              │
│  ┌─────────────────────────────────────────────────┐       │
│  │              Storage Engine                      │       │
│  │         (WAL, Snapshots, Recovery)              │       │
│  └─────────────────────────────────────────────────┘       │
│                                                              │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
                    ┌─────────────────┐
                    │  ./my-database/ │
                    │    (on disk)    │
                    └─────────────────┘
```

---

## Surface 1: Rust (Native)

### Audience
- Systems programmers
- Performance-critical applications
- Embedded systems
- Other database/infrastructure projects

### Installation
```toml
[dependencies]
strata = "0.1"
```

### API Example
```rust
use strata::prelude::*;

fn main() -> Result<()> {
    // Open database (creates if not exists)
    let db = Strata::open("./my-agent-db")?;

    // Simple key-value
    db.kv.set("user:1", "Alice")?;
    db.kv.set("count", 42)?;

    // JSON documents
    let mut profile = HashMap::new();
    profile.insert("name".to_string(), Value::from("Alice"));
    profile.insert("score".to_string(), Value::from(100));
    db.json.set("profile:1", profile)?;

    // Vector search
    db.vectors.create_collection(&run, "embeddings", 384, DistanceMetric::Cosine)?;
    db.vectors.upsert(&run, "embeddings", "doc-1", &embedding, Some(metadata))?;

    let results = db.vectors.search(&run, "embeddings", &query, 10, None)?;

    // Event streams
    let mut event = HashMap::new();
    event.insert("action".to_string(), Value::from("login"));
    db.events.append("audit-log", event)?;

    // Runs for isolation
    let run = db.runs.create(None)?;
    db.kv.set_in(&run, "scoped-key", "scoped-value")?;
    db.runs.close(&run)?;

    Ok(())
}
```

### Implementation
- Pure Rust, no FFI
- Direct access to all internals
- Full type safety
- Zero-cost abstractions

### Status: ✅ Complete

---

## Surface 2: Python (PyO3)

### Audience
- AI/ML engineers
- Data scientists
- Jupyter notebook users
- LangChain/LlamaIndex developers
- The vast majority of AI agent developers

### Installation
```bash
pip install strata
```

### API Example
```python
from strata import Strata, DistanceMetric

# Open database
db = Strata.open("./my-agent-db")

# Simple key-value (Pythonic!)
db.kv.set("user:1", "Alice")
db.kv.set("count", 42)
db.kv.set("config", {"debug": True, "max_retries": 3})

name = db.kv.get("user:1")  # Returns "Alice"

# JSON documents
db.json.set("profile:1", {
    "name": "Alice",
    "preferences": {"theme": "dark"}
})

# Vector search (the killer feature for AI)
db.vectors.create_collection("embeddings", dimension=384)

# Store with metadata
db.vectors.upsert("embeddings", "doc-1",
    vector=embedding,  # numpy array or list
    metadata={"title": "Hello World", "category": "greeting"}
)

# Search
results = db.vectors.search("embeddings", query_vector, k=10)
for match in results:
    print(f"{match.key}: {match.score:.3f}")
    print(f"  metadata: {match.metadata}")

# Event streams for agent traces
db.events.append("agent-trace", {
    "step": 1,
    "action": "search",
    "query": "What is the capital of France?"
})

# Runs for conversation isolation
run = db.runs.create(metadata={"conversation_id": "abc123"})
db.kv.set_in(run, "context", "User is asking about geography")
db.runs.close(run)

# Context manager support
with Strata.open("./db") as db:
    db.kv.set("key", "value")
# Auto-closes on exit
```

### LangChain Integration Example
```python
from strata import Strata
from langchain.vectorstores import VectorStore

class StrataVectorStore(VectorStore):
    """LangChain-compatible vector store backed by Strata."""

    def __init__(self, path: str, collection: str, embedding_fn):
        self.db = Strata.open(path)
        self.collection = collection
        self.embedding_fn = embedding_fn

    def add_texts(self, texts: list[str], metadatas: list[dict] = None):
        for i, text in enumerate(texts):
            embedding = self.embedding_fn(text)
            metadata = metadatas[i] if metadatas else {}
            metadata["text"] = text
            self.db.vectors.upsert(self.collection, f"doc-{i}", embedding, metadata)

    def similarity_search(self, query: str, k: int = 4):
        query_embedding = self.embedding_fn(query)
        results = self.db.vectors.search(self.collection, query_embedding, k=k)
        return [Document(page_content=r.metadata["text"], metadata=r.metadata)
                for r in results]
```

### Implementation
```
strata-python/
├── Cargo.toml          # PyO3 + maturin setup
├── src/
│   └── lib.rs          # Rust bindings
├── python/
│   └── strata/
│       ├── __init__.py
│       └── py.typed    # Type hints
└── tests/
    └── test_strata.py
```

**Tech Stack:**
- **PyO3**: Rust ↔ Python bindings
- **maturin**: Build and publish to PyPI
- **numpy**: Zero-copy array support for vectors

### Status: Planned (High Priority)

---

## Surface 3: Node.js (napi-rs)

### Audience
- Web developers
- Serverless/Edge function developers
- Electron app developers
- Full-stack JavaScript developers

### Installation
```bash
npm install strata
# or
yarn add strata
# or
pnpm add strata
```

### API Example
```typescript
import { Strata, DistanceMetric } from 'strata';

// Open database
const db = await Strata.open('./my-agent-db');

// Key-value (supports any JSON-serializable value)
await db.kv.set('user:1', 'Alice');
await db.kv.set('config', { debug: true, maxRetries: 3 });

const name = await db.kv.get('user:1'); // 'Alice'

// JSON documents
await db.json.set('profile:1', {
  name: 'Alice',
  preferences: { theme: 'dark' }
});

// Vector search
await db.vectors.createCollection('embeddings', {
  dimension: 384,
  metric: DistanceMetric.Cosine
});

await db.vectors.upsert('embeddings', 'doc-1', {
  vector: embedding, // Float32Array or number[]
  metadata: { title: 'Hello World' }
});

const results = await db.vectors.search('embeddings', queryVector, { k: 10 });
for (const match of results) {
  console.log(`${match.key}: ${match.score.toFixed(3)}`);
}

// Event streams
await db.events.append('agent-trace', {
  step: 1,
  action: 'search',
  query: 'What is the capital of France?'
});

// Runs
const run = await db.runs.create({ conversationId: 'abc123' });
await db.kv.setIn(run, 'context', 'Geography discussion');
await db.runs.close(run);

// Cleanup
await db.close();
```

### Vercel AI SDK Integration Example
```typescript
import { Strata } from 'strata';
import { openai } from '@ai-sdk/openai';
import { generateText } from 'ai';

const db = await Strata.open('./agent-memory');

// Store conversation in Strata
async function chat(userMessage: string) {
  const run = await db.runs.create();

  // Store user message
  await db.events.appendIn(run, 'messages', {
    role: 'user',
    content: userMessage,
    timestamp: Date.now()
  });

  // Get relevant context from vector search
  const embedding = await embed(userMessage);
  const context = await db.vectors.search('knowledge', embedding, { k: 5 });

  // Generate response
  const { text } = await generateText({
    model: openai('gpt-4'),
    prompt: `Context: ${JSON.stringify(context)}\n\nUser: ${userMessage}`
  });

  // Store assistant response
  await db.events.appendIn(run, 'messages', {
    role: 'assistant',
    content: text,
    timestamp: Date.now()
  });

  return text;
}
```

### Implementation
```
strata-node/
├── Cargo.toml          # napi-rs setup
├── src/
│   └── lib.rs          # Rust bindings
├── index.js            # JS entry point
├── index.d.ts          # TypeScript types
└── package.json
```

**Tech Stack:**
- **napi-rs**: Rust ↔ Node.js bindings
- Native N-API for stability across Node versions
- TypeScript definitions included

### Status: Planned (Medium Priority)

---

## Surface 4: CLI (Interactive REPL)

### Audience
- Developers debugging their databases
- DevOps inspecting production data
- Users exploring Strata without writing code
- Scripting and automation

### Installation
```bash
# macOS
brew install strata

# Linux
curl -sSL https://strata.dev/install.sh | sh

# Windows
scoop install strata

# From source
cargo install strata-cli
```

### Interactive Mode
```
$ strata ./my-agent-db

   _____ _             _
  / ____| |           | |
 | (___ | |_ _ __ __ _| |_ __ _
  \___ \| __| '__/ _` | __/ _` |
  ____) | |_| | | (_| | || (_| |
 |_____/ \__|_|  \__,_|\__\__,_|

 Database: ./my-agent-db
 Size: 142 MB | Runs: 23 | Last modified: 2 hours ago

strata> kv.set("greeting", "Hello, World!")
✓ OK (version: 42)

strata> kv.get("greeting")
"Hello, World!"

strata> vectors.search("embeddings", [0.1, 0.2, ...], k=5)
┌────┬───────────┬─────────┬────────────────────────────┐
│ #  │ Key       │ Score   │ Metadata                   │
├────┼───────────┼─────────┼────────────────────────────┤
│ 1  │ doc-42    │ 0.9523  │ {"title": "Introduction"}  │
│ 2  │ doc-17    │ 0.8891  │ {"title": "Getting Start"} │
│ 3  │ doc-103   │ 0.8456  │ {"title": "API Reference"} │
└────┴───────────┴─────────┴────────────────────────────┘

strata> runs.list()
┌──────────────────────────────────────┬─────────┬─────────────────────┐
│ Run ID                               │ State   │ Created             │
├──────────────────────────────────────┼─────────┼─────────────────────┤
│ 550e8400-e29b-41d4-a716-446655440000 │ active  │ 2025-01-24 10:30:00 │
│ 6ba7b810-9dad-11d1-80b4-00c04fd430c8 │ closed  │ 2025-01-24 09:15:00 │
└──────────────────────────────────────┴─────────┴─────────────────────┘

strata> events.tail("agent-trace", limit=5)
[seq: 142] {"action": "search", "query": "weather in NYC"}
[seq: 143] {"action": "tool_call", "tool": "weather_api"}
[seq: 144] {"action": "response", "tokens": 127}

strata> .help
Commands:
  kv.set(key, value)              Set a key-value pair
  kv.get(key)                     Get a value
  kv.delete(key)                  Delete a key
  kv.list(prefix?)                List keys

  json.set(key, doc)              Set a JSON document
  json.get(key)                   Get a document
  json.query(key, path)           Query with JSONPath

  vectors.create(name, dim)       Create a collection
  vectors.upsert(coll, key, vec)  Upsert a vector
  vectors.search(coll, vec, k)    Search similar vectors

  events.append(stream, event)    Append an event
  events.read(stream, limit?)     Read events
  events.tail(stream, limit?)     Tail events (live)

  runs.create(metadata?)          Create a new run
  runs.list()                     List all runs
  runs.close(run_id)              Close a run

  .help                           Show this help
  .stats                          Database statistics
  .exit                           Exit the CLI

strata> .stats
Database Statistics:
  Path:        ./my-agent-db
  Size:        142.3 MB

  KV entries:  12,456
  JSON docs:   1,234
  Events:      89,012
  Vectors:     50,000 across 3 collections

  Active runs: 2
  Total runs:  23

  WAL size:    12.1 MB
  Last flush:  30 seconds ago

strata> .exit
Goodbye!
```

### Non-Interactive Mode (Scripting)
```bash
# Single commands
$ strata ./db -c 'kv.get("key")'
"value"

# Pipe commands
$ echo 'kv.set("key", "value")' | strata ./db

# JSON output for scripting
$ strata ./db --json -c 'vectors.search("emb", [0.1, ...], k=3)'
[{"key":"doc-1","score":0.95},{"key":"doc-2","score":0.89}]

# Batch operations
$ strata ./db < commands.txt
```

### Design Philosophy
- **Like Claude Code**: Rich, interactive experience - not a bare REPL
- **Syntax**: Python-like function calls (familiar to target audience)
- **Output**: Pretty tables for humans, JSON for scripts
- **Features**: Autocomplete, history, syntax highlighting

### Implementation
```
strata-cli/
├── Cargo.toml
├── src/
│   ├── main.rs           # Entry point
│   ├── repl.rs           # Interactive REPL
│   ├── parser.rs         # Command parser
│   ├── commands/         # Command implementations
│   │   ├── kv.rs
│   │   ├── json.rs
│   │   ├── vectors.rs
│   │   ├── events.rs
│   │   └── runs.rs
│   └── display.rs        # Pretty printing
└── tests/
```

**Tech Stack:**
- **ratatui** or **rustyline**: Interactive terminal
- **tabled**: Pretty table output
- **clap**: Argument parsing
- **syntect**: Syntax highlighting

### Status: Planned (Medium Priority)

---

## Surface 5: MCP Server

### Audience
- AI agents (Claude, GPT, etc.)
- AI-native applications
- Developers building agent tooling
- The future of computing

### What is MCP?
Model Context Protocol (MCP) is a standard for AI models to interact with external tools and data sources. Instead of the AI generating code that uses Strata, the AI directly calls Strata operations.

### Installation
The MCP server is bundled with the CLI:
```bash
brew install strata  # Includes strata-mcp
```

### Configuration (Claude Desktop)
```json
{
  "mcpServers": {
    "strata": {
      "command": "strata",
      "args": ["mcp", "--db", "./my-agent-memory"]
    }
  }
}
```

### MCP Tools Exposed
```yaml
tools:
  # Key-Value
  - name: strata_kv_set
    description: Store a value by key
    parameters:
      key: string
      value: any

  - name: strata_kv_get
    description: Retrieve a value by key
    parameters:
      key: string

  # Vector Search
  - name: strata_vectors_search
    description: Find similar vectors
    parameters:
      collection: string
      query: array[number]
      k: number

  - name: strata_vectors_upsert
    description: Store a vector with metadata
    parameters:
      collection: string
      key: string
      vector: array[number]
      metadata: object

  # Events
  - name: strata_events_append
    description: Append an event to a stream
    parameters:
      stream: string
      event: object

  - name: strata_events_read
    description: Read events from a stream
    parameters:
      stream: string
      limit: number

  # Runs (for conversation memory)
  - name: strata_runs_create
    description: Create a new run for isolated operations
    parameters:
      metadata: object

  - name: strata_runs_close
    description: Close a run
    parameters:
      run_id: string
```

### AI Agent Example
When Claude has Strata MCP configured, it can:

```
User: "Remember that my favorite color is blue"

Claude: [Thinking: I should store this preference]
        [Calls: strata_kv_set(key="user:preferences:color", value="blue")]

        I've noted that your favorite color is blue!

---

User: "What's my favorite color?"

Claude: [Thinking: I should check stored preferences]
        [Calls: strata_kv_get(key="user:preferences:color")]
        [Result: "blue"]

        Your favorite color is blue!

---

User: "Find documents similar to 'machine learning basics'"

Claude: [Thinking: I should search the knowledge base]
        [Calls: strata_vectors_search(
            collection="knowledge",
            query=[0.12, 0.34, ...],  # embedding of query
            k=5
        )]
        [Result: [{key: "doc-42", score: 0.94, metadata: {...}}, ...]]

        I found 5 relevant documents. The most relevant is "Introduction
        to ML" with a 94% similarity score...
```

### Architecture
```
Claude Desktop
    │
    │ spawns subprocess (stdio)
    │ JSON-RPC protocol
    ▼
┌─────────────────┐
│   strata mcp    │  (strata-cli with mcp subcommand)
│                 │
│  ┌───────────┐  │
│  │ MCP Layer │  │  JSON-RPC ↔ Strata calls
│  └───────────┘  │
│        │        │
│        ▼        │
│  ┌───────────┐  │
│  │  strata   │  │  Embedded database
│  └───────────┘  │
└─────────────────┘
        │
        ▼
  ./my-agent-db/
```

### Implementation
The MCP server is a subcommand of the CLI:
```
strata-cli/
├── src/
│   ├── main.rs
│   ├── mcp/
│   │   ├── mod.rs        # MCP server entry
│   │   ├── protocol.rs   # JSON-RPC handling
│   │   ├── tools.rs      # Tool definitions
│   │   └── handlers.rs   # Tool implementations
│   └── ...
```

**Tech Stack:**
- **stdio**: Standard input/output for communication
- **JSON-RPC 2.0**: MCP's protocol
- **serde_json**: JSON handling

### Status: Planned (High Priority - AI Native!)

---

## Implementation Priority

Based on Strata's mission of being the database for AI agents:

### Phase 1: AI-Native (Immediate)
1. **MCP Server** - Direct AI agent integration
2. **Python Bindings** - Where AI developers live

### Phase 2: Developer Experience
3. **CLI** - Debugging, exploration, scripting

### Phase 3: Full Stack
4. **Node.js Bindings** - Web and serverless developers

### Phase 4: Ecosystem
5. LangChain integration
6. LlamaIndex integration
7. Vercel AI SDK integration

---

## Package Naming

| Surface | Package Name | Registry |
|---------|--------------|----------|
| Rust | `strata` | crates.io |
| Python | `stratadb` | PyPI |
| Node.js | `stratadb` | npm |
| CLI | `strata` | Homebrew, apt, etc. |
| MCP | (bundled with CLI) | - |

Note: `strata` may be taken on PyPI/npm, so we use `stratadb`.

---

## API Consistency

All surfaces expose the same conceptual API:

```
db.kv.{set, get, delete, exists, list}
db.json.{set, get, delete, merge, query}
db.events.{append, read, tail, streams}
db.state.{set, get, cas, delete}
db.vectors.{createCollection, upsert, search, delete}
db.runs.{create, get, list, close, pause, resume}
```

Language idioms may vary:
- Rust: `db.kv.set("key", "value")?`
- Python: `db.kv.set("key", "value")`
- Node: `await db.kv.set("key", "value")`
- CLI: `kv.set("key", "value")`
- MCP: `strata_kv_set(key="key", value="value")`

But the operations, parameters, and semantics are identical.

---

## Success Criteria

Strata surfaces are successful when:

1. **Rust**: Production apps run reliably (✅ done)
2. **Python**: Can replace Chroma/Pinecone in LangChain apps
3. **Node.js**: Can replace Pinecone in Vercel AI apps
4. **CLI**: Developers prefer it over raw code for exploration
5. **MCP**: Claude can maintain persistent memory across sessions

---

## References

- [PyO3 User Guide](https://pyo3.rs/)
- [napi-rs Documentation](https://napi.rs/)
- [Model Context Protocol](https://modelcontextprotocol.io/)
- [SQLite Architecture](https://www.sqlite.org/arch.html)
- [LangChain Vector Stores](https://python.langchain.com/docs/modules/data_connection/vectorstores/)
