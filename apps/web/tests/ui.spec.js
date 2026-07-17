import { test, expect } from '@playwright/test'

// The model-readiness probe gates Start Engine. Stub it so these UI tests don't
// depend on a local model actually being installed and fast enough to answer.
const stubModelReady = (page) =>
  page.route('**/api/settings/llm/status', (route) =>
    route.fulfill({
      json: { success: true, data: { ready: true, model: 'test-model', base_url: 'http://stub/v1', reason: '' } },
    })
  )

// Whether a real ChatGPT session exists on this machine decides whether the card
// offers "Sign in" or "Sign out". Pin it so the assertion is about the UI.
const stubChatgptSession = (page, logged_in) =>
  page.route('**/api/settings/llm/chatgpt/status', (route) =>
    route.fulfill({
      json: {
        success: true,
        data: logged_in
          ? { logged_in: true, email: 'someone@example.com', plan: 'plus', status: 'idle' }
          : { logged_in: false, email: '', plan: '', status: 'idle' },
      },
    })
  )

test.describe('Home — reality seeds', () => {
  test.beforeEach(async ({ page }) => {
    await stubModelReady(page)
    await page.goto('/')
  })

  test('Start Engine unlocks only with a prompt AND a seed', async ({ page }) => {
    const start = page.locator('.start-engine-btn')
    await expect(start).toBeDisabled()

    // A prompt alone is not enough — the sim must be grounded in real data.
    await page.locator('.chat-input').fill('Will the public accept a bus fare rise?')
    await expect(start).toBeDisabled()

    await page.getByRole('button', { name: /Write text/ }).click()
    await page.locator('.text-seed-input').fill('Sophea: too expensive. Against.')
    await expect(start).toBeEnabled()
  })

  test('written text uploads as a markdown file', async ({ page }) => {
    let body = null
    await page.route('**/api/graph/ontology/generate', async (route) => {
      body = route.request().postData()
      await route.fulfill({ status: 400, json: { success: false, error: 'stubbed' } })
    })

    await page.locator('.chat-input').fill('Will the public accept a bus fare rise?')
    await page.getByRole('button', { name: /Write text/ }).click()
    await page.locator('.text-seed-input').fill('# Notes\n\nSophea: too expensive. Against.')
    await page.locator('.start-engine-btn').click()

    await expect.poll(() => body).not.toBeNull()
    expect(body).toContain('filename="written-note.md"')
    expect(body).toContain('Sophea: too expensive. Against.')
    expect(body).toContain('Will the public accept a bus fare rise?')
  })

  test('seed toggles are reversible and stay inside the box', async ({ page }) => {
    await page.getByRole('button', { name: /Write text/ }).click()
    await expect(page.locator('.text-seed-input')).toBeVisible()
    await page.locator('.text-seed .chip-x').click()
    await expect(page.locator('.text-seed-input')).toHaveCount(0)

    // Three attach buttons + the CTA must not overflow a narrow chat box.
    await page.setViewportSize({ width: 400, height: 900 })
    const box = await page.locator('.chat-box').boundingBox()
    for (const b of await page.locator('.chat-toolbar .attach-btn').all()) {
      const r = await b.boundingBox()
      expect(r.x + r.width).toBeLessThanOrEqual(box.x + box.width + 1)
    }
  })
})

test.describe('Worlds — saved runs are grouped and reachable', () => {
  // Two runs in one World, one in another, matching projects for labels.
  const stubWorlds = (page) => {
    page.route('**/api/simulation/list*', (route) =>
      route.fulfill({
        json: {
          success: true,
          data: {
            simulations: [
              { simulation_id: 's1', name: 'Base run', graph_id: 'g-fuel', kind: 'canonical', parent_id: null, status: 'completed', num_agents: 42, created_at: '2026-07-16T10:00:00Z' },
              { simulation_id: 's2', name: 'With subsidy', graph_id: 'g-fuel', kind: 'alt', parent_id: 's1', status: 'prepared', num_agents: 42, created_at: '2026-07-17T10:00:00Z' },
              { simulation_id: 's3', name: 'Telecom base', graph_id: 'g-tel', kind: 'canonical', parent_id: null, status: 'created', num_agents: 0, created_at: '2026-07-15T10:00:00Z' },
            ],
          },
        },
      })
    )
    page.route('**/api/graph/projects', (route) =>
      route.fulfill({
        json: {
          success: true,
          data: {
            projects: [
              { project_id: 'p1', name: 'Fuel levy', graph_id: 'g-fuel', node_count: 61, edge_count: 88 },
              { project_id: 'p2', name: 'Telecom price', graph_id: 'g-tel', node_count: 30, edge_count: 12 },
            ],
          },
        },
      })
    )
  }

  test('the index groups runs into Worlds, not a flat list', async ({ page }) => {
    stubWorlds(page)
    await page.goto('/worlds')
    await expect(page.locator('.world')).toHaveCount(2)
    await expect(page.getByText('Fuel levy')).toBeVisible()
    await expect(page.getByText('2 runs')).toBeVisible() // the World with two runs
  })

  test('a World lists its runs with lineage and status, and reopens one', async ({ page }) => {
    stubWorlds(page)
    await page.goto('/world/g-fuel')
    await expect(page.locator('.run')).toHaveCount(2)
    // Canonical sorts before the derived alt.
    await expect(page.locator('.run .kind').first()).toHaveText('canonical')
    await expect(page.getByText('↳ derived')).toBeVisible()
    await expect(page.locator('.badge.completed')).toBeVisible()

    await page.locator('.run').first().click()
    await expect(page).toHaveURL(/\/simulation\/s1$/)
  })
})

