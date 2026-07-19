<template>
  <div class="sim-run">
    <!-- Command bar: status + live stats + run controls -->
    <header class="cmd-bar">
      <div class="cmd-status">
        <span class="status-dot" :class="statusClass"></span>
        <div class="status-meta">
          <span class="eyebrow">Live run</span>
          <span class="status-text">{{ statusLabel }}</span>
        </div>
      </div>

      <div class="cmd-stats">
        <div class="rstat">
          <span class="rstat-num mono">{{ currentRound }}<span class="rstat-den">/{{ maxRounds || '—' }}</span></span>
          <span class="stat-label">Round</span>
        </div>
        <div class="rstat"><span class="rstat-num mono">{{ postCount }}</span><span class="stat-label">Posts</span></div>
        <div class="rstat"><span class="rstat-num mono">{{ allActions.length }}</span><span class="stat-label">Actions</span></div>
        <div class="rstat"><span class="rstat-num mono">{{ agentCount }}</span><span class="stat-label">Agents</span></div>
      </div>

      <div class="cmd-actions">
        <button
          v-if="status === 'running' || status === 'initializing'"
          class="btn btn-danger stop-btn"
          :disabled="stopping"
          @click="doStop"
        >
          <span v-if="stopping" class="spinner spinner-dark"></span>
          <span v-else class="stop-glyph"></span>
          {{ stopping ? 'Stopping…' : 'Stop' }}
        </button>

        <button class="btn btn-primary report-btn" :disabled="!canReport || isGeneratingReport" @click="handleNextStep">
          <span v-if="isGeneratingReport" class="spinner spinner-dark"></span>
          {{ isGeneratingReport ? 'Starting…' : 'Generate Report' }}
          <span v-if="!isGeneratingReport" class="arrow">→</span>
        </button>
      </div>
    </header>

    <!-- Body: focused activity stream + persistent control rail -->
    <div class="sim-body">
      <!-- Activity feed: one honest stream of what the agents actually did -->
      <section class="feed" ref="scrollContainer">
        <div v-if="!allActions.length" class="feed-empty">
          <div class="pulse-ring"></div>
          <span>{{ status === 'running' ? 'Agents are deciding…' : 'Waiting for the simulation…' }}</span>
        </div>

        <TransitionGroup name="act" tag="div" class="feed-stream">
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
              <div v-else-if="a.action === 'like_post'" class="act-muted">liked post #<span class="mono">{{ a.info && a.info.target_post_id }}</span></div>
              <div v-else-if="a.action === 'dislike_post'" class="act-muted">disliked post #<span class="mono">{{ a.info && a.info.target_post_id }}</span></div>
              <div v-else-if="a.action === 'follow'" class="act-muted">followed an account</div>
            </div>
          </div>
        </TransitionGroup>
      </section>

      <!-- Control rail: stance, steer, spread -->
      <aside class="rail">
        <!-- Live stance snapshot (post-weighted; the honest split lives in the report's credibility panel) -->
        <section class="rail-block" v-if="stanceRows.length">
          <div class="rail-title">
            <span class="eyebrow">Live stance</span>
            <span class="rail-sub mono">posts so far</span>
          </div>
          <div class="stance-bar">
            <div v-for="s in stanceRows" :key="s.stance" class="seg" :class="'st-' + s.stance" :style="{ width: s.pct + '%' }"></div>
          </div>
          <div class="stance-legend">
            <span v-for="s in stanceRows" :key="s.stance" class="leg"><i :class="'st-' + s.stance"></i>{{ s.stance }} <b class="mono">{{ s.pct }}%</b></span>
          </div>
        </section>

        <!-- Mid-run injection: red-team the population while the discussion is live -->
        <section class="rail-block steer" v-if="status === 'running' || status === 'initializing'">
          <div class="rail-title"><span class="eyebrow">Steer the discussion</span></div>
          <p class="rail-hint">Drop a message into the live discussion — disinformation, a policy reversal, an opponent's statement.</p>
          <div class="inject-row">
            <input
              v-model="injectContent"
              class="field inject-input"
              placeholder="e.g. Leaked memo: the ministry plans to cancel the program"
              :disabled="injecting"
              @keyup.enter="doInject"
            />
            <button class="inject-btn" :disabled="injecting || !injectContent.trim()" @click="doInject">
              {{ injecting ? 'Injecting…' : 'Inject' }}
            </button>
          </div>
          <div v-if="injectNote" class="inject-note" :class="{ err: injectError }">{{ injectNote }}</div>
        </section>

        <!-- Spread: where the seed & injected messages landed -->
        <section class="rail-block" v-if="spreadRows.length">
          <div class="rail-title">
            <span class="eyebrow">Spread</span>
            <button class="spread-refresh" :disabled="spreadLoading" @click="loadSpread">
              {{ spreadLoading ? '…' : 'Refresh' }}
            </button>
          </div>
          <div v-for="p in spreadRows" :key="p.post_id" class="spread-row">
            <div class="spread-content">
              <span class="spread-tag mono" :class="{ injected: p.injected }">{{ p.injected ? `INJECTED R${p.round}` : 'SEED' }}</span>
              <span class="spread-text">{{ p.content }}</span>
            </div>
            <div class="spread-metrics mono">
              <span>reach {{ p.reach }}</span>
              <span>{{ p.engagement.likes }} likes · {{ p.engagement.dislikes }} dislikes · {{ p.engagement.comments }} replies</span>
              <span :title="shiftTitle(p)">{{ shiftText(p) }}</span>
            </div>
          </div>
          <p class="spread-caption">
            Shift compares agents who were served the post (acting after exposure) with those who never saw it,
            on a support&nbsp;+1&hellip;oppose&nbsp;&minus;1 axis. Observational — exposure follows network position, not random assignment.
          </p>
        </section>
      </aside>
    </div>

    <!-- Monitor log: slim bottom ribbon -->
    <footer class="system-logs">
      <div class="log-header">
        <span class="log-title mono">SIMULATION MONITOR</span>
        <span class="log-id mono">{{ simulationId || 'NO_SIMULATION' }}</span>
      </div>
      <div class="log-content" ref="logContent">
        <div class="log-line" v-for="(log, idx) in systemLogs" :key="idx">
          <span class="log-time mono">{{ log.time }}</span>
          <span class="log-msg">{{ log.msg }}</span>
        </div>
      </div>
    </footer>
  </div>
</template>

<script setup>
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue'
import { useRouter } from 'vue-router'
import { startSimulation, stopSimulation, getRunStatusDetail, getSimulationActions, getAgentStats, injectPost, getSpread } from '../api/simulation'
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
const injectContent = ref('')
const injecting = ref(false)
const injectNote = ref('')
const injectError = ref(false)
const spreadRows = ref([])
const spreadLoading = ref(false)
const stopping = ref(false)
// Set once the user stops the run, so a late in-flight poll can't overwrite the
// "stopped" status with the "not_running" the backend reports after the handle drops.
const terminated = ref(false)
let pollTick = 0

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
    for (const a of res.data.agents || []) names.value[a.user_id] = a.user_name
  } catch { /* names are best-effort */ }
}

