<template>
  <div class="field">
    <div class="stage">
      <div class="canvas-wrap" ref="wrap">
        <svg class="vector" :viewBox="`0 0 ${W} ${H}`" :width="W" :height="H" aria-hidden="true">
          <defs>
            <marker id="flow-head" markerWidth="5" markerHeight="5" refX="4" refY="2.5" orient="auto">
              <path d="M0,0 L5,2.5 L0,5 Z" fill="rgba(230,166,86,0.7)" />
            </marker>
          </defs>
          <g class="grid">
            <line v-for="(gx, i) in gridX" :key="'vx' + i" :x1="gx" y1="0" :x2="gx" :y2="H" />
            <line v-for="(gy, i) in gridY" :key="'hy' + i" x1="0" :y1="gy" :x2="W" :y2="gy" />
          </g>
          <g class="iso">
            <path v-for="(d, i) in layers.iso.con" :key="'c' + i" :d="d" :style="{ stroke: `rgba(245,144,120,${0.12 + i * 0.06})` }" />
            <path v-for="(d, i) in layers.iso.pro" :key="'p' + i" :d="d" :style="{ stroke: `rgba(31,176,163,${0.14 + i * 0.06})` }" />
          </g>
          <path v-if="layers.fracture" class="fracture" :class="{ strong: layers.fracture.strong }" :d="layers.fracture.d" />
          <g class="flow">
            <line v-for="(f, i) in layers.flow" :key="'f' + i" :x1="f.x1" :y1="f.y1" :x2="f.x2" :y2="f.y2"
                  :style="{ opacity: 0.18 + Math.min(1, f.mag) * 0.6 }" marker-end="url(#flow-head)" />
          </g>
        </svg>
        <canvas ref="canvas" class="marks" @mousemove="onMove" @mouseleave="clearHover"></canvas>
        <div v-if="selected" class="sel-ring" :style="{ left: mx(selected) + 'px', top: my(selected) + 'px' }"></div>
        <div class="axis oppose">OPPOSE</div>
        <div class="axis support">SUPPORT</div>
        <div class="count">{{ shownCount }} personas<span v-if="refusedCount" class="refused"> · {{ refusedCount }} refused</span></div>
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
import { createMarkRenderer } from './field/markRenderer'
import { computeLayers } from './field/layers'
import { isRefused, easeOutQuint, scalarClass, scalarLabel, SWEEP_MS, STAGGER_MS } from './field/marks'

const props = defineProps({
  personas: { type: Array, default: () => [] },
})

const wrap = ref(null)
const canvas = ref(null)
const selected = ref(null)
const round = ref(0)
const playing = ref(false)
const shownCount = ref(0)
const refusedCount = ref(0)
const layers = ref({ iso: { con: [], pro: [] }, fracture: null, flow: [] })
const W = ref(0)
const H = ref(0)
const PAD = 28

// The 1000 marks are held in a plain array, deliberately out of Vue's reactivity
// (redesign-plan §10): the render loop owns them, Vue owns the DOM chrome.
let marks = []
let fromArr = new Float32Array(0)
let toArr = new Float32Array(0)
let delayArr = new Float32Array(0)
let renderer = null
let raf = null
let startMs = -1e9
let dpr = 1
let selIdx = -1
let playTimer

const gridX = computed(() => Array.from({ length: 7 }, (_, i) => ((i + 1) / 8) * W.value))
const gridY = computed(() => Array.from({ length: 5 }, (_, i) => ((i + 1) / 6) * H.value))

const factionScalar = (p) => (p.faction === 'con' ? -0.7 : p.faction === 'pro' ? 0.7 : 0)
const roundCount = computed(() => {
  let n = 1
  for (const p of props.personas) if (Array.isArray(p.trajectory)) n = Math.max(n, p.trajectory.length)
  return n
})
const stanceAt = (p, r) =>
  Array.isArray(p.trajectory) && p.trajectory.length ? p.trajectory[Math.min(r, p.trajectory.length - 1)] : factionScalar(p)
const deltaAt = (p, r) => stanceAt(p, r) - (r > 0 ? stanceAt(p, r - 1) : stanceAt(p, 0))

const selectedStance = computed(() => (selected.value ? stanceAt(selected.value, round.value) : 0))
const px = (v) => PAD + v * (W.value - 2 * PAD)
const py = (v) => (H.value - PAD) - v * (H.value - 2 * PAD)
const mx = (m) => px(m._x)
const my = (m) => py(m._y)

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
    if (Array.isArray(p.position)) return { ...p, _x: p.position[0], _y: p.position[1] }
    const sx = p.faction === 'con' ? 0.2 : p.faction === 'pro' ? 0.8 : 0.5
    const band = types.indexOf(p.source_entity_type || 'Entity')
    const by = (band + 0.5) / bands
    const [jx, jy] = jitter(p.user_id ?? 0)
    return { ...p, _x: Math.min(0.98, Math.max(0.02, sx + jx * 0.12)), _y: Math.min(0.98, Math.max(0.02, by + jy * (0.8 / bands))) }
  })
}

