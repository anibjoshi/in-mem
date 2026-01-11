# in-mem Documentation

Complete documentation for the in-mem project.

## Quick Navigation

### 🚀 Getting Started
- **[Getting Started Guide](development/GETTING_STARTED.md)** - Start here if you're new to the project

### 📐 Architecture
- **[M1 Architecture](architecture/M1_ARCHITECTURE.md)** - Complete M1 architecture specification
- **[Architecture Diagrams](diagrams/m1-architecture.md)** - Visual architecture diagrams
- **[Original Spec](architecture/spec.md)** - Original project specification

### 💻 Development
- **[Getting Started](development/GETTING_STARTED.md)** - Quick start for new developers
- **[TDD Methodology](development/TDD_METHODOLOGY.md)** - Test-Driven Development approach
- **[Development Workflow](development/DEVELOPMENT_WORKFLOW.md)** - Git workflow for parallel development
- **[Claude Coordination](development/CLAUDE_COORDINATION.md)** - Multi-Claude coordination guide

### 📊 Project Management
- **[Milestones](milestones/MILESTONES.md)** - Project roadmap (M1-M5)
- **[Project Status](milestones/PROJECT_STATUS.md)** - Current status and progress
- **[Story Summaries](milestones/)** - Individual story completion summaries

## Documentation Structure

```
docs/
├── README.md                          # This file
├── architecture/                      # Architecture & design docs
│   ├── M1_ARCHITECTURE.md            # M1 complete specification
│   └── spec.md                       # Original project spec
├── development/                       # Development process docs
│   ├── GETTING_STARTED.md            # Quick start guide
│   ├── TDD_METHODOLOGY.md            # Testing strategy
│   ├── DEVELOPMENT_WORKFLOW.md       # Git workflow
│   └── CLAUDE_COORDINATION.md        # Multi-Claude coordination
├── diagrams/                          # Architecture diagrams
│   └── m1-architecture.md            # M1 visual diagrams
└── milestones/                        # Milestone tracking
    ├── MILESTONES.md                 # Roadmap (M1-M5)
    ├── PROJECT_STATUS.md             # Current status
    └── STORY_6_SUMMARY.md            # Story summaries
```

## Reading Order for New Developers

1. **[Getting Started](development/GETTING_STARTED.md)** - Setup and first story
2. **[M1 Architecture](architecture/M1_ARCHITECTURE.md)** - Understand the system
3. **[TDD Methodology](development/TDD_METHODOLOGY.md)** - Learn testing approach
4. **[Development Workflow](development/DEVELOPMENT_WORKFLOW.md)** - Git workflow
5. **[Claude Coordination](development/CLAUDE_COORDINATION.md)** - Multi-Claude work

## Key Concepts

### Milestones
- **M1: Foundation** - Storage, WAL, recovery (current)
- **M2: Transactions** - OCC with snapshot isolation
- **M3: Primitives** - KV, Events, StateMachine, Trace, RunIndex
- **M4: Durability** - Snapshots, production recovery
- **M5: Replay & Polish** - Deterministic replay, benchmarks

### Architecture Principles
1. Accept MVP limitations, design for evolution
2. Trait abstractions prevent ossification
3. Run-scoped operations everywhere
4. Conservative recovery (fail-safe)
5. Stateless primitives (facades over engine)

### Development Process
- **TDD**: Test-first development with phase-specific strategies
- **Git Flow**: Story → Epic → Develop → Main
- **Parallel Work**: 4 Claudes working simultaneously on different stories
- **Quality Gates**: Tests, formatting, linting on every PR

## Documentation Updates

When updating documentation:
1. Keep README.md links up to date
2. Maintain relative paths between docs
3. Update PROJECT_STATUS.md as stories complete
4. Add story summaries to milestones/ folder
5. Keep cross-references working

---

**Return to**: [Main README](../README.md)
