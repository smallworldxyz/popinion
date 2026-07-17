import { defineConfig } from '@playwright/test'

// Drives the real stack: `bun run dev` at the repo root starts the API (:5001)
// and this dev server (:3000). An already-running stack is reused as-is.
export default defineConfig({
  testDir: './tests',
  use: { baseURL: 'http://localhost:3000', trace: 'retain-on-failure' },
  webServer: {
    command: 'bun run dev',
    cwd: '../..',
    url: 'http://localhost:3000',
    reuseExistingServer: true,
    timeout: 180_000,
  },
})
