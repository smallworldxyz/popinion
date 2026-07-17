# Popinion Redesign Plan: The Field

Status: proposed, 2026-07-17.
Direction chosen: FIELD.
Organizing decision: the World is the primary noun, Runs are disposable, and Worlds accumulate and are reused.

This plan is the deliverable.
It does not write application code.
It is the thinking a team executes from.

---

## 1. Why we are doing this

The current app rebuilds a throwaway world for every question.
That is backwards.
A Digital Mirror World of a society is the expensive, durable asset.
A policy question fired at it is cheap and disposable.

The client stated the real goal directly.
Simulations must not live in isolation.
They must be saved for review, reused as a base for related questions, and built upon so prior work is not thrown away.

Four complaints about today's UI frame the design constraints.
Too plain and undesigned.
Too wizard-y and linear.
Not cinematic enough.
Buries the actual value (the graph, stance edges, provenance, and honesty tests are hidden behind panels and JSON).

---

## 2. The core idea

The interface is a synoptic chart of a population, a weather map.

A **World** is the **basemap**: corpus, knowledge graph, Personas, and the evidence bar that produced them, projected into a fixed 2D field where position is semantic.
It is durable.
It accumulates material over time.
You build on it.

A **Run** is a **weather system** fired across the basemap: one event and seed that answers one policy question.
It is cheap, there are many, and each is filed in the archive.

The key inversion: in a force-directed cloud everything wiggles and nothing means anything.
Here the land is fixed and the weather moves over it.
Every Persona is one mark at a fixed coordinate.
That coordinate never moves during a run.
What changes per round is each mark's stance colour, so the map recolours like a front sweeping through.

This choice pays a dividend the one-shot framing could never pay.
Because the basemap is owned by the World and not the Run, every Run against a World is drawn on the same coordinate system, so two Runs are comparable by construction, not by trusting two independent projections to line up.
Reuse is not a filing convenience here.
Reuse is what makes runs comparable and what lets a growing corpus visibly tighten the world.

---

## 3. Art direction

### 3.1 Theme

Scene: a duty forecaster at 04:10 in a national situation room, lit only by the wall-sized map she is reading, watching a pressure system deform across a coastline while the room behind her stays unlit so nothing competes with the map.

This forces **dark**.
Not "AI dark", forecast-desk dark: the room is dark because the map is the only light source, and the map itself is a paper-dense field of marks, not a void with glowing dots.
The dark is around the data.
The data is light-on-dark terrain, ink-dense.

### 3.2 Color

Strategy: **Committed**.
Brand hue is a slate-indigo, `oklch(0.62 0.13 265)`.
All neutrals are tinted toward 265.

```
Deep ground (app frame, room)   oklch(0.19 0.021 265)
Map ground (field substrate)    oklch(0.245 0.026 265)
Raised ground (rails, sheets)   oklch(0.29 0.028 265)
Hairline / graticule            oklch(0.38 0.022 265)
Body text                       oklch(0.88 0.012 265)
Quiet text                      oklch(0.66 0.018 265)
Ink (max, headline)             oklch(0.96 0.008 265)
Alert (World event injection)   oklch(0.72 0.19 42)   ember
Refusal / below-bar             oklch(0.55 0.03 265)  desaturated, not red
```

Stance is the key encoding, and it is a diverging luminance-plus-hue ramp on the blue to amber axis, never red to green.

```
Oppose         oklch(0.60 0.15 250)  ->  strong oppose  oklch(0.48 0.17 255)
Neutral        oklch(0.74 0.02 265)  near-achromatic, deliberately the lightest point
Support        oklch(0.66 0.14 78)   ->  strong support oklch(0.55 0.16 66)
```

Blue and amber is the tritan-safe axis, so it survives deuteranopia and protanopia, and it stays separable in tritanopia by luminance (oppose is dark, neutral is bright, support is mid).
Three redundant channels carry stance: hue, lightness, and mark shape (oppose is a downward chevron, neutral a flat bar, support an upward chevron).
Read the map in greyscale and it still works: neutrals glow, poles darken.
Sentiment, which is separate from stance, is encoded as mark opacity and agitation, not a second hue.

### 3.3 Typography

Offline, self-hosted woff2, no CDN, because the app ships as an offline-capable Tauri desktop build.