function recomputeLayers() {
  layers.value = computeLayers(marks, (m) => stanceAt(m, round.value), (m) => deltaAt(m, round.value), W.value, H.value, PAD)
}

function frame() {
  const elapsed = (performance.now() - startMs) / 1000
  renderer.render(elapsed, SWEEP_MS / 1000, selIdx)
  if (elapsed * 1000 < STAGGER_MS + SWEEP_MS) raf = requestAnimationFrame(frame)
  else raf = null
}
function paint() {
  if (!renderer) return
  if (raf) return // the loop is already painting
  renderer.render((performance.now() - startMs) / 1000, SWEEP_MS / 1000, selIdx)
}

// Advance to round r: freeze each mark's currently-shown colour as the sweep's
// start, retarget to round r, and stagger the delays outward from the biggest
// mover (the Persona whose stance shifted most). Colour only, layout never moves.
function retarget(r) {
  const n = marks.length
  const now = performance.now()
  const prevElapsed = (now - startMs) / 1000
  const dur = SWEEP_MS / 1000
  const from = new Float32Array(n)
  const to = new Float32Array(n)
  for (let i = 0; i < n; i++) {
    const t = Math.max(0, Math.min(1, (prevElapsed - delayArr[i]) / dur))
    from[i] = fromArr[i] + (toArr[i] - fromArr[i]) * easeOutQuint(t)
    to[i] = stanceAt(marks[i], r)
  }
  let oi = -1, best = 0
  for (let i = 0; i < n; i++) { const d = Math.abs(to[i] - from[i]); if (d > best) { best = d; oi = i } }
  const delay = new Float32Array(n)
  if (oi >= 0 && best > 1e-3) {
    const ox = marks[oi]._x, oy = marks[oi]._y
    let maxD = 1e-6
    for (let i = 0; i < n; i++) { const d = Math.hypot(marks[i]._x - ox, marks[i]._y - oy); if (d > maxD) maxD = d }
    for (let i = 0; i < n; i++) delay[i] = (Math.hypot(marks[i]._x - ox, marks[i]._y - oy) / maxD) * (STAGGER_MS / 1000)
  }
  fromArr = from; toArr = to; delayArr = delay
  renderer.setRound(from, to, delay)
  startMs = now
  round.value = r
  recomputeLayers()
  if (!raf) raf = requestAnimationFrame(frame)
}
function seek(r) { retarget(r) }

function toggle() { playing.value = !playing.value; if (playing.value) tick() }
function tick() {
  if (!playing.value) return
  if (round.value >= roundCount.value - 1) { playing.value = false; return }
  seek(round.value + 1)
  playTimer = setTimeout(tick, 900)
}

// Picking reads a 1px offscreen ID buffer, which forces a GPU sync. Coalesce to
// at most one read per frame so a fast drag can't stall the pipeline repeatedly
// (the WebKitGTK/Tauri target is the sensitive one).
let pendingPick = null
let pickRaf = null
function runPick() {
  pickRaf = null
  if (!pendingPick || !renderer) return
  const r = canvas.value.getBoundingClientRect()
  const idx = renderer.pick(pendingPick.x - r.left, pendingPick.y - r.top)
  pendingPick = null
  const next = idx >= 0 ? marks[idx] : null
  if (next !== selected.value) { selected.value = next; selIdx = idx; paint() }
}
function onMove(e) {
  pendingPick = { x: e.clientX, y: e.clientY }
  if (!pickRaf) pickRaf = requestAnimationFrame(runPick)
}
function clearHover() { pendingPick = null; if (selected.value) { selected.value = null; selIdx = -1; paint() } }

function rebuild() {
  marks = layout(props.personas || [])
  const n = marks.length
  shownCount.value = marks.filter((m) => !isRefused(m)).length
  refusedCount.value = marks.filter(isRefused).length
  selected.value = null; selIdx = -1
  round.value = 0
  fromArr = new Float32Array(n)
  toArr = new Float32Array(n)
  delayArr = new Float32Array(n)
  for (let i = 0; i < n; i++) fromArr[i] = toArr[i] = stanceAt(marks[i], 0)
  startMs = -1e9 // land on the final frame with no opening animation
  if (renderer) { renderer.setData(marks); renderer.setRound(fromArr, toArr, delayArr) }
  recomputeLayers()
  paint()
}

