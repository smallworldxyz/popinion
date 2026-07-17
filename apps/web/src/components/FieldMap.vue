<template>
  <div class="field" :class="{ dark: true }">
    <div class="canvas-wrap" ref="wrap">
      <canvas ref="canvas" @mousemove="onMove" @mouseleave="hover = null"></canvas>
      <div class="axis oppose">OPPOSE</div>
      <div class="axis support">SUPPORT</div>
      <div class="count">{{ marks.length }} personas</div>
    </div>

    <aside class="sheet">
      <template v-if="selected">
        <div class="sheet-name">{{ selected.name || selected.user_name }}</div>
        <div class="sheet-type">{{ selected.source_entity_type || 'entity' }}</div>
        <div class="sheet-stance" :class="stanceClass(selected)">
          {{ stanceLabel(selected) }}
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
import { ref, onMounted, onBeforeUnmount, watch } from 'vue'

const props = defineProps({ personas: { type: Array, default: () => [] } })

const wrap = ref(null)
const canvas = ref(null)
const hover = ref(null)
const selected = ref(null)

// FIELD palette (DESIGN.md). Canvas takes oklch() in modern browsers.
const C = {
  ground: 'oklch(0.245 0.026 265)',
  graticule: 'oklch(0.38 0.022 265 / 0.35)',
  oppose: 'oklch(0.56 0.16 252)',
  neutral: 'oklch(0.74 0.02 265)',
  support: 'oklch(0.64 0.15 74)',
  ink: 'oklch(0.92 0.012 265)',
}

// Client-side mirror of the backend layout, used only when a Persona has no
// server-assigned position (demo data, or a run prepared before positions).
function jitter(id) {
  let z = (BigInt(id) * 0x9E3779B97F4A7C15n + 0x2545F4914F6CDD1Dn) & 0xFFFFFFFFFFFFFFFFn
  z = ((z ^ (z >> 30n)) * 0xBF58476D1CE4E5B9n) & 0xFFFFFFFFFFFFFFFFn
  z ^= z >> 27n
  const a = Number(z & 0xFFFFn) / 65536 - 0.5
  const b = Number((z >> 16n) & 0xFFFFn) / 65536 - 0.5
  return [a, b]
}
function layout(personas) {
  const types = []
  for (const p of personas) {
    const t = p.source_entity_type || 'Entity'
    if (!types.includes(t)) types.push(t)
  }
  const bands = Math.max(types.length, 1)
  return personas.map((p) => {
    if (Array.isArray(p.position)) return { ...p, _x: p.position[0], _y: p.position[1] }
    const sx = p.faction === 'con' ? 0.2 : p.faction === 'pro' ? 0.8 : 0.5
    const t = p.source_entity_type || 'Entity'
    const band = types.indexOf(t)
    const by = (band + 0.5) / bands
    const [jx, jy] = jitter(p.user_id ?? 0)
    return {
      ...p,
      _x: Math.min(0.98, Math.max(0.02, sx + jx * 0.12)),
      _y: Math.min(0.98, Math.max(0.02, by + jy * (0.8 / bands))),
    }
  })
}

const marks = ref([])
const stanceOf = (p) => (p.faction === 'con' ? 'oppose' : p.faction === 'pro' ? 'support' : 'neutral')
const stanceLabel = (p) => ({ oppose: 'Opposes', support: 'Supports', neutral: 'Neutral' }[stanceOf(p)])
const stanceClass = (p) => stanceOf(p)

let ctx, dpr, W, H, ro

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
  draw()
}

// oppose = downward chevron, neutral = flat bar, support = upward chevron.
function chevron(x, y, s, kind) {
  ctx.beginPath()
  if (kind === 'oppose') {
    ctx.moveTo(x - s, y - s * 0.6); ctx.lineTo(x, y + s * 0.7); ctx.lineTo(x + s, y - s * 0.6)
  } else if (kind === 'support') {
    ctx.moveTo(x - s, y + s * 0.6); ctx.lineTo(x, y - s * 0.7); ctx.lineTo(x + s, y + s * 0.6)
  } else {
    ctx.moveTo(x - s, y); ctx.lineTo(x + s, y)
  }
}

