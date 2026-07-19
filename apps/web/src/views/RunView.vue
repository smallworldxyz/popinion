<template>
  <div class="run">
    <header class="run-head">
      <div class="crumbs">
        <router-link :to="`/world/${graphId}`" class="crumb">← World</router-link>
        <span class="sep">·</span>
        <span class="run-name">{{ name }}</span>
      </div>
      <router-link :to="`/simulation/${simId}`" class="classic">Open classic controls →</router-link>
    </header>

    <div v-if="loading" class="state">Loading the run…</div>

    <div v-else-if="!personas.length" class="state empty">
      <div class="state-title">This run has no prepared Personas yet.</div>
      <p class="state-body">
        Personas are compiled from the World's graph when a run is prepared. If preparation was
        refused, the evidence bar was not met by any entity. Prepare or lower the bar in the classic
        controls, then this run will populate.
      </p>
      <router-link :to="`/simulation/${simId}`" class="state-link">Open classic controls →</router-link>
    </div>

    <div v-else class="summary">
      <div class="stats">
        <div class="stat"><span class="stat-num mono">{{ total }}</span><span class="stat-label">Agents</span></div>
        <div class="stat"><span class="stat-num mono">{{ grounded }}</span><span class="stat-label">Grounded</span></div>
        <div class="stat"><span class="stat-num mono">{{ synthetic }}</span><span class="stat-label">Synthetic</span></div>
        <div class="stat"><span class="stat-num mono">{{ contributions }}</span><span class="stat-label">Posts + Comments</span></div>
        <div class="stat"><span class="stat-num">{{ statusLabel }}</span><span class="stat-label">Status</span></div>
      </div>

      <section class="sheet stance-panel">
        <div class="eyebrow">Public reaction · one vote per agent</div>
        <template v-if="stanceRows.length">
          <div class="stance-bar">
            <div v-for="s in stanceRows" :key="s.stance" class="seg" :class="'st-' + s.stance" :style="{ width: s.pct + '%' }"></div>
          </div>
          <div class="stance-legend">
            <span v-for="s in stanceRows" :key="s.stance" class="leg"><i :class="'st-' + s.stance"></i>{{ s.stance }} {{ s.pct }}%</span>
          </div>
          <p class="note">
            Agent-weighted - each agent counts once, by its latest stance. This is the honest
            population read; conversation volume can be dominated by a few loud agents.
          </p>
        </template>
        <p v-else class="note">No stance-bearing activity recorded yet. Run this simulation from the classic controls.</p>
      </section>

      <!-- One CTA, and it lands on the report itself. Sending a finished run
           back through the setup screens just to reach its report was the long
           way round - and Step 2 reopens on "start the simulation again". -->
      <button class="report-cta" :disabled="reportBusy" @click="openReport">
        {{ reportBusy ? 'Opening report…' : (reportId ? 'View the full report' : 'Generate the full report') }}
        - sentiment, themes, and recommendations →
      </button>
      <p v-if="reportErr" class="note err">{{ reportErr }}</p>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { getSimulation, getSimulationProfiles, getCredibility, getAgentStats } from '../api/simulation'
import { checkReport, generateReport } from '../api/report'
import { runLifecycleStatus } from '../runStatus'

const props = defineProps({
  graphId: { type: String, required: true },
  simId: { type: String, required: true },
})

const router = useRouter()

const loading = ref(true)
const personas = ref([])
const name = ref('Run')
const cred = ref(null)
const agents = ref([])
const runLifecycle = ref('')
const reportId = ref(null)
const reportBusy = ref(false)
const reportErr = ref('')

// The report registry is in-process, so a report generated before a backend
// restart reads as missing. Generating again is the recovery, not an error.
const openReport = async () => {
  if (reportBusy.value) return
  if (reportId.value) return router.push(`/report/${reportId.value}`)
  reportBusy.value = true
  reportErr.value = ''
  try {
    const res = await generateReport({ simulation_id: props.simId })
    const id = res.data?.report_id
    if (!id) throw new Error('no report id')
    router.push(`/report/${id}`)
  } catch (e) {
    reportErr.value = 'Could not start the report. Open the classic controls to run it manually.'
  } finally {
    reportBusy.value = false
  }
}

const total = computed(() => cred.value?.total ?? personas.value.length)
const grounded = computed(() => cred.value?.grounded ?? 0)
const synthetic = computed(() => cred.value?.synthetic ?? 0)
// Agents almost always reply rather than open a thread, so post_count alone
// reads as 1 on a run that produced hundreds of comments.
const contributions = computed(() =>
  agents.value.reduce((n, a) => n + (a.posts || 0) + (a.comments || 0), 0)
)

const statusLabel = computed(() => runLifecycleStatus(runLifecycle.value).label)

