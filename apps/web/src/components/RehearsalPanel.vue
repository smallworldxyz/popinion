<template>
  <section class="rehearsal-panel">
    <div class="panel-heading">
      <span class="panel-label">Scenario Rehearsal</span>
      <span class="panel-note">Test an alternative against the same population</span>
    </div>

    <!-- Idle: offer the rehearsal -->
    <div v-if="phase === 'idle'" class="block">
      <p class="block-text">
        This report reflects one scenario. The same population — same agents, same
        randomness — can rehearse an alternative announcement, so any difference in
        opinion is caused by the wording, not by the dice.
      </p>
      <p v-if="baselineEvent" class="event-line">
        <span class="event-tag">Scenario A</span> {{ baselineEvent }}
      </p>
      <textarea
        v-model="altEvent"
        class="event-input"
        rows="3"
        placeholder="The alternative announcement or policy to rehearse (Scenario B)&hellip;"
      ></textarea>
      <button class="check-btn" :disabled="!altEvent.trim()" @click="rehearse">
        Rehearse alternative
      </button>
      <p v-if="error" class="error-line">{{ error }}</p>
    </div>

    <!-- Running: waiting for the B run -->
    <div v-else-if="phase === 'running'" class="block">
      <p class="block-text">
        <span class="btn-spinner dark"></span>
        Running the alternative against the same population, same seed&hellip;
        <template v-if="bPostCount > 0">{{ bPostCount }} posts so far.</template>
      </p>
      <p v-if="altEvent" class="event-line"><span class="event-tag">Scenario B</span> {{ altEvent }}</p>
      <p class="quiet-line">This makes real model calls and can take a few minutes. The comparison appears when both runs are finished.</p>
      <p v-if="error" class="error-line">{{ error }}</p>
    </div>

    <!-- Done: the A/B comparison -->
    <div v-else-if="phase === 'done' && cmp" class="block">
      <div class="stance-compare">
        <div class="stance-col">
          <div class="col-head">Scenario A <span class="col-badge">{{ shortEvent(cmp.a.event) }}</span></div>
          <div v-for="row in rows" :key="'a-' + row.stance" class="stance-row">
            <span class="stance-name">{{ row.stance }}</span>
            <span class="stance-track"><span class="stance-fill" :style="{ width: pct(row.a) }"></span></span>
            <span class="stance-pct">{{ pct(row.a) }}</span>
          </div>
        </div>
        <div class="stance-col">
          <div class="col-head">Scenario B <span class="col-badge">{{ shortEvent(cmp.b.event) }}</span></div>
          <div v-for="row in rows" :key="'b-' + row.stance" class="stance-row">
            <span class="stance-name">{{ row.stance }}</span>
            <span class="stance-track"><span class="stance-fill alt" :style="{ width: pct(row.b) }"></span></span>
            <span class="stance-pct">{{ pct(row.b) }}</span>
          </div>
        </div>
      </div>

      <p class="callout" :class="cmp.significant ? 'moves' : 'noise'">
        <template v-if="cmp.significant">
          <strong>This change moves opinion beyond the simulation's noise floor.</strong>
          The two scenarios differ by {{ points(cmp.tv_distance) }} points, above the
          {{ points(cmp.threshold) }}-point bar for a real effect.
        </template>
        <template v-else>
          <strong>Within noise — not a distinguishable difference.</strong>
          The two scenarios differ by {{ points(cmp.tv_distance) }} points, below the
          {{ points(cmp.threshold) }}-point bar; seed randomness alone can produce a gap this size.
        </template>
      </p>
      <p class="block-caption">
        Agent-weighted (one vote per agent), same population and seed on both sides —
        the announcement is the only variable.
        <template v-if="cmp.noise_floor === 0">
          The noise floor was not measured (no rerun data), so the default 5-point bar applies.
        </template>
      </p>
      <button class="check-btn ghost" @click="reset">Rehearse another alternative</button>
    </div>

    <p v-else-if="phase === 'error'" class="error-line">
      Rehearsal failed: {{ error }}
      <button class="check-btn ghost" @click="reset">Try again</button>
    </p>
  </section>
</template>

<script setup>
import { ref, computed, onMounted, onUnmounted } from 'vue'
import {
  duplicateSimulation,
  startSimulation,
  getRunStatus,
  getSimulationActions,
  getSimulationConfig,
  compareSimulations
} from '../api/simulation'

