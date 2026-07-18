<template>
  <div class="prepare">
    <header class="prep-head">
      <div class="crumbs">
        <router-link :to="`/world/${graphId}`" class="crumb">← World</router-link>
        <span class="sep">·</span>
        <span class="prep-title">Evidence bar</span>
      </div>
      <span class="note">Raise the bar. Watch who the graph can no longer ground.</span>
    </header>

    <div class="stage">
      <div class="canvas-wrap" ref="wrap">
        <canvas ref="canvas" @mousemove="onMove" @mouseleave="clearHover"></canvas>
        <div class="axis-y">EVIDENCE ↑</div>
        <div class="refused-tag">REFUSED · below the bar</div>

        <div class="readout">
          <span class="mono bar-val">min_evidence {{ minEvidence }}</span>
          <span class="mono count"><b>{{ eligibleCount }}</b> eligible<span class="of"> / {{ totalCount }} entities</span></span>
        </div>

        <div v-if="hover" class="tip" :style="tipStyle">
          <div class="tip-name">{{ hover.name || 'Unnamed' }}</div>
          <div class="tip-type mono">{{ hover.entity_type }}</div>
          <div class="tip-ev mono" :class="hover.eligible ? 'ok' : 'refused'">
            {{ hover.evidence_score }} evidence · {{ hover.eligible ? 'eligible' : 'below bar' }}
          </div>
        </div>

        <div v-if="loading" class="overlay mono">reading the graph…</div>
        <div v-else-if="error" class="overlay err">{{ error }}</div>
        <div v-else-if="!marks.length" class="overlay mono">no entities in this graph</div>
      </div>

      <div class="control">
        <span class="mono lo">1</span>
        <input
          class="bar-slider"
          type="range"
          min="1"
          :max="sliderMax"
          step="1"
          :value="minEvidence"
          :disabled="!marks.length"
          @input="onSlide(+$event.target.value)"
        />
        <span class="mono hi">{{ sliderMax }}</span>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onBeforeUnmount, nextTick } from 'vue'
import { preparePreviewAtBar } from '../api/simulation'

const props = defineProps({
  graphId: { type: String, required: true },
  simId: { type: String, required: true },
})

const wrap = ref(null)
const canvas = ref(null)
const loading = ref(true)
const error = ref('')
const minEvidence = ref(2)
const eligibleCount = ref(0)
const totalCount = ref(0)
const sliderMax = ref(8)
const marks = ref([])
const hover = ref(null)

// Neutral stance ramp is wired for the day /prepare/preview carries a stance
// sign per entity; the payload only exposes evidence_score + eligible today, so
// marks render near-achromatic (honest: stance isn't classified until the run).
const OPPOSE = [70, 120, 224]
const NEUTRAL = [176, 178, 190]
const SUPPORT = [212, 158, 60]
const lerp = (a, b, t) => a + (b - a) * t
const stanceColor = (s) => {
  const t = Math.max(-1, Math.min(1, s || 0))
  const [r, g, b] = t < 0
    ? [0, 1, 2].map((i) => lerp(NEUTRAL[i], OPPOSE[i], -t))
    : [0, 1, 2].map((i) => lerp(NEUTRAL[i], SUPPORT[i], t))
  return `rgb(${r | 0}, ${g | 0}, ${b | 0})`
}

// Deterministic per-uuid jitter so the same graph lays out the same way twice.
function hash(str) {
  let h = 2166136261
  for (let i = 0; i < str.length; i++) {
    h ^= str.charCodeAt(i)
    h = Math.imul(h, 16777619)
  }
  return ((h >>> 0) % 100000) / 100000
}

let ctx, dpr, W, H, ro, yMax = 8
const PAD = 34

// Vertical axis is evidence. A mark with evidence_score = ev sits at yFor(ev);
// the bar sits at the eligibility boundary so ev >= min_evidence render above it.
const yFor = (ev) => H - PAD - (Math.min(ev, yMax) / yMax) * (H - 2 * PAD)
const barY = () => yFor(minEvidence.value - 0.5)

function layout(entities, types) {
  return entities.map((e) => {
    const bands = Math.max(types.length, 1)
    const bi = Math.max(0, types.indexOf(e.entity_type))
    const bw = (W - 2 * PAD) / bands
    return {
      ...e,
      _x: PAD + (bi + 0.12 + 0.76 * hash(e.uuid || e.name || '')) * bw,
      _r: 3 + (Math.min(e.evidence_score, yMax) / yMax) * 7,
    }
  })
}