const poll = async () => {
  if (!props.simulationId || terminated.value) return
  try {
    const res = await getRunStatusDetail(props.simulationId)
    const prev = status.value
    status.value = res.data.status || status.value
    postCount.value = res.data.post_count || 0
    stance.value = Array.isArray(res.data.stance) ? res.data.stance : []
    if (prev !== status.value) {
      addLog(`Status: ${statusLabel.value}`)
      emit('update-status', canReport.value ? 'completed' : 'processing')
    }
  } catch (err) { /* transient */ }

  try {
    const res = await getSimulationActions(props.simulationId, { limit: 200 })
    // Endpoint returns newest-first; render oldest-first and append new ones.
    const list = (res.data.actions || []).slice().reverse()
    for (const a of list) {
      const id = `${a.round}:${a.user_id}:${a.action}:${a.created_at}`
      if (seen.value.has(id)) continue
      seen.value.add(id)
      allActions.value.push({ ...a, _id: id })
    }
    if (agentCount.value === 0) loadNames()
  } catch (err) { /* transient */ }

  // Spread is a cheap read but not worth every tick — every 4th poll (~10s).
  pollTick += 1
  if (pollTick % 4 === 1) loadSpread()

  // Once the run reaches a terminal state there's nothing left to refresh —
  // stop the 2.5s interval instead of polling forever.
  if (canReport.value) stopPolling()
}

