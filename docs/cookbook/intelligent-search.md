# Intelligent Search

This recipe shows how to use StrataDB's structured search interface to investigate incidents across primitives — combining time-range filtering, query expansion, and reranking to surface the most relevant data.

## Pattern

An agent writes data across multiple primitives during normal operation:
- **KV Store** holds configuration and results
- **Event Log** records actions and errors
- **JSON Store** holds structured reports

Later, a human or agent needs to investigate what happened during a specific time window. The structured search interface searches across all primitives at once, with optional LLM-powered expansion and reranking for better results.

## Setup: Populate Data

```
$ strata --db ./data
strata:default/default> kv put error:auth "authentication failed for user admin"
(version) 1
strata:default/default> kv put error:timeout "request timeout after 30s on /api/users"
(version) 1
strata:default/default> kv put config:retry_policy "exponential backoff with 3 retries"
(version) 1
strata:default/default> event append error '{"code":401,"message":"invalid credentials","endpoint":"/login"}'
(seq) 1
strata:default/default> event append error '{"code":504,"message":"gateway timeout","endpoint":"/api/users"}'
(seq) 2
strata:default/default> event append deploy '{"version":"2.3.1","status":"rolled_back","reason":"health check failures"}'
(seq) 3
strata:default/default> json set report:incident-42 $ '{"title":"Auth service outage","severity":"P1","root_cause":"expired TLS certificate","resolution":"certificate renewed"}'
(version) 1
```

## Basic Cross-Primitive Search

Search across all primitives with a single query:

```bash
strata --db ./data search "authentication error" --k 5
```

Output:

```
[kv] error:auth (score: 0.91, rank: 1)
  authentication failed for user admin
[event] seq:1 (score: 0.78, rank: 2)
  {"code":401,"message":"invalid credentials","endpoint":"/login"}
[json] report:incident-42 (score: 0.65, rank: 3)
  {"title":"Auth service outage",...}
```

## Time-Scoped Investigation

Narrow results to a specific incident window:

```bash
# What happened between Feb 7 and Feb 9?
strata --db ./data search "timeout error" \
  --time-start "2026-02-07T00:00:00Z" \
  --time-end "2026-02-09T23:59:59Z" \
  --k 10
```

Only data created within that time window is returned. This is useful when the database has months of history but you only care about a specific incident.

## Filtering by Primitive

Focus on a single data source:

```bash
# Only search events
strata --db ./data search "gateway timeout" --primitives event

# Only search KV and JSON
strata --db ./data search "outage" --primitives kv,json
```

## Search Modes

```bash
# Keyword-only (BM25) — fast, exact matching
strata --db ./data search "401 invalid credentials" --mode keyword

# Hybrid (BM25 + vector) — default, better for natural language
strata --db ./data search "why did authentication break" --mode hybrid
```

## Controlling Expansion and Reranking

When a model is configured, expansion and reranking are automatic. Override per-query:

```bash
# Disable expansion (useful when you want exact keyword matches)
strata --db ./data search "TLS certificate expired" --expand false

# Disable reranking (faster, uses raw RRF scores)
strata --db ./data search "deployment rollback" --rerank false

# Force both on
strata --db ./data search "what caused the outage" --expand true --rerank true

# Disable both (plain search, no LLM calls)
strata --db ./data search "error:auth" --expand false --rerank false
```

## Scripted Incident Investigation

Combine search with other primitives in a shell script:

```bash
#!/bin/bash
set -euo pipefail

DB="--db ./data"
WINDOW_START="2026-02-07T00:00:00Z"
WINDOW_END="2026-02-09T23:59:59Z"

echo "=== Errors in incident window ==="
strata $DB search "error failure timeout" \
  --time-start "$WINDOW_START" \
  --time-end "$WINDOW_END" \
  --primitives event,kv \
  --k 20

echo ""
echo "=== Deployments in incident window ==="
strata $DB search "deploy rollback release" \
  --time-start "$WINDOW_START" \
  --time-end "$WINDOW_END" \
  --primitives event \
  --k 10

echo ""
echo "=== Related incident reports ==="
strata $DB search "outage root cause" \
  --primitives json \
  --k 5
```

## Python: Structured Investigation

```python
from stratadb import Strata

db = Strata.open("./data")

# Search with time range
results = db.search(
    "authentication failure",
    k=10,
    time_range={
        "start": "2026-02-07T00:00:00Z",
        "end": "2026-02-09T23:59:59Z",
    },
)

for hit in results:
    print(f"[{hit['primitive']}] {hit['entity']} (score: {hit['score']:.2f})")
    if hit.get("snippet"):
        print(f"  {hit['snippet']}")

# Keyword-only search, no LLM calls
exact = db.search("401", mode="keyword", expand=False, rerank=False)

# Drill into a specific result
for hit in results:
    if hit["primitive"] == "kv":
        detail = db.kv_get(hit["entity"])
        print(f"Full value: {detail}")
    elif hit["primitive"] == "json":
        detail = db.json_get(hit["entity"], "$")
        print(f"Full document: {detail}")
```

## Node.js: Structured Investigation

```typescript
import { Strata } from '@stratadb/core';

const db = Strata.open('./data');

// Search with time range
const results = await db.search('authentication failure', {
  k: 10,
  timeRange: {
    start: '2026-02-07T00:00:00Z',
    end: '2026-02-09T23:59:59Z',
  },
});

for (const hit of results) {
  console.log(`[${hit.primitive}] ${hit.entity} (score: ${hit.score.toFixed(2)})`);
}

// Drill into results
for (const hit of results) {
  if (hit.primitive === 'kv') {
    const value = await db.kv.get(hit.entity);
    console.log(`Full value: ${JSON.stringify(value)}`);
  } else if (hit.primitive === 'json') {
    const doc = await db.json.get(hit.entity, '$');
    console.log(`Full document: ${JSON.stringify(doc)}`);
  }
}

await db.close();
```

## See Also

- [Search Guide](../guides/search.md) — full search interface reference
- [RAG with Vectors](rag-with-vectors.md) — embedding-based retrieval
- [Agent State Management](agent-state-management.md) — multi-primitive agent sessions