function resize() {
  if (!canvas.value || !wrap.value) return
  dpr = window.devicePixelRatio || 1
  const r = wrap.value.getBoundingClientRect()
  W = r.width
  H = r.height
  canvas.value.width = W * dpr
  canvas.value.height = H * dpr
  canvas.value.style.width = W + 'px'
  canvas.value.style.height = H + 'px'
  ctx = canvas.value.getContext('2d')
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
  relayout()
  draw()
}

let lastEntities = []
let lastTypes = []
function relayout() {
  if (W) marks.value = layout(lastEntities, lastTypes)
}

function draw() {
  if (!ctx) return
  ctx.fillStyle = 'oklch(0.245 0.026 265)'
  ctx.fillRect(0, 0, W, H)

  // Graticule.
  ctx.strokeStyle = 'oklch(0.38 0.022 265 / 0.3)'
  ctx.lineWidth = 1
  for (let i = 1; i < 8; i++) {
    const g = (i / 8) * W
    ctx.beginPath(); ctx.moveTo(g, 0); ctx.lineTo(g, H); ctx.stroke()
  }
  for (let i = 1; i < 6; i++) {
    const g = (i / 6) * H
    ctx.beginPath(); ctx.moveTo(0, g); ctx.lineTo(W, g); ctx.stroke()
  }

  // The refused zone below the bar, washed a touch darker so it reads as a gutter.
  const by = barY()
  ctx.fillStyle = 'oklch(0.21 0.02 265 / 0.55)'
  ctx.fillRect(0, by, W, H - by)

  // Marks: eligible are filled and stance-coloured; below-bar are hollow, refused.
  for (const m of marks.value) {
    const y = yFor(m.evidence_score)
    ctx.beginPath()
    ctx.arc(m._x, y, m._r, 0, Math.PI * 2)
    if (m.eligible) {
      ctx.fillStyle = stanceColor(0)
      ctx.globalAlpha = m === hover.value ? 1 : 0.92
      ctx.fill()
    } else {
      ctx.strokeStyle = 'oklch(0.55 0.03 265)'
      ctx.globalAlpha = m === hover.value ? 1 : 0.75
      ctx.lineWidth = 1.5
      ctx.stroke()
    }
    ctx.globalAlpha = 1
  }

  // The bar itself: a bright hairline that carries its own value.
  ctx.strokeStyle = 'oklch(0.82 0.05 74)'
  ctx.lineWidth = 1.5
  ctx.setLineDash([6, 4])
  ctx.beginPath(); ctx.moveTo(0, by); ctx.lineTo(W, by); ctx.stroke()
  ctx.setLineDash([])
  ctx.fillStyle = 'oklch(0.82 0.05 74)'
  ctx.font = '600 11px ui-monospace, "IBM Plex Mono", monospace'
  ctx.fillText(`min_evidence ${minEvidence.value}`, 10, by - 6)
}

let debounce
function onSlide(v) {
  minEvidence.value = v
  draw() // move the bar immediately; the server confirms eligibility next.
  clearTimeout(debounce)
  debounce = setTimeout(() => refetch(), 180)
}

async function refetch() {
  try {
    const res = await preparePreviewAtBar(props.simId, minEvidence.value)
    apply(res?.data)
    error.value = ''
  } catch (e) {
    error.value = 'Could not read the graph for this run.'
  } finally {
    loading.value = false
  }
}

function apply(data) {
  if (!data) return
  const groups = data.groups || []
  const types = groups.map((g) => g.entity_type)
  const entities = groups.flatMap((g) => (g.entities || []).map((e) => ({ ...e, entity_type: g.entity_type })))
  const maxEv = entities.reduce((m, e) => Math.max(m, e.evidence_score || 0), 0)
  yMax = Math.max(maxEv + 1, data.min_evidence || 1, 4)
  sliderMax.value = Math.max(maxEv + 1, 4)
  if (typeof data.min_evidence === 'number') minEvidence.value = data.min_evidence
  eligibleCount.value = data.eligible_count ?? entities.filter((e) => e.eligible).length
  totalCount.value = (data.eligible_count ?? 0) + (data.below_bar_count ?? 0) || entities.length
  lastEntities = entities
  lastTypes = types
  relayout()
  draw()
}

