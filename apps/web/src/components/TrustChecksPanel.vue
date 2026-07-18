<template>
  <section class="trust-panel">
    <div class="panel-heading">
      <span class="panel-label">Trust Checks</span>
      <span class="panel-note">Turn the simulation against itself</span>
    </div>

    <!-- Variance check: how much does the sim wobble on its own? -->
    <div class="block">
      <h4 class="block-title">Does it wobble? <span class="title-note">Variance check</span></h4>

      <template v-if="vPhase === 'idle'">
        <p class="block-text">
          Any single run of this simulation could be a fluke of the dice. Re-running the
          identical scenario &mdash; same population, different randomness &mdash; measures how
          much the stance numbers move on their own. Every claim in this report should be
          read against that wobble.
        </p>
        <button class="check-btn" @click="runVariance">Run it again ({{ RERUNS }} more runs)</button>
        <p class="quiet-line">Optional. Each rerun makes real model calls and can take a few minutes.</p>
        <p v-if="vError" class="error-line">{{ vError }}</p>
      </template>

      <template v-else-if="vPhase === 'running'">
        <p class="block-text">
          <span class="btn-spinner dark"></span>
          Running variance check: {{ vDone }}/{{ rerunIds.length }} reruns finished&hellip;
        </p>
        <p class="quiet-line">Same scenario, fresh seeds. The floor appears when every rerun is done.</p>
        <p v-if="vError" class="error-line">{{ vError }}</p>
      </template>

      <template v-else-if="vPhase === 'done' && vResult">
        <p class="block-text">
          <strong class="figure">&plusmn;{{ points(vResult.noise_floor) }} points</strong>
          <template v-if="vResult.runs">
            &mdash; across {{ vResult.runs.length }} runs of the identical scenario, the
            population's stance shares wobbled by an average of
            {{ points(vResult.noise_floor) }} points from randomness alone.
          </template>
          <template v-else>
            &mdash; the noise floor measured for this run. Any claimed effect smaller than
            this is indistinguishable from seed noise.
          </template>
        </p>
        <p class="block-caption">
          Treat any difference smaller than ~{{ points(threshold(vResult.noise_floor)) }} points
          (1.5&times; this floor) as noise, not signal. The scenario rehearsal above now uses this
          measured floor instead of the default 5-point bar.
        </p>
        <button class="check-btn ghost" @click="resetVariance">Measure again</button>
      </template>

      <p v-else-if="vPhase === 'error'" class="error-line">
        Variance check failed: {{ vError }}
        <button class="check-btn ghost" @click="resetVariance">Try again</button>
      </p>
    </div>

    <!-- Ablation check: do the grounded personas actually drive the result? -->
    <div class="block">
      <h4 class="block-title">Do the personas matter? <span class="title-note">Ablation check</span></h4>

      <template v-if="aPhase === 'idle'">
        <p class="block-text">
          The numbers above could be the model's own prior wearing masks. This check re-runs
          the same scenario with every persona shuffled onto a different agent &mdash; same seed,
          same schedules, wrong identities. If the outcome doesn't move, the real data isn't
          doing the work.
        </p>
        <button class="check-btn" @click="runAblation">Run the ablation</button>
        <p class="quiet-line">Optional. One full rerun; real model calls, a few minutes.</p>
        <p v-if="aError" class="error-line">{{ aError }}</p>
      </template>

      <template v-else-if="aPhase === 'running'">
        <p class="block-text">
          <span class="btn-spinner dark"></span>
          Running the persona-shuffled rerun&hellip;
          <template v-if="aPostCount > 0">{{ aPostCount }} posts so far.</template>
        </p>
        <p v-if="aError" class="error-line">{{ aError }}</p>
      </template>

      <template v-else-if="aPhase === 'done' && aResult">
        <p class="callout" :class="aResult.personas_matter ? 'moves' : 'noise'">
          <template v-if="aResult.personas_matter">
            <strong>Shuffling the personas DID change the outcome.</strong>
            The permuted run landed {{ points(aResult.distance) }} points away from the
            baseline &mdash; above the {{ points(aResult.threshold) }}-point noise bar. The
            grounded personas are doing real work in this result.
          </template>
          <template v-else>
            <strong>Shuffling the personas DID NOT change the outcome.</strong>
            The permuted run landed only {{ points(aResult.distance) }} points from the
            baseline &mdash; within the {{ points(aResult.threshold) }}-point noise bar. The
            personas are inert here: these numbers likely reflect the model's prior, not
            this population. Treat the report's conclusions with suspicion.
          </template>
        </p>
        <p class="block-caption">
          Same seed and activity schedules on both sides; only the persona&rarr;agent mapping
          changed.
          <template v-if="!hasFloor">
            The noise bar is the 0.05 default &mdash; run the variance check for a measured one.
          </template>
        </p>
        <button class="check-btn ghost" @click="resetAblation">Run again</button>
      </template>

      <p v-else-if="aPhase === 'error'" class="error-line">
        Ablation failed: {{ aError }}
        <button class="check-btn ghost" @click="resetAblation">Try again</button>
      </p>
    </div>
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
  getSimulation,
  validateSimulation
} from '../api/simulation'

