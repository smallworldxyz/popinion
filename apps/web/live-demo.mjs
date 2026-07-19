// Watchable end-to-end simulation demo, driven headed so you can SEE it run.
//   DISPLAY=:1 bun run scripts/live-demo.mjs
// Requires the dev stack up (bun run dev) and the ChatGPT provider signed in.
import { chromium } from '@playwright/test'

const BASE = 'http://localhost:3000'
const SCENARIO = 'How will the public react to phasing out the national fuel subsidy over 12 months?'
const SEED = `The Ministry of Economy announced a plan to phase out the national fuel subsidy over 12 months, arguing it is too costly and mainly benefits the wealthy. Transport unions strongly oppose the move, warning that tuk-tuk and truck drivers will be hit hardest as diesel prices rise. The Consumer Protection Association supports a slower phase-out paired with cash transfers to poor households. Opposition MP Sokha called it a betrayal of ordinary commuters and demanded the plan be scrapped. Small business owners are divided: some fear higher delivery costs, others welcome ending a wasteful subsidy. Urban commuters worry about rising bus fares, while economists at the National Institute broadly back the reform as fiscally necessary.`
const ROUNDS = 10               // small run so the demo finishes quickly
const MIN = 60_000              // 1 min
const step = (m) => console.log(`\n▶ ${m}`)

const browser = await chromium.launch({ headless: false, slowMo: 350 })
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } })

step('Opening Popinion')
await page.goto(BASE)

step('Typing the scenario + a reality seed')
await page.getByPlaceholder(/Describe the scenario/).fill(SCENARIO)
await page.getByRole('button', { name: /Write text/ }).click()
await page.getByPlaceholder(/Paste real opinions/).fill(SEED)

step('Start Engine → building the knowledge graph (LLM)')
await page.getByRole('button', { name: /Start Engine/ }).click()

step('Waiting for the graph to build…')
await page.getByRole('button', { name: /Enter Environment Setup/ }).click({ timeout: 5 * MIN })

step('Preparing the population (personas from the graph)')
await page.getByRole('button', { name: /Start Preparation/ }).click()

step('Waiting for personas + config, then capping the run at ' + ROUNDS + ' rounds')
await page.getByText(/Simulation Rounds Count Setting/i).waitFor({ timeout: 6 * MIN })
await page.evaluate((rounds) => {
  const cb = [...document.querySelectorAll('input[type=checkbox]')].find(c => c.closest('.switch-control'))
  if (cb && !cb.checked) cb.click()
  const r = document.querySelector('input[type=range].minimal-slider') || document.querySelector('input[type=range]')
  if (r) {
    const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set
    set.call(r, String(rounds)); r.dispatchEvent(new Event('input', { bubbles: true }))
  }
}, ROUNDS)

step('Starting the simulation — watch the agents post and react')
await page.getByRole('button', { name: /Start Dual-World Parallel Simulation/ }).click({ timeout: MIN })
// Navigation + engine spawn can take a bit; wait for the run view to render.
await page.getByText(/SIMULATION MONITOR/i).waitFor({ timeout: 3 * MIN })

step('Simulation running… (letting rounds play out)')
await page.getByRole('button', { name: /Generate Report/ }).click({ timeout: 8 * MIN })

step('Generating the report (report agent) — this reads the discussion')
await page.getByText(/Executive Summary|Stance Distribution/i).first().waitFor({ timeout: 6 * MIN })

step('Done — report is on screen. Leaving the window open for 10 minutes.')
await page.waitForTimeout(10 * MIN)
await browser.close()
