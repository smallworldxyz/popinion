# DESIGN.md — Popinion (FIELD)

Visual system for the FIELD direction.
Strategic context is in PRODUCT.md.
Full rationale and phased build plan are in docs/redesign-plan.md.

Status: chrome foundation built (Phase 0); the FIELD map skin (Phase 4) is not yet built.
When code and this file disagree during the build, update this file in the same change.

**Palette and type: Baray (adopted).**
The visual system is **Baray** - gold `#F2B01E` primary, purple `#6F4D9F` secondary, navy `#0E2340` surfaces; Fraunces (display) + Inter (body) + JetBrains Mono (numbers), self-hosted woff2, no CDN.
This is the whole design, not a chrome layer over a separate map palette: the earlier slate-indigo / Inter-Tight / IBM-Plex proposal is retired and its values are gone from this file.
The one thing Baray does not touch is the stance **data** ramp (blue↔amber below) - that stays functional and colourblind-safe, never brand gold/purple, because a data encoding is not decoration.
Implementation: `apps/web/src/assets/theme.css` (primitive + semantic tokens, dark primary with `[data-theme="light"]` alternate).

---

## Theme

Dark, forecast-desk dark.
Scene: a duty forecaster at 04:10 in a national situation room, lit only by the wall-sized map she is reading, the room behind her unlit so nothing competes with the map.
The room is dark because the map is the only light source.
The map is a paper-dense field of matte ink marks, not a void with glowing dots.
Dark is the primary theme, because a forecast map is read in a dark room; a light theme ships as the alternate for daylight/print use, not as the default.

---

## Color

Strategy: Baray, committed.
Two brand hues carry all chrome: gold (the primary, CTA, the light in the room) and purple (the secondary, links, secondary CTA, World-agent event injection).
Navy is the ground; gold and purple sit on it.
Dark is the primary theme; a light theme (`[data-theme="light"]`) is the alternate.

### Primitives (Baray)

```
--gold          #F2B01E   primary / CTA            --purple          #6F4D9F   secondary
--gold-hover    #D9990F                            --purple-hover    #5C3F86
--gold-bright   #F5C243   gold as text on navy     --purple-bright   #A98BD6   purple as text on navy
--gold-deep     #9C6F00   gold as text on light    --purple-soft     #E7DEF0
--gold-soft     #FBE7BF

--navy          #0E2340   dark ground              --surface-light   #FFFFFF
--navy-raised   #16345C   dark raised              --surface-light2  #F4F6F9

--ink           #1A2433   text on light            --on-dark         #FFFFFF   text on navy
--ink-muted     #5D6B7C   muted on light           --on-dark-muted   #C9D6E5   muted on navy
--border-light  #D9E0E8                            --border-dark     #2C4A70
```

### Semantic map

Dark is primary (`:root`); light is the alternate (`:root[data-theme="light"]`).

```
--color-bg / --color-surface     navy / navy-raised   (light: surface2 / white)
--color-text / --color-text-muted  on-dark / on-dark-muted   (light: ink / ink-muted)
--color-accent (fill) / --color-accent-text   gold / gold-bright   (light text: gold-deep)
--color-accent-2 (fill) / --color-accent-2-text   purple / purple-bright
--color-on-accent   navy   (text on a gold fill)
--color-on-accent-2 white  (text on a purple fill)
--color-border      border-dark   (light: border-light)
```

Contrast rule, enforced in `.btn`/`.badge`: gold fill takes navy text; gold as text uses `--color-accent-text` (bright on dark, deep on light); purple fill takes white text.

### Meaningful color

