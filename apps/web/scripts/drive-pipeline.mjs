// Drives the real Home → Process flow in a browser against the live API and the
// configured model. Unlike tests/ui.spec.js (which stubs the model), this spends
// real LLM calls — run it deliberately, not in CI.
//
//   bun run scripts/drive-pipeline.mjs [minutes]
import { chromium } from '@playwright/test'

const BUDGET_MIN = Number(process.argv[2] || 20)
const OUT = process.env.SHOT_DIR || '/tmp/popinion-pipeline'
const t0 = Date.now()
const stamp = () => `${String(Math.floor((Date.now() - t0) / 1000)).padStart(4)}s`
const log = (...a) => console.log(stamp(), ...a)

const CORPUS = `# Phnom Penh bus fare debate — public posts, June 2026

Sophea Chan (student): I take the C3 bus daily. Raising the fare from 1500 to 3000 riel would eat my whole food budget. Strongly against.
Dara Kim (tuk-tuk driver): The bus fare going up is good for me, more passengers come back to tuk-tuks. But I feel bad for students. Support only if drivers get the money.
Ministry of Public Works: The subsidy is unsustainable. Without a fare adjustment to 3000 riel we cannot maintain the fleet. We support the phased increase.
Bopha Sok (nurse): I work night shifts and the bus is already unreliable. If they raise the fare they must add night service. Neutral, leaning against.
Vichea Long (economist): A fare increase without a means-tested student pass is regressive. Bangkok saw ridership drop 20 percent after such hikes. Oppose the flat increase.
Chenda Meas (market vendor): I carry goods on the bus every morning. 3000 riel twice a day is 6000. Too much. Against.
Student Union of Cambodia: We reject the fare increase. Over 4000 students signed our petition. We demand a student pass at 750 riel.
Transport Workers Association: Our drivers have not had a raise in three years. We support the increase because management promised 15 percent of new revenue goes to wages.`

// Headed and slowed down on purpose: this is meant to be watched.
const browser = await chromium.launch({
  headless: process.env.HEADLESS === '1',
  slowMo: 600,
  args: ['--window-position=0,0', '--window-size=1500,1000'],
})
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } })
const shot = (n) => page.screenshot({ path: `${OUT}/${n}.png`, fullPage: false }).catch(() => {})

// The API calls are the ground truth — the UI can look busy while nothing works.
page.on('response', async (r) => {
  const u = new URL(r.url()).pathname
  if (!u.startsWith('/api/')) return
  if (u.includes('/status') || u.includes('/task/')) return // pollers: too chatty
  let detail = ''
  try {
    const b = await r.text()
    detail = b.length > 220 ? b.slice(0, 220) + '…' : b
  } catch {}
  log(`HTTP ${r.status()} ${u} ${detail}`)
})
page.on('pageerror', (e) => log('PAGE ERROR', e.message))

// Optionally switch the model through the real Settings UI first.
if (process.env.MODEL) {
  log(`switching model to ${process.env.MODEL} via Settings…`)
  await page.goto('http://localhost:3000/settings')
  await page.locator('.chip', { hasText: process.env.MODEL }).first().click()
  await page.locator('.save').click()
  await page.waitForTimeout(2000)
  await shot('00-model')
}

await page.goto('http://localhost:3000/')
log('typing the scenario…')
await page.locator('.chat-input').pressSequentially(
  'Will the public accept raising the Phnom Penh bus fare from 1500 to 3000 riel?',
  { delay: 25 }
)
log('attaching a written reality seed…')
await page.getByRole('button', { name: /Write text/ }).click()
await page.locator('.text-seed-input').fill(CORPUS)
await page.waitForTimeout(1500)
await shot('01-seeded')

const start = page.locator('.start-engine-btn')
if (!(await start.isEnabled())) throw new Error('Start Engine disabled — is a model configured and ready?')
log('starting engine…')
await start.click()
await page.waitForURL(/\/process\//, { timeout: 30_000 })
log('on', page.url())

// Watch until the graph is built, a hard failure surfaces, or the budget runs out.
const deadline = Date.now() + BUDGET_MIN * 60_000
let last = ''
while (Date.now() < deadline) {
  const body = await page.locator('body').innerText().catch(() => '')
  const line = body.split('\n').map((s) => s.trim()).filter(Boolean).slice(0, 6).join(' | ')
  if (line !== last) {
    log('UI:', line.slice(0, 160))
    last = line
  }
  if (/failed|error/i.test(body) && !/no error/i.test(body)) {
    await shot('99-failed')
    log('FAILURE text on page')
    break
  }
  await page.waitForTimeout(5000)
}

await shot('02-final')
log('done — screenshots in', OUT)
await browser.close()
