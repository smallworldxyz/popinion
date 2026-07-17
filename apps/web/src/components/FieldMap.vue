<template>
  <div class="field">
    <div class="stage">
      <div class="canvas-wrap" ref="wrap">
        <canvas ref="canvas" @mousemove="onMove" @mouseleave="clearHover"></canvas>
        <div class="axis oppose">OPPOSE</div>
        <div class="axis support">SUPPORT</div>
        <div class="count">{{ marks.length }} personas</div>
      </div>

      <div v-if="roundCount > 1" class="ribbon">
        <button class="play" @click="toggle">{{ playing ? '❚❚' : '▶' }}</button>
        <input class="scrub" type="range" min="0" :max="roundCount - 1" step="1"
               :value="round" @input="seek(+$event.target.value)" />
        <span class="round">r{{ round }}<span class="of">/{{ roundCount - 1 }}</span></span>
      </div>
    </div>

    <aside class="sheet">
      <template v-if="selected">
        <div class="sheet-name">{{ selected.name || selected.user_name }}</div>
        <div class="sheet-type">{{ selected.source_entity_type || 'entity' }}</div>
        <div class="sheet-stance" :class="scalarClass(selectedStance)">
          {{ scalarLabel(selectedStance) }}
          <span class="val">{{ selectedStance.toFixed(2) }}</span>
          <span v-if="selected.synthetic" class="synthetic">synthetic</span>
        </div>
        <div class="sheet-ev-head">Grounded in {{ (selected.evidence || []).length }} fact{{ (selected.evidence || []).length === 1 ? '' : 's' }}</div>
        <ul class="sheet-ev">
          <li v-for="(f, i) in (selected.evidence || []).slice(0, 8)" :key="i">{{ f }}</li>
          <li v-if="!(selected.evidence || []).length" class="none">No recorded facts.</li>
        </ul>
      </template>
      <div v-else class="sheet-empty">Hover a mark to read the Persona it stands for.</div>
    </aside>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onBeforeUnmount, watch } from 'vue'
import { contourDensity, geoPath } from 'd3'

const props = defineProps({
  personas: { type: Array, default: () => [] },
})

const wrap = ref(null)
const canvas = ref(null)
const selected = ref(null)
const round = ref(0)
const playing = ref(false)

// Stance colour anchors (approx RGB of the DESIGN.md oklch values), lerped by a
// scalar in [-1,1]: oppose (blue) -> neutral (pale) -> support (amber).
const OPPOSE = [70, 120, 224]
const NEUTRAL = [176, 178, 190]
const SUPPORT = [212, 158, 60]
const lerp = (a, b, t) => a + (b - a) * t
const stanceColor = (s) => {
  const t = Math.max(-1, Math.min(1, s))
  const [r, g, b] = t < 0
    ? [0, 1, 2].map(i => lerp(NEUTRAL[i], OPPOSE[i], -t))
    : [0, 1, 2].map(i => lerp(NEUTRAL[i], SUPPORT[i], t))
  return `rgb(${r | 0}, ${g | 0}, ${b | 0})`
}
const scalarClass = (s) => (s < -0.15 ? 'oppose' : s > 0.15 ? 'support' : 'neutral')
const scalarLabel = (s) => (s < -0.15 ? 'Opposes' : s > 0.15 ? 'Supports' : 'Neutral')

const factionScalar = (p) => (p.faction === 'con' ? -0.7 : p.faction === 'pro' ? 0.7 : 0)
const roundCount = computed(() => {
  let n = 1
  for (const p of props.personas) if (Array.isArray(p.trajectory)) n = Math.max(n, p.trajectory.length)
  return n
})
const stanceAt = (p, r) =>
  Array.isArray(p.trajectory) && p.trajectory.length ? p.trajectory[Math.min(r, p.trajectory.length - 1)] : factionScalar(p)

const selectedStance = computed(() => (selected.value ? stanceAt(selected.value, round.value) : 0))

// ---- layout (mirror of the backend; used when a Persona has no position) ----
function jitter(id) {
  let z = (BigInt(id) * 0x9E3779B97F4A7C15n + 0x2545F4914F6CDD1Dn) & 0xFFFFFFFFFFFFFFFFn
  z = ((z ^ (z >> 30n)) * 0xBF58476D1CE4E5B9n) & 0xFFFFFFFFFFFFFFFFn
  z ^= z >> 27n
  return [Number(z & 0xFFFFn) / 65536 - 0.5, Number((z >> 16n) & 0xFFFFn) / 65536 - 0.5]
}
function layout(personas) {
  const types = []
  for (const p of personas) { const t = p.source_entity_type || 'Entity'; if (!types.includes(t)) types.push(t) }
  const bands = Math.max(types.length, 1)
  return personas.map((p) => {
    const base = { ...p, cs: factionScalar(p), ts: factionScalar(p) }
    if (Array.isArray(p.position)) return { ...base, _x: p.position[0], _y: p.position[1] }
    const sx = p.faction === 'con' ? 0.2 : p.faction === 'pro' ? 0.8 : 0.5
    const band = types.indexOf(p.source_entity_type || 'Entity')
    const by = (band + 0.5) / bands
    const [jx, jy] = jitter(p.user_id ?? 0)
    return { ...base, _x: Math.min(0.98, Math.max(0.02, sx + jx * 0.12)), _y: Math.min(0.98, Math.max(0.02, by + jy * (0.8 / bands))) }
  })
}