Color is never decoration; it is a claim.
Event injection is the gold family (the World-agent's ember); a refusal (below the evidence bar) is desaturated navy, never red.

### Stance (the key data encoding)

A diverging luminance-plus-hue ramp on the blue-to-amber axis, never red-to-green.

```
--stance-oppose         oklch(0.60 0.15 250)
--stance-oppose-strong  oklch(0.48 0.17 255)
--stance-neutral        oklch(0.74 0.02 265)   near-achromatic, the lightest point
--stance-support        oklch(0.66 0.14 78)
--stance-support-strong oklch(0.55 0.16 66)
```

Blue and amber is the tritan-safe axis (survives deuteranopia and protanopia), and it stays separable in tritanopia by luminance: oppose is dark, neutral is bright, support is mid.
Stance is carried by three redundant channels, so it reads in greyscale:

1. Hue (blue to amber).
2. Lightness (poles dark, neutral bright).
3. Mark shape: oppose is a downward chevron, neutral a flat bar, support an upward chevron.

Sentiment is separate from stance and is encoded as mark opacity and agitation, never a second hue.

---

## Typography

Self-hosted, subset woff2, no CDN, because the app ships as an offline-capable Tauri desktop build.
Test rendering on the Tauri Linux target (WebKitGTK) before shipping.

| Role | Family | Weights | Notes |
|---|---|---|---|
| Display, headings, map labels | Fraunces | 500, 600, 700 | Baray display face; weight carries the headline, not size |
| Body, UI | Inter | 400, 500, 600 | prose capped 65 to 72ch |
| Data, provenance, counts | JetBrains Mono | 400, 600 | every number, ID, tick, endpoint |

Mono is a truth signal: monospace means measured from real data, Inter means the product wrote it, Fraunces is the voice of the product.

Scale, ratio 1.333: 11 / 13 / 15 / 20 / 27 / 36 / 48 (`--fs-2xs` … `--fs-2xl`).
Hierarchy comes from weight contrast (600 against 400), not size inflation.

---

## Layout and surfaces

No cards.
Surfaces are sheets and rails, differentiated by ground inset, not by boxes or floating shadows.

- Sheet: right-side, full-height, flush, no rounding, no float. The Evidence Sheet is the canonical one.
- Rail: the left provenance ladder (Sources, Ontology, Graph, Personas), narrow, always present.
- Time Ribbon: bottom transport with the event-injection track and the action ticker.

The map fills the center at full detail (FIELD) or generalized to contours (BRIEF).
At roughly 400px the field becomes a stacked strip map, one stance-over-time band per cohort, and the Time Ribbon becomes the primary control.

Vary spacing for rhythm; do not pad everything equally.

---

## Signature components

- Mark: one Persona at a fixed semantic coordinate. Shape encodes stance, fill encodes eligibility (solid eligible, hollow below-bar in the REFUSED gutter), opacity encodes sentiment.
- Isopleths: contour lines of stance density, drawn like isobars. Auto-disabled below 30 marks, because a contour over 8 points is a fabrication.
- Flow: Minard-style tapered ribbons showing which Persona moved which others, sparse and directional.
- Fracture: a hard jagged rule where the stance gradient exceeds a threshold, a geological fault where the population splits.
- Time Ribbon: scrubbable rounds, the video head; the map is the video.
- Evidence Sheet: stratified provenance, one horizontal stratum per seed batch, each fact tagged to its batch.
- RESOLVE sheet: duplicate reconciliation; confidence as a filled-dot glyph (full high, half medium, empty low), never a traffic light.
- Lineage cross-section: ancestry as sediment layers, divergence drawn as distance on the page, parent as a dashed ghost stratum.

---

## Motion

Never animate layout properties.
Ease-out only, exponential curves (quart, quint, expo), no bounce, no elastic.

- Recolour sweep: advancing a round flips marks in the order actions occurred, spatially staggered outward from the originator, 700ms ease-out-quint, colour and canvas transform only.
- Scrub: dragging the Time Ribbon plays forward and backward at frame rate.
- Evidence on hover: ghosts provenance into the always-mounted sheet with no layout shift; click pins it.
- Tectonic settle: the only time marks move is EXTEND, when new entities land and existing marks drift once to accommodate, at most 800ms.

---

## Rendering architecture

- 1000 marks on a WebGL point-sprite layer (regl or hand-rolled WebGL2). Positions are frozen, so the position VBO uploads once; per round upload a `Float32Array` of stance and interpolate in the vertex shader. Keep the marks entirely out of Vue's reactivity.
- Isopleths, fracture, flow, and labels on one SVG layer above the canvas, low element count, computed off a density grid on a worker with `d3-contour`.
- Picking via an offscreen ID-colour buffer.
- Profile on the Tauri Linux target (WebKitGTK), not Chrome; its canvas and WebGL throughput is materially worse.

---

## Absolute bans (in force here)

No glow, neon, cyan, or force-directed node cloud (reflex one).
No warm-paper editorial restraint, body serif, or evenly-quiet flatness (reflex two); the only serif is Fraunces, and only as the display/heading face over the Baray palette.
No side-stripe borders, gradient text, decorative glassmorphism, hero-metric template, identical card grids, or modal-as-first-thought.
No em dashes in UI copy.
No percentage presented as certainty on the leader surface; confidence is Signal, Noise, or Unmeasured.