const props = defineProps({
  simulationId: String
})

const RERUNS = 2 // baseline + 2 = 3 runs of the same scenario

const vPhase = ref('idle') // idle | running | done | error
const rerunIds = ref([])
const vDone = ref(0)
const vResult = ref(null)
const vError = ref(null)

const aPhase = ref('idle')
const permutedId = ref(null)
const aPostCount = ref(0)
const aResult = ref(null)
const aError = ref(null)

const stateKey = computed(() => `trust:${props.simulationId}`)

const points = (x) => Math.round((x || 0) * 100)
const threshold = (floor) => (floor > 0 ? floor * 1.5 : 0.05)
const hasFloor = computed(() => vResult.value && vResult.value.noise_floor > 0)

const saveState = () => {
  sessionStorage.setItem(
    stateKey.value,
    JSON.stringify({
      variance: { phase: vPhase.value, ids: rerunIds.value },
      ablation: { phase: aPhase.value, id: permutedId.value }
    })
  )
}

// The scenario the reruns must repeat verbatim.
const baselineEvent = async () => {
  const res = await getSimulationConfig(props.simulationId)
  const event = res.data?.event_config?.initial_posts?.[0]?.content
  if (!event) throw new Error('this simulation has no scenario event to re-run')
  return event
}

// Match rerun length to what the baseline actually ran (rounds are 0-based).
const baselineRounds = async () => {
  try {
    const res = await getSimulationActions(props.simulationId, { limit: 1 })
    const last = res.data?.actions?.[0]?.round
    return Number.isInteger(last) ? last + 1 : null
  } catch {
    return null
  }
}

const runFinished = async (id) => {
  const res = await getRunStatus(id)
  const s = res.data?.status
  const posts = res.data?.post_count || 0
  return { done: s === 'alive' || s === 'stopped' || (s === 'not_running' && posts > 0), posts }
}

const runVariance = async () => {
  vError.value = null
  try {
    const event = await baselineEvent()
    const maxRounds = await baselineRounds()
    const ids = []
    for (let i = 0; i < RERUNS; i++) {
      const dup = await duplicateSimulation(props.simulationId, { event })
      const id = dup.data.simulation_id
      const params = { simulation_id: id, seed: Math.floor(Math.random() * 2 ** 31) }
      if (maxRounds) params.max_rounds = maxRounds
      await startSimulation(params)
      ids.push(id)
    }
    rerunIds.value = ids
    vDone.value = 0
    vPhase.value = 'running'
    saveState()
    startPolling()
  } catch (err) {
    vError.value = err.message
  }
}

const runAblation = async () => {
  aError.value = null
  try {
    const event = await baselineEvent()
    const maxRounds = await baselineRounds()
    const dup = await duplicateSimulation(props.simulationId, { event })
    permutedId.value = dup.data.simulation_id
    // No seed override: the duplicate carries the baseline's effective seed, so
    // the shuffled personas are the only variable.
    const params = { simulation_id: permutedId.value, permute_personas: true }
    if (maxRounds) params.max_rounds = maxRounds
    await startSimulation(params)
    aPhase.value = 'running'
    saveState()
    startPolling()
  } catch (err) {
    aError.value = err.message
  }
}

// One validate call carries everything measured so far, so the ablation verdict
// uses the freshly measured floor whenever it exists.
const loadResults = async () => {
  const body = {}
  if (vPhase.value !== 'idle' && rerunIds.value.length > 0) {
    body.simulation_ids = [props.simulationId, ...rerunIds.value]
  }
  if (aPhase.value !== 'idle' && permutedId.value) {
    body.baseline_id = props.simulationId
    body.permuted_id = permutedId.value
  }
  const res = await validateSimulation(body)
  if (body.simulation_ids && vPhase.value === 'running') {
    // /validate persisted this floor onto the Run itself, so it's the
    // authoritative source (RehearsalPanel's /compare reads it from there).
    vResult.value = { noise_floor: res.data.noise_floor, runs: res.data.runs }
    vPhase.value = 'done'
  }
  if (res.data.persona_ablation && aPhase.value === 'running') {
    const ab = res.data.persona_ablation
    aResult.value = {
      distance: ab.distance,
      personas_matter: ab.personas_matter,
      threshold: threshold(res.data.noise_floor)
    }
    aPhase.value = 'done'
  }
  saveState()
}