// ---- stop the run early ----
// A graceful stop: the engine finishes the round in flight, then halts. The
// posts already written stay on disk, so the report can be generated from the
// partial run. We flip the local status to "stopped" (which enables the report
// action) since the backend drops the live handle once stopped.
const doStop = async () => {
  if (stopping.value || terminated.value) return
  stopping.value = true
  addLog('Stopping simulation…')
  try {
    await stopSimulation({ simulation_id: props.simulationId })
    terminated.value = true
    status.value = 'stopped'
    stopPolling()
    addLog('✓ Simulation stopped — report can be generated from the partial run')
    emit('update-status', 'completed')
  } catch (err) {
    addLog(`✗ Stop failed: ${err.message}`)
  } finally {
    stopping.value = false
  }
}

// ---- mid-run injection + spread ----
const doInject = async () => {
  const content = injectContent.value.trim()
  if (!content || injecting.value) return
  injecting.value = true
  injectError.value = false
  injectNote.value = ''
  try {
    const res = await injectPost(props.simulationId, { content })
    const round = res.data?.injected_at_round
    injectNote.value = `Landed at round ${round} — agents see it from the next round`
    addLog(`✓ Injected post at round ${round}`)
    injectContent.value = ''
    loadSpread()
  } catch (err) {
    const msg = err.response?.data?.error || err.message
    injectError.value = true
    injectNote.value = `Injection failed: ${msg}`
    addLog(`✗ Injection failed: ${msg}`)
  } finally {
    injecting.value = false
  }
}

const loadSpread = async () => {
  if (!props.simulationId || spreadLoading.value) return
  spreadLoading.value = true
  try {
    const res = await getSpread(props.simulationId)
    spreadRows.value = res.data?.posts || []
  } catch { /* no seed/injected posts yet — nothing to show */ }
  spreadLoading.value = false
}