- **Inter Tight** at 600 to 700, tight tracking, for display and map labels. Forecast maps label with a condensed grotesque, never a serif.
- **Inter** 400 and 500 for body and UI, set at 65 to 72ch in prose surfaces.
- **IBM Plex Mono** 400 and 600 for every number, provenance ID, tick label, and count.

Mono is a truth signal.
If it is monospace, it was measured from real data.
If it is Inter, the product wrote it.

Scale, ratio 1.333: 11 / 13 / 15 / 20 / 27 / 36 / 48.
Weight contrast carries hierarchy (600 against 400), not size inflation.

### 3.4 What replaces the wizard and the cards

The five-step rail becomes a persistent **Field** with a **Time Ribbon** at the bottom.
Prep stages are not steps, they are states of the field itself.
No seed is an empty basemap with only a graticule.
A built graph with no eligible Personas is a basemap of hollow marks and an honest count.
A run in progress is marks colouring in.
The user never advances a wizard, they fill in a map, and the map's emptiness is the to-do list.

The cards become **sheets** (right-side, full-height, flush, no rounding, no float) and **rails**.
On top of the marks sit three derived layers, each toggleable.
Isopleths are contour lines of stance density, drawn like isobars, so you literally see a support ridge over a coastal district.
Flow is Minard-style tapered ribbons showing which Persona's post moved which others, sparse and directional, not a hairball.
Fracture is a hard, jagged rule drawn where the stance gradient exceeds a threshold, a geological fault where the population splits.

The black SYSTEM DASHBOARD log becomes a thin ticker along the ribbon: actions stream past as one-line mono entries, and each one flashes its author's mark on the map.
Log and map are the same object.

### 3.5 Honesty is drawn at true scale

The map is drawn at true population scale.
Three eligible Personas is three marks on a graticule, and the empty basemap is shown, not cropped away.
It looks sparse because it is sparse.
Below-bar entities render as hollow outlined marks in a margin gutter labelled REFUSED: visible, never coloured, never counted, never contoured.
You can always see what the world refused to invent.

### 3.6 Motion signature

The recolour sweep.
Advancing a round does not redraw the map, it propagates: marks flip stance in the order the actions actually occurred, over 700ms, ease-out-quint, spatially staggered outward from the originating Persona.
You see influence travel.
Only colour and a canvas transform animate, never layout.

Scrub is the primary verb.
Dragging the Time Ribbon plays the field forward and backward at frame rate.
The ribbon is a video head and the map is the video.
This is what kills the wizard: the product's core gesture is scrubbing time, not clicking next.

Evidence on hover, commit on click.
Hovering a mark ghosts its provenance into the always-mounted right sheet with no layout shift.
Clicking pins it and draws its flow ribbons.
No modals anywhere.

---

## 4. Information architecture

### 4.1 The primary noun is the World

A World is `graph_id`: the corpus, the ontology, the graph, the Persona roster, the evidence bar, and now the basemap projection.
A Run is one `simulation_id`: an event and a seed fired against that World.

The basemap is owned by the World, not the Run.
All Runs against a World share one projection, one coordinate system, one graticule.
That is the whole reason two Runs are comparable: they are drawn on the same land.
EXTEND redraws the land, rarely and deliberately, with a diff.
Runs never touch it.

Defence of the noun.
The World is what is expensive and what is reused: crawl, ontology, graph build, and Persona compile all happen once.
`POST /:id/duplicate` already copies Personas and the seed and changes only the event, so the World is literally the invariant in the one iteration operation the backend provides.
Organising around the disposable Run and hiding the durable World is exactly today's bug.

### 4.2 Two surfaces, one field, different altitude

**FIELD (analyst).**
The map at full detail.
Left rail (narrow) is a provenance ladder, Sources to Ontology to Graph to Personas, that you can enter at any rung, and clicking a rung changes what the map shows rather than navigating away.
Right sheet is the Evidence Sheet for the selected Persona: name, demographics, stance, evidence_score, fact_count, stance_facts, summary, provenance links back to the source, and the interview surface (talk to this agent under its evidence).
Bottom is the Time Ribbon and transport, with the World-agent event injection track pinned to round positions like weather-front annotations.

**BRIEF (leader).**
The same map, generalized.
Marks aggregate into isopleths, individual marks suppress, one verdict line at 36px, three fronts narrated in prose at 65ch, the fracture drawn, and the honesty result stated flatly ("effect exceeds noise floor" or "does not").
Panel chat and surveys live here as consultations, not tabs.

