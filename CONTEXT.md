# Popinion — Domain & Architecture Vocabulary

Source of truth for naming. When code and this file disagree, this file wins —
fix the code. Established during the 2026-07 architecture pass.

## The product

Popinion rehearses public opinion: crawl/upload real data → build a knowledge
graph (embedded SQLite) → compile graph entities into grounded **Personas** →
run an in-process multi-agent LLM simulation → produce a report. A "Digital
Mirror World" — real data fused with LLM-agent simulation.

## Domain terms (use these exact words; rename synonyms to them)

- **Persona** — a grounded individual *record* compiled from the knowledge
  graph: demographics, stance, evidence. Serialized to `profiles.json`; the
  thing the report's survey / interview / panel-chat polls.
  Replaces the synonyms: `AgentProfile`, `panelist`, `respondent`, and the
  persona-role sense of scraped users.
- **Agent** — the *live simulated actor* during a run: wraps a Persona plus
  runtime state (activity schedule, memory, feed). This is the ONLY thing
  called "agent". A Persona is data; an Agent is data + runtime.
- **Ontology** — the schema of entity types + relation types the graph is
  built against.
- **Entity type** — a node category in the ontology. Field: `entity_type`.
  Drop the Neo4j-ism "labels".
- **Relation type** — an edge category. Field: `relation_type`. Emit this one
  name — not the `name` / `fact_type` / `rel_type` / `type` drift.
- **Attributes** — a node's key/value properties. Drop "properties" and
  "attribute_schema" as synonyms.
- **Reality seed** — real crawled/uploaded data used to ground a simulation.
  Raw crawl records (`ScrapedUser`, `ScrapedPost`) are *inputs* that get
  compiled into Personas; keep them clearly "raw", never call them Personas.

## Architecture conventions

- **Response envelope** — one type, imported everywhere as `Success` (no
  `Ok` / `Payload` / `Envelope` aliases). The wire shape is ALWAYS
  `{ "success": true, "data": <payload> }` — payload nested under `data`, for
  objects and arrays alike. The frontend reads `res.data` uniformly.
- **Registry** — one generic in-memory registry
  (`OnceLock<Mutex<HashMap<..>>>`); graph-build tasks, reports, and surveys are
  typed instances of it, with one shared lifecycle-status vocabulary.
- **Manager owns the sim layout** — the on-disk layout of a simulation
  (`{id}/profiles.json`, `config.json`, `social.db`) is known ONLY to
  `sim::Manager`. No caller reconstructs those paths.
- **Handlers** — `verb_noun` names, no `_h` suffix. Routes are resource-style
  (`/simulation/:id/posts`), not RPC verbs in the path.
