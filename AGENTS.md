# Project agent memory

This file is the project's committed home for project-intrinsic agent knowledge: build, test, release, architecture, and sharp-edge notes that should travel with the code.

- Add durable project-specific notes here as they are discovered through real work.
- **Wizard surfaces are one shell:** the five routes `Process` / `Simulation` / `SimulationRun` / `Report` / `Interaction` all render `apps/web/src/views/WorldShell.vue`, which branches on `route.name` to host Step1-5 (there is no `MainView`/`SimulationView`/etc. anymore). It re-inits on `route.fullPath` change since the instance is reused across those routes. The FIELD surfaces `World` (`WorldView`) and `Run` (`RunView`) are separate.
- **E2E from a git worktree:** `apps/web` Playwright (`bun run test:e2e`) targets `http://localhost:3000` with `reuseExistingServer`. If the primary checkout (or another worktree) already holds `:3000`, your tests silently run against *that* server's stale code. Start your worktree's own `bunx vite --port <free> --strictPort` and run `bunx playwright test` with a temp config that sets `use.baseURL` to it and drops `webServer`. Routes are stubbed via `page.route`, so the Rust API need not run.

## Maintaining this file

Keep this file for knowledge useful to almost every future agent session in this project.
Do not repeat what the codebase already shows; point to the authoritative file or command instead.
Prefer rewriting or pruning existing entries over appending new ones.
When updating this file, preserve this bar for all agents and keep entries concise.