Relation.
BRIEF is FIELD zoomed out with a semantic level of detail, not a different app.
One control toggles altitude, and the transition is a continuous generalization: marks dissolve into contours.
Anything in BRIEF is one click from its evidence in FIELD.
A verdict you cannot drill into is a verdict you should not trust.

At roughly 400px, rails collapse, the field becomes a strip map (one stance-over-time band per cohort, stacked), and the Time Ribbon becomes the primary control.
The map does not shrink to illegibility, it changes projection.

### 4.3 Confidence without lying

The leader surface never shows a percentage.
It shows three states.

- **Signal**: `tv_distance > threshold` (1.5x the noise floor).
- **Noise**: at or below the floor, with copy "this shift is indistinguishable from seed noise".
- **Unmeasured**: no replicates exist, so `/compare` fell back to the 0.05 absolute bar, with one click to measure it and no imputed floor.

Every verdict carries its denominators: grounded Personas of total entities, the `min_evidence` used, the floor, and the replicate count.

---

## 5. The reuse model (the heart of this plan)

Three levels, all in scope, staged.

### 5.1 GROUP

Every Run is saved, listed, and reopenable under the World it came from.

Today `GET /api/simulation/list` returns `SimMeta` records that already carry `graph_id`, and `listSimulations()` is exported in the frontend but called from nowhere.
So every finished Run is unreachable the moment you leave its URL, and `TrustChecksPanel` and `RehearsalPanel` spawn real Runs that cost real tokens and become orphans.
Grouping `/list` by `graph_id` makes the World appear for free.

### 5.2 EXTEND

A related topic merges its corpus into an existing World.

The backend already merges by entity name.
Node identity is `PRIMARY KEY (graph_id, name)`, and `upsert_entity` unions entity types and extends attributes on collision rather than overwriting.
Edges merge on `(graph_id, source_name, target_name, relation_type)`.
The only blocker is that `spawn_build` always calls `create_graph` and mints a new `graph_id`.
Let a build target an existing `graph_id` and the merge machinery does the rest.

Extending is a geological event on the basemap, shown as a before and after with a diff, never a silent redraw.

1. The user drops a new corpus onto a World, and the field does not change yet, it enters EXTEND REVIEW, a staged overlay.
2. New entities that fuse to existing marks light up at their existing coordinate with an ingest pulse, the mark thickens (denser glyph, higher evidence_score, a second provenance batch attaches), drawn as two ink strokes settling into one heavier mark.
3. New entities with no match land as new marks, which shifts the projection, and this is the only time marks move: a slow tectonic settle (existing marks drift to accommodate, once, at most 800ms) with the new marks fading in after they land.
4. A DENSITY DIFF reads out in mono: `+18 entities · 12 fused · 6 new · evidence +0.09 avg · 4 personas crossed the bar`.

That last clause is the reuse payoff, and it is grafted from the rejected NOISE FLOOR direction on purpose because it is skin-independent.
More evidence tightens the world's error bar.
Extending narrows the noise floor and promotes previously refused entities into eligible Personas, drawn as an admittance mark (hollow filling in), so you watch the World gain citizens from new evidence.
A World whose floor dropped below a past Run's effect gets a quiet flag on that Run: re-runnable, the floor now sees this.

Two-batch provenance on one mark.
The Evidence Sheet's provenance section becomes stratified: seed batches are horizontal strata, newest on top, each labelled with its source and date, each fact tagged to its batch.
A fused mark shows `seed A: tg/fuel-grp · 3 posts` over `seed B: mpwt-memo.pdf · p.4`, and carries a tiny batch-count pip so multi-sourced Personas are spottable on the map, because they are the well-attested ones.

### 5.3 FORK

A new Run or World descends from a finished one and diverges.

`POST /:id/duplicate` exists and changes only the event.
It needs to carry lineage.

Ancestry is drawn as a geological cross-section, not a git graph, because a git graph is edges-and-dots (the force-cloud cliche in a different costume) and it does not carry how much diverged.
Time reads downward.
Each World or Run is a layer.
A fork is a layer that splits, and the two halves drift apart by the amount they actually diverged, measured as stance-field distance.
Divergence is distance on the page: two runs that ended near-identical sit as almost-overlapping strata, a fork that fractured the population splits wide.
The base World at the bottom is bedrock, EXTENDs thicken a layer, FORKs branch it.
Each stratum is labelled in mono with its delta from its parent, and the parent shows as a dashed ghost stratum behind the child, so "this builds on that" is literally "this solid layer sits on that ghost layer".
Selecting any stratum loads that World or Run into FIELD, and bedrock is always one click away, so a Run can never orphan itself from the reality it descends from.

