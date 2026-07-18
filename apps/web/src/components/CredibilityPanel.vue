<template>
  <section class="credibility-panel" v-if="!hidden">
    <div class="panel-heading">
      <span class="panel-label">Credibility &amp; Limits</span>
      <span class="panel-note">How much to trust what follows</span>
    </div>

    <!-- Loading -->
    <div v-if="loading" class="quiet-line">Loading credibility data&hellip;</div>

    <template v-else-if="cred">
      <!-- Population provenance -->
      <div class="block">
        <h4 class="block-title">Who was simulated</h4>
        <p class="block-text">
          <template v-if="cred.total > 0">
            <strong>{{ cred.grounded }} of {{ cred.total }}</strong> agents are grounded in real
            entities from the knowledge graph<template v-if="cred.synthetic > 0">;
            <strong>{{ cred.synthetic }}</strong> ({{ pct(cred.synthetic / cred.total) }}) are a
            synthetic audience with invented attributes</template>.
            <template v-if="cred.synthetic > 0">
              Findings about the synthetic share reflect the model's assumptions, not observed people.
            </template>
          </template>
          <template v-else>No agent roster found for this simulation.</template>
        </p>
        <div class="provenance-bar" v-if="cred.total > 0" :title="`${cred.grounded} grounded / ${cred.synthetic} synthetic`">
          <div class="bar-grounded" :style="{ width: pct(cred.grounded / cred.total) }"></div>
          <div class="bar-synthetic" :style="{ width: pct(cred.synthetic / cred.total) }"></div>
        </div>
        <div class="bar-legend" v-if="cred.total > 0">
          <span class="legend-item"><span class="swatch swatch-grounded"></span>Grounded</span>
          <span class="legend-item" v-if="cred.synthetic > 0"><span class="swatch swatch-synthetic"></span>Synthetic</span>
        </div>
      </div>

      <!-- Stance: agent-weighted vs post-weighted -->
      <div class="block">
        <h4 class="block-title">How opinion is counted</h4>
        <template v-if="stanceRows.length > 0">
          <div class="stance-compare">
            <div class="stance-col">
              <div class="col-head">Agent-weighted <span class="col-badge">one vote per agent</span></div>
              <div v-for="row in stanceRows" :key="'a-' + row.stance" class="stance-row">
                <span class="stance-name">{{ row.stance }}</span>
                <span class="stance-track"><span class="stance-fill" :style="{ width: pct(row.agentShare) }"></span></span>
                <span class="stance-pct">{{ pct(row.agentShare) }}</span>
              </div>
            </div>
            <div class="stance-col">
              <div class="col-head muted">Post-weighted <span class="col-badge">one vote per post</span></div>
              <div v-for="row in stanceRows" :key="'p-' + row.stance" class="stance-row">
                <span class="stance-name">{{ row.stance }}</span>
                <span class="stance-track"><span class="stance-fill light" :style="{ width: pct(row.postShare) }"></span></span>
                <span class="stance-pct">{{ pct(row.postShare) }}</span>
              </div>
            </div>
          </div>
          <p class="block-caption">
            The agent-weighted split is the trustworthy one: each agent counts once, by their latest
            stance. Post-weighting lets a hyperactive agent dominate the numbers.
          </p>
          <p v-if="divergence > 0.1" class="callout">
            These two splits differ by {{ Math.round(divergence * 100) }} points. That gap is itself a
            finding: a few loud agents produced an outsized share of the posts, so conversation volume
            here does not equal people. Read the agent-weighted column.
          </p>
        </template>
        <p v-else class="quiet-line">No stance-bearing activity recorded yet.</p>
      </div>

      <!-- Independent stance check -->
      <div class="block">
        <h4 class="block-title">Independent check</h4>
        <template v-if="!check">
          <p class="block-text">
            The stance splits above rest on the agents' self-reported labels &mdash; the acting model
            grading its own homework. An independent model can re-read every post and comment and
            report how often it agrees.
          </p>
          <button class="check-btn" :disabled="checking" @click="runCheck">
            <span v-if="checking" class="btn-spinner"></span>
            {{ checking ? 'Re-reading every post and comment…' : 'Run independent check' }}
          </button>
          <p v-if="checking" class="quiet-line">This makes real model calls and can take a few minutes.</p>
          <p v-if="checkError" class="error-line">Check failed: {{ checkError }}</p>
        </template>
        <template v-else>
          <p class="block-text" v-if="check.labelled > 0">
            <strong class="agree-figure" :class="verdictClass">{{ pct(check.agreement_rate) }}</strong>
            &mdash; an independent model agreed with {{ pct(check.agreement_rate) }} of the agents'
            self-reported stances ({{ check.agree }} of {{ check.labelled }} labelled items).
            {{ verdictText }}
          </p>
          <p class="block-text" v-else>Nothing to check yet &mdash; the simulation has no stance-bearing posts.</p>
          <div class="agree-compare" v-if="agreeRows.length > 0">
            <div class="agree-head">
              <span></span><span>Self-reported</span><span>Independent</span>
            </div>
            <div v-for="row in agreeRows" :key="row.stance" class="agree-row">
              <span class="stance-name">{{ row.stance }}</span>
              <span class="stance-pct">{{ pct(row.selfShare) }}</span>
              <span class="stance-pct">{{ pct(row.indShare) }}</span>
            </div>
          </div>
        </template>
      </div>
    </template>

    <p v-else-if="error" class="quiet-line">Credibility data unavailable: {{ error }}</p>
  </section>