const ORDER = { support: 0, neutral: 1, oppose: 2 }
const stanceRows = computed(() => {
  const dist = cred.value?.agent_weighted || []
  const rows = dist.filter((s) => s.stance && s.stance !== 'seed' && s.stance !== 'unknown')
  const t = rows.reduce((n, s) => n + (s.count || 0), 0)
  if (!t) return []
  return rows
    .map((s) => ({ stance: s.stance, pct: Math.round((s.count / t) * 100) }))
    .sort((a, b) => (ORDER[a.stance] ?? 9) - (ORDER[b.stance] ?? 9))
})

onMounted(async () => {
  try {
    const [meta, profs] = await Promise.all([
      getSimulation(props.simId).catch(() => null),
      getSimulationProfiles(props.simId).catch(() => null),
    ])
    const n = meta?.data?.name
    name.value = (n && n !== 'Untitled Simulation') ? n : 'Baseline run'
    runLifecycle.value = meta?.data?.status || ''
    personas.value = profs?.data?.profiles || []
    if (personas.value.length) {
      const [c, s, rep] = await Promise.all([
        getCredibility(props.simId).catch(() => null),
        getAgentStats(props.simId).catch(() => null),
        checkReport(props.simId).catch(() => null),
      ])
      cred.value = c?.data || null
      agents.value = s?.data?.agents || []
      reportId.value = rep?.data?.has_report ? rep.data.report_id : null
    }
  } finally {
    loading.value = false
  }
})
</script>

<style scoped>
.run { min-height: 100vh; background: var(--color-bg); color: var(--color-text); }
.run-head {
  display: flex; align-items: center; justify-content: space-between;
  padding: 16px 24px; border-bottom: 1px solid var(--color-border);
}
.crumbs { display: flex; align-items: center; gap: 10px; min-width: 0; }
.crumb { color: var(--color-accent-text); text-decoration: none; font-size: var(--fs-xs); }
.crumb:hover { text-decoration: underline; }
.sep { color: var(--color-text-muted); }
.run-name { font-weight: 650; font-size: var(--fs-sm); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.classic { color: var(--color-text-muted); text-decoration: none; font-size: var(--fs-xs); }
.classic:hover { color: var(--color-text); text-decoration: underline; }

.state { display: flex; flex-direction: column; align-items: center; justify-content: center; text-align: center; gap: 10px; color: var(--color-text-muted); padding: 80px 40px; }
.state-title { font-size: var(--fs-md); font-weight: 600; color: var(--color-text); }
.state-body { max-width: 480px; line-height: 1.55; font-size: var(--fs-sm); margin: 0; }
.state-link { color: var(--color-accent-text); text-decoration: none; font-size: var(--fs-xs); }

.summary { max-width: 760px; margin: 0 auto; padding: 32px 24px; display: flex; flex-direction: column; gap: 24px; }
.stats { display: flex; flex-wrap: wrap; gap: 28px; }
.stat { display: flex; flex-direction: column; gap: 2px; }
.stat-num { font-family: var(--font-mono); font-size: var(--fs-lg); font-weight: 600; line-height: 1; color: var(--color-text); }
.stat-label { font-size: var(--fs-2xs); letter-spacing: .08em; text-transform: uppercase; color: var(--color-text-muted); }

.stance-panel { padding: 20px 22px; }
.eyebrow { font-family: var(--font-mono); font-size: var(--fs-2xs); font-weight: 600; letter-spacing: .1em; text-transform: uppercase; color: var(--color-text-muted); }
.stance-bar { display: flex; height: 12px; border-radius: 999px; overflow: hidden; margin: 12px 0 10px; background: color-mix(in srgb, var(--color-text-muted) 18%, transparent); }
.seg { height: 100%; }
.stance-legend { display: flex; flex-wrap: wrap; gap: 8px 18px; font-size: var(--fs-sm); color: var(--color-text-muted); text-transform: capitalize; }
.leg i { display: inline-block; width: 10px; height: 10px; border-radius: 2px; margin-right: 6px; vertical-align: middle; }
.st-support { background: var(--stance-support); }
.st-neutral { background: var(--stance-neutral); }
.st-oppose { background: var(--stance-oppose); }
.note { margin: 12px 0 0; font-size: var(--fs-xs); color: var(--color-text-muted); line-height: 1.55; }

.report-cta {
  align-self: flex-start; border: none; background: none; padding: 0;
  color: var(--color-accent-text); font-size: var(--fs-sm); font-weight: 600;
  text-align: left; cursor: pointer;
}
.report-cta:hover:not(:disabled) { text-decoration: underline; }
.report-cta:disabled { color: var(--color-text-muted); cursor: default; }
.note.err { color: var(--stance-oppose); }
</style>