### 5.4 The Library

The front door is an atlas contact sheet, not a card grid.
Each World is a live thumbnail of its own field, rendered small from the same WebGL layer at low detail, tinted to its resting stance, so you recognize a World the way a forecaster recognizes a region, by its shape.

Layout is a left World rail plus a main strata band.
Worlds are listed by mass (entity count, evidence density), not by date, because the organizing truth is how much reality is behind this, so a thin World with 3 Personas renders physically smaller than a World with 340.
The archive shows you weight.
Under each World, its Runs are a horizontal ribbon of ribbons, each Run a compressed stance-histogram strip, so a World's history reads as a stack of weather events across one coastline.
Reopening a Run is clicking its strip.
Firing a new Run is the plus at the end of the ribbon.

```
┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│ ATLAS                                                    12 worlds · 47 runs · 1,204 personas    │
├────────────────────┬─────────────────────────────────────────────────────────────────────────┬─┤
│ WORLDS   by mass   │  FUEL-LEVY & TRANSPORT                          340 personas · ev 0.71   │ │
│                    │  4 seeds · 61 entities · urban-rural fault          extended 2d ago  [◱] │ │
│ ▓ fuel-levy   340  │    ╭─────────────────╮                                                   │ │
│ ▓ border-talks 210 │   ╭╯ ⌄⌄⌄⌄⌄ ══ ^^^^^^ ╰╮      RUNS  ▸ fire new [+]                        │ │
│ ▒ media-trust 156  │   │⌄⌄⌄⌄⌄⌄⌄ ══ ^^^^^^^^ │                                                 │ │
│ ▒ flood-relief 98  │   ╰╮⌄⌄⌄ ══ ^^^^^^^^^^╭╯   r01 ░▒▓▓▓▒░  base run        Δ—      30rd      │ │
│ ░ vaccine-dis  61  │    ╰──────────────────╯   r02 ░▒▓███▓▒  +subsidy msg   Δ0.18  ✓real     │ │
│ ░ fx-policy    44  │                           r03 ▒▓█████▓  ⑃ fork of r02  Δ0.31  fracture  │ │
│ · land-title   28  │                           r04 ░▒▒▓▓▒░░  counter-camp   Δ0.09  ~noise    │ │
│ · ferry-fares  19  │                                                                          │ │
│ · e-gov-id      9  │  ── LINEAGE ─────────────────────────────────────────────────────────── │ │
│                    │   fuel-levy·seedA ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓  bedrock                               │ │
│ + new world       │        └ +seedB   ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓  extended  +18e +0.09ev  floor↓  │ │
│                    │              r02  ·······╌╌╌╌╌╌╌╌╌╮ ghost parent                          │ │
│ ── recent runs ──  │               └⑃ r03      ▓▓▓▓▓▓▓▓█████████  Δ0.31 diverged wide         │ │
│ media-trust r07 ▸  │                                                                          │ │
│ border r12 ⑃ ▸     │  ⚠ 2 suspected duplicates unresolved in this world      [ RESOLVE ▸ ]   │ │
├────────────────────┴─────────────────────────────────────────────────────────────────────────┴─┤
│  drop a corpus on any world to EXTEND it · drop here to seed a NEW world          ⇩             │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

Everything is the FIELD object at zoom-out, no cards, no second visual language for home.

---

## 6. Honesty: suspected duplicates and RESOLVE

Exact-name merge is shallow.
`MPWT`, `the Ministry`, and `Ministry of Public Works` fragment into three entities, or two genuinely distinct entities collide on a common name.
Silent fragmentation is the exact dishonesty this direction exists to prevent, so it gets a first-class surface, not a settings toggle.

A persistent "suspected duplicates" affordance sits on every World, visible in the Library and in the Sources rung of FIELD.
It never blocks, it accrues, and the count is honest debt.

Candidates are the near-misses the backend did not merge: marks that are close in the basemap projection and high on string or embedding similarity, computed from the embedding Persona-prep already produces.
They are candidates, never auto-fused.

RESOLVE is a reconciliation sheet.

```
   RESOLVE DUPLICATES · fuel-levy                                     2 candidates
   ─────────────────────────────────────────────────────────────────────────────
   ◐ MPWT                    proximity 0.94 · name-sim 0.41       confidence  MED
     ↔ the Ministry          both oppose · 6 shared stance-facts
     seed A: mpwt-memo.pdf ·11 facts        [ merge → ]  [ keep separate ]  [ ? ]
     seed B: tg/fuel-grp    · 4 facts        preview: one mark, ev 0.71→0.83
   ─────────────────────────────────────────────────────────────────────────────
   ○ ferry op. / Ferry Ltd   proximity 0.55 · name-sim 0.30       confidence  LOW
     ↔ different stance (oppose vs neutral)  [ keep separate ]  [ merge → ]  [ ? ]