const props = defineProps({
  simulationId: String
})

const phase = ref('idle') // idle | running | done | error
const altEvent = ref('')
const baselineEvent = ref('')
const altId = ref(null)
const bPostCount = ref(0)
const cmp = ref(null)
const error = ref(null)

const storageKey = computed(() => `rehearsal:${props.simulationId}`)

const pct = (x) => `${Math.round((x || 0) * 100)}%`
const points = (x) => Math.round((x || 0) * 100)
const shortEvent = (e) => {
  if (!e) return ''
  return e.length > 90 ? e.slice(0, 90) + '…' : e
}

// Union of stances across both sides, ordered by scenario-A share desc.
const rows = computed(() => {
  if (!cmp.value) return []
  const a = cmp.value.a.shares || {}
  const b = cmp.value.b.shares || {}
  const names = [...new Set([...Object.keys(a), ...Object.keys(b)])]
  return names
    .map((stance) => ({ stance, a: a[stance] || 0, b: b[stance] || 0 }))
    .sort((x, y) => y.a - x.a)
})

const loadBaselineEvent = async () => {
  try {
    const res = await getSimulationConfig(props.simulationId)
    baselineEvent.value = res.data?.event_config?.initial_posts?.[0]?.content || ''
  } catch { /* the panel works without the label */ }
}

// Match run B's length to what run A actually ran (actions are newest-first;
// rounds are 0-based, so the count is last round + 1).
const baselineRounds = async () => {
  try {
    const res = await getSimulationActions(props.simulationId, { limit: 1 })
    const last = res.data?.actions?.[0]?.round
    return Number.isInteger(last) ? last + 1 : null
  } catch {
    return null
  }
}

const rehearse = async () => {
  error.value = null
  try {
    const dupRes = await duplicateSimulation(props.simulationId, { event: altEvent.value.trim() })
    altId.value = dupRes.data.simulation_id
    const params = { simulation_id: altId.value }
    const maxRounds = await baselineRounds()
    if (maxRounds) params.max_rounds = maxRounds
    await startSimulation(params)
    sessionStorage.setItem(storageKey.value, JSON.stringify({ altId: altId.value, event: altEvent.value.trim() }))
    phase.value = 'running'
    startPolling()
  } catch (err) {
    error.value = err.message
  }
}

const poll = async () => {
  if (!altId.value) return
  try {
    const res = await getRunStatus(altId.value)
    bPostCount.value = res.data?.post_count || 0
    const s = res.data?.status
    // 'alive' = engine finished, agents standing by; 'stopped' = done; a
    // 'not_running' env that already produced posts finished before a reload.
    if (s === 'alive' || s === 'stopped' || (s === 'not_running' && bPostCount.value > 0)) {
      stopPolling()
      await loadComparison()
    }
  } catch { /* transient */ }
}

const loadComparison = async () => {
  try {
    // /compare reads the noise floor from run A's persisted property (set by the
    // Trust-checks variance check), so no rerun ids need to be threaded here.
    const res = await compareSimulations(props.simulationId, altId.value)
    cmp.value = res.data
    phase.value = 'done'
  } catch (err) {
    error.value = err.message
    phase.value = 'error'
  }
}

let timer = null
const startPolling = () => {
  if (timer) return
  poll()
  timer = setInterval(poll, 4000)
}
const stopPolling = () => {
  if (timer) clearInterval(timer)
  timer = null
}

const reset = () => {
  stopPolling()
  sessionStorage.removeItem(storageKey.value)
  phase.value = 'idle'
  altEvent.value = ''
  altId.value = null
  bPostCount.value = 0
  cmp.value = null
  error.value = null
}

onMounted(() => {
  if (!props.simulationId) return
  loadBaselineEvent()
  // Resume an in-flight rehearsal after a reload.
  const saved = sessionStorage.getItem(storageKey.value)
  if (saved) {
    try {
      const { altId: savedId, event } = JSON.parse(saved)
      altId.value = savedId
      altEvent.value = event
      phase.value = 'running'
      startPolling()
    } catch {
      sessionStorage.removeItem(storageKey.value)
    }
  }
})

onUnmounted(stopPolling)
</script>

