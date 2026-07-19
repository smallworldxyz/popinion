# Popinion Redesign - Execution TODO

Derived from `docs/redesign-plan.md` (FIELD direction).
Design system: **Baray** color + type, adopted for all chrome. Data-layer stance colors stay functional (see §0.3).

Legend: `[ ]` todo · `[~]` partially done on `feat/reusable-worlds` · `[x]` done · **(BE)** backend/Rust · **(FE)** frontend/Vue · **(DS)** design system.

---

## Phase 0 - Baray design foundation (NEW, front-loaded; blocks the Phase 4 skin)

Popinion ships as an offline Tauri build, so fonts are **self-hosted woff2, no CDN** (overrides Baray's "Google Fonts via CDN").

### 0.1 Tokens (DS)
- [x] Add primitive + semantic CSS custom properties in a real (non-`scoped`) global stylesheet. Built in `apps/web/src/assets/theme.css`, imported once from `main.js` - the dead scoped `:root` no longer applies.
- [x] Primitives (Baray): `--gold #F2B01E` / hover `#D9990F` / bright `#F5C243` / deep `#9C6F00` / soft `#FBE7BF`; `--purple #6F4D9F` / hover `#5C3F86` / bright `#A98BD6` / soft `#E7DEF0`; navy `--navy #0E2340` / raised `#16345C`; surfaces `#FFFFFF` / `#F4F6F9`; text `#1A2433` / `#5D6B7C`; on-dark `#FFFFFF` / `#C9D6E5`; borders `#D9E0E8` / `#2C4A70`.
- [x] Semantic map: `--color-bg`, `--color-surface`, `--color-text`, `--color-text-muted`, `--color-accent` (gold), `--color-accent-2` (purple), `--color-border`, `--color-on-accent` (navy), `--color-on-accent-2` (white). Dark is the primary theme in `:root`; `:root[data-theme="light"]` is the alternate. `index.html` sets `data-theme="light"` transitionally so the not-yet-skinned views stay coherent - removing it activates dark (Phase 4).
- [x] Contrast rule enforced in the `.btn`/`.badge` classes: gold fill -> navy text; gold text via `--color-accent-text` (`#F5C243` on dark, `#9C6F00` on light); purple fill -> white text.

### 0.2 Type (DS)
- [x] Self-host woff2 (subset): **Fraunces** (500/600/700), **Inter** (400/500/600), **JetBrains Mono** (400/600) in `apps/web/src/assets/fonts/`, `@font-face`d in theme.css. Google Fonts `<link>` removed from `index.html`; build bundles the woff2 as local assets.
- [x] Type scale ratio 1.333 (11/13/15/20/27/36/48) as `--fs-2xs..--fs-2xl`; weight carries hierarchy.
- [x] "mono = a measured truth signal": `.mono`/`code` map to JetBrains Mono.
- [ ] Verify Fraunces + Inter rendering on Tauri Linux (WebKitGTK) before shipping - NOT done (no Tauri target in this environment; verified in Chromium via the dev build).

### 0.3 Stance data ramp - DECIDED: functional ramp, Baray chrome
- [x] Diverging luminance+hue ramp as `--stance-support` (amber `oklch(0.66 0.14 78)`) / `--stance-neutral` (lightest) / `--stance-oppose` (blue `oklch(0.60 0.15 250)`) + `-strong` poles, in theme.css. Applied to `Step3Simulation.vue` (replaced the banned red/blue). Mark-shape channel (chevron/bar) lands with the Phase 4 WebGL layer.
- [x] Purple is **chrome only** - no stance uses it; the stance ramp is the blue↔amber data axis.

### 0.4 Component restyle (DS/FE)
- [~] Reusable token classes shipped in theme.css: `.btn`(primary/secondary/outline/ghost/danger), `.eyebrow`, `.badge`(gold/purple), `.sheet`, `.stat-num`/`.stat-label`, `.code`, `.pill`, `.field`. Per-component adoption across the 15 views is Phase 4 skin work.
- [x] `Models` button (settings fab in `App.vue`) moved onto the token system.

---

## Phase 1 - The backbone: World is the primary noun

### Backend (BE)
- [ ] `SimMeta.kind` enum (`canonical | replicate | ablation | alt`), backfill existing rows to `canonical`.
- [ ] `SimMeta.parent_id` (nullable) for lineage.
- [ ] Add `completed` to the simulation status vocabulary (plan §7: today a finished run reads `alive`/`prepared`, indistinguishable from never-run - this misreports).
- [ ] A Run resolves to a World via `graph_id`.

### Frontend (FE)
- [~] Routes `/world/:graphId` and `/world/:graphId/run/:simId` (present per ui.spec.js Worlds tests - verify against final schema).
- [~] Group `/list` by `graph_id`; wire the unused `listSimulations()`.
- [~] World view lists Runs (event, kind, status, Personas, stance-share, created_at).
- [ ] Report becomes a **tab on a Run**, not a route.

Ships: every past Run reachable, grouped under its World. Riskiest item = `SimMeta.kind`; land schema + backfill before UI.

---

## Phase 2 - Fold in the cleanup

- [~] Collapse the 5 view shells (`MainView`, `SimulationView`, `SimulationRunView`, `ReportView`, `InteractionView`) into one `WorldShell` (a `WorldShell #12` refactor exists off-branch - reconcile, don't redo).
- [ ] Delete `views/Process.vue` (52KB dead code, still has an "under development" alert).
- [ ] Remove the 10 `alert()` calls.
- [ ] Remove `pendingUpload` module (state lost on refresh) and the `trust:reruns` sessionStorage channel.
- [x] Fix dead CSS custom props (`:root` inside `<style scoped>`) - done via Phase 0.1 (real global theme.css).
- [x] Drop the unused Google Fonts `<link>` (offline build) - removed from `index.html`; fonts now self-hosted.
- [ ] Reuse as-is: `GraphPanel`, `KnowledgePad`, `EntitySelectionModal`, `CredibilityPanel`, `TrustChecksPanel`, `RehearsalPanel`, `PanelChat`.

Ships: same app, far less code, one shell.

---

## Phase 3 - Un-bury honesty

- [~] Move `CredibilityPanel` + `TrustChecksPanel` out of `Step4Report` (v-if'd out of the DOM) to the Run header (an "un-bury honesty #11" change exists off-branch - reconcile).
- [ ] Noise floor becomes a **persisted Run property**, not a sessionStorage secret.
- [ ] `RehearsalPanel` -> a compare surface with a real run picker (uses the fully-built `GET /compare`).

Ships: trust visible without generating a report.

---

## Phase 4 - The Field skin and the map

- [ ] WebGL mark layer (`regl` ~8kb or hand-rolled WebGL2): static position VBO (frozen), per-round `Float32Array` stance interpolated in vertex shader, chevron/bar glyphs from a sprite atlas, picking via offscreen ID-colour buffer. Keep 1000 marks out of Vue reactivity.
- [ ] Basemap projection from a **server-side embedding computed at Persona-prep** (the whole bet - plan §10). Below ~30 Personas fall back to the deterministic stratified lattice (§7).
- [~] Stance on the map: colour by ramp, size by `evidence_score`, hollow the ineligible. (FIELD demo commits exist on branch - extend, don't restart.)
- [~] Time Ribbon + recolour-sweep motion (700ms ease-out-quint, spatially staggered; only colour/transform animate). (Partially present.)
- [ ] Isopleth / fracture / flow SVG layers above the canvas (`d3-contour` on a worker; auto-disable isopleths below 30 marks).
- [ ] Provenance-on-hover into the always-mounted right sheet; REFUSED gutter for below-bar entities.
- [ ] `min_evidence` slider over synchronous `POST /prepare/preview` that repaints the map live.
- [ ] Degenerate states designed, not errored (plan §7): 0 edges, 0 eligible Personas (block before action + evidence histogram), thin corpus (<30 -> lattice), run-still-going, model-slow per-action gate.

Ships: the World looks like a world.

---

## Phase 5 - The Library and EXTEND (open backend deps - de-risk first)

- [ ] Atlas Library front door (World thumbnails reuse WebGL layer at low detail; ordered by mass, not date).
- [ ] **(BE)** Let build target an existing `graph_id` (small; merge machinery already exists via `upsert_entity`).
- [ ] **(BE, critical path)** Anchored + versioned re-projection so EXTEND never moves existing marks; store basemap version on each Run.
- [ ] **(BE, ~1.5-2wk)** Fuzzy duplicate detection to populate RESOLVE (treat scorer as a first-class run object).
- [ ] **(BE)** Recorded + reversible merge (not a bare upsert), stored as a lineage event.
- [ ] **(BE)** Floor recomputation on corpus growth (real recalc, not a frontend guess).
- [ ] EXTEND REVIEW overlay: ingest pulse, tectonic settle, density diff, admittance marks, stratified provenance.
- [ ] Suspected-duplicates affordance + RESOLVE sheet (confidence = filled-dot glyph, never traffic-light; safe default keep-separate).

Ships: reuse level EXTEND, the heart of the request.

---

## Phase 6 - FORK, BRIEF, replay

- [ ] **(BE)** `/duplicate` carries `parent_id` (lineage).
- [ ] Geological cross-section ancestry (dashed parent ghost, divergence = stance-field distance).
- [ ] BRIEF leader surface = FIELD zoomed out (semantic LOD, marks dissolve into contours, one 36px verdict, three states never a %).
- [ ] **(BE, scoped separately)** Fork a run at round N with a substituted event -> counterfactual replay (original dashed ghost vs solid).
- [ ] Time Ribbon spread replay from pre-fetched `/timeline` + `/spread` (HTTP polling; no SSE/WS).

Ships: reuse level FORK, leader surface, cinematic replay.

---

## Blockers before Phase 5 (plan §11 - "if these stay ad-hoc, the design is a lie")
- [ ] Noise floor exposed as first-class run objects (seed-variance + persona-permutation ablation), not ad-hoc scripts.
- [ ] Basemap embedding computed + stored at Persona-prep.
- [ ] Anchored + versioned re-projection on EXTEND.

---

## Proposed Nimmit crew waves

**Wave 1 (safe, no open deps) - launch now:**
- Crew A (DS): Phase 0 Baray foundation - tokens, self-hosted fonts, component restyle.
- Crew B (BE): Phase 1 backend - `kind`, `parent_id`, `completed`; then Phase 5's cheap "build into existing graph_id".
- Crew C (FE): Phase 1 frontend + Phase 3 - reconcile grouping/routes, report-as-tab, un-bury honesty, persisted floor.
- Crew D (FE): Phase 2 cleanup - delete Process.vue, reconcile WorldShell, kill alerts/pendingUpload/sessionStorage.

**Wave 2 (the hard rendering) - after Wave 1 lands:** Phase 4 map (single owner, WebGL is the one genuinely hard part).

**Wave 3 (needs backend de-risking) - do not start until §11 blockers resolved:** Phase 5 + 6.
