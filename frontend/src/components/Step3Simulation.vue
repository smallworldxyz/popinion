<template>
  <div class="sim-run">
    <!-- Run header: status + live stats + report action -->
    <div class="run-header">
      <div class="run-status">
        <span class="status-dot" :class="statusClass"></span>
        <span class="status-text">{{ statusLabel }}</span>
      </div>

      <div class="run-stats">
        <div class="rstat">
          <span class="rstat-num mono">{{ currentRound }}<span class="rstat-den">/{{ maxRounds || '—' }}</span></span>
          <span class="rstat-label">Round</span>
        </div>
        <div class="rstat"><span class="rstat-num mono">{{ postCount }}</span><span class="rstat-label">Posts</span></div>
        <div class="rstat"><span class="rstat-num mono">{{ allActions.length }}</span><span class="rstat-label">Actions</span></div>
        <div class="rstat"><span class="rstat-num mono">{{ agentCount }}</span><span class="rstat-label">Agents</span></div>
      </div>

      <button class="report-btn" :disabled="!canReport || isGeneratingReport" @click="handleNextStep">
        <span v-if="isGeneratingReport" class="spinner"></span>
        {{ isGeneratingReport ? 'Starting…' : 'Generate Report' }}
        <span v-if="!isGeneratingReport" class="arrow">→</span>
      </button>
    </div>

    <!-- Live stance snapshot (post-weighted; the honest split lives in the report's credibility panel) -->
    <div class="stance-strip" v-if="stanceRows.length">
      <span class="strip-label">Live stance · posts so far</span>
      <div class="stance-bar">
        <div v-for="s in stanceRows" :key="s.stance" class="seg" :class="'st-' + s.stance" :style="{ width: s.pct + '%' }"></div>
      </div>
      <div class="stance-legend">
        <span v-for="s in stanceRows" :key="s.stance" class="leg"><i :class="'st-' + s.stance"></i>{{ s.stance }} {{ s.pct }}%</span>
      </div>
    </div>

    <!-- Activity feed: one honest stream of what the agents actually did -->
    <div class="feed" ref="scrollContainer">
      <div v-if="!allActions.length" class="feed-empty">
        <div class="pulse-ring"></div>
        <span>{{ status === 'running' ? 'Agents are deciding…' : 'Waiting for the simulation…' }}</span>
      </div>

      <TransitionGroup name="act">
        <div v-for="a in allActions" :key="a._id" class="act">
          <div class="act-avatar">{{ (nameFor(a.user_id) || 'A')[0] }}</div>
          <div class="act-body">
            <div class="act-head">
              <span class="act-name">{{ nameFor(a.user_id) }}</span>
              <span class="act-badge" :class="badgeClass(a.action)">{{ badgeLabel(a.action) }}</span>
              <span v-if="a.info && a.info.stance && a.info.stance !== 'seed'" class="act-stance" :class="'st-' + a.info.stance">{{ a.info.stance }}</span>
              <span class="act-round mono">R{{ a.round }}</span>
            </div>
            <div v-if="a.info && a.info.content" class="act-content">{{ a.info.content }}</div>
            <div v-else-if="a.action === 'do_nothing'" class="act-muted">
              skipped this round<span v-if="a.info && a.info.reasoning"> — {{ a.info.reasoning }}</span>
            </div>
            <div v-else-if="a.action === 'like_post'" class="act-muted">liked post #{{ a.info && a.info.target_post_id }}</div>
            <div v-else-if="a.action === 'dislike_post'" class="act-muted">disliked post #{{ a.info && a.info.target_post_id }}</div>
            <div v-else-if="a.action === 'follow'" class="act-muted">followed an account</div>
          </div>
        </div>
      </TransitionGroup>
    </div>

    <!-- Monitor log -->
    <div class="system-logs">
      <div class="log-header">
        <span class="log-title">SIMULATION MONITOR</span>
        <span class="log-id mono">{{ simulationId || 'NO_SIMULATION' }}</span>
      </div>
      <div class="log-content" ref="logContent">
        <div class="log-line" v-for="(log, idx) in systemLogs" :key="idx">
          <span class="log-time mono">{{ log.time }}</span>
          <span class="log-msg">{{ log.msg }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue'
import { useRouter } from 'vue-router'
import { startSimulation, getRunStatusDetail, getSimulationActions, getAgentStats } from '../api/simulation'
import { generateReport } from '../api/report'

const props = defineProps({
  simulationId: String,
  maxRounds: Number,
  projectData: Object,
  graphData: Object,
  systemLogs: Array,
})
const emit = defineEmits(['go-back', 'next-step', 'add-log', 'update-status'])
const router = useRouter()

const status = ref('initializing')
const postCount = ref(0)
const stance = ref([])
const allActions = ref([])
const seen = ref(new Set())
const names = ref({}) // user_id -> user_name
const isGeneratingReport = ref(false)
const scrollContainer = ref(null)
const logContent = ref(null)

const addLog = (msg) => emit('add-log', msg)

// ---- derived ----
const STATUS = {
  initializing: { label: 'Initializing…', cls: 'run' },
  running: { label: 'Running', cls: 'run' },
  alive: { label: 'Complete — agents standing by', cls: 'done' },
  stopped: { label: 'Stopped', cls: 'done' },
  not_running: { label: 'Not running', cls: 'idle' },
}
const statusLabel = computed(() => (STATUS[status.value] || { label: status.value }).label)
const statusClass = computed(() => (STATUS[status.value] || { cls: 'err' }).cls)
const canReport = computed(() => status.value === 'alive' || status.value === 'stopped')
const agentCount = computed(() => Object.keys(names.value).length)
const currentRound = computed(() =>
  allActions.value.reduce((m, a) => Math.max(m, a.round ?? 0), 0)
)
const stanceRows = computed(() => {
  const rows = (stance.value || []).filter((s) => s.stance !== 'seed')
  const total = rows.reduce((n, s) => n + (s.count || 0), 0)
  if (!total) return []
  return rows.map((s) => ({ stance: s.stance, pct: Math.round((s.count / total) * 100) }))
})

const nameFor = (uid) => names.value[uid] || (uid === -1 ? 'Event' : `Agent ${uid}`)

const BADGE = {
  create_post: ['POST', 'b-post'],
  create_comment: ['REPLY', 'b-reply'],
  like_post: ['LIKE', 'b-react'],
  dislike_post: ['DISLIKE', 'b-react'],
  follow: ['FOLLOW', 'b-meta'],
  do_nothing: ['IDLE', 'b-idle'],
  interview: ['INTERVIEW', 'b-meta'],
}
const badgeLabel = (a) => (BADGE[a] || [a?.toUpperCase() || '—'])[0]
const badgeClass = (a) => (BADGE[a] || ['', 'b-default'])[1]

// ---- lifecycle ----
const doStart = async () => {
  if (!props.simulationId) return
  addLog('Starting simulation…')
  emit('update-status', 'processing')
  try {
    const params = { simulation_id: props.simulationId }
    if (props.maxRounds) {
      params.max_rounds = props.maxRounds
      addLog(`Max rounds: ${props.maxRounds}`)
    }
    await startSimulation(params)
    addLog('✓ Simulation engine started')
  } catch (err) {
    // Already-running is fine — just attach to it; anything else is a real error.
    if (/already running/i.test(err.message || '')) {
      addLog('Simulation already running — attaching')
    } else {
      addLog(`✗ Start failed: ${err.message}`)
      emit('update-status', 'error')
    }
  }
  await loadNames()
  startPolling()
}

const loadNames = async () => {
  try {
    const res = await getAgentStats(props.simulationId)
    for (const a of res.agents || []) names.value[a.user_id] = a.user_name
  } catch { /* names are best-effort */ }
}

const poll = async () => {
  if (!props.simulationId) return
  try {
    const res = await getRunStatusDetail(props.simulationId)
    const prev = status.value
    status.value = res.status || status.value
    postCount.value = res.post_count || 0
    stance.value = Array.isArray(res.stance) ? res.stance : []
    if (prev !== status.value) {
      addLog(`Status: ${statusLabel.value}`)
      emit('update-status', canReport.value ? 'completed' : 'processing')
    }
  } catch (err) { /* transient */ }

  try {
    const res = await getSimulationActions(props.simulationId, { limit: 200 })
    // Endpoint returns newest-first; render oldest-first and append new ones.
    const list = (res.actions || []).slice().reverse()
    for (const a of list) {
      const id = `${a.round}:${a.user_id}:${a.action}:${a.created_at}`
      if (seen.value.has(id)) continue
      seen.value.add(id)
      allActions.value.push({ ...a, _id: id })
    }
    if (agentCount.value === 0) loadNames()
  } catch (err) { /* transient */ }
}

let timer = null
const startPolling = () => {
  poll()
  timer = setInterval(poll, 2500)
}
const stopPolling = () => {
  if (timer) clearInterval(timer)
  timer = null
}

const handleNextStep = async () => {
  if (!props.simulationId || isGeneratingReport.value) return
  isGeneratingReport.value = true
  addLog('Starting report generation…')
  try {
    const res = await generateReport({ simulation_id: props.simulationId, force_regenerate: true })
    const reportId = res.report_id || (res.data && res.data.report_id)
    if (reportId) {
      addLog(`✓ Report task started: ${reportId}`)
      router.push({ name: 'Report', params: { reportId } })
    } else {
      addLog('✗ Report generation returned no id')
      isGeneratingReport.value = false
    }
  } catch (err) {
    addLog(`✗ Report generation failed: ${err.message}`)
    isGeneratingReport.value = false
  }
}

watch(() => props.systemLogs?.length, () => {
  nextTick(() => {
    if (logContent.value) logContent.value.scrollTop = logContent.value.scrollHeight
  })
})

onMounted(() => {
  addLog('Simulation run initializing')
  if (props.simulationId) doStart()
})
onUnmounted(stopPolling)
</script>

<style scoped>
.sim-run { height: 100%; display: flex; flex-direction: column; background: #fff; overflow: hidden; }
.mono { font-family: 'JetBrains Mono', ui-monospace, monospace; }

/* header */
.run-header {
  display: flex; align-items: center; gap: 24px;
  padding: 14px 24px; border-bottom: 1px solid #eaeaea; flex-shrink: 0;
}
.run-status { display: flex; align-items: center; gap: 8px; min-width: 220px; }
.status-dot { width: 9px; height: 9px; border-radius: 50%; background: #9ca3af; }
.status-dot.run { background: #2563eb; animation: pulse 1.4s infinite; }
.status-dot.done { background: #059669; }
.status-dot.err { background: #dc2626; }
.status-text { font-size: 0.95rem; font-weight: 600; }
@keyframes pulse { 0%,100% { opacity: 1; } 50% { opacity: 0.35; } }

.run-stats { display: flex; gap: 26px; flex: 1; }
.rstat { display: flex; flex-direction: column; }
.rstat-num { font-size: 1.25rem; font-weight: 700; line-height: 1.1; }
.rstat-den { font-size: 0.85rem; color: #9ca3af; font-weight: 400; }
.rstat-label { font-size: 0.7rem; letter-spacing: 0.08em; text-transform: uppercase; color: #9ca3af; }

.report-btn {
  display: inline-flex; align-items: center; gap: 8px;
  padding: 10px 20px; background: #000; color: #fff; border: none; border-radius: 8px;
  font-family: inherit; font-size: 0.95rem; font-weight: 600; cursor: pointer;
}
.report-btn:disabled { opacity: 0.35; cursor: not-allowed; }
.report-btn:hover:not(:disabled) { background: #333; }

/* stance strip */
.stance-strip { padding: 12px 24px; border-bottom: 1px solid #f0f0f0; }
.strip-label { font-size: 0.72rem; letter-spacing: 0.06em; text-transform: uppercase; color: #9ca3af; }
.stance-bar { display: flex; height: 8px; border-radius: 999px; overflow: hidden; margin: 8px 0 6px; background: #f3f4f6; }
.seg { height: 100%; }
.stance-legend { display: flex; gap: 16px; font-size: 0.8rem; color: #4b5563; }
.leg i { display: inline-block; width: 9px; height: 9px; border-radius: 2px; margin-right: 5px; vertical-align: middle; }
.st-support { background: #2563eb; } .st-support.act-stance, .st-support.leg { background: none; }
.st-oppose { background: #dc2626; }
.st-neutral { background: #9ca3af; }
.st-unknown { background: #d1d5db; }

/* feed */
.feed { flex: 1; overflow-y: auto; padding: 16px 24px; max-width: 820px; margin: 0 auto; width: 100%; }
.feed-empty {
  display: flex; flex-direction: column; align-items: center; gap: 14px;
  color: #b0b0b0; font-size: 0.85rem; padding: 60px 0;
}
.pulse-ring { width: 32px; height: 32px; border-radius: 50%; border: 1px solid #e5e7eb; animation: ripple 2s infinite; }
@keyframes ripple { 0% { transform: scale(0.8); opacity: 1; } 100% { transform: scale(2.4); opacity: 0; } }

.act { display: flex; gap: 12px; padding: 12px 0; border-bottom: 1px solid #f3f4f6; }
.act-avatar {
  width: 30px; height: 30px; border-radius: 50%; background: #111; color: #fff;
  display: flex; align-items: center; justify-content: center; font-size: 0.8rem; font-weight: 700;
  text-transform: uppercase; flex-shrink: 0;
}
.act-body { flex: 1; min-width: 0; }
.act-head { display: flex; align-items: center; gap: 8px; margin-bottom: 3px; }
.act-name { font-weight: 600; font-size: 0.92rem; }
.act-badge { font-size: 0.62rem; letter-spacing: 0.05em; padding: 2px 6px; border-radius: 4px; font-weight: 700; }
.b-post { background: #eef2ff; color: #3730a3; }
.b-reply { background: #f0f0f0; color: #444; }
.b-react { background: #fff; color: #666; border: 1px solid #e5e7eb; }
.b-meta { background: #fafafa; color: #999; border: 1px dashed #ddd; }
.b-idle { background: #f9fafb; color: #bbb; }
.b-default { background: #f0f0f0; color: #666; }
.act-stance { font-size: 0.68rem; padding: 1px 7px; border-radius: 999px; color: #fff; font-weight: 600; }
.act-stance.st-support { background: #2563eb; }
.act-stance.st-oppose { background: #dc2626; }
.act-stance.st-neutral { background: #9ca3af; }
.act-round { margin-left: auto; font-size: 0.72rem; color: #b0b0b0; }
.act-content { font-size: 0.95rem; line-height: 1.55; color: #1f2937; }
.act-muted { font-size: 0.85rem; color: #9ca3af; font-style: italic; }

.act-enter-active { transition: all 0.35s ease; }
.act-enter-from { opacity: 0; transform: translateY(8px); }

/* logs */
.system-logs { background: #000; color: #ddd; padding: 14px 16px; border-top: 1px solid #222; flex-shrink: 0; }
.log-header { display: flex; justify-content: space-between; border-bottom: 1px solid #333; padding-bottom: 6px; margin-bottom: 6px; font-size: 0.62rem; color: #666; letter-spacing: 0.05em; }
.log-content { display: flex; flex-direction: column; gap: 3px; height: 84px; overflow-y: auto; }
.log-line { font-size: 0.72rem; display: flex; gap: 12px; line-height: 1.5; }
.log-time { color: #555; min-width: 72px; }
.log-msg { color: #bbb; word-break: break-word; }
.spinner {
  width: 13px; height: 13px; border: 2px solid rgba(255,255,255,0.3); border-top-color: #fff;
  border-radius: 50%; animation: spin 0.8s linear infinite;
}
@keyframes spin { to { transform: rotate(360deg); } }
</style>