test.describe('Settings — model selection', () => {
  test.beforeEach(async ({ page }) => {
    await stubModelReady(page)
    await page.goto('/settings')
  })

  test('one model by default; the split is opt-in', async ({ page }) => {
    await expect(page.locator('.card')).toHaveCount(1)
    await expect(page.locator('.card h3')).toHaveText('Model')

    await page.locator('.split input').check()
    await expect(page.locator('.card')).toHaveCount(2)
    await expect(page.locator('.card h3')).toHaveText(['Main model', 'Report & persona model'])

    await page.locator('.split input').uncheck()
    await expect(page.locator('.card')).toHaveCount(1)
  })

  test('remote and local providers are separated by tab', async ({ page }) => {
    await page.getByRole('tab', { name: 'Local' }).click()
    await expect(page.locator('.prov')).toHaveText([/Ollama/, /LM Studio/])

    await page.getByRole('tab', { name: 'Remote' }).click()
    const labels = await page.locator('.prov-label').allInnerTexts()
    expect(labels).toEqual(expect.arrayContaining(['Moonshot (Kimi)', 'DeepSeek', 'Anthropic']))
    // Same vendor, two products — the label must say which is which, for both
    // the OpenAI/ChatGPT pair and Z.ai's subscription-vs-metered pair.
    expect(labels).toContain('ChatGPT (subscription)')
    expect(labels).toContain('OpenAI (API key)')
    expect(labels).toContain('Z.ai GLM (coding plan)')
    expect(labels).toContain('Z.ai GLM (pay per token)')
  })

  test('the Z.ai coding plan points at the subscription endpoint, not the wallet one', async ({ page }) => {
    const baseUrl = page.locator('.card input.input').first()

    // Same key authenticates on both; the wrong base_url silently bills wallet
    // balance instead of the plan, so the two must not be confusable.
    await page.getByRole('tab', { name: 'Remote' }).click()
    await page.locator('.prov', { hasText: 'Z.ai GLM (coding plan)' }).click()
    await expect(baseUrl).toHaveValue('https://api.z.ai/api/coding/paas/v4')

    await page.locator('.prov', { hasText: 'Z.ai GLM (pay per token)' }).click()
    await expect(baseUrl).toHaveValue('https://api.z.ai/api/paas/v4')
  })

  test('picking ChatGPT needs no key and no typing', async ({ page }) => {
    await stubChatgptSession(page, false)
    await page.reload()
    await page.getByRole('tab', { name: 'Remote' }).click()
    await page.locator('.prov', { hasText: 'ChatGPT (subscription)' }).click()

    // A model is preselected, so a working setup needs no typing. It must be one
    // Codex actually accepts for ChatGPT-account auth — gpt-5/gpt-5-codex are
    // retired on this path and 400 at request time.
    await expect(page.locator('.card input.input').nth(1)).toHaveValue('gpt-5.6-terra')
    await expect(page.getByRole('button', { name: /Sign in with ChatGPT/ })).toBeVisible()
    // Subscription auth — an API key *field* here would be a lie. (Match the
    // input, not the text: "no API key" appears in the copy.)
    await expect(page.locator('.card input[type="password"]')).toHaveCount(0)
  })

  test('a signed-in ChatGPT session shows the account, not a sign-in prompt', async ({ page }) => {
    await stubChatgptSession(page, true)
    await page.reload()
    await page.getByRole('tab', { name: 'Remote' }).click()
    await page.locator('.prov', { hasText: 'ChatGPT (subscription)' }).click()

    await expect(page.getByText('someone@example.com · plus')).toBeVisible()
    await expect(page.getByRole('button', { name: 'Sign out' })).toBeVisible()
    await expect(page.getByRole('button', { name: /Sign in with ChatGPT/ })).toHaveCount(0)
  })

  test('the Ollama pull control stays out of the Remote tab', async ({ page }) => {
    await page.getByRole('tab', { name: 'Local' }).click()
    await page.locator('.prov', { hasText: 'Ollama' }).click()
    await expect(page.getByText('Pull a model')).toBeVisible()

    await page.getByRole('tab', { name: 'Remote' }).click()
    await expect(page.getByText('Pull a model')).toHaveCount(0)
  })

  test('Back link and the readiness badge never collide', async ({ page }) => {
    for (const width of [1440, 900, 400]) {
      await page.setViewportSize({ width, height: 900 })
      const back = await page.locator('.back').boundingBox()
      const ready = await page.locator('.ready-line').boundingBox()
      const overlaps =
        ready.x < back.x + back.width && ready.x + ready.width > back.x &&
        ready.y < back.y + back.height && ready.y + ready.height > back.y
      expect(overlaps, `overlap at ${width}px`).toBe(false)
    }
  })
})