```

Rules that keep it honest.
Confidence is a filled-dot glyph (full is high, half is medium, empty is low), never a red or green traffic light, so it rides the same neutral-luminance logic as stance and survives colourblindness.
It combines projection proximity, name similarity, and stance agreement.
Merging is the user's committed act, previewed before commit (shows the resulting single mark and its thickened evidence_score), and reversible (a merge is recorded as a lineage event so it can be split back).
Divergent stance lowers confidence and is called out, because two marks that disagree are probably genuinely two people, and fusing them would launder a real fracture into a fake consensus.
The safe default is keep separate: openly fragmented beats silently wrong.

Note: all three art-direction crews independently arrived at the stance-conflict rule.
That convergence is strong evidence it is a law of this product, not a preference.

---

## 7. The degenerate states (common, not edge cases)

These happen constantly and must be designed, not treated as errors.

0 edges extracted.
Today a graph that built and produced nothing renders as a blank canvas with a legend.
New state: "Graph built. 9 entities, 0 relations." with two actions, Rebuild (extraction is nondeterministic, three identical runs produced 2, 3, and 0 stance edges) and Review ontology.
Personas are still possible because `evidence_score` counts facts and summary, not edges, but every Persona will have no faction, so say so.

0 eligible Personas.
Today the button stays enabled, the task fails inside a spawned task with "no entities matched (min_evidence=2); loosen selection or lower the bar", and that message is only visible via `POST /prepare/status`, never as an HTTP status.
New state: block the action before it is taken, show an `evidence_score` histogram with the bar drawn on it, and offer three exits with their costs stated, including that `selected_entity_ids` bypasses the bar entirely.

Thin corpus (12 posts, 9 entities, 3 eligible Personas).
Allowed, never hidden, and this is where FIELD earns or loses.
Below roughly 30 Personas, do not project a semantic basemap, fall back to a deterministic stratified lattice (grouped by strongest demographic facet, ordered by stance), which is honest, readable, and still recolours over time.
The isopleth layer auto-disables below 30 marks, because a contour over 8 points is a fabrication.
The World header reads "Thin: 3 grounded Personas from 9 entities", and every Run born of this World inherits a thin-corpus flag.

A run still going.
`GET /:id/run-status` is the poll, but the status vocabulary is broken: there is no `completed` for simulations, a finished run reads `alive` while resident and reverts to `prepared` once dropped, which is indistinguishable from never-run.
This must be fixed in the API (add `completed`) or the runs table will misreport.

Model slow or unauthorized.
Move the readiness gate from the front door to per-action on the World bar, because `/classify-stance` runs on the `llm_boost` slot and can fail while bulk works.

---

## 8. Better functions unlocked

Ranked by value to effort.
Each is grounded in an endpoint or field that already exists, unless flagged as new work.

1. Run list and World grouping. `GET /api/simulation/list` grouped by `graph_id`. Zero backend. Makes every past Run reachable for the first time.
2. Compare any two Runs. `GET /compare?a&b&runs` is fully built and returns `tv_distance`, `noise_floor`, `threshold`, `significant`. Because runs share one basemap, the comparison is honest by construction. Needs a run picker.
3. Stance on the map. Colour marks pro and con on the diverging axis, size by `evidence_score`, hollow the ineligible. `sim/persona.rs` already classifies edges and `/prepare/preview` returns `stance_facts` and `eligible` per entity. Zero backend. This is what makes it read as a world.
4. The evidence bar as a live control. A `min_evidence` slider over the synchronous `POST /prepare/preview` that repaints the map, so you watch Personas fall below the bar. `min_evidence` appears zero times in the frontend today.
5. Fracture forecasting and counterfactual replay. Because stance is a fixed-position field over time, compute the stance gradient per round to detect where a fault opens and at which round, then fork the run at that round, inject a counter-message, and render both futures as two contour sets on the same basemap (original dashed ghost, counterfactual solid). The delta is drawn as directed flow and is checked against the noise floor before it is allowed to render solid. This is the diplomatic save game and the cognitive-vaccination workflow made literal, and it is the direction's signature new capability.
6. Extend narrows the floor. The reuse payoff in section 5.2, made a first-class animation.
7. Self-report versus independent audit. `POST /:id/classify-stance` re-reads posts on `llm_boost` and returns `agreement_rate` and a confusion matrix. Promote from a button to a tile: "agents claim X, an independent model reading their words says Y".
8. Spread as replay. `GET /:id/spread` and `GET /:id/timeline` combined into the Time Ribbon scrubber with the map overlay, with the endpoint's own honest caveat printed (exposure follows network position, not random assignment).

Honest about new backend work.
No endpoint orchestrates a replicate set, no endpoint measures extraction variance, `SimMeta.kind` and `parent_id` do not exist, the semantic embedding for the basemap must be computed at Persona-prep, and there is no SSE or WebSocket, so every live view is HTTP polling and "cinematic" must be built as replay over pre-fetched `/timeline` and `/spread`, not a live wire.

---

## 9. Phased build plan

Each phase ships something usable on its own.

### Phase 1: the backbone (chosen as first)

Goal: kill "simulations live in isolation" with the object model, minimal new skin.

Backend.
Add `SimMeta.kind` (`canonical | replicate | ablation | alt`) and `parent_id`, backfill `kind = canonical`.
Add `completed` to the simulation status vocabulary.
A Run must resolve to a World via `graph_id`.

Frontend.
New routes `/world/:graphId` and `/world/:graphId/run/:simId`.
Group `/list` by `graph_id`.
Wire the unused `listSimulations()`.
A World shows its Runs in a table (event, kind, status, Personas, stance-share, created_at).
The report becomes a tab on a Run, not a route.

Ships: every past Run reachable, grouped under its World.
This alone answers the client's stated problem.

Riskiest item: `SimMeta.kind`.
Grouping by `graph_id` today mixes canonical Runs with the replicate and ablation orphans the trust panels spawn, so land the schema field and backfill before any UI work.

### Phase 2: fold in the cleanup

Delete `views/Process.vue` (52KB, dead, still contains an "under development" alert).
Collapse the five duplicated view shells (`MainView`, `SimulationView`, `SimulationRunView`, `ReportView`, `InteractionView`) into one `WorldShell`.
Remove the 10 `alert()` calls, the `pendingUpload` module (lost on refresh), and the `trust:reruns` sessionStorage channel two siblings coordinate through today.
Fix the dead CSS custom properties (`:root` is declared inside `<style scoped>`, so `--black`, `--font-mono`, and the rest resolve to nothing) and drop the unused Google Fonts link.
Reuse as-is: `GraphPanel`, `KnowledgePad`, `EntitySelectionModal`, `CredibilityPanel`, `TrustChecksPanel`, `RehearsalPanel`, `PanelChat`.

Ships: the same app, far less code, one shell.

### Phase 3: un-bury honesty

Move `CredibilityPanel` and `TrustChecksPanel` out of `Step4Report` (where they live under `v-if="reportOutline"` and do not exist in the DOM unless a report is generated) to the Run header.
Make the noise floor a persisted Run property, not a sessionStorage secret.
Move `RehearsalPanel` to a compare surface with a real run picker.

Ships: trust visible without generating a report.

### Phase 4: the Field skin and the map

Build the WebGL mark layer and the basemap projection.
Apply the FIELD art direction: tokens, the three vendored fonts, the map, the Time Ribbon, the isopleth and fracture and flow layers, the provenance sheet, the REFUSED gutter.
Add the `min_evidence` slider over `/prepare/preview` with stance-coloured, evidence-sized marks.

Ships: the World looks like a world.

### Phase 5: the Library and EXTEND

Build the atlas Library front door (World thumbnails reuse the WebGL layer at low detail).
Enable build-into-existing-`graph_id` so a related corpus deposits into the World.
Build the EXTEND REVIEW overlay: ingest pulse, tectonic settle, density diff, admittance marks, stratified provenance.
Build the suspected-duplicates affordance and the RESOLVE sheet.

New backend, the critical path.
Let build target an existing `graph_id` (small).
Anchored re-projection so EXTEND does not move existing marks (pin existing marks, solve new marks into the residual space, version the basemap, and store the basemap version on each Run so an old Run replayed on a newer basemap shows a "replayed on basemap v2" badge rather than lying).
Fuzzy duplicate detection to populate RESOLVE (genuinely new, roughly 1.5 to 2 weeks, treat the candidate scorer as a first-class run object like seed-variance and ablation).
A merge that is recorded and reversible, not just an upsert.
Floor recomputation on corpus growth (a real recalc, not a frontend guess, or the "floor dropped" claim is a lie).

Ships: reuse levels EXTEND, the heart of the request.

### Phase 6: FORK, the Brief, and replay

Fork with lineage (`/duplicate` carries `parent_id`), the geological cross-section, the dashed parent ghost.
The BRIEF leader surface as a zoomed-out semantic level of detail.
Fracture forecasting and counterfactual replay (needs a backend change to fork a run at round N with a substituted event, scoped separately).
Time Ribbon and spread replay.

Ships: reuse level FORK, the leader surface, and the cinematic replay.

---

## 10. Risks and honest cost

Frontend, one strong developer, roughly 6 to 6.5 weeks for FIELD plus BRIEF plus LIBRARY plus EXTEND plus LINEAGE plus RESOLVE at 1440x900.
The 400px strip-map projection and the counterfactual fork are scoped separately.

The WebGL mark layer is the only genuinely hard rendering part, roughly 4 to 5 days: one canvas wrapped in a component with a static position VBO uploaded once (positions are frozen), a per-round `Float32Array` of stance values interpolated in the vertex shader, chevron and bar glyphs from a small sprite atlas selected by the stance attribute, and picking via an offscreen ID-colour buffer.
Keep the 1000 marks entirely out of Vue's reactivity, Vue owns the DOM chrome and the render loop owns the canvas.
Isopleths, fracture, and flow are a separate SVG layer above the canvas, low element count, computed off a density grid on a worker with `d3-contour`.
New dependency: `regl` (roughly 8kb) or hand-rolled WebGL2, plus `d3-contour` if not already bundled.

The basemap projection is the whole bet.
If Persona 2D positions are not semantically meaningful, the map is a lie with contour lines on it.
It needs a real embedding computed server-side at Persona-prep, and below roughly 30 Personas it falls back to the deterministic lattice in section 7.

The sharp reuse risk.
The basemap must stay stable across EXTEND, or every saved Run's coordinates lie: an old Run replayed on a re-projected basemap is drawn on moved land, silently corrupting historical comparison.
The mitigation is the anchored, versioned re-projection in Phase 5, and it is real algorithmic work, not a UI concern.

Backend, not in the frontend number and now the critical path.
World as a first-class object and Run-under-World listing are cheap, days.
The embedding for the basemap, fuzzy duplicate detection, anchored re-projection, a recorded and reversible merge, and floor recomputation on growth are the schedule risk, and they must be scoped and de-risked before Phase 5.

The product risk that grew with reuse.
Users will be tempted to treat an extended World as automatically more trustworthy.
Mitigation is in the language: the floor is shown, a thin specimen says it is thin, and a fork's inherited bar is stamped, so accumulation that did not narrow the floor is drawn as accumulation that did not help.

Theme.
One theme, dark, forecast-desk dark.
The marks are matte ink, not glow, and the only light in the scene is the map itself.

Fonts.
Three self-hosted woff2, subset.
Test the variable rendering on the Tauri Linux target (WebKitGTK) before committing.

Perf.
The real perf risk is Tauri on Linux (WebKitGTK), whose canvas and WebGL throughput is materially worse than Chromium and where the repo already disables the DMABUF renderer, so profile the map there specifically, not in Chrome.
The whole timeline (up to 50 rounds times 1000 agents is a 200KB typed array) fits in memory and is scrubbable client-side with no round-trips.

---

## 11. Open dependencies to resolve before Phase 5

The noise floor costs roughly 5x tokens and time, so if it is optional users will disable it and the reuse payoff (the floor dropping on EXTEND) collapses.
The backend must expose seed-variance and persona-permutation ablation as first-class run objects, not ad-hoc scripts.
The basemap embedding must be computed and stored at Persona-prep, and re-projection on EXTEND must be anchored and versioned.
If these stay ad-hoc, the design is a lie and should not be built.