const shiftText = (p) => {
  if (p.shift == null) return 'shift —'
  return `shift ${p.shift >= 0 ? '+' : ''}${p.shift.toFixed(2)}`
}
const shiftTitle = (p) => {
  const e = p.exposed_stance || {}
  const u = p.unexposed_stance || {}
  if (p.shift == null) return 'Not enough stance-bearing agents in one of the groups yet'
  return `${e.agents} exposed agents (mean ${e.mean?.toFixed(2)}) vs ${u.agents} unexposed (mean ${u.mean?.toFixed(2)})`
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
    const reportId = res.data?.report_id
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
.sim-run {
  height: 100%; display: flex; flex-direction: column;
  background: var(--color-bg); color: var(--color-text);
  font-family: var(--font-body); overflow: hidden;
}

/* ─── Command bar ──────────────────────────────────────────────────────────── */
.cmd-bar {
  display: flex; align-items: center; gap: 32px; flex-shrink: 0;
  padding: 12px 24px;
  background: var(--color-surface); border-bottom: 1px solid var(--color-border);
}
.cmd-status { display: flex; align-items: center; gap: 10px; min-width: 190px; }
.status-meta { display: flex; flex-direction: column; gap: 1px; line-height: 1.1; }
.status-text { font-size: var(--fs-sm); font-weight: 600; color: var(--color-text); }
.status-dot { width: 9px; height: 9px; border-radius: 50%; background: var(--color-text-muted); flex-shrink: 0; }
.status-dot.run { background: var(--stance-oppose); animation: pulse 1.4s infinite; }
.status-dot.done { background: var(--color-accent); }
.status-dot.idle { background: var(--color-text-muted); }
.status-dot.err { background: var(--stance-oppose-strong); }
@keyframes pulse { 0%,100% { opacity: 1; } 50% { opacity: 0.3; } }

.cmd-stats { display: flex; gap: 30px; flex: 1; }
.rstat { display: flex; flex-direction: column; gap: 2px; }
.rstat-num {
  font-family: var(--font-mono); font-size: var(--fs-md); font-weight: 600;
  line-height: 1; color: var(--color-text);
}
.rstat-den { font-size: var(--fs-xs); color: var(--color-text-muted); font-weight: 400; }

.cmd-actions { display: flex; align-items: center; gap: 10px; }
.stop-btn { padding: 9px 16px; }
.stop-glyph { width: 9px; height: 9px; border-radius: 2px; background: currentColor; }
.report-btn .arrow { font-weight: 400; }

/* ─── Body: feed + rail ────────────────────────────────────────────────────── */
.sim-body { flex: 1; display: flex; min-height: 0; }

/* Activity feed — a single flush stream on the ground inset, no cards. */
.feed { flex: 1; overflow-y: auto; padding: 20px 28px; min-width: 0; }
.feed-stream { max-width: 760px; margin: 0 auto; }
.feed-empty {
  display: flex; flex-direction: column; align-items: center; gap: 14px;
  color: var(--color-text-muted); font-size: var(--fs-sm); padding: 72px 0;
}
.pulse-ring { width: 32px; height: 32px; border-radius: 50%; border: 1px solid var(--color-border); animation: ripple 2s infinite; }
@keyframes ripple { 0% { transform: scale(0.8); opacity: 1; } 100% { transform: scale(2.4); opacity: 0; } }

.act { display: flex; gap: 12px; padding: 14px 0; border-bottom: 1px solid var(--color-border); }
.act:last-child { border-bottom: none; }
.act-avatar {
  width: 30px; height: 30px; border-radius: 50%;
  background: var(--color-accent-2); color: var(--color-on-accent-2);
  display: flex; align-items: center; justify-content: center;
  font-size: var(--fs-xs); font-weight: 600; text-transform: uppercase; flex-shrink: 0;
}
.act-body { flex: 1; min-width: 0; }
.act-head { display: flex; align-items: center; gap: 8px; margin-bottom: 3px; }
.act-name { font-weight: 600; font-size: var(--fs-sm); color: var(--color-text); }
.act-badge {
  font-family: var(--font-mono); font-size: var(--fs-2xs); letter-spacing: 0.05em;
  padding: 2px 6px; border-radius: 4px; font-weight: 600;
  background: var(--color-bg); color: var(--color-text-muted); border: 1px solid var(--color-border);
}
.b-post { background: var(--color-accent); color: var(--color-on-accent); border-color: transparent; }
.b-reply { background: var(--color-bg); color: var(--color-text); }
.b-react { background: transparent; color: var(--color-text-muted); }
.b-meta { background: transparent; color: var(--color-text-muted); border-style: dashed; }
.b-idle { background: transparent; color: var(--color-text-muted); opacity: 0.7; }
.b-default { background: var(--color-bg); color: var(--color-text-muted); }
.act-stance {
  font-size: var(--fs-2xs); padding: 1px 8px; border-radius: 999px; color: #fff; font-weight: 600;
  text-transform: capitalize;
}
.act-stance.st-support { background: var(--stance-support-strong); }
.act-stance.st-oppose { background: var(--stance-oppose); }
.act-stance.st-neutral { background: var(--stance-neutral); color: var(--ink); }
.act-round { margin-left: auto; font-size: var(--fs-xs); color: var(--color-text-muted); }
.act-content { font-size: var(--fs-sm); line-height: 1.55; color: var(--color-text); }
.act-muted { font-size: var(--fs-xs); color: var(--color-text-muted); font-style: italic; }

.act-enter-active { transition: opacity 0.35s ease, transform 0.35s ease; }
.act-enter-from { opacity: 0; transform: translateY(8px); }

/* ─── Control rail (sheet) ─────────────────────────────────────────────────── */
.rail {
  width: 340px; flex-shrink: 0; overflow-y: auto;
  background: var(--color-surface); border-left: 1px solid var(--color-border);
  display: flex; flex-direction: column;
}
.rail-block { padding: 18px 20px; border-bottom: 1px solid var(--color-border); }
.rail-block:last-child { border-bottom: none; }
.rail-title { display: flex; align-items: baseline; justify-content: space-between; gap: 10px; margin-bottom: 12px; }
.rail-sub { font-size: var(--fs-2xs); color: var(--color-text-muted); }
.rail-hint { margin: 0 0 12px; font-size: var(--fs-xs); line-height: 1.5; color: var(--color-text-muted); }

/* stance bar */
.stance-bar {
  display: flex; height: 8px; border-radius: 999px; overflow: hidden;
  margin-bottom: 10px; background: var(--color-bg);
}
.seg { height: 100%; }
.stance-legend { display: flex; flex-wrap: wrap; gap: 8px 16px; font-size: var(--fs-xs); color: var(--color-text-muted); text-transform: capitalize; }
.leg { display: inline-flex; align-items: center; }
.leg b { font-weight: 600; color: var(--color-text); margin-left: 4px; }
.leg i { display: inline-block; width: 9px; height: 9px; border-radius: 2px; margin-right: 5px; }
/* Functional stance ramp (blue↔amber, tritan-safe) — not the brand pair. */
.st-support { background: var(--stance-support); }
.st-oppose { background: var(--stance-oppose); }
.st-neutral { background: var(--stance-neutral); }
.st-unknown { background: var(--color-border); }

/* steer / inject — event injection carries the gold ember. */
.steer { background: var(--color-bg); }
.inject-row { display: flex; flex-direction: column; gap: 8px; }
.inject-input { width: 100%; }
.inject-input:disabled { opacity: 0.6; cursor: not-allowed; }
.inject-btn {
  padding: 8px 16px; background: var(--color-accent); color: var(--color-on-accent);
  border: none; border-radius: var(--radius-sm);
  font-family: var(--font-body); font-size: var(--fs-sm); font-weight: 600; cursor: pointer;
  transition: background 0.15s;
}
.inject-btn:disabled { opacity: 0.4; cursor: not-allowed; }
.inject-btn:hover:not(:disabled) { background: var(--color-accent-hover); }
.inject-note { margin-top: 8px; font-size: var(--fs-xs); color: var(--color-accent-text); }
.inject-note.err { color: var(--stance-oppose); }

/* spread */
.spread-refresh {
  border: 1px solid var(--color-border); background: transparent; color: var(--color-text-muted);
  padding: 3px 12px; border-radius: var(--radius-sm);
  font-family: var(--font-mono); font-size: var(--fs-2xs); font-weight: 600; cursor: pointer;
  transition: border-color 0.15s, color 0.15s;
}
.spread-refresh:disabled { opacity: 0.5; cursor: default; }
.spread-refresh:hover:not(:disabled) { border-color: var(--color-accent); color: var(--color-accent-text); }
.spread-row { padding: 10px 0; border-bottom: 1px solid var(--color-border); }
.spread-row:last-of-type { border-bottom: none; }
.spread-content { display: flex; align-items: baseline; gap: 8px; }
.spread-tag {
  font-size: var(--fs-2xs); letter-spacing: 0.04em; padding: 2px 6px; border-radius: 4px;
  font-weight: 600; background: var(--color-bg); color: var(--color-text-muted);
  border: 1px solid var(--color-border); flex-shrink: 0;
}
.spread-tag.injected { background: var(--color-accent); color: var(--color-on-accent); border-color: transparent; }
.spread-text {
  font-size: var(--fs-xs); color: var(--color-text);
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
.spread-metrics { display: flex; flex-wrap: wrap; gap: 4px 14px; margin-top: 6px; font-size: var(--fs-2xs); color: var(--color-text-muted); }
.spread-caption { margin: 12px 0 0; font-size: var(--fs-2xs); color: var(--color-text-muted); line-height: 1.5; }

/* ─── Monitor log ribbon ───────────────────────────────────────────────────── */
.system-logs {
  flex-shrink: 0; background: var(--navy); color: var(--on-dark-muted);
  padding: 12px 20px; border-top: 1px solid var(--border-dark);
}
.log-header {
  display: flex; justify-content: space-between; border-bottom: 1px solid var(--border-dark);
  padding-bottom: 6px; margin-bottom: 6px; font-size: var(--fs-2xs); letter-spacing: 0.08em;
}
.log-title { color: var(--gold-bright); }
.log-id { color: var(--on-dark-muted); opacity: 0.7; }
.log-content { display: flex; flex-direction: column; gap: 3px; height: 76px; overflow-y: auto; }
.log-line { font-size: var(--fs-xs); display: flex; gap: 12px; line-height: 1.5; }
.log-time { color: var(--purple-bright); min-width: 72px; }
.log-msg { color: var(--on-dark-muted); word-break: break-word; }

/* ─── Spinners ─────────────────────────────────────────────────────────────── */
.spinner {
  width: 13px; height: 13px; border: 2px solid rgba(255,255,255,0.3); border-top-color: #fff;
  border-radius: 50%; animation: spin 0.8s linear infinite;
}
.spinner-dark { border: 2px solid rgba(0,0,0,0.25); border-top-color: currentColor; }
@keyframes spin { to { transform: rotate(360deg); } }

/* ─── Responsive ───────────────────────────────────────────────────────────── */
@media (max-width: 1100px) {
  .cmd-bar { gap: 18px; flex-wrap: wrap; }
  .cmd-stats { gap: 22px; order: 3; flex-basis: 100%; }
  .rail { width: 300px; }
}
</style>