const marks = ref([])
let ctx, dpr, W, H, ro, raf, animT0, iso = { con: [], pro: [] }
const PAD = 28
const px = (v) => PAD + v * (W - 2 * PAD)
const py = (v) => (H - PAD) - v * (H - 2 * PAD)

// Isopleths: density contours of where opposition and support concentrate at the
// current round, drawn like isobars. Recomputed per round (membership shifts as
// stance moves), so the ridges themselves drift, which is the weather-map read.
function computeIso() {
  if (!W || !marks.value.length) { iso = { con: [], pro: [] }; return }
  const density = (pred) =>
    contourDensity()
      .x((m) => px(m._x))
      .y((m) => py(m._y))
      .size([Math.round(W), Math.round(H)])
      .bandwidth(26)
      .thresholds(5)(marks.value.filter(pred))
  iso = {
    con: density((m) => m.ts < -0.2),
    pro: density((m) => m.ts > 0.2),
  }
}

function resize() {
  if (!canvas.value || !wrap.value) return
  dpr = window.devicePixelRatio || 1
  const r = wrap.value.getBoundingClientRect()
  W = r.width; H = r.height
  canvas.value.width = W * dpr; canvas.value.height = H * dpr
  canvas.value.style.width = W + 'px'; canvas.value.style.height = H + 'px'
  ctx = canvas.value.getContext('2d')
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
  computeIso()
  draw()
}

function chevron(x, y, s, kind) {
  ctx.beginPath()
  if (kind === 'oppose') { ctx.moveTo(x - s, y - s * 0.6); ctx.lineTo(x, y + s * 0.7); ctx.lineTo(x + s, y - s * 0.6) }
  else if (kind === 'support') { ctx.moveTo(x - s, y + s * 0.6); ctx.lineTo(x, y - s * 0.7); ctx.lineTo(x + s, y + s * 0.6) }
  else { ctx.moveTo(x - s, y); ctx.lineTo(x + s, y) }
}

function draw() {
  if (!ctx) return
  ctx.fillStyle = 'oklch(0.245 0.026 265)'
  ctx.fillRect(0, 0, W, H)
  ctx.strokeStyle = 'oklch(0.38 0.022 265 / 0.35)'
  ctx.lineWidth = 1
  for (let i = 1; i < 8; i++) { const g = (i / 8) * W; ctx.beginPath(); ctx.moveTo(g, 0); ctx.lineTo(g, H); ctx.stroke() }
  for (let i = 1; i < 6; i++) { const g = (i / 6) * H; ctx.beginPath(); ctx.moveTo(0, g); ctx.lineTo(W, g); ctx.stroke() }

  // Isopleths under the marks.
  const path = geoPath(null, ctx)
  for (const [rings, color] of [[iso.con, '70, 120, 224'], [iso.pro, '212, 158, 60']]) {
    rings.forEach((c, i) => {
      ctx.beginPath()
      path(c)
      ctx.strokeStyle = `rgba(${color}, ${0.1 + i * 0.06})`
      ctx.lineWidth = 1
      ctx.stroke()
    })
  }

  for (const m of marks.value) {
    const x = px(m._x), y = py(m._y)
    ctx.strokeStyle = stanceColor(m.cs)
    ctx.globalAlpha = m.synthetic ? 0.5 : 1
    ctx.lineWidth = m === selected.value ? 3 : 2
    chevron(x, y, m === selected.value ? 7 : 5, scalarClass(m.cs))
    ctx.stroke()
  }
  ctx.globalAlpha = 1
  if (selected.value) {
    ctx.strokeStyle = 'oklch(0.92 0.012 265)'; ctx.globalAlpha = 0.6; ctx.lineWidth = 1
    ctx.beginPath(); ctx.arc(px(selected.value._x), py(selected.value._y), 12, 0, Math.PI * 2); ctx.stroke()
    ctx.globalAlpha = 1
  }
}

