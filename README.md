# Popinion

> **Popinion** (Public Opinion) — Don't guess the future. Rehearse it.

---

## ⚡ Overview

**Popinion** is an AI prediction engine. It builds a knowledge graph of a real
opinion landscape from crawled and uploaded data, compiles that graph into a
population of AI agents, and runs a multi-agent simulation to see how opinion
forms, shifts, and spreads. Test a policy, a message, or a strategy here first —
then act with the evidence in hand.

### Pipeline

```
Real Data  →  Knowledge Graph  →  Personas  →  Simulation  →  Report
 (crawl/       (SQLite, entity       (graph-      (in-process    (grounded
  upload)       extraction)         grounded)    agents)        analysis)
```

### Core capabilities

| Feature | Description |
|---------|-------------|
| **Crawlers** | Telegram / X / Facebook scraping via headless Chrome (CDP). |
| **Knowledge graph** | LLM entity + relationship extraction into an embedded graph store (SQLite), guided by a generated ontology. |
| **Graph-grounded personas** | `/prepare` compiles each entity into a persona from its *observed evidence* (summary + relationship facts, incl. stance edges) — with provenance, no fabricated attributes. |
| **Simulation** | In-process multi-agent engine; agents post/comment/react over rounds. Stance & sentiment are captured at action time. |
| **Reports & panels** | Report agent (with a `web_search` tool), panel chat, and surveys over the live agents. |
| **Honesty tests** | Seed-variance noise floor + persona-permutation ablation, so a claimed effect can be told apart from model noise. |

## 🛠️ Tech stack

- **Frontend**: Vue 3 + Vite (Bun)
- **Backend**: Rust (`apps/api`, Axum) — the `popinion` binary
- **Graph store**: embedded SQLite (no external database)
- **Per-sim store**: SQLite (stance/sentiment as first-class columns)
- **LLM**: any OpenAI-compatible endpoint — local (Ollama) for bulk work, a metered API for quality

## 🚀 Quick start

### Prerequisites

| Tool | Purpose | Check |
|------|---------|-------|
| **Rust** (stable) | Backend | `cargo --version` |
| **Bun** | Frontend | `bun -v` |
| **Ollama** *(optional)* | Free local LLM for bulk work | `ollama --version` |

No database to run — the knowledge graph is embedded (SQLite). Docker is only
needed for the optional X/Facebook crawlers.

### 1. Configure environment

```bash
cp .env.example .env          # then edit .env
```

The LLM is split into two slots (see `.env.example` for the full notes):

```env
# Bulk work (extraction, ontology, the simulation loop) — free local model
LLM_API_KEY=ollama            # any non-empty string; Ollama ignores it
LLM_BASE_URL=http://localhost:11434/v1
LLM_MODEL_NAME=qwen2.5:14b

# Quality path (report/persona synthesis) — metered OpenAI-standard API
# Set ALL THREE or it silently falls back to the local model above.
LLM_BOOST_API_KEY=your_openai_api_key_here
LLM_BOOST_BASE_URL=https://api.openai.com/v1
LLM_BOOST_MODEL_NAME=gpt-4o

# Optional: web search for the report agent (blank = disabled)
TAVILY_API_KEY=
```

For a local model: `ollama serve && ollama pull qwen2.5:14b`.

### 2. Run

```bash
# Backend (Rust) — serves the API on :5001
cd apps/api && cargo run

# Frontend (Vue) — dev server on :3000, proxies /api to :5001
cd apps/web && bun install && bun run dev
```

Or start both from the repo root:

```bash
bun run dev
```

**Service URLs** — Frontend `http://localhost:3000`, Backend API `http://localhost:5001`.

## 🔄 Workflow

1. **Crawl / upload** — gather real posts and documents.
2. **Build the graph** — entity & relationship extraction into the embedded graph store.
3. **Prepare** — compile eligible graph entities into grounded personas
   (`/api/simulation/prepare/preview` → `/prepare` → `/prepare/status`).
4. **Simulate** — run the agents over rounds; stance/sentiment recorded per action.
5. **Analyze** — generate a report, run panel chat / surveys, or interview any agent.
6. **Validate** — `/api/simulation/validate` compares runs to report the noise floor
   and whether personas actually move the outcome.

## 🧪 Development

```bash
cd apps/api
cargo test            # unit tests
cargo clippy          # lints
cargo run             # start the API
```

## 📄 Acknowledgments

Popinion's design draws on prior work:

| Project | Contribution |
|---------|--------------|
| **[OASIS](https://github.com/camel-ai/oasis)** | Multi-agent social-simulation concepts (by [CAMEL-AI](https://github.com/camel-ai)); the simulation is now a native Rust engine. |

## 📝 License

MIT — see [LICENSE](LICENSE).
