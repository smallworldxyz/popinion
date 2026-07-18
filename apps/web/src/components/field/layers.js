// FIELD flow & fracture layers (redesign-plan §8.5 item 5), plus the stance
// isopleths from #5. All computed from data that already exists on the client:
// per-mark position + stance trajectory. The counterfactual two-futures replay
// (fork at round N, inject a counter-message, draw both futures) needs a backend
// fork endpoint that does not exist yet, so that part is data-gated and no-ops
// upstream in FieldMap; here we render only what is derivable:
//
//   isopleths — density of where con / pro concentrate (as #5)
//   fracture  — the zero-stance divide, drawn hard where the gradient is steep
//   flow      — per-mark ribbons along the local stance gradient, sized by how
//               much the mark moved this round (its trajectory delta)
//
// None of this is fabricated: fracture is the real zero-crossing of the sampled
// stance field, flow direction is the real field gradient, flow magnitude is the
// real per-round delta. Where there is no movement (single-round runs) flow is
// empty and the layer honestly shows nothing.

import { contourDensity, contours, geoPath, geoTransform } from 'd3'

const FLOW_MIN_DELTA = 0.05
const FLOW_MAX = 80

export function computeLayers(marks, stanceOf, deltaOf, W, H, pad) {
  const empty = { iso: { con: [], pro: [] }, fracture: null, flow: [] }
  if (!W || !H || !marks.length) return empty
  const px = (v) => pad + v * (W - 2 * pad)
  const py = (v) => (H - pad) - v * (H - 2 * pad)
  const eligible = marks.filter((m) => m.eligible !== false)
  if (!eligible.length) return empty

  const path = geoPath(null)
  const density = (pred) =>
    contourDensity().x((m) => px(m._x)).y((m) => py(m._y))
      .size([Math.round(W), Math.round(H)]).bandwidth(26).thresholds(5)(eligible.filter(pred))
  const iso = {
    con: density((m) => stanceOf(m) < -0.2).map((c) => path(c)),
    pro: density((m) => stanceOf(m) > 0.2).map((c) => path(c)),
  }

  // Sample a coarse stance field by inverse-distance weighting, shared by the
  // fracture (its zero-crossing) and the flow (its gradient).
  const step = 16
  const gw = Math.max(3, Math.ceil(W / step) + 1)
  const gh = Math.max(3, Math.ceil(H / step) + 1)
  const field = new Float32Array(gw * gh)
  const pts = eligible.map((m) => ({ x: px(m._x), y: py(m._y), s: stanceOf(m) }))
  for (let j = 0; j < gh; j++) {
    for (let i = 0; i < gw; i++) {
      const cx = i * step, cy = j * step
      let ws = 0, acc = 0
      for (const p of pts) { const dx = cx - p.x, dy = cy - p.y; const w = 1 / (dx * dx + dy * dy + 64); ws += w; acc += w * p.s }
      field[j * gw + i] = ws ? acc / ws : 0
    }
  }
  const at = (i, j) => field[Math.max(0, Math.min(gh - 1, j)) * gw + Math.max(0, Math.min(gw - 1, i))]

  // Fracture: the zero-stance contour, in pixel space. Strength = mean gradient
  // magnitude near the divide; a steep field is a real fault, a gentle one is a
  // seam. Drawn solid+bright when strong, faint+dashed when weak.
  let fracture = null
  const grid = geoPath(geoTransform({ point(x, y) { this.stream.point(x * step, y * step) } }))
  const geom = contours().size([gw, gh]).thresholds([0])(field)[0]
  if (geom && geom.coordinates.length) {
    let gsum = 0, gn = 0
    for (let j = 0; j < gh; j++) for (let i = 0; i < gw; i++) {
      if (Math.abs(at(i, j)) > 0.15) continue
      const gx = (at(i + 1, j) - at(i - 1, j)) / (2 * step)
      const gy = (at(i, j + 1) - at(i, j - 1)) / (2 * step)
      gsum += Math.hypot(gx, gy); gn++
    }
    const strength = gn ? gsum / gn : 0
    fracture = { d: grid(geom), strong: strength > 0.006 }
  }

  // Flow: for every mark that moved this round, a short ribbon along the local
  // field gradient, pointed the way the mark's stance went, length by |delta|.
  const flow = []
  for (const m of eligible) {
    const d = deltaOf(m)
    if (Math.abs(d) < FLOW_MIN_DELTA) continue
    const mx = px(m._x), my = py(m._y)
    const i = Math.round(mx / step), j = Math.round(my / step)
    let gx = (at(i + 1, j) - at(i - 1, j)) / (2 * step)
    let gy = (at(i, j + 1) - at(i, j - 1)) / (2 * step)
    const len = Math.hypot(gx, gy)
    if (len < 1e-6) continue
    const sign = Math.sign(d)
    gx = (gx / len) * sign; gy = (gy / len) * sign
    const L = 8 + Math.min(1, Math.abs(d)) * 34
    flow.push({ x1: mx, y1: my, x2: mx + gx * L, y2: my + gy * L, mag: Math.abs(d) })
  }
  flow.sort((a, b) => b.mag - a.mag)
  return { iso, fracture, flow: flow.slice(0, FLOW_MAX) }
}