// Recolour sweep: marks ease toward the round's target stance, staggered left to
// right so influence visibly travels across the field. Colour only, no layout.
const SWEEP_MS = 850
function animate(ts) {
  if (!animT0) animT0 = ts
  const elapsed = ts - animT0
  let moving = false
  for (const m of marks.value) {
    const delay = m._x * 350 // left edge moves first
    const p = Math.max(0, Math.min(1, (elapsed - delay) / (SWEEP_MS - 350)))
    const e = 1 - Math.pow(1 - p, 4) // ease-out-quart
    const next = m.cs + (m.ts - m.cs) * (p >= 1 ? 1 : e * 0.35)
    if (Math.abs(m.ts - next) > 0.002) moving = true
    m.cs = p >= 1 ? m.ts : next
  }
  draw()
  if (moving) raf = requestAnimationFrame(animate)
  else raf = null
}
function retarget() {
  for (const m of marks.value) m.ts = stanceAt(m, round.value)
  computeIso()
  animT0 = 0
  if (!raf) raf = requestAnimationFrame(animate)
}
function seek(r) { round.value = r; retarget() }
function toggle() { playing.value = !playing.value; if (playing.value) tick() }
function tick() {
  if (!playing.value) return
  if (round.value >= roundCount.value - 1) { playing.value = false; return }
  seek(round.value + 1)
  playTimer = setTimeout(tick, 900)
}
let playTimer

function onMove(e) {
  const r = canvas.value.getBoundingClientRect()
  const mx = e.clientX - r.left, my = e.clientY - r.top
  let best = null, bestD = 16 * 16
  for (const m of marks.value) {
    const dx = px(m._x) - mx, dy = py(m._y) - my, d = dx * dx + dy * dy
    if (d < bestD) { bestD = d; best = m }
  }
  if (best !== selected.value) { selected.value = best; draw() }
}
function clearHover() { selected.value = null; draw() }

watch(() => props.personas, (v) => {
  marks.value = layout(v || [])
  round.value = 0
  for (const m of marks.value) { m.cs = stanceAt(m, 0); m.ts = m.cs }
  computeIso()
  draw()
}, { immediate: true })

onMounted(() => { resize(); ro = new ResizeObserver(resize); ro.observe(wrap.value) })
onBeforeUnmount(() => { ro && ro.disconnect(); raf && cancelAnimationFrame(raf); clearTimeout(playTimer) })
</script>

<style scoped>
.field { display: flex; height: 100%; min-height: 520px; background: oklch(0.19 0.021 265); color: oklch(0.88 0.012 265); }
.stage { flex: 1; min-width: 0; display: flex; flex-direction: column; }
.canvas-wrap { position: relative; flex: 1; min-height: 0; }
canvas { display: block; }
.axis { position: absolute; bottom: 8px; font: 600 11px/1 ui-monospace, monospace; letter-spacing: .1em; color: oklch(0.6 0.02 265); }
.axis.oppose { left: 12px; }
.axis.support { right: 12px; }
.count { position: absolute; top: 10px; right: 12px; font: 500 11px/1 ui-monospace, monospace; color: oklch(0.6 0.02 265); }
.ribbon { display: flex; align-items: center; gap: 12px; padding: 10px 16px; border-top: 1px solid oklch(0.32 0.02 265); background: oklch(0.22 0.024 265); }
.play { width: 30px; height: 26px; border: 1px solid oklch(0.4 0.03 265); background: oklch(0.28 0.028 265); color: oklch(0.9 0.01 265); border-radius: 6px; cursor: pointer; font-size: 11px; }
.scrub { flex: 1; accent-color: oklch(0.66 0.14 74); }
.round { font: 600 12px/1 ui-monospace, monospace; color: oklch(0.85 0.01 265); }
.round .of { color: oklch(0.55 0.02 265); }
.sheet { width: 280px; flex: none; border-left: 1px solid oklch(0.32 0.02 265); padding: 20px 18px; overflow-y: auto; background: oklch(0.22 0.024 265); }
.sheet-name { font-size: 16px; font-weight: 650; }
.sheet-type { font-size: 12px; color: oklch(0.62 0.02 265); margin-top: 2px; }
.sheet-stance { margin-top: 12px; font: 600 12px/1 ui-monospace, monospace; letter-spacing: .04em; }
.sheet-stance.oppose { color: oklch(0.62 0.16 252); }
.sheet-stance.support { color: oklch(0.7 0.15 74); }
.sheet-stance.neutral { color: oklch(0.78 0.02 265); }
.sheet-stance .val { margin-left: 8px; color: oklch(0.6 0.02 265); font-weight: 400; }
.synthetic { margin-left: 8px; color: oklch(0.55 0.03 265); font-weight: 400; }
.sheet-ev-head { margin-top: 18px; font-size: 12px; color: oklch(0.66 0.018 265); }
.sheet-ev { list-style: none; padding: 0; margin: 8px 0 0; display: flex; flex-direction: column; gap: 8px; }
.sheet-ev li { font-size: 12.5px; line-height: 1.45; color: oklch(0.82 0.012 265); padding-left: 10px; border-left: 1px solid oklch(0.34 0.02 265); }
.sheet-ev li.none { color: oklch(0.55 0.02 265); border: none; }
.sheet-empty { color: oklch(0.55 0.02 265); font-size: 13px; line-height: 1.5; }
</style>