<style scoped>
.rehearsal-panel {
  border: 1px solid #E5E7EB;
  border-top: 2px solid #111827;
  padding: 20px 24px 8px;
  margin-bottom: 36px;
  background: #FCFCFB;
  font-family: 'Times New Roman', Times, serif;
}

.panel-heading {
  display: flex;
  align-items: baseline;
  gap: 12px;
  margin-bottom: 16px;
}

.panel-label {
  font-size: 12px;
  font-weight: 700;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: #111827;
}

.panel-note {
  font-size: 13px;
  font-style: italic;
  color: #9CA3AF;
}

.block {
  padding: 14px 0 16px;
  border-top: 1px solid #ECECEA;
}

.block-text {
  font-size: 14px;
  line-height: 1.65;
  color: #374151;
  margin: 0 0 10px;
}

.block-caption {
  font-size: 13px;
  font-style: italic;
  line-height: 1.6;
  color: #6B7280;
  margin: 10px 0 12px;
}

.quiet-line {
  font-size: 13px;
  font-style: italic;
  color: #9CA3AF;
  margin: 4px 0 12px;
}

.error-line {
  font-size: 13px;
  color: #B91C1C;
  margin: 8px 0 12px;
}

.event-line {
  font-size: 13.5px;
  line-height: 1.6;
  color: #374151;
  background: #FFFFFF;
  border-left: 3px solid #111827;
  padding: 8px 12px;
  margin: 0 0 12px;
}

.event-tag {
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: #6B7280;
  margin-right: 8px;
}

.event-input {
  width: 100%;
  box-sizing: border-box;
  font-family: inherit;
  font-size: 14px;
  line-height: 1.5;
  color: #111827;
  background: #FFFFFF;
  border: 1px solid #D1D5DB;
  padding: 10px 12px;
  margin-bottom: 10px;
  resize: vertical;
}

.event-input:focus {
  outline: none;
  border-color: #111827;
}

.check-btn {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  font-family: inherit;
  font-size: 13px;
  font-weight: 600;
  color: #111827;
  background: #FFFFFF;
  border: 1px solid #111827;
  padding: 8px 16px;
  cursor: pointer;
  margin-bottom: 12px;
  transition: background 0.15s ease, color 0.15s ease;
}

.check-btn:hover:not(:disabled) {
  background: #111827;
  color: #FFFFFF;
}

.check-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.check-btn.ghost {
  border-color: #D1D5DB;
  color: #6B7280;
}

.check-btn.ghost:hover {
  border-color: #111827;
  background: #FFFFFF;
  color: #111827;
}

.btn-spinner {
  display: inline-block;
  width: 12px;
  height: 12px;
  border: 2px solid currentColor;
  border-top-color: transparent;
  border-radius: 50%;
  vertical-align: -1px;
  margin-right: 6px;
  animation: rehearsal-spin 0.8s linear infinite;
}

@keyframes rehearsal-spin {
  to { transform: rotate(360deg); }
}

/* A/B comparison — mirrors CredibilityPanel's stance-compare grid */
.stance-compare {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 24px;
}

.col-head {
  font-size: 13px;
  font-weight: 700;
  color: #111827;
  margin-bottom: 8px;
}

.col-badge {
  display: block;
  font-weight: 400;
  font-style: italic;
  font-size: 12px;
  line-height: 1.5;
  color: #9CA3AF;
}

.stance-row {
  display: grid;
  grid-template-columns: minmax(60px, auto) 1fr 40px;
  align-items: center;
  gap: 8px;
  margin-bottom: 5px;
}

.stance-name {
  font-size: 13px;
  color: #374151;
  text-transform: capitalize;
}

.stance-track {
  display: block;
  height: 8px;
  background: #F3F4F6;
  overflow: hidden;
}

.stance-fill {
  display: block;
  height: 100%;
  background: #111827;
}

.stance-fill.alt {
  background: #4B5563;
}

.stance-pct {
  font-size: 12px;
  color: #6B7280;
  text-align: right;
  font-variant-numeric: tabular-nums;
}

.callout {
  font-size: 13.5px;
  line-height: 1.6;
  color: #111827;
  background: #FFFFFF;
  padding: 10px 14px;
  margin: 14px 0 0;
}

.callout.moves {
  border-left: 3px solid #166534;
}

.callout.noise {
  border-left: 3px solid #B45309;
}
</style>