</template>

<script setup>
import { ref, computed, onMounted, watch } from 'vue'
import { getCredibility, classifyStance } from '../api/simulation'

const props = defineProps({
  simulationId: String
})

const cred = ref(null)
const loading = ref(false)
const error = ref(null)
const checking = ref(false)
const check = ref(null)
const checkError = ref(null)

// If the sim's data is entirely missing (e.g. deleted), hide rather than nag.
const hidden = computed(() => !loading.value && !cred.value && !error.value)

const pct = (x) => `${Math.round((x || 0) * 100)}%`

const shares = (rows) => {
  const total = (rows || []).reduce((s, r) => s + r.count, 0)
  const out = {}
  for (const r of rows || []) out[r.stance] = total > 0 ? r.count / total : 0
  return out
}

// Union of stances, ordered by agent-weighted share (the honest metric) desc.
const stanceRows = computed(() => {
  if (!cred.value) return []
  const agent = shares(cred.value.agent_weighted)
  const post = shares(cred.value.post_weighted)
  const names = [...new Set([...Object.keys(agent), ...Object.keys(post)])]
  if ((cred.value.agent_weighted || []).length === 0) return []
  return names
    .map((stance) => ({ stance, agentShare: agent[stance] || 0, postShare: post[stance] || 0 }))
    .sort((a, b) => b.agentShare - a.agentShare)
})

// Total variation distance between the two splits: half the sum of absolute gaps.
const divergence = computed(() => {
  return stanceRows.value.reduce((s, r) => s + Math.abs(r.agentShare - r.postShare), 0) / 2
})

const verdictClass = computed(() => {
  if (!check.value) return ''
  const r = check.value.agreement_rate
  if (r >= 0.8) return 'ok'
  if (r >= 0.6) return 'warn'
  return 'bad'
})

const verdictText = computed(() => {
  if (!check.value) return ''
  const r = check.value.agreement_rate
  if (r >= 0.8) return 'Treat the headline split with roughly that much confidence — the self-reports are broadly reliable.'
  if (r >= 0.6) return 'Treat the headline split with caution: a meaningful share of self-reports were mislabelled. Where the two disagree, prefer the independent column below.'
  return 'This is a red flag. The agents’ self-reported stances are unreliable, and the headline split should not be trusted — use the independent column below instead.'
})

const agreeRows = computed(() => {
  if (!check.value || check.value.labelled === 0) return []
  const self = shares(check.value.self_distribution)
  const ind = shares(check.value.independent_distribution)
  const names = [...new Set([...Object.keys(self), ...Object.keys(ind)])]
  return names
    .map((stance) => ({ stance, selfShare: self[stance] || 0, indShare: ind[stance] || 0 }))
    .sort((a, b) => b.indShare - a.indShare)
})

