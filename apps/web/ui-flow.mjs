// Headed UI test flow over the recent changes: ChatGPT-only settings, the
// collapsible sidebar, the report tool-call/result display + no stuck "Waiting",
// the chat agent names, and Knowledge Pad removal. Uses existing data so it's fast.
//   cd apps/web && DISPLAY=:1 bun run ui-flow.mjs
import { chromium } from '@playwright/test'

const BASE = 'http://localhost:3000'
const API = 'http://localhost:5001'
let pass = 0, fail = 0
const check = (name, cond) => { cond ? (pass++, console.log(`   \x1b[32m✓\x1b[0m ${name}`)) : (fail++, console.log(`   \x1b[31m✗ ${name}\x1b[0m`)) }
const step = (m) => console.log(`\n▶ ${m}`)
const txt = (page) => page.evaluate(() => document.body.innerText)

// Pick a completed report + its sim from the live backend.
const list = await (await fetch(`${API}/api/report/list`)).json()
const reports = (list.data?.reports || list.data || []).filter(r => r.status === 'completed' && r.sections?.length)
const report = reports.sort((a, b) => (b.created_at || '').localeCompare(a.created_at || ''))[0]
if (!report) { console.log('No completed report to drive the flow. Run a simulation first.'); process.exit(1) }
const REPORT_ID = report.report_id
console.log(`Using report ${REPORT_ID}`)

const browser = await chromium.launch({ headless: false, slowMo: 300 })
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } })

// 1) Settings — ChatGPT/remote only, no local models
step('Home → Model settings: hosted providers only, no local')
await page.goto(BASE)
await page.getByRole('button', { name: /Models/ }).click()
await page.locator('.settings').waitFor()
{
  const t = await txt(page)
  check('Settings drawer opens', /Model Settings|Model/i.test(t))
  check('No Ollama / LM Studio (local removed)', !/Ollama|LM Studio/i.test(t))
  check('No Remote/Local tab switcher', (await page.getByRole('tab').count()) === 0)
  check('ChatGPT subscription offered', /ChatGPT/i.test(t))
}
await page.locator('.settings .close, .settings').first().press('Escape').catch(() => {})
await page.mouse.click(30, 450) // click outside to dismiss

// 2) Report view — renders, tool call/result reveal, no stuck "Waiting"
step('Report view: body renders, tool Call/Result reveal params + output')
await page.goto(`${BASE}/report/${REPORT_ID}`)
await page.getByText(/Executive Summary|Stance Distribution/i).first().waitFor({ timeout: 60000 })
{
  const t = await txt(page)
  check('Report body renders (not stuck on Waiting)', !/Waiting for Report Agent/.test(t))
  const toolNames = await page.locator('.tool-badge').allInnerTexts().catch(() => [])
  check('Tool names render (was blank)', toolNames.some(n => /search_posts|statistics/i.test(n)))
  // expand the first "Show Params"
  const paramsBtn = page.getByRole('button', { name: /Show Params/i }).first()
  if (await paramsBtn.count()) {
    await paramsBtn.click()
    const params = await page.locator('.tool-params').first().innerText().catch(() => '')
    check('Show Params reveals the tool arguments', /query|limit/i.test(params))
  } else check('Show Params control present', false)
}

// 3) Collapsible sidebar — present on the shell views, toggle collapses/expands
step('Sidebar: collapse to icon rail and expand back')
await page.goto(`${BASE}/interaction/${REPORT_ID}`)
await page.locator('.app-sidebar').waitFor({ timeout: 15000 })
{
  const expanded = await page.locator('.app-sidebar').evaluate(el => el.getBoundingClientRect().width)
  check('Sidebar renders expanded (~240px)', expanded > 200)
  await page.locator('.collapse-toggle').click()
  await page.waitForTimeout(400)
  const collapsed = await page.locator('.app-sidebar').evaluate(el => el.getBoundingClientRect().width)
  check('Collapses to a narrow rail (<100px)', collapsed < 100)
  await page.locator('.collapse-toggle').click()
  await page.waitForTimeout(400)
  const reexpanded = await page.locator('.app-sidebar').evaluate(el => el.getBoundingClientRect().width)
  check('Expands back', reexpanded > 200)
  check('No Knowledge Pad in the view switcher', !(await txt(page)).includes('Knowledge Pad'))
}

// 4) Chat with any agent — real names + roles, no "Unknown Profession"
step('Interaction: Chat with any agent shows real names + faction roles')
{
  await page.getByText(/Chat with any agent/i).first().click()
  await page.getByText(/SELECT CHAT TARGET/i).waitFor({ timeout: 8000 }).catch(() => {})
  const t = await txt(page)
  check('No "Unknown Profession"', !/Unknown Profession/.test(t))
  check('Real persona names shown (e.g. Ministry / Transport / Sokha)', /Ministry|Transport|Sokha|National Institute/i.test(t))
  check('Faction roles shown (Opponent / Supporter / Neutral)', /Opponent|Supporter|Neutral/i.test(t))
}

console.log(`\n──────── UI FLOW: ${pass} passed, ${fail} failed ────────`)
await page.waitForTimeout(90000) // hold the window open ~1.5 min to inspect
await browser.close()
process.exit(fail ? 1 : 0)