const poll = async () => {
  try {
    let pending = false
    if (vPhase.value === 'running') {
      const states = await Promise.all(rerunIds.value.map(runFinished))
      vDone.value = states.filter((s) => s.done).length
      if (vDone.value < rerunIds.value.length) pending = true
    }
    if (aPhase.value === 'running' && permutedId.value) {
      const s = await runFinished(permutedId.value)
      aPostCount.value = s.posts
      if (!s.done) pending = true
    }
    if (!pending && (vPhase.value === 'running' || aPhase.value === 'running')) {
      stopPolling()
      await loadResults()
    }
  } catch { /* transient */ }
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

const resetVariance = () => {
  vPhase.value = 'idle'
  rerunIds.value = []
  vDone.value = 0
  vResult.value = null
  vError.value = null
  saveState()
}

const resetAblation = () => {
  aPhase.value = 'idle'
  permutedId.value = null
  aPostCount.value = 0
  aResult.value = null
  aError.value = null
  saveState()
}

// The floor is a persisted Run property: show it straight away, so trust is
// visible on any Run without re-running the variance check.
const loadPersistedFloor = async () => {
  try {
    const res = await getSimulation(props.simulationId)
    const floor = res.data?.noise_floor
    if (vPhase.value === 'idle' && typeof floor === 'number' && floor > 0) {
      vResult.value = { noise_floor: floor, runs: null }
      vPhase.value = 'done'
    }
  } catch { /* the panel still offers to measure */ }
}

onMounted(() => {
  if (!props.simulationId) return
  loadPersistedFloor()
  const saved = sessionStorage.getItem(stateKey.value)
  if (!saved) return
  try {
    const { variance, ablation } = JSON.parse(saved)
    if (variance?.ids?.length > 0 && variance.phase !== 'idle') {
      rerunIds.value = variance.ids
      vPhase.value = 'running' // results are a cheap re-read; poll settles it
    }
    if (ablation?.id && ablation.phase !== 'idle') {
      permutedId.value = ablation.id
      aPhase.value = 'running'
    }
    if (vPhase.value === 'running' || aPhase.value === 'running') startPolling()
  } catch {
    sessionStorage.removeItem(stateKey.value)
  }
})

onUnmounted(stopPolling)
</script>

<style scoped>
.trust-panel {
  border: 1px solid var(--border);
  border-top: 2px solid var(--emerald);
  border-radius: var(--r-md);
  box-shadow: var(--elev-1);
  padding: 20px 24px 8px;
  margin-bottom: 36px;
  background: var(--white);
  font-family: var(--font-sans);
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
  color: var(--text-strong);
}

.panel-note {
  font-size: 13px;
  font-style: italic;
  color: var(--text-muted);
}

.block {
  padding: 14px 0 16px;
  border-top: 1px solid var(--border);
}

.block-title {
  font-size: 14px;
  font-weight: 700;
  color: var(--text-strong);
  margin: 0 0 6px;
}

.title-note {
  font-weight: 400;
  font-style: italic;
  font-size: 12px;
  color: var(--text-muted);
  margin-left: 6px;
}

.block-text {
  font-size: 14px;
  line-height: 1.65;
  color: var(--text-strong);
  margin: 0 0 10px;
}

.block-caption {
  font-size: 13px;
  font-style: italic;
  line-height: 1.6;
  color: var(--text-muted);
  margin: 10px 0 12px;
}

.quiet-line {
  font-size: 13px;
  font-style: italic;
  color: var(--text-muted);
  margin: 4px 0 12px;
}

.error-line {
  font-size: 13px;
  color: var(--coral-on-light);
  margin: 8px 0 12px;
}

.figure {
  font-family: var(--font-serif);
  font-size: 22px;
  font-weight: 500;
  color: var(--text-strong);
  font-variant-numeric: tabular-nums;
}

.check-btn {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  font-family: inherit;
  font-size: 13px;
  font-weight: 600;
  color: var(--navy);
  background: #fff;
  border: 1px solid var(--navy);
  padding: 8px 16px;
  cursor: pointer;
  margin-bottom: 4px;
  transition: background 0.15s ease, color 0.15s ease;
}

.check-btn:hover:not(:disabled) {
  background: var(--navy);
  color: #fff;
}

.check-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.check-btn.ghost {
  border-color: var(--border);
  color: var(--text-muted);
}

.check-btn.ghost:hover {
  border-color: var(--navy);
  background: #fff;
  color: var(--navy);
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
  animation: trust-spin 0.8s linear infinite;
}

@keyframes trust-spin {
  to { transform: rotate(360deg); }
}

.callout {
  font-size: 13.5px;
  line-height: 1.6;
  color: var(--text-strong);
  background: var(--white);
  padding: 10px 14px;
  margin: 0 0 10px;
}

.callout.moves {
  border-left: 3px solid var(--emerald);
}

.callout.noise {
  border-left: 3px solid var(--coral-on-light);
}
</style>
