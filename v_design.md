# Popinion V: Collective Intelligence Through Graph Fusion

> *"Don't guess the future. Rehearse it — standing on the shoulders of collective intelligence."*
> *"The goal isn't to be right. It's to become less wrong."*

---

## 📊 Feature Status Overview

| Feature | Status | Description |
|---------|--------|-------------|
| **Graph Building** | ✅ Implemented | Build knowledge graphs from documents |
| **Ontology Generation** | ✅ Implemented | LLM-generated entity/relation types |
| **Graph Verification** | ✅ Implemented (MVP) | Review modal with flag/gap capabilities |
| **Agent Generation** | ✅ Implemented | Create agents from graph entities |
| **Simulation Engine** | ✅ Implemented | Multi-round agent interactions |
| **Panel Chat** | ✅ Implemented | Query all agents simultaneously |
| **Agora Debates** | ✅ Implemented | Structured multi-agent debates |
| **Knowledge Workbench** | ✅ Implemented | Split-view: Captured Insights + Knowledge Gaps |
| **Knowledge Injection** | ✅ Implemented | Push knowledge into agent prompts |
| **Quick Survey** | ✅ Implemented (MVP) | Likert/Yes-No polling across agents |
| **Graph Editing** | 🔮 Future | Inline entity/relationship editing |
| **Graph Fusion** | 🔮 Future | Merge graphs from multiple users |
| **Global Graph Library** | 🔮 Future | Platform for shared worldviews |
| **Live Env Integration** | 🔮 Future | Use live simulation env for deep interactions |

---

## Document Structure

This document is organized to show how features build upon each other:

1. **Core Philosophy** — Why Popinion exists
2. **The Problem** — What we're solving
3. **The Workflow** — How users move through the system
4. **Feature Deep Dives** — Detailed feature documentation
5. **Future Vision** — Where we're headed

---

# Part 1: Philosophy & Problem

## The Core Philosophy

**"Become less wrong."**

No single perspective is complete. No simulation captures the full truth. Popinion V is built on four foundational principles:

| Principle | What It Means | How We Enable It |
|-----------|---------------|------------------|
| **Epistemic Humility** | Your worldview has blind spots | Graph Verification, Knowledge Gaps |
| **Adversarial Testing** | Ideas strengthen when they survive opposition | Agora Debates, Panel Chat |
| **Collective Intelligence** | Many perspectives > single perspective | Multi-agent simulation, Graph Fusion |
| **Iterative Refinement** | Every simulation is diagnostic | Knowledge Workbench, Injection loop |

---

## The Problem

Today, each Popinion user builds an **isolated graph**:
- Their own sources, their own entities, their own agents
- Blind spots they don't know exist
- Simulations limited by a single worldview
- No mechanism to challenge assumptions with opposing views

**The result**: Users confirm what they already believe instead of discovering what they're missing.

---

## How We Solve It

Popinion V attacks the problem from two directions:

| Approach | Scope | Features |
|----------|-------|----------|
| **Deep Interaction Tools** | Single-user | Graph Verification, Agora, Panel Chat, Knowledge Workbench |
| **Graph Fusion** | Multi-user | Four-Phase Model, Global Graph Library |

Both approaches serve the same philosophy: **become less wrong by exposing your ideas to opposition.**

---

# Part 2: The User Workflow

## The Complete User Journey

