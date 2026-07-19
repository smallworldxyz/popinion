// Headed end-to-end run: Cambodia's H1 2026 renewable energy approvals.
//   DISPLAY=:1 bun run cambodia-renewables.mjs
// Requires the API on :5001 and this dev server on :3100, ChatGPT provider signed in.
// Modelled on live-demo.mjs; the seed below is sourced reporting, not invention.
import { chromium } from '@playwright/test'

const BASE = 'http://localhost:3100'

const SCENARIO =
  'Will the Cambodian public accept the government’s approval of more than $1 billion in solar, wind and biomass projects announced in July 2026?'

const SEED = `The Council for the Development of Cambodia (CDC) approved five renewable energy projects worth more than $1 billion between January and June 2026, per a CDC statement issued 14 July 2026. The projects are a $226 million 350 MW solar plant in Pursat by Billgate Construction; two wind farms in Mondulkiri of 150 MW each, valued at $200 million and $186 million; a combined $131 million for a 50 MW biomass plant and 40 MW solar facility in Pursat by Champion Group; and a $300 million 200 MW solar project in Stung Treng by Royal Group Power.

The $200 million Mondulkiri wind plant in Sen Monorom City was approved by the Cambodian Investment Board in January 2026. The investor is a Malaysian-based company with nearly three decades of operations in Cambodia. CIB Secretary General Chea Vuthy said the project is expected to generate 42 jobs. The National Assembly separately endorsed a law covering 24 electricity investment projects, and six wind projects in Mondulkiri now total 900 MW of planned capacity.

Cambodia targets 70 percent clean energy by 2030, yet solar supplies under 10 percent of electricity today despite some of the highest solar irradiance in the region. Electricite du Cambodge has held household tariffs flat through the 2026 fuel shock, absorbing roughly $40 million and later about $80 million in losses. Mines and Energy Minister Keo Rottanak said the system is predominantly renewable-based and told businesses not to worry about the cost, quantity or reliability of electricity.

A turn toward coal has hampered the 70 percent goal. The government vowed to build no new coal plants, though a 900 MW LNG-fired plant remains under construction. Rooftop solar is capped at a 30 MW national quota; medium and large owners pay compensation tariffs of $0.037 to $0.060 per kWh, excess feed-in earns no price incentive, and the tariffs are revisited every 6 to 12 months.

Across H1 2026 Cambodia approved $4.7 billion in fixed-asset investment over 276 projects, expected to create around 160,000 jobs, with China accounting for 35.7 percent of capital.

Points of contention raised publicly: a $200 million project producing only 42 jobs raises the question of who captures the benefit. Utility-scale generation is approved at speed while rooftop solar stays capped and charged a compensation tariff. It is unclear whether these approvals will reach household electricity bills or only stem EDC's losses. Pursat, Mondulkiri and Stung Treng residents are asking what their provinces receive in return for the land used.`

const ROUNDS = 12
const MIN = 60_000
const step = (m) => console.log(`\n▶ ${m}`)

const browser = await chromium.launch({ headless: false, slowMo: 350 })
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } })

step('Opening Popinion')
await page.goto(BASE)

step('Typing the scenario + the reality seed')
await page.getByPlaceholder(/Describe the scenario/).fill(SCENARIO)
await page.getByRole('button', { name: /Write text/ }).click()
await page.getByPlaceholder(/Paste real opinions/).fill(SEED)

step('Start Engine → building the knowledge graph (LLM)')
await page.getByRole('button', { name: /Start Engine/ }).click()

step('Waiting for the graph to build…')
await page.getByRole('button', { name: /Enter Environment Setup/ }).click({ timeout: 8 * MIN })

step('Preparing the population (personas from the graph)')
await page.getByRole('button', { name: /Start Preparation/ }).click()

step(`Waiting for personas + config, then capping the run at ${ROUNDS} rounds`)
await page.getByText(/Simulation Rounds Count Setting/i).waitFor({ timeout: 10 * MIN })
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
await page.getByText(/SIMULATION MONITOR/i).waitFor({ timeout: 5 * MIN })

step('Simulation running… (letting rounds play out)')
await page.getByRole('button', { name: /Generate Report/ }).click({ timeout: 12 * MIN })

step('Generating the report (report agent) — this reads the discussion')
await page.getByText(/Executive Summary|Stance Distribution/i).first().waitFor({ timeout: 10 * MIN })

step('Done — report is on screen. Leaving the window open.')
console.log('\nRun URL:', page.url())
await page.waitForTimeout(30 * MIN)
await browser.close()
