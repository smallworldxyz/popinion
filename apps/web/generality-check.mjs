// A second scenario, deliberately unlike the Cambodia one, to check the graph
// and persona fixes are not tuned to it. Different domain (consumer software,
// not energy policy), different geography, non-government actors.
//
// It is built to stress three specific things:
//   1. Ravi Menon is attacked by everyone and never answers. He is a person, so
//      he must survive as an agent - the structural "debate subject" shape must
//      not silence him.
//   2. The objections are phrased in varied language (broken promise, gambling,
//      predatory, quitting) with no OPPOSES-shaped verb in the prose, so the
//      stance mandate has to recognise them.
//   3. Non-actors are present and must stay out of the population: the game,
//      the loot box system, the price point, the terms of service.
//
//   DISPLAY=:1 bun run generality-check.mjs
import { chromium } from '@playwright/test'

const BASE = 'http://localhost:3100'

const SCENARIO =
  'Will Meridian Studios players accept paid loot boxes being added to Starfall Drift?'

const SEED = `Meridian Studios announced that Starfall Drift, its free-to-play racing game, will add paid loot boxes costing $4.99 each in the next update. The studio said the revenue keeps the game free for everyone and funds two more years of content.

Studio founder Ravi Menon has not commented publicly since the announcement. Players have flooded his account with complaints. The Starfall Players Council called the move a broken promise, pointing to a 2024 developer post that said the game would never sell randomised rewards. Community moderators say the forums have become unmanageable.

Streamer Kaya Lindqvist, who has built her channel on the game, said she will stop covering it if the change ships, calling the mechanic gambling aimed at teenagers. The Nordic Consumer Board has opened an inquiry into whether randomised paid rewards can lawfully be sold to minors, citing its own guidance on predatory monetisation.

Meridian's community manager Tom Okafor defended the change in a livestream, saying the odds will be published and nothing is exclusive to loot boxes. Competitive players are split: some welcome the funding for tournaments, others say paid randomised upgrades will wreck ranked matchmaking. Parents in the game's subreddit report their children spending pocket money on similar systems in other titles.

Meridian's publisher Halcyon Interactive backed the decision, noting the studio has missed revenue targets for three consecutive quarters. A rival studio, Northwind Games, publicly pledged never to add loot boxes to its own racing title, and picked up players quitting Starfall Drift.

The revised terms of service take effect at the same time as the update. Retention among daily players has fallen 12 percent in the week since the announcement.`

const MIN = 60_000
const step = (m) => console.log(`\n▶ ${m}`)

const browser = await chromium.launch({ headless: false, slowMo: 300 })
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } })

step('Opening Popinion')
await page.goto(BASE)

step('Typing a scenario from a different domain entirely')
await page.getByPlaceholder(/Describe the scenario/).fill(SCENARIO)
await page.getByRole('button', { name: /Write text/ }).click()
await page.getByPlaceholder(/Paste real opinions/).fill(SEED)

step('Start Engine → building the knowledge graph (LLM)')
await page.getByRole('button', { name: /Start Engine/ }).click()

step('Waiting for the graph to build…')
await page.getByRole('button', { name: /Enter Environment Setup/ }).click({ timeout: 8 * MIN })

step('Preparing the population')
await page.getByRole('button', { name: /Start Preparation/ }).click()

step('Waiting for personas + config')
await page.getByText(/Simulation Rounds Count Setting/i).waitFor({ timeout: 10 * MIN })

step('Starting the simulation')
await page.getByRole('button', { name: /Start Dual-World Parallel Simulation/ }).click({ timeout: MIN })
await page.getByText(/SIMULATION MONITOR/i).waitFor({ timeout: 5 * MIN })

step('Simulation running…')
await page.getByRole('button', { name: /Generate Report/ }).click({ timeout: 12 * MIN })

step('Generating the report')
await page.getByText(/Executive Summary|Stance Distribution/i).first().waitFor({ timeout: 10 * MIN })

step('Done')
console.log('\nRun URL:', page.url())
await browser.close()