```
┌────────────────────────────────────────────────────────────────────────────────────────────┐
│                              POPINION V USER WORKFLOW                                       │
├────────────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                            │
│  STEP 1         STEP 1.5        STEP 2           STEP 3         STEP 4         STEP 5     │
│  ┌────────┐    ┌────────┐      ┌────────┐       ┌────────┐     ┌────────┐     ┌────────┐  │
│  │ GRAPH  │ → │ GRAPH  │  →   │ ENV    │   →   │  SIM   │  →  │REPORT  │  →  │ DEEP   │  │
│  │ BUILD  │    │ VERIFY │      │ SETUP  │       │  RUN   │     │        │     │ INTER  │  │
│  └────────┘    └────────┘      └────────┘       └────────┘     └────────┘     └────────┘  │
│       │             │               │                │              │              │       │
│   Upload       Review          Select           Watch          Read         Explore:      │
│   Documents    Graph          Agents          Simulation      Summary      - Panel Chat   │
│   + Generate   + Flag         + Set           + Observe       + Key        - Agora        │
│   Ontology     Issues         Rounds          Interactions    Insights     - Surveys      │
│                                                                                            │
└────────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## The Iterative Refinement Loop

> *"You often don't know what's missing until you see agents fail to account for it."*

Every simulation is not an endpoint — it's a **diagnostic opportunity**:

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                         THE ITERATIVE REFINEMENT LOOP                                  │
├────────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                        │
│     ┌─────────────┐      ┌─────────────┐      ┌─────────────┐      ┌─────────────┐    │
│     │   BUILD     │      │   RUN       │      │  DISCOVER   │      │   REFINE    │    │
│     │   GRAPH     │ ───→ │   SIM       │ ───→ │   GAPS      │ ───→ │   GRAPH     │    │
│     └─────────────┘      └─────────────┘      └─────────────┘      └─────────────┘    │
│            ↑                                                              │            │
│            └──────────────────────────────────────────────────────────────┘            │
│                                                                                        │
│                    Each cycle produces a LESS WRONG simulation                         │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

| Phase | What Happens | Tools Used |
|-------|--------------|------------|
| **BUILD** | Create knowledge graph from sources | Graph Builder, Ontology Generator |
| **VERIFY** | Check graph quality, flag issues | Graph Verification |
| **SIMULATE** | Run agents, observe interactions | Simulation Runner |
| **INTERACT** | Deep dive into agent perspectives | Panel Chat, Agora, Surveys |
| **CAPTURE** | Record insights from simulation | Knowledge Pad (left panel) |
| **IDENTIFY** | Note missing perspectives | Knowledge Gaps (right panel) |
| **INJECT** | Push knowledge into next simulation | Knowledge Injection |
| **REFINE** | Improve graph based on discoveries | Graph Verification (future: edit mode) |
| **REPEAT** | Run again with improved foundation | — |

---

# Part 3: Feature Deep Dives

## 🔍 Graph Verification (Step 1.5)

> *"Verify before you simulate. Fix before you fail."*

A quality gate between Graph Building and Environment Setup.

### MVP Implementation ✅

When user clicks "Enter Environment Setup", a verification modal appears:

| Section | Content |
|---------|---------|
| **Purpose (Can Do)** | Verify stakeholders, check relationships, catch errors, flag issues, create gaps |
| **Limitations (Cannot Do Yet)** | Edit entities, add/delete entities, modify relationships |
| **Quick Checks** | Entity types with counts, graph size with warnings |
| **Review Notes** | List of flagged items (if any) |
| **Actions** | Re-upload Documents, Review Graph, Looks Good |

#### Size Warnings

| Condition | Warning |
|-----------|---------|
| `< 10` entities | ⚠️ Low entity count — simulation may be shallow |
| `> 80` entities | ⚠️ High entity count — simulation may be expensive and slow |

#### Review Mode

When user clicks "Review Graph":
- Modal closes, review mode bar appears at bottom
- User can click entities to inspect details
- Action buttons appear in detail panel:
  - 🚩 **Flag Issue** — Mark entity with issue type + optional note
  - 📝 **Create Gap** — Create Knowledge Gap from this entity

#### Issue Types for Flagging

| Type | When To Use |
|------|-------------|
| `Wrong Name` | Entity name is incorrect or misspelled |
| `Wrong Type` | Entity is classified incorrectly |
| `Should Delete` | Garbage extraction (e.g., "Page 3") |
| `Missing Connection` | Entity should be connected to others |
| `Other` | Any other issue |

### Future Enhancements 🔮

Requires backend API endpoints:

| Endpoint | Method | Purpose | Difficulty |
|----------|--------|---------|------------|
| `/api/graph/entity/{uuid}` | `PATCH` | Edit entity | Medium |
| `/api/graph/entity/{uuid}` | `DELETE` | Remove entity | Medium |
| `/api/graph/entity` | `POST` | Create entity | High |
| `/api/graph/edge/{uuid}` | `PATCH` | Edit relationship | Medium |
| `/api/graph/edge/{uuid}` | `DELETE` | Remove relationship | Medium |
| `/api/graph/edge` | `POST` | Create relationship | High |
| `/api/graph/import-gaps` | `POST` | Convert gaps to entities | High |

---

## 💬 Panel Chat

> *"Ask one question, hear from everyone."*

Query multiple agents simultaneously and see aggregated perspectives.

### How It Works

1. User poses a question: *"What are your views on the new economic policy?"*
2. All selected agents (or all) respond in parallel
3. Responses are aggregated and displayed:
   - **By stance**: Supporters vs. critics vs. neutral
   - **By faction**: Government vs. civil society vs. business
   - **By theme**: Economic vs. political vs. social impacts

### Use Cases

| Use Case | Example |
|----------|---------|
| **Quick Pulse** | "What's your initial reaction to this proposal?" |
| **Consensus Check** | "Is there anything everyone agrees on?" |
| **Fault Line Discovery** | "Where do you see the biggest risks?" |
| **Pre-Debate Recon** | Identify most polarizing issues before Agora |

> [!IMPORTANT]
> **Current Implementation**: Uses profile-based LLM responses (agent profiles fed to LLM) due to Flask environment constraints. This works well but future enhancement will use live simulation environment for richer, memory-aware responses.

---

## 🏛️ Agora: Structured Debate Arena

> *"Where ideas clash and truth emerges."*

Not a free-for-all — a structured, moderated debate with clear format.

### Debate Formats

| Format | Agents | Description |
|--------|--------|-------------|
| **Point-Counterpoint** | 2 | Classic debate, alternating arguments |
| **Panel Discussion** | 3-5 | Moderator + panelists discussing topic |
| **Town Hall** | All | Open forum with rotating speakers |

### How It Works

1. **Setup**: Select format, topic, and participants
2. **Configure**: Set number of rounds, time limits
3. **Run**: Agents debate in structured rounds
4. **Monitor**: Real-time stance tracking, turn-by-turn transcript
5. **Summarize**: AI-generated summary of key points and tensions

### Unique Features

- **Live Streaming**: Watch agents respond in real-time
- **Stance Visualization**: Track how positions shift across rounds
- **Pivot Topics**: Inject new angles mid-debate to steer discussion
- **Knowledge Capture**: Highlight and save key arguments to Knowledge Pad

> [!IMPORTANT]
> **Current Implementation**: Uses profile-based LLM responses (agent profiles fed to LLM) due to Flask environment constraints. Future enhancement will integrate with live simulation environment for debates where agents remember prior simulation events.

---

## 📋 Knowledge Workbench (Split-View)

> *"Capture what was said. Note what was missing. Build smarter simulations."*

A unified interface for capturing insights and identifying knowledge gaps.

```
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                            KNOWLEDGE WORKBENCH                                          │
├───────────────────────────────────┬─────────────────────────────────────────────────────┤
│  📋 CAPTURED INSIGHTS (Left)     │  ❓ KNOWLEDGE GAPS (Right)                          │
│                                   │                                                     │
│  Highlights from agent responses  │  User-authored notes on missing perspectives       │
│  - Tagged by source agent         │  - Tagged by gap type                              │
│  - Searchable and filterable      │  - Target specific agent or global                 │
│                                   │                                                     │
├───────────────────────────────────┴─────────────────────────────────────────────────────┤
│  [ 🗑️ Delete Selected ]  [ 💉 Inject Selected ]  [ 📤 Export ]  [ 📥 Import ]         │
└─────────────────────────────────────────────────────────────────────────────────────────┘
```

### Left Panel: Captured Insights

| Feature | Description |
|---------|-------------|
| **Highlight Text** | Select text during interaction → Add to pad |
| **Tag by Agent** | Each highlight tracks its source |
| **Search & Filter** | Find insights by agent or keyword |

### Right Panel: Knowledge Gaps

| Feature | Description |
|---------|-------------|
| **Add Gap** | Author notes on what's missing |
| **Target Agent** | Apply to specific agent or all (global) |
| **Gap Tags** | missing_perspective, missing_stakeholder, counter_argument, etc. |

### Gap Tags

| Tag | When To Use |
|-----|-------------|
| `missing_perspective` | Agent didn't consider a viewpoint |
| `missing_stakeholder` | Key player not represented |
| `missing_relationship` | Connection not acknowledged |
| `counter_argument` | Opposing argument not addressed |
| `blind_spot` | Obvious gap in reasoning |
| `other` | Miscellaneous |

### The Complete Workflow

```
OBSERVE → CAPTURE → IDENTIFY GAPS → INJECT → SIMULATE AGAIN
```

| Step | Panel | Action |
|------|-------|--------|
| 1. Run simulation | — | Agents interact |
| 2. Observe patterns | Left | Capture quotes/insights |
| 3. Notice missing | Right | Author gap notes |
| 4. Select items | Both | Check boxes |
| 5. Inject | Footer | Push into simulation |
| 6. Run again | — | Agents now have this context |

### Gap Injection Approach

**MVP (Current)**: Verbatim Reminder
- Gap text injected exactly as written into agent's prompt context
- User maintains full control over injected content
- No additional LLM calls

**Future Enhancement**: LLM Research & Fill
- System researches gap and generates substantive content
- Better for gaps user can't fill themselves
- Requires additional LLM calls

---

## 📊 Quick Survey

> *"Quantify what agents believe."*

Structured polling across the agent population.

### MVP Implementation ✅

| Feature | Status |
|---------|--------|
| **Single Question Input** | ✅ Implemented |
| **Likert Scale** | ✅ Strongly Agree → Strongly Disagree (5 options) |
| **Yes/No/Neutral** | ✅ Agree, Disagree, Neutral (3 options) |
| **Results Bar Chart** | ✅ Percentage distribution |
| **Faction Breakdown** | ✅ Responses grouped by agent type |

### How It Works

1. Navigate to Step 5 → **Quick Survey** tab
2. Enter survey question
3. Select response type (Likert or Yes/No)
4. Click **Run Survey**
5. View aggregated results with faction breakdown

### Example Use Case

> *"Survey all agents: 'Would you support a 10% increase in minimum wage?'"*
> Results: 60% support (labor groups), 25% oppose (business), 15% neutral

### Future Enhancements 🔮

| Feature | Description |
|---------|-------------|
| **Multi-Question Surveys** | Create surveys with multiple questions |
| **Save Templates** | Reuse surveys across simulations |
| **Before/After Comparison** | Measure stance shift pre/post intervention |
| **Ranking Questions** | Prioritize options instead of agree/disagree |
| **Export Results** | Download as CSV/JSON for external analysis |

---

# Part 4: Future Vision

## Graph Fusion: The Four-Phase Model

> *"What if you could overlay your graph with others?"*

Beyond single-user simulations — merge knowledge graphs for richer perspectives.

### Phase 1: Enrichment (Additive Mode)

User A has **Economic Policy** graph. User B has **Labor Demographics** graph.

When merged:
- New entities appear (labor unions, migrant workers)
- New relationships emerge (policy affects labor mobility)
- Simulation gains agents that wouldn't have existed

**Key Feature**: Detect conflicting claims before merging. Present "perspective tensions" that need resolution.

### Phase 2: Convergence (Readiness Check)

Before adversarial mode, detect overlap:

| Metric | Description |
|--------|-------------|
| **Entity Overlap** | Shared entities between graphs |
| **Topic Similarity** | Semantic theme alignment |
| **Stance Divergence** | Opposing views on shared topics |

### Phase 3: Adversarial (A vs. B Mode)

Both graphs clash:
- **Policy Testing**: A's agents vs. B's opposition agents
- **Narrative Clash**: A's media narrative vs. B's counter-narrative
- **Faction Simulation**: Government (A) vs. Civil Society (B)

Output: Predictions with **tension built-in**.

### Phase 4: Synthesis (New Understanding)

After adversarial testing:
- Identify where User A's model broke
- Highlight blind spots exposed by User B
- Suggest graph refinements
- Option: Create merged "synthesis graph"

---

## Global Graph Library

> *"A platform for collective intelligence."*

| Analogy | Popinion Equivalent |
|---------|---------------------|
| **GitHub** | But for knowledge graphs |
| **Hugging Face** | But for simulations |
| **npm/pip** | But for worldviews |

### How It Works

1. **Publish**: Users publish graphs with metadata (topics, keywords, domain)
2. **Discover**: Search the library using natural language or tags
3. **Connect**: Request to merge graphs, negotiate overlap
4. **Enrich**: Run simulations with combined worldviews

### Discovery Mechanism

| Search Type | Example |
|-------------|---------|
| **Topical** | "Find graphs about energy policy" |
| **Adversarial** | "Find graphs that oppose my position on X" |
| **Complementary** | "Find graphs about topics my graph doesn't cover" |

---

## Open Design Questions

### For Graph Fusion

| Question | Considerations |
|----------|----------------|
| **Merge ownership** | Shared graph? Fork? Enriched view only? |
| **Visibility during merge** | Full transparency? Summary only? Blind merge? |
| **Conflict resolution** | Manual? AI-suggested? User decides? |

### For Global Library

| Question | Considerations |
|----------|----------------|
| **Quality control** | Reviews? Usage stats? Verified contributors? |
| **Versioning** | What happens when source graph updates? |
| **Privacy** | Which graphs are public vs. private? |

---

# Part 5: Alignment with Philosophy

Every feature connects back to the core principles:

| Principle | How Features Enable It |
|-----------|------------------------|
| **Epistemic Humility** | Graph Verification assumes first graph is incomplete; Knowledge Gaps captures what's missing |
| **Adversarial Testing** | Agora debates force ideas to survive opposition; Phase 3 clashes worldviews |
| **Collective Intelligence** | Split-view workbench combines agent insights + user expertise; Graph Library pools knowledge |
| **Iterative Refinement** | Every simulation is diagnostic; gaps feed next run; loop continues until less wrong |

---

## Conclusion

Every feature, every phase, every tool serves the same goal:

> **"The goal isn't to be right. It's to become less wrong."**