function onMove(e) {
  const r = canvas.value.getBoundingClientRect()
  const mx = e.clientX - r.left
  const my = e.clientY - r.top
  let best = null
  let bestD = 18 * 18
  for (const m of marks.value) {
    const dx = m._x - mx
    const dy = yFor(m.evidence_score) - my
    const d = dx * dx + dy * dy
    if (d < bestD) { bestD = d; best = m }
  }
  if (best !== hover.value) { hover.value = best; draw() }
}
function clearHover() { hover.value = null; draw() }

const tipStyle = computed(() => {
  if (!hover.value) return {}
  return { left: Math.min(hover.value._x + 14, W - 180) + 'px', top: yFor(hover.value.evidence_score) + 'px' }
})

onMounted(async () => {
  await refetch()
  await nextTick()
  resize()
  ro = new ResizeObserver(resize)
  ro.observe(wrap.value)
})
onBeforeUnmount(() => { ro && ro.disconnect(); clearTimeout(debounce) })
</script>

<style scoped>
.prepare { display: flex; flex-direction: column; height: 100vh; background: oklch(0.19 0.021 265); color: oklch(0.88 0.012 265); }
.prep-head { display: flex; align-items: center; justify-content: space-between; padding: 14px 20px; padding-right: 108px; flex: none; gap: 16px; }
.crumbs { display: flex; align-items: center; gap: 10px; min-width: 0; }
.crumb { color: oklch(0.7 0.05 250); text-decoration: none; font-size: 13px; }
.crumb:hover { text-decoration: underline; }
.sep { color: oklch(0.45 0.02 265); }
.prep-title { font-weight: 650; font-size: 15px; }
.note { font-size: 13px; color: oklch(0.6 0.02 265); white-space: nowrap; }

.stage { flex: 1; min-height: 0; display: flex; flex-direction: column; }
.canvas-wrap { position: relative; flex: 1; min-height: 0; }
canvas { display: block; }

.axis-y { position: absolute; top: 10px; left: 12px; font: 600 11px/1 ui-monospace, monospace; letter-spacing: .08em; color: oklch(0.6 0.02 265); }
.refused-tag { position: absolute; bottom: 10px; left: 12px; font: 600 11px/1 ui-monospace, monospace; letter-spacing: .06em; color: oklch(0.55 0.03 265); }

.readout { position: absolute; top: 10px; right: 14px; display: flex; flex-direction: column; align-items: flex-end; gap: 4px; }
.mono { font-family: ui-monospace, 'IBM Plex Mono', monospace; }
.bar-val { font-size: 12px; color: oklch(0.82 0.05 74); letter-spacing: .02em; }
.count { font-size: 12px; color: oklch(0.78 0.02 265); }
.count b { color: oklch(0.96 0.008 265); }
.count .of { color: oklch(0.58 0.02 265); }

.tip { position: absolute; transform: translateY(-50%); pointer-events: none; background: oklch(0.29 0.028 265); border: 1px solid oklch(0.4 0.02 265); border-radius: 6px; padding: 8px 10px; min-width: 150px; box-shadow: 0 8px 24px oklch(0.12 0.02 265 / 0.5); z-index: 5; }
.tip-name { font-size: 13px; font-weight: 600; color: oklch(0.92 0.012 265); }
.tip-type { font-size: 11px; color: oklch(0.62 0.02 265); margin-top: 2px; }
.tip-ev { font-size: 11px; margin-top: 6px; }
.tip-ev.ok { color: oklch(0.7 0.05 74); }
.tip-ev.refused { color: oklch(0.6 0.03 265); }

.overlay { position: absolute; inset: 0; display: flex; align-items: center; justify-content: center; color: oklch(0.6 0.02 265); font-size: 13px; }
.overlay.err { color: oklch(0.72 0.19 42); }

.control { flex: none; display: flex; align-items: center; gap: 12px; padding: 12px 20px; border-top: 1px solid oklch(0.32 0.02 265); background: oklch(0.22 0.024 265); }
.control .lo, .control .hi { font-size: 11px; color: oklch(0.58 0.02 265); }
.bar-slider { flex: 1; accent-color: oklch(0.66 0.14 74); cursor: pointer; }
.bar-slider:disabled { opacity: 0.4; cursor: default; }
</style>