function draw() {
  if (!ctx) return
  ctx.fillStyle = C.ground
  ctx.fillRect(0, 0, W, H)

  // Graticule.
  ctx.strokeStyle = C.graticule
  ctx.lineWidth = 1
  for (let i = 1; i < 8; i++) {
    const gx = (i / 8) * W
    ctx.beginPath(); ctx.moveTo(gx, 0); ctx.lineTo(gx, H); ctx.stroke()
  }
  for (let i = 1; i < 6; i++) {
    const gy = (i / 6) * H
    ctx.beginPath(); ctx.moveTo(0, gy); ctx.lineTo(W, gy); ctx.stroke()
  }

  const pad = 28
  const px = (mx) => pad + mx * (W - 2 * pad)
  const py = (my) => (H - pad) - my * (H - 2 * pad) // flip: up is up

  for (const m of marks.value) {
    const x = px(m._x)
    const y = py(m._y)
    const st = stanceOf(m)
    ctx.strokeStyle = C[st]
    ctx.globalAlpha = m.synthetic ? 0.5 : 1
    ctx.lineWidth = m === selected.value ? 3 : 2
    chevron(x, y, m === selected.value ? 7 : 5, st)
    ctx.stroke()
  }
  ctx.globalAlpha = 1

  // Selected halo.
  if (selected.value) {
    const x = px(selected.value._x)
    const y = py(selected.value._y)
    ctx.strokeStyle = C.ink
    ctx.globalAlpha = 0.6
    ctx.lineWidth = 1
    ctx.beginPath(); ctx.arc(x, y, 12, 0, Math.PI * 2); ctx.stroke()
    ctx.globalAlpha = 1
  }
}

function onMove(e) {
  const r = canvas.value.getBoundingClientRect()
  const mx = e.clientX - r.left
  const my = e.clientY - r.top
  const pad = 28
  const px = (v) => pad + v * (W - 2 * pad)
  const py = (v) => (H - pad) - v * (H - 2 * pad)
  let best = null
  let bestD = 16 * 16
  for (const m of marks.value) {
    const dx = px(m._x) - mx
    const dy = py(m._y) - my
    const d = dx * dx + dy * dy
    if (d < bestD) { bestD = d; best = m }
  }
  hover.value = best
  selected.value = best
  draw()
}

watch(() => props.personas, (v) => { marks.value = layout(v || []); draw() }, { immediate: true })
onMounted(() => {
  resize()
  ro = new ResizeObserver(resize)
  ro.observe(wrap.value)
})
onBeforeUnmount(() => ro && ro.disconnect())
</script>

<style scoped>
.field { display: flex; gap: 0; height: 100%; min-height: 520px; background: oklch(0.19 0.021 265); color: oklch(0.88 0.012 265); }
.canvas-wrap { position: relative; flex: 1; min-width: 0; }
canvas { display: block; }
.axis { position: absolute; bottom: 8px; font: 600 11px/1 ui-monospace, monospace; letter-spacing: .1em; color: oklch(0.6 0.02 265); }
.axis.oppose { left: 12px; }
.axis.support { right: 12px; }
.count { position: absolute; top: 10px; right: 12px; font: 500 11px/1 ui-monospace, monospace; color: oklch(0.6 0.02 265); }
.sheet { width: 280px; flex: none; border-left: 1px solid oklch(0.32 0.02 265); padding: 20px 18px; overflow-y: auto; background: oklch(0.22 0.024 265); }
.sheet-name { font-size: 16px; font-weight: 650; }
.sheet-type { font-size: 12px; color: oklch(0.62 0.02 265); margin-top: 2px; }
.sheet-stance { margin-top: 12px; font: 600 12px/1 ui-monospace, monospace; letter-spacing: .04em; }
.sheet-stance.oppose { color: oklch(0.62 0.16 252); }
.sheet-stance.support { color: oklch(0.7 0.15 74); }
.sheet-stance.neutral { color: oklch(0.78 0.02 265); }
.synthetic { margin-left: 8px; color: oklch(0.55 0.03 265); font-weight: 400; }
.sheet-ev-head { margin-top: 18px; font-size: 12px; color: oklch(0.66 0.018 265); }
.sheet-ev { list-style: none; padding: 0; margin: 8px 0 0; display: flex; flex-direction: column; gap: 8px; }
.sheet-ev li { font-size: 12.5px; line-height: 1.45; color: oklch(0.82 0.012 265); padding-left: 10px; border-left: 1px solid oklch(0.34 0.02 265); }
.sheet-ev li.none { color: oklch(0.55 0.02 265); border: none; }
.sheet-empty { color: oklch(0.55 0.02 265); font-size: 13px; line-height: 1.5; }
</style>
