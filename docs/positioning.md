# Strata: Positioning

## The State of AI

LLMs are a powerful cortex. They can reason, plan, generate, and execute. But they're missing the cognitive structures that enable continuity — the hippocampus and prefrontal cortex that turn raw intelligence into something that learns, adapts, and compounds over time.

Every session is anterograde amnesia. The intelligence is real, but nothing persists, nothing is learned, nothing compounds. An AI without these structures can think, but it can't form new associations, can't navigate based on experience, can't experiment safely, can't build on yesterday's work.

This is the bottleneck. Not intelligence. Not tooling. Not compute. The missing piece is the cognitive infrastructure that completes the system.

## Every Paradigm Gets Its Own Database

Mainframes got hierarchical databases. Client-server got relational databases — Oracle, PostgreSQL. Web scale got NoSQL — MongoDB, Redis. Mobile got embedded databases — SQLite, Realm.

Each one was purpose-built for how that paradigm's primary consumer thought about and accessed data. Relational databases assumed a human developer writing SQL. NoSQL assumed a web developer who needed flexible schemas and horizontal scale. SQLite assumed a mobile app that needed a database in a single file with no server.

AI agents are the next paradigm's primary consumer. And they're still using databases built for the previous ones.

## The Trajectory

**Phase 1 (now):** Humans use AI to build traditional applications. AI writes SQL, designs schemas, configures ORMs — doing human developer work, including all the database ceremony.

**Phase 2 (next):** AI builds AI. An AI agent builds another agent that monitors metrics and acts on them. That second agent needs persistence — not for "remembering conversations" but for doing its job. Storing thresholds, tracking history, recording decisions, evolving its own rules.

In phase 2, who designs the schema? Who writes the migrations? Who configures the connection pool? There's no human developer in the loop for those decisions. The entire ceremony that exists because databases were built for human developers becomes an obstacle.

## Not "Memory"

The industry is crowded with products that stitch together a vector database and some glue and call it "agent memory." That framing reduces persistence to recall — "remember what the user said last time." It's building a clipboard and calling it cognition.

The hippocampus does far more than recall. It forms associations, provides temporal context, enables navigation based on experience. The prefrontal cortex enables planning, experimentation, and decision-making. Together they're what separate an organism that reacts from one that learns and plans.

The "memory" products give AI a notepad. That's not what's missing.

## What Strata Is

Strata is the hippocampus and prefrontal cortex of AI.

Not a database the agent connects to. A cognitive component that completes it.

**Hippocampus** — persistence, contextual association, temporal recall. Not just "what was stored" but "what is this related to" and "what did this look like before." Store structured state, search by meaning across everything, recall any key as it was at any past point in time. Every write is versioned. Nothing is ever lost.

**Prefrontal cortex** — working state, planning, safe experimentation, decision tracking. Fork the entire state before risky operations. Try things, evaluate, merge if it works, discard if it doesn't. Record decisions and actions as immutable events that form a permanent audit trail.

Together: an AI that doesn't just reason, but persists, associates, experiments, and learns.

## How It Works

One process, one data directory, zero configuration. An agent connects and immediately has a complete cognitive persistence layer through 8 intent-driven tools:

- **Store** structured JSON by key, with surgical nested updates via JSONPath
- **Recall** by key, with time-travel to any past version
- **Search** by natural language across everything stored
- **Forget** by key, with history preserved
- **Log** immutable events — actions, decisions, errors — that can never be rewritten
- **Branch** the entire state for safe experimentation
- **History** for every key — full version audit, time range discovery
- **Status** to orient at the start of any session

No schemas. No migrations. No query language. No infrastructure. The agent doesn't think about persistence mechanics. It just persists.

## What Makes It Different

**Versioning as the storage model.** Every write creates a new version. Time-travel is a natural consequence, not a bolt-on. This is what temporal context looks like as infrastructure — the ability to understand how state evolved, not just what it is now.

**Branching as a primitive.** Copy-on-write branching of the entire state as a first-class operation. Fork, experiment, merge or discard. This is executive function as infrastructure — the ability to plan, explore alternatives, and evaluate outcomes without risk.

**Search by meaning.** Hybrid keyword and semantic search across all stored data. When the agent doesn't remember the exact key — which is often — it describes what it's looking for. This is associative recall as infrastructure.

**Local inference.** Embedding and search happen in-process. No API calls to external services, no network round-trips. Cognition should not depend on someone else's infrastructure.

**Eight tools, not eighty.** Tool selection accuracy degrades sharply past 10 tools. Strata collapses its full capabilities into 8 tools that map to cognitive intents: store, recall, search, forget, log, branch, history, status.

## Where Strata Needs to Evolve

The primitives are in place — the bones and flesh. But the cognitive metaphor demands more. If Strata is truly the hippocampus and prefrontal cortex, it needs to close the gap between what it is today (a very good agent-friendly database) and what it should become (a cognitive component that handles persistence the way a colleague would — you throw something at it, say "remember this," and it handles the rest).

That means the agent shouldn't need to decide whether something is a document or an event. Shouldn't need to pick a key or construct a path. Shouldn't need to reason about when to branch. The intelligence of persistence — not just the mechanics — should live in Strata.

The interface between AI and its state layer should feel less like a database API and more like the interface between the cortex and the hippocampus: seamless, automatic, and invisible.

## The Narrative

Stateless agents do tasks. Stateful agents do work.

But the right state layer isn't a database with a friendlier API. It's not a vector store marketed as "memory." It's a cognitive component — the hippocampus and prefrontal cortex that LLMs are missing. Persistence, association, temporal context, safe experimentation, decision tracking.

AI has the cortex. Strata is the rest.