function resize() {
  if (!canvas.value || !wrap.value || !renderer) return
  dpr = window.devicePixelRatio || 1
  const r = wrap.value.getBoundingClientRect()
  W.value = r.width; H.value = r.height
  renderer.setViewport(r.width, r.height, dpr, PAD)
  recomputeLayers()
  paint()
}

let ro
watch(() => props.personas, rebuild)
onMounted(() => {
  renderer = createMarkRenderer(canvas.value)
  rebuild()
  resize()
  ro = new ResizeObserver(resize)
  ro.observe(wrap.value)
})
onBeforeUnmount(() => {
  ro && ro.disconnect()
  raf && cancelAnimationFrame(raf)
  pickRaf && cancelAnimationFrame(pickRaf)
  clearTimeout(playTimer)
  renderer && renderer.dispose()
})
</script>

<style scoped>
.field { display: flex; height: 100%; min-height: 520px; background: var(--navy); color: var(--on-dark); font-family: var(--font-sans); }
.stage { flex: 1; min-width: 0; display: flex; flex-direction: column; }
.canvas-wrap { position: relative; flex: 1; min-height: 0; background: var(--navy); overflow: hidden; }
.vector, .marks { position: absolute; inset: 0; display: block; }
.vector { pointer-events: none; }
.vector .grid line { stroke: rgba(201, 214, 229, 0.06); stroke-width: 1; }
.vector .iso path { fill: none; stroke-width: 1.2; }
.vector .fracture { fill: none; stroke: rgba(201, 214, 229, 0.24); stroke-width: 1; stroke-dasharray: 3 5; }
.vector .fracture.strong { stroke: var(--coral-bright); stroke-opacity: 0.7; stroke-width: 1.6; stroke-dasharray: none; }
.vector .flow line { stroke: rgba(230, 166, 86, 0.7); stroke-width: 1.4; }
.sel-ring { position: absolute; width: 26px; height: 26px; margin: -13px 0 0 -13px; border: 1px solid rgba(255, 255, 255, 0.6); border-radius: 50%; pointer-events: none; }
.marks { cursor: crosshair; }
.axis { position: absolute; bottom: 10px; font: 700 10px/1 var(--font-sans); letter-spacing: .14em; text-transform: uppercase; color: var(--on-dark-muted); }
.axis.oppose { left: 14px; }
.axis.support { right: 14px; }
.count { position: absolute; top: 12px; right: 14px; font: 600 11px/1 var(--font-sans); font-variant-numeric: tabular-nums; color: var(--on-dark-muted); }
.count .refused { color: rgba(201, 214, 229, 0.6); }
.ribbon { display: flex; align-items: center; gap: 12px; padding: 10px 16px; border-top: 1px solid rgba(201, 214, 229, 0.14); background: var(--navy-raised); }
.play { width: 30px; height: 26px; border: 1px solid rgba(201, 214, 229, 0.24); background: rgba(255, 255, 255, 0.06); color: var(--on-dark); border-radius: var(--r-sm); cursor: pointer; font-size: 11px; }
.scrub { flex: 1; accent-color: #F3D096; }
.round { font: 600 12px/1 var(--font-sans); font-variant-numeric: tabular-nums; color: var(--on-dark); }
.round .of { color: var(--on-dark-muted); }
.sheet { width: 280px; flex: none; border-left: 1px solid rgba(201, 214, 229, 0.14); padding: 20px 18px; overflow-y: auto; background: var(--navy-raised); }
.sheet-name { font-family: var(--font-serif); font-size: 17px; font-weight: 500; }
.sheet-type { font-size: 12px; color: var(--on-dark-muted); margin-top: 2px; }
.sheet-stance { margin-top: 12px; font: 700 11px/1 var(--font-sans); letter-spacing: .1em; text-transform: uppercase; }
.sheet-stance.oppose { color: var(--coral-bright); }
.sheet-stance.support { color: var(--emerald-bright); }
.sheet-stance.neutral { color: var(--on-dark-muted); }
.sheet-stance .val { margin-left: 8px; color: var(--on-dark-muted); font-weight: 400; font-variant-numeric: tabular-nums; }
.synthetic { margin-left: 8px; color: var(--on-dark-muted); font-weight: 400; text-transform: none; letter-spacing: 0; }
.sheet-ev-head { margin-top: 18px; font-size: 12px; color: var(--on-dark-muted); }
.sheet-ev { list-style: none; padding: 0; margin: 8px 0 0; display: flex; flex-direction: column; gap: 8px; }
.sheet-ev li { font-size: 12.5px; line-height: 1.45; color: var(--on-dark); padding-left: 10px; border-left: 1px solid rgba(201, 214, 229, 0.2); }
.sheet-ev li.none { color: var(--on-dark-muted); border: none; }
.sheet-empty { color: var(--on-dark-muted); font-size: 13px; line-height: 1.5; }
</style>
