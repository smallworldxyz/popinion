# PRODUCT.md — Popinion

Register: product.
Design serves the product; this is an instrument, not a campaign.

Source of truth for strategic design decisions.
The visual system lives in DESIGN.md.
The full redesign rationale lives in docs/redesign-plan.md.

---

## Product purpose

Popinion is an AI prediction engine, a "Digital Mirror World".
It ingests real public-opinion data (crawled posts, uploaded documents, typed notes), builds a knowledge graph, compiles graph entities into grounded Personas, runs a multi-agent LLM simulation where those agents post and react over rounds, then produces a report.

It is a social wind tunnel: a crash test for laws, a diplomatic save game, a cognitive-vaccination range.
The promise is "don't guess the future, rehearse it".
Fail 999 times in the simulation so you succeed the one time that matters in reality.

The organizing object is the World.
A World (corpus, graph, Personas, evidence bar, basemap) is the durable, expensive, reusable asset.
A Run (one policy question fired at a World) is cheap and disposable.
Worlds accumulate: related topics extend an existing World, and new questions fork from finished ones.
The old app inverted this, treating the disposable Run as permanent and the durable World as throwaway, which is the core problem this redesign fixes.

---

## Target users

Two users on two surfaces of one engine.

Analyst / operator.
Builds the World, seeds it with real data, tunes the evidence bar, checks the noise floor, validates that an effect is real.
Wants control, provenance, and knobs.
Default surface: FIELD, the full-detail map.

Leader / decision-maker (a minister or negotiator).
Asks "will this policy survive contact with the public?"
Wants the verdict, the confidence, and the reason, in four minutes, not the pipeline.
Default surface: BRIEF, the same map generalized to a verdict with drill-down.

There is no auth layer in the API, so these are modes of one object, not roles or separate apps.
The analyst sends the leader a link; the leader opens the same World at a different altitude.

---

## Brand personality

Instrument, not illustration.
A metrology bench and a forecast desk, not a sci-fi HUD.

Honest by construction.
Popinion's rarest property is that it measures its own error: seed-variance noise floor, persona-permutation ablation, and it refuses to build Personas below an evidence bar rather than fabricating them.
Extraction is nondeterministic (three identical runs produced 2, 3, and 0 stance edges), and the design must make that refusal read as rigor, not failure.
A reading without its error bar should look unfinished.

Grounded.
Every mark on screen is a real Persona with a name and provenance back to a real post.
The map is drawn at true population scale: three eligible Personas is three marks, and a sparse world looks sparse on purpose.

Tone in copy: plain, declarative, quantitative.
Numbers that came from data are set in monospace; prose the product wrote is not.
No hype, no percentages presented as certainty, no em dashes.

---

## Anti-references

The current UI, explicitly rejected by the client.
Editorial serif body text, white navbar, grey cards, a chat-style prompt box, a rigid five-step wizard rail (Graph Build to Env Setup to Simulation to Report to Interaction), a d3 graph beside step cards, and a black SYSTEM DASHBOARD log pane.
Client verdict, verbatim: too plain and undesigned, too wizard-y and linear, not cinematic enough, buries the actual value.

Reflex one (rejected).
Simulation or AI product, therefore dark navy plus neon cyan plus a glowing force-directed network.
No glow, no neon, no cyan, no force-directed hairball.
Positions are fixed and semantic, the opposite of an animated meaningless node cloud.

Reflex two (rejected).
Not-that, therefore editorial serif on warm paper with typographic restraint.
That is the current design the client already rejected.
No serif, no paper, no evenly-quiet mutedness.

---

## Strategic design principles

The map is the interface.
Opinion is terrain and the weather moves over it; the verdict is the map, not a chart beside it.

Scrub time, do not click next.
The core gesture is dragging the Time Ribbon, which is what kills the wizard.

Honesty is load-bearing, not a footnote.
Provenance, the evidence bar, the noise floor, and the REFUSED gutter are always reachable, never gated behind report generation.

Reuse is the point.
Group, Extend, Fork.
Extending a World with more evidence visibly narrows its noise floor and promotes previously refused entities into Personas, so building on prior work is a measurable good, not just filing.

Never launder disagreement into consensus.
When two entities might be the same, stance conflict lowers merge confidence, and the safe default is keep separate.
Openly fragmented beats silently wrong.

Design the degenerate states first.
Zero edges, zero eligible Personas, a thin corpus, a run still going, an unauthorized model: these are common, not edge cases, and each has a designed state with a clear next action.
