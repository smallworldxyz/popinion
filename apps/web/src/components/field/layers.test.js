// Runnable check for the flow/fracture math (bun test). The e2e suite exercises
// the rendered map; this pins the derived geometry so a refactor can't silently
// break the fault line or the flow ribbons.
import { test, expect } from 'bun:test'
import { computeLayers } from './layers'

const mk = (id, x, y, stance, delta, eligible = true) => ({
  user_id: id, _x: x, _y: y, eligible,
  _stance: stance, _delta: delta,
})
const stanceOf = (m) => m._stance
const deltaOf = (m) => m._delta

// A con block on the left, a pro block on the right: a real divide down the middle.
const split = []
for (let i = 0; i < 40; i++) split.push(mk(i, 0.15 + (i % 4) * 0.02, 0.2 + i * 0.015, -0.7, -0.2))
for (let i = 0; i < 40; i++) split.push(mk(100 + i, 0.8 + (i % 4) * 0.02, 0.2 + i * 0.015, 0.7, 0.1))

test('a split population opens a fracture and emits flow for movers', () => {
  const { iso, fracture, flow } = computeLayers(split, stanceOf, deltaOf, 800, 600, 28)
  expect(iso.con.length).toBeGreaterThan(0)
  expect(iso.pro.length).toBeGreaterThan(0)
  expect(fracture).not.toBeNull()
  expect(fracture.d).toContain('M')
  expect(flow.length).toBeGreaterThan(0)
  expect(flow.every((f) => Number.isFinite(f.x1) && Number.isFinite(f.x2))).toBe(true)
})

test('marks that did not move emit no flow', () => {
  const still = split.map((m) => ({ ...m, _delta: 0 }))
  const { flow } = computeLayers(still, stanceOf, deltaOf, 800, 600, 28)
  expect(flow.length).toBe(0)
})

test('degenerate inputs never throw', () => {
  expect(computeLayers([], stanceOf, deltaOf, 800, 600, 28).flow).toEqual([])
  const refused = split.map((m) => ({ ...m, eligible: false }))
  const out = computeLayers(refused, stanceOf, deltaOf, 800, 600, 28)
  expect(out.fracture).toBeNull()
  expect(out.iso.con).toEqual([])
  expect(computeLayers(split, stanceOf, deltaOf, 0, 0, 28).flow).toEqual([])
})
