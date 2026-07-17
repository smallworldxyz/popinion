# DESIGN.md — Popinion (FIELD)

Visual system for the FIELD direction.
Strategic context is in PRODUCT.md.
Full rationale and phased build plan are in docs/redesign-plan.md.

Status: proposed, not yet built.
When code and this file disagree during the build, update this file in the same change.

---

## Theme

Dark, forecast-desk dark.
Scene: a duty forecaster at 04:10 in a national situation room, lit only by the wall-sized map she is reading, the room behind her unlit so nothing competes with the map.
The room is dark because the map is the only light source.
The map is a paper-dense field of matte ink marks, not a void with glowing dots.
One theme only, no light mode, because a forecast map is read in a dark room.

---

## Color

Strategy: Committed.
Brand hue is slate-indigo, `oklch(0.62 0.13 265)`.
All neutrals are tinted toward 265.
No pure black or white anywhere.

### Neutrals and grounds

```
--ground-deep    oklch(0.19 0.021 265)   app frame, the room
--ground-map     oklch(0.245 0.026 265)  field substrate
--ground-raised  oklch(0.29 0.028 265)   rails, sheets
--hairline       oklch(0.38 0.022 265)   graticule, rules
--text-quiet     oklch(0.66 0.018 265)   labels, secondary
--text-body      oklch(0.88 0.012 265)   body
--text-ink       oklch(0.96 0.008 265)   max, headline
```

### Meaningful color

Color is never decoration; it is a claim.

```
--alert-event    oklch(0.72 0.19 42)     ember, World-agent event injection only
--refusal        oklch(0.55 0.03 265)    below the evidence bar, desaturated, never red
```

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
Test variable-axis rendering on the Tauri Linux target (WebKitGTK) before committing.

| Role | Family | Weights | Notes |
|---|---|---|---|
| Display, map labels | Inter Tight | 600, 700 | tight tracking; condensed grotesque, not a serif |
| Body, UI | Inter | 400, 500 | prose capped 65 to 72ch |
| Data, provenance, counts | IBM Plex Mono | 400, 600 | every number and ID |

Mono is a truth signal: monospace means measured from real data, Inter means the product wrote it.

Scale, ratio 1.333: 11 / 13 / 15 / 20 / 27 / 36 / 48.
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
No serif, warm paper, or evenly-quiet restraint (reflex two).
No side-stripe borders, gradient text, decorative glassmorphism, hero-metric template, identical card grids, or modal-as-first-thought.
No em dashes in UI copy.
No percentage presented as certainty on the leader surface; confidence is Signal, Noise, or Unmeasured.