const load = async () => {
  if (!props.simulationId) return
  loading.value = true
  error.value = null
  try {
    const res = await getCredibility(props.simulationId)
    const d = res.data
    if (res.success && typeof d.total === 'number') cred.value = d
  } catch (err) {
    error.value = err.message
  } finally {
    loading.value = false
  }
}

const runCheck = async () => {
  checking.value = true
  checkError.value = null
  try {
    const res = await classifyStance(props.simulationId)
    const d = res.data
    if (res.success && d.agreement) check.value = d.agreement
  } catch (err) {
    checkError.value = err.message
  } finally {
    checking.value = false
  }
}

watch(() => props.simulationId, load)
onMounted(load)
</script>

<style scoped>
.credibility-panel {
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
  font-family: inherit;
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
  margin: 10px 0 0;
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
  margin: 8px 0 0;
}

/* Provenance bar */
.provenance-bar {
  display: flex;
  height: 10px;
  width: 100%;
  overflow: hidden;
  background: var(--subtle);
}

.bar-grounded {
  background: var(--emerald);
}

.bar-synthetic {
  background: repeating-linear-gradient(
    45deg,
    var(--border),
    var(--border) 4px,
    var(--subtle) 4px,
    var(--subtle) 8px
  );
}

.bar-legend {
  display: flex;
  gap: 16px;
  margin-top: 6px;
}

.legend-item {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--text-muted);
}

.swatch {
  width: 10px;
  height: 10px;
  display: inline-block;
}

.swatch-grounded {
  background: var(--emerald);
}

.swatch-synthetic {
  background: repeating-linear-gradient(45deg, var(--border), var(--border) 3px, var(--subtle) 3px, var(--subtle) 6px);
  border: 1px solid var(--border);
}

/* Stance comparison */
.stance-compare {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 24px;
}

.col-head {
  font-size: 13px;
  font-weight: 700;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: var(--text-strong);
  margin-bottom: 8px;
}

.col-head.muted {
  color: var(--text-muted);
}

.col-badge {
  font-weight: 400;
  font-style: italic;
  font-size: 12px;
  letter-spacing: normal;
  text-transform: none;
  color: var(--text-muted);
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
  color: var(--text-strong);
  text-transform: capitalize;
}

.stance-track {
  display: block;
  height: 8px;
  background: var(--subtle);
  overflow: hidden;
}

.stance-fill {
  display: block;
  height: 100%;
  background: var(--emerald);
}

.stance-fill.light {
  background: var(--text-muted);
}

.stance-pct {
  font-size: 12px;
  color: var(--text-muted);
  text-align: right;
  font-variant-numeric: tabular-nums;
}

.callout {
  font-size: 13.5px;
  line-height: 1.6;
  color: var(--text-strong);
  background: var(--white);
  border-left: 3px solid var(--coral-on-light);
  padding: 10px 14px;
  margin: 12px 0 0;
}

/* Independent check */
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
  transition: background 0.15s ease, color 0.15s ease;
}

.check-btn:hover:not(:disabled) {
  background: var(--navy);
  color: #fff;
}

.check-btn:disabled {
  opacity: 0.7;
  cursor: wait;
}

.btn-spinner {
  width: 12px;
  height: 12px;
  border: 2px solid currentColor;
  border-top-color: transparent;
  border-radius: 50%;
  animation: cred-spin 0.8s linear infinite;
}

@keyframes cred-spin {
  to { transform: rotate(360deg); }
}

.agree-figure {
  font-family: var(--font-serif);
  font-size: 22px;
  font-weight: 500;
  font-variant-numeric: tabular-nums;
}

.agree-figure.ok { color: var(--emerald); }
.agree-figure.warn { color: var(--coral-on-light); }
.agree-figure.bad { color: var(--coral-on-light); }

.agree-compare {
  max-width: 360px;
  margin-top: 8px;
}

.agree-head,
.agree-row {
  display: grid;
  grid-template-columns: 1fr 90px 90px;
  gap: 8px;
  align-items: baseline;
}

.agree-head {
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  color: var(--text-muted);
  border-bottom: 1px solid var(--border);
  padding-bottom: 4px;
  margin-bottom: 4px;
}

.agree-head span:nth-child(2),
.agree-head span:nth-child(3) {
  text-align: right;
}

.agree-row .stance-pct {
  font-size: 13px;
}
</style>
