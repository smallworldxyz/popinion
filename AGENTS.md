# Project agent memory

This file is the project's committed home for project-intrinsic agent knowledge: build, test, release, architecture, and sharp-edge notes that should travel with the code.

- Add durable project-specific notes here as they are discovered through real work.
- **E2E from a git worktree:** `apps/web` Playwright (`bun run test:e2e`) targets `http://localhost:3000` with `reuseExistingServer`. If the primary checkout (or another worktree) already holds `:3000`, your tests silently run against *that* server's stale code. Start your worktree's own `bunx vite --port <free> --strictPort` and run `bunx playwright test` with a temp config that sets `use.baseURL` to it and drops `webServer`. Routes are stubbed via `page.route`, so the Rust API need not run.
- **FIELD map render layers** (`apps/web/src/components/FieldMap.vue` + `components/field/`): marks are a hand-rolled **WebGL2** point-sprite layer (`markRenderer.js`, 2D fallback when WebGL2 is absent); the 1000+ marks are held in a plain array, kept out of Vue reactivity - the render loop owns the canvas, Vue owns the chrome (redesign-plan §10). The recolour sweep is eased *in the vertex shader* (per-round from/to/delay Float32Arrays); positions are a frozen VBO. Isopleths/fracture/flow are an **SVG** overlay computed per round from `layers.js` (d3-contour, already bundled - no `regl`/new dep). Counterfactual two-futures replay is data-gated (no backend fork endpoint) and no-ops.

## Maintaining this file

Keep this file for knowledge useful to almost every future agent session in this project.
Do not repeat what the codebase already shows; point to the authoritative file or command instead.
Prefer rewriting or pruning existing entries over appending new ones.
When updating this file, preserve this bar for all agents and keep entries concise.
