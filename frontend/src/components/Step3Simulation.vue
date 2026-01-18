<template>
  <div class="simulation-panel">
    <!-- Top Control Bar -->
    <div class="control-bar">
      <div class="status-group">
        <!-- Twitter Platform Progress -->
        <div class="platform-status twitter" :class="{ active: runStatus.twitter_running, completed: runStatus.twitter_completed }">
          <div class="platform-header">
            <svg class="platform-icon" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="12" cy="12" r="10"></circle><line x1="2" y1="12" x2="22" y2="12"></line><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"></path>
            </svg>
            <span class="platform-name">Info Plaza</span>
            <span v-if="runStatus.twitter_completed" class="status-badge">
              <svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="3">
                <polyline points="20 6 9 17 4 12"></polyline>
              </svg>
            </span>
          </div>
          <div class="platform-stats">
            <span class="stat">
              <span class="stat-label">ROUND</span>
              <span class="stat-value mono">{{ runStatus.twitter_current_round || 0 }}<span class="stat-total">/{{ runStatus.total_rounds || maxRounds || '-' }}</span></span>
            </span>
            <span class="stat">
              <span class="stat-label">Elapsed Time</span>
              <span class="stat-value mono">{{ twitterElapsedTime }}</span>
            </span>
            <span class="stat">
              <span class="stat-label">ACTS</span>
              <span class="stat-value mono">{{ runStatus.twitter_actions_count || 0 }}</span>
            </span>
          </div>
          <!-- Available ActionsHint -->
          <div class="actions-tooltip">
            <div class="tooltip-title">Available Actions</div>
            <div class="tooltip-actions">
              <span class="tooltip-action">POST</span>
              <span class="tooltip-action">LIKE</span>
              <span class="tooltip-action">REPOST</span>
              <span class="tooltip-action">QUOTE</span>
              <span class="tooltip-action">FOLLOW</span>
              <span class="tooltip-action">IDLE</span>
            </div>
          </div>
        </div>
        
        <!-- Reddit Platform Progress -->
        <div class="platform-status reddit" :class="{ active: runStatus.reddit_running, completed: runStatus.reddit_completed }">
          <div class="platform-header">
            <svg class="platform-icon" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M21 11.5a8.38 8.38 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.38 8.38 0 0 1-3.8-.9L3 21l1.9-5.7a8.38 8.38 0 0 1-.9-3.8 8.5 8.5 0 0 1 4.7-7.6 8.38 8.38 0 0 1 3.8-.9h.5a8.48 8.48 0 0 1 8 8v.5z"></path>
            </svg>
            <span class="platform-name">Topic Community</span>
            <span v-if="runStatus.reddit_completed" class="status-badge">
              <svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="3">
                <polyline points="20 6 9 17 4 12"></polyline>
              </svg>
            </span>
          </div>
          <div class="platform-stats">
            <span class="stat">
              <span class="stat-label">ROUND</span>
              <span class="stat-value mono">{{ runStatus.reddit_current_round || 0 }}<span class="stat-total">/{{ runStatus.total_rounds || maxRounds || '-' }}</span></span>
            </span>
            <span class="stat">
              <span class="stat-label">Elapsed Time</span>
              <span class="stat-value mono">{{ redditElapsedTime }}</span>
            </span>
            <span class="stat">
              <span class="stat-label">ACTS</span>
              <span class="stat-value mono">{{ runStatus.reddit_actions_count || 0 }}</span>
            </span>
          </div>
          <!-- Available ActionsHint -->
          <div class="actions-tooltip">
            <div class="tooltip-title">Available Actions</div>
            <div class="tooltip-actions">
              <span class="tooltip-action">POST</span>
              <span class="tooltip-action">COMMENT</span>
              <span class="tooltip-action">LIKE</span>
              <span class="tooltip-action">DISLIKE</span>
              <span class="tooltip-action">SEARCH</span>
              <span class="tooltip-action">TREND</span>
              <span class="tooltip-action">FOLLOW</span>
              <span class="tooltip-action">MUTE</span>
              <span class="tooltip-action">REFRESH</span>
              <span class="tooltip-action">IDLE</span>
            </div>
          </div>
        </div>
      </div>

      <div class="action-controls">
        <button 
          class="action-btn"
          :class="{'warning': runStatus.runner_status === 'running', 'success': runStatus.runner_status === 'paused'}"
          v-if="phase === 1" 
          @click="togglePauseResume"
          :disabled="isToggling"
        >
          <span v-if="isToggling" class="loading-spinner-small"></span>
          <span v-if="runStatus.runner_status === 'running'">PAUSE</span>
          <span v-else-if="runStatus.runner_status === 'paused'">RESUME</span>
          <span v-else>PAUSE</span>
        </button>

        <button 
          class="action-btn primary"
          :disabled="(phase !== 2 && runStatus.runner_status !== 'stopped' && runStatus.runner_status !== 'completed') || isGeneratingReport"
          @click="handleNextStep"
        >
          <span v-if="isGeneratingReport" class="loading-spinner-small"></span>
          {{ isGeneratingReport ? 'Starting...' : 'Start Generating Result Report' }} 
          <span v-if="!isGeneratingReport" class="arrow-icon">→</span>
        </button>
      </div>

    </div>

    <!-- MultiVerse Map (Tree) -->
    <SimulationTree 
        :simulation-id="props.simulationId" 
        :height="150"
        @switch-simulation="handleSwitchSimulation"
    />

    <!-- Replay Control Bar (Visible only when Completed/Stopped) -->
    <div class="replay-bar" v-if="phase === 2">
        <div class="replay-toggle">
            <label class="switch">
                <input type="checkbox" v-model="isReplayMode">
                <span class="slider round"></span>
            </label>
            <span class="replay-label">SESSION REPLAY</span>
        </div>
        
        <div class="replay-controls" v-if="isReplayMode">
            <button class="control-btn" @click="replayRound = Math.max(1, replayRound - 1)">
                <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><path d="M11 19l-7-7 7-7m8 14l-7-7 7-7"></path></svg>
            </button>
            
            <div class="scrubber-container">
                <input 
                    type="range" 
                    min="1" 
                    :max="maxReplayRound" 
                    v-model.number="replayRound" 
                    class="scrubber"
                >
                <div class="scrubber-value">Round {{ replayRound }} / {{ maxReplayRound }}</div>
            </div>

            <button class="control-btn" @click="replayRound = Math.min(maxReplayRound, replayRound + 1)">
                <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor"><path d="M13 5l7 7-7 7M5 5l7 7-7 7"></path></svg>
            </button>
        </div>
    </div>

    <!-- Omega Dashboard Content Grid -->
    <div class="omega-grid">
        <!-- LEFT COLUMN: CONTEXT (Live Feed) -->
        <div class="grid-col left-col">
            <div class="col-header">
                <span class="col-title">LIVE FEED</span>
                <div class="feed-tools">
                    <input 
                        v-model="feedSearchQuery" 
                        type="text" 
                        class="feed-search-input" 
                        placeholder="Filter feed..."
                    >
                </div>
            </div>
            
            <div class="feed-container" ref="scrollContainer">
                 <!-- Waiting State -->
                <div v-if="allActions.length === 0" class="waiting-state">
                  <div class="pulse-ring"></div>
                  <span>Waiting for signal...</span>
                </div>
                
                <TransitionGroup name="timeline-item">
                  <div 
                    v-for="action in sortedActions" 
                    :key="action._uniqueId" 
                    class="timeline-item"
                    :class="action.platform"
                  >
                        <div class="card-header compact">
                            <span class="agent-name">{{ action.agent_name }}</span>
                            <span class="action-badge-mini" :class="action.action_type">{{ action.action_type }}</span>
                        </div>
                        <div class="card-body compact">
                             {{ truncateContent(action.action_args?.content || action.action_args?.quote_content || '', 140) }}
                        </div>
                        <div class="card-footer compact">
                            <span class="time-tag">R{{ action.round_num }}</span>
                        </div>
                  </div>
                </TransitionGroup>
            </div>
        </div>
        
        <!-- CENTER COLUMN: CONTROL (Director Console) -->
        <div class="grid-col center-col">
            <!-- Sentiment Pulse -->
            <div class="pulse-widget">
               <SentimentPulse :simulation-id="props.simulationId" :rounds="runStatus.rounds_history || []" :height="80" />
            </div>
            
            <!-- Director Tools -->
             <div class="director-tools-container">
                <div class="tool-group">
                    <div class="tool-label">INJECTION</div>
                     <textarea 
                        v-model="injectionText" 
                        placeholder="Inject Event..." 
                        rows="2" 
                        class="director-input"
                    ></textarea>
                    <button class="director-btn small" @click="handleInjectEvent" :disabled="isInjecting">INJECT</button>
                </div>
                
                <div class="tool-group">
                    <div class="tool-label">TIME MACHINE</div>
                     <div class="fork-row">
                        <input type="number" v-model.number="forkRoundInput" placeholder="Round #" class="director-input small">
                        <button class="director-btn small warning" @click="handleForkRound" :disabled="isForking">FORK</button>
                     </div>
                </div>
                
                 <div class="tool-group">
                    <div class="tool-label">LIVE SEARCH</div>
                    <div class="search-row">
                        <input type="text" v-model="searchQuery" placeholder="Search Google..." class="director-input small" @keyup.enter="handleLiveSearch">
                        <button class="director-btn small secondary" @click="handleLiveSearch" :disabled="isSearching">GO</button>
                    </div>
                </div>
            </div>
            
            <!-- Global Controls -->
            <div class="global-controls">
                 <button 
                  class="action-btn"
                  :class="{'warning': runStatus.runner_status === 'running', 'success': runStatus.runner_status === 'paused'}"
                  v-if="phase === 1" 
                  @click="togglePauseResume"
                  :disabled="isToggling"
                >
                  {{ runStatus.runner_status === 'running' ? 'PAUSE SIMULATION' : 'RESUME SIMULATION' }}
                </button>
                 <button 
                  class="action-btn primary"
                  :disabled="(phase !== 2 && runStatus.runner_status !== 'stopped' && runStatus.runner_status !== 'completed') || isGeneratingReport"
                  @click="handleNextStep"
                >
                  <span v-if="isGeneratingReport" class="loading-spinner-small"></span>
                  {{ isGeneratingReport ? 'Generating...' : 'GENERATE REPORT' }} 
                </button>
            </div>

        </div>
        
        <!-- RIGHT COLUMN: ACTORS (Agent Census) -->
        <div class="grid-col right-col">
            <div class="col-header">
                <span class="col-title">AGENTS</span>
                <span class="agent-count">{{ availableAgents.length }} Active</span>
            </div>
            
             <div class="agent-list">
                <div 
                    v-for="agent in availableAgents" 
                    :key="agent.id" 
                    class="agent-card-mini"
                    :class="{ active: activeAgentForGreenRoom?.id === agent.id }"
                    @click="activeAgentForGreenRoom = agent"
                >
                    <div class="agent-avatar-mini">{{ agent.name[0] }}</div>
                    <div class="agent-details">
                        <div class="agent-name">{{ agent.name }}</div>
                        <div class="agent-role">{{ agent.profession }}</div>
                    </div>
                     <button class="interrogate-btn" @click.stop="openInterrogation(agent)">
                        <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"></path></svg>
                     </button>
                </div>
            </div>
        </div>
    </div>
    
    <GreenRoomModal 
        :is-open="greenRoomOpen" 
        :simulation-id="props.simulationId"
        :agent="activeAgentForGreenRoom"
        @close="greenRoomOpen = false"
    />

    <!-- System Logs Footer -->
    <div class="system-logs">
      <div class="log-header">
        <span class="log-title">SYSTEM LOGS</span>
      </div>
      <div class="log-content" ref="logContent">
        <div class="log-line" v-for="(log, idx) in systemLogs" :key="idx">
          <span class="log-time">{{ log.time }}</span>
          <span class="log-msg">{{ log.msg }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue'
import { useRouter } from 'vue-router'
import { 
  startSimulation, 
  stopSimulation,
  pauseSimulation,
  resumeSimulation,
  forkSimulation,
  getRunStatus, 
  getRunStatusDetail
} from '../api/simulation'
import { generateReport } from '../api/report'
import { 
    injectEvent, 
    getProjectAgents, 
    annotateAction, 
    getAnnotations 
} from '../api/simulation' 
import { searchRealWorld } from '../api/tools' 
import GreenRoomModal from './GreenRoomModal.vue'
import AnnotationMarker from './AnnotationMarker.vue'
import SimulationTree from './SimulationTree.vue'
import SentimentPulse from './SentimentPulse.vue'


const props = defineProps({
  simulationId: String,
  maxRounds: Number, // max rounds count passed from Step 2
  minutesPerRound: {
    type: Number,
    default: 30 // Default 30 minutes per round
  },
  projectData: Object,
  graphData: Object,
  systemLogs: Array
})

const emit = defineEmits(['go-back', 'next-step', 'add-log', 'update-status', 'switch-simulation'])

const router = useRouter()

// State
const isGeneratingReport = ref(false)
const phase = ref(0) // 0: Not Started, 1: Running, 2: Completed
const isStarting = ref(false)
const isStopping = ref(false)
const isToggling = ref(false) // For Pause/Resume
const startError = ref(null)
const runStatus = ref({})
const allActions = ref([]) // All Actions (Incremental Accumulation)
const actionIds = ref(new Set()) // Action ID set for deduplication
const scrollContainer = ref(null)

const isInjecting = ref(false)
const injectionText = ref('')
const searchQuery = ref('')
const isSearching = ref(false)
const searchResults = ref([])
// Feed Search
const feedSearchQuery = ref('')
const forkRoundInput = ref(null)
const isForking = ref(false)

// Green Room State
const greenRoomOpen = ref(false)
const activeAgentForGreenRoom = ref(null)
const availableAgents = ref([])
const loadingAgents = ref(false)

// Replay & Annotation State
const isReplayMode = ref(false)
const replayRound = ref(1)
const annotations = ref({}) // { action_id: { content, author } }

// --- Computed ---

const sortedActions = computed(() => {
    let actions = [...allActions.value]
    // Filter by Feed Search
    if (feedSearchQuery.value.trim()) {
        const q = feedSearchQuery.value.toLowerCase()
        actions = actions.filter(a => 
            (a.content && a.content.toLowerCase().includes(q)) ||
            (a.agent_name && a.agent_name.toLowerCase().includes(q))
        )
    }
    // Sort chronological for display (usually latest at bottom of scroll, or reverse?)
    // Popinion usually appends.
    return actions.sort((a, b) => a.timestamp - b.timestamp)
})

const maxReplayRound = computed(() => {
    return runStatus.value.total_rounds || props.maxRounds || 10
})

const twitterElapsedTime = computed(() => {
    const hours = runStatus.value.twitter_simulated_hours || 0
    return `${hours.toFixed(1)}h`
})

const redditElapsedTime = computed(() => {
    const hours = runStatus.value.reddit_simulated_hours || 0
    return `${hours.toFixed(1)}h`
})

// --- Helpers ---

const addLog = (msg) => emit('add-log', msg)

const handleSwitchSimulation = (id) => {
    emit('switch-simulation', id)
}

const formatTime = (ts) => {
    if (!ts) return ''
    return new Date(ts * 1000).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
}

const truncateContent = (text, len = 50) => {
    if (!text) return ''
    return text.length > len ? text.substring(0, len) + '...' : text
}

// --- Polling ---

let statusTimer = null
let detailTimer = null

const stopPolling = () => {
    if (statusTimer) { clearInterval(statusTimer); statusTimer = null; }
    if (detailTimer) { clearInterval(detailTimer); detailTimer = null; }
}

const checkPlatformsCompleted = (data) => {
  if (!data) return false
  const twitterFinished = (data.twitter_completed === true) || (data.twitter_current_round > 0 && data.twitter_current_round >= data.total_rounds)
  const redditFinished = (data.reddit_completed === true) || (data.reddit_current_round > 0 && data.reddit_current_round >= data.total_rounds)
  return twitterFinished && redditFinished
}

const fetchRunStatus = async () => {
  if (!props.simulationId) return
  
  try {
    const res = await getRunStatus(props.simulationId)
    if (res.success && res.data) {
      const data = res.data
      runStatus.value = data
      
      const isCompleted = data.runner_status === 'completed' || data.runner_status === 'stopped'
      const platformsCompleted = checkPlatformsCompleted(data)
      
      if (isCompleted || platformsCompleted) {
         if (platformsCompleted && !isCompleted && phase.value !== 2) {
             addLog('✓ Platforms finished')
         }
         if (phase.value !== 2) {
             addLog('✓ Simulation Completed')
             phase.value = 2
             stopPolling()
             emit('update-status', 'completed')
         }
      } else {
          phase.value = 1
      }
    }
  } catch (err) {
    console.error("Poll Error", err)
  }
}

const fetchRunStatusDetail = async () => {
  if (!props.simulationId) return
  try {
    const res = await getRunStatusDetail(props.simulationId)
    if (res.success && res.data) {
      const serverActions = res.data.all_actions || []
      serverActions.forEach(action => {
        // Safe ID generation
        const actionId = action.id || action._uniqueId || `${action.timestamp}-${action.platform}-${action.agent_id}-${action.action_type}`
        if (!actionIds.value.has(actionId)) {
          actionIds.value.add(actionId)
          allActions.value.push({ ...action, _uniqueId: actionId })
        }
      })
    }
  } catch (err) {
    console.warn('Detail poll failed', err)
  }
}

const startPolling = () => {
    stopPolling()
    fetchRunStatus()
    fetchRunStatusDetail()
    statusTimer = setInterval(fetchRunStatus, 2000)
    detailTimer = setInterval(fetchRunStatusDetail, 3000)
}

// --- Agent & Green Room ---

const fetchAgents = async () => {
    if (!props.simulationId || !props.projectData?.id) return
    loadingAgents.value = true
    try {
        const res = await getProjectAgents(props.projectData.id, props.simulationId)
        if (res.success && res.data) {
             availableAgents.value = res.data.map(a => ({
                id: a.agent_id || a.id,
                name: a.agent_name || a.name || `Agent ${a.id}`,
                profession: a.profession || 'Unknown',
                bio: a.bio || a.background || '',
                stance: a.stance || 'Neutral', 
                personality: a.personality || ''
            }))
        }
    } catch (e) {
        console.error("Fetch Agents Failed", e)
    } finally {
        loadingAgents.value = false
    }
}

const openGreenRoom = () => {
    if (activeAgentForGreenRoom.value) greenRoomOpen.value = true
}

const openInterrogation = (agent) => {
    activeAgentForGreenRoom.value = agent
    openGreenRoom()
}

// --- Actions ---

const handleStart = async () => {
    isStarting.value = true
    try {
        const res = await startSimulation(props.simulationId)
        if (res.success) {
            addLog('Simulation Started')
            phase.value = 1
            startPolling()
        } else {
            startError.value = res.error
        }
    } catch (e) {
        startError.value = e.message
    } finally {
        isStarting.value = false
    }
}

const togglePauseResume = async () => {
    isToggling.value = true
    try {
        if (runStatus.value.runner_status === 'running') {
            await pauseSimulation(props.simulationId)
            addLog('Paused')
        } else {
            await resumeSimulation(props.simulationId)
            addLog('Resumed')
        }
        await fetchRunStatus() // Refresh immediately
    } catch (e) {
        console.error(e)
    } finally {
        isToggling.value = false
    }
}

const handleInjectEvent = async () => {
     if (!injectionText.value.trim()) return
     isInjecting.value = true
     try {
         await injectEvent(props.simulationId, injectionText.value)
         addLog(`Injected: ${injectionText.value}`)
         injectionText.value = ''
     } catch (e) { console.error(e) }
     finally { isInjecting.value = false }
}

const handleLiveSearch = () => {
    // Placeholder
    console.log("Live search not implemented yet")
}

const handleForkRound = async () => {
    // Placeholder
    console.log("Forking not implemented yet")
    if (forkRoundInput.value) {
        addLog(`Mock Fork from Round ${forkRoundInput.value}`)
    }
}

const handleNextStep = () => {
    emit('next-step')
}

const handleSaveAnnotation = ({ actionId, content }) => {
    // Placeholder
    console.log("Saving annotation", actionId, content)
}

// --- Lifecycle ---

onMounted(() => {
    addLog('Entering Director Mode')
    startPolling()
    fetchAgents()
})

onUnmounted(() => {
    stopPolling()
})
</script>
<style src="../views/director_panel.css" scoped></style>

<style scoped>
.simulation-panel {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: #FFFFFF;
  font-family: 'Space Grotesk', -apple-system, BlinkMacSystemFont, sans-serif;
  overflow: hidden;
}

/* --- Control Bar --- */
.control-bar {
  background: #FFF;
  padding: 12px 24px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  border-bottom: 1px solid #EAEAEA;
  z-index: 10;
  height: 64px;
}

.status-group {
  display: flex;
  gap: 12px;
}

/* Platform Status Cards */
.platform-status {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 6px 12px;
  border-radius: 4px;
  background: #FAFAFA;
  border: 1px solid #EAEAEA;
  opacity: 0.7;
  transition: all 0.3s;
  min-width: 140px;
  position: relative;
  cursor: pointer;
}

.platform-status.active {
  opacity: 1;
  border-color: #333;
  background: #FFF;
}

.platform-status.completed {
  opacity: 1;
  border-color: #1A936F;
  background: #F2FAF6;
}

/* Actions Tooltip */
.actions-tooltip {
  position: absolute;
  top: 100%;
  left: 50%;
  transform: translateX(-50%);
  margin-top: 8px;
  padding: 10px 14px;
  background: #000;
  color: #FFF;
  border-radius: 4px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  opacity: 0;
  visibility: hidden;
  transition: all 0.2s ease;
  z-index: 100;
  min-width: 180px;
  pointer-events: none;
}

.actions-tooltip::before {
  content: '';
  position: absolute;
  top: -6px;
  left: 50%;
  transform: translateX(-50%);
  border-left: 6px solid transparent;
  border-right: 6px solid transparent;
  border-bottom: 6px solid #000;
}

.platform-status:hover .actions-tooltip {
  opacity: 1;
  visibility: visible;
}

.tooltip-title {
  font-size: 10px;
  font-weight: 600;
  color: #999;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  margin-bottom: 8px;
}

.tooltip-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.tooltip-action {
  font-size: 10px;
  font-weight: 600;
  padding: 3px 8px;
  background: rgba(255, 255, 255, 0.15);
  border-radius: 2px;
  color: #FFF;
  letter-spacing: 0.03em;
}

.platform-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 2px;
}

.platform-name {
  font-size: 11px;
  font-weight: 700;
  color: #000;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.platform-status.twitter .platform-icon { color: #000; }
.platform-status.reddit .platform-icon { color: #000; }

.platform-stats {
  display: flex;
  gap: 10px;
}

.stat {
  display: flex;
  align-items: baseline;
  gap: 3px;
}

.stat-label {
  font-size: 8px;
  color: #999;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.stat-value {
  font-size: 11px;
  font-weight: 600;
  color: #333;
}

.stat-total, .stat-unit {
  font-size: 9px;
  color: #999;
  font-weight: 400;
}

.status-badge {
  margin-left: auto;
  color: #1A936F;
  display: flex;
  align-items: center;
}

/* Action Button */
.action-btn {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 10px 20px;
  font-size: 13px;
  font-weight: 600;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  transition: all 0.2s ease;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.action-btn.primary {
  background: #000;
  color: #FFF;
}

.action-btn.primary:hover:not(:disabled) {
  background: #333;
}

.action-btn:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}

/* --- Main Content Area --- */
.main-content-area {
  flex: 1;
  overflow-y: auto;
  position: relative;
  background: #FFF;
}

/* Timeline Header */
.timeline-header {
  position: sticky;
  top: 0;
  background: rgba(255, 255, 255, 0.9);
  backdrop-filter: blur(8px);
  padding: 12px 24px;
  border-bottom: 1px solid #EAEAEA;
  z-index: 5;
  display: flex;
  justify-content: center;
}

.timeline-stats {
  display: flex;
  align-items: center;
  gap: 16px;
  font-size: 11px;
  color: #666;
  background: #F5F5F5;
  padding: 4px 12px;
  border-radius: 20px;
}

.total-count {
  font-weight: 600;
  color: #333;
}

.platform-breakdown {
  display: flex;
  align-items: center;
  gap: 8px;
}

.breakdown-item {
  display: flex;
  align-items: center;
  gap: 4px;
}

.breakdown-divider { color: #DDD; }
.breakdown-item.twitter { color: #000; }
.breakdown-item.reddit { color: #000; }

/* --- Timeline Feed --- */
.timeline-feed {
  padding: 24px 0;
  position: relative;
  min-height: 100%;
  max-width: 900px;
  margin: 0 auto;
}

.timeline-axis {
  position: absolute;
  left: 50%;
  top: 0;
  bottom: 0;
  width: 1px;
  background: #EAEAEA; /* Cleaner line */
  transform: translateX(-50%);
}

.timeline-item {
  display: flex;
  justify-content: center;
  margin-bottom: 32px;
  position: relative;
  width: 100%;
}

.timeline-marker {
  position: absolute;
  left: 50%;
  top: 24px;
  width: 10px;
  height: 10px;
  background: #FFF;
  border: 1px solid #CCC;
  border-radius: 50%;
  transform: translateX(-50%);
  z-index: 2;
  display: flex;
  align-items: center;
  justify-content: center;
}

.marker-dot {
  width: 4px;
  height: 4px;
  background: #CCC;
  border-radius: 50%;
}

.timeline-item.twitter .marker-dot { background: #000; }
.timeline-item.reddit .marker-dot { background: #000; }
.timeline-item.twitter .timeline-marker { border-color: #000; }
.timeline-item.reddit .timeline-marker { border-color: #000; }

/* Card Layout */
.timeline-card {
  width: calc(100% - 48px);
  background: #FFF;
  border-radius: 2px;
  padding: 16px 20px;
  border: 1px solid #EAEAEA;
  box-shadow: 0 2px 10px rgba(0,0,0,0.02);
  position: relative;
  transition: all 0.2s;
}

.timeline-card:hover {
  box-shadow: 0 4px 12px rgba(0,0,0,0.05);
  border-color: #DDD;
}

/* Left side (Twitter) */
.timeline-item.twitter {
  justify-content: flex-start;
  padding-right: 50%;
}
.timeline-item.twitter .timeline-card {
  margin-left: auto;
  margin-right: 32px; /* Gap from axis */
}

/* Right side (Reddit) */
.timeline-item.reddit {
  justify-content: flex-end;
  padding-left: 50%;
}
.timeline-item.reddit .timeline-card {
  margin-right: auto;
  margin-left: 32px; /* Gap from axis */
}

/* Card Content Styles */
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 12px;
  padding-bottom: 12px;
  border-bottom: 1px solid #F5F5F5;
}

.agent-info {
  display: flex;
  align-items: center;
  gap: 10px;
}

.avatar-placeholder {
  width: 24px;
  height: 24px;
  background: #000;
  color: #FFF;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  font-weight: 700;
  text-transform: uppercase;
}

.agent-name {
  font-size: 13px;
  font-weight: 600;
  color: #000;
}

.header-meta {
  display: flex;
  align-items: center;
  gap: 8px;
}

.platform-indicator {
  color: #999;
  display: flex;
  align-items: center;
}

.action-badge {
  font-size: 9px;
  padding: 2px 6px;
  border-radius: 2px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  border: 1px solid transparent;
}

/* Monochromatic Badges */
.badge-post { background: #F0F0F0; color: #333; border-color: #E0E0E0; }
.badge-comment { background: #F0F0F0; color: #666; border-color: #E0E0E0; }
.badge-action { background: #FFF; color: #666; border: 1px solid #E0E0E0; }
.badge-meta { background: #FAFAFA; color: #999; border: 1px dashed #DDD; }
.badge-idle { opacity: 0.5; }

.content-text {
  font-size: 13px;
  line-height: 1.6;
  color: #333;
  margin-bottom: 10px;
}

.content-text.main-text {
  font-size: 14px;
  color: #000;
}

/* Info Blocks (Quote, Repost, etc) */
.quoted-block, .repost-content {
  background: #F9F9F9;
  border: 1px solid #EEE;
  padding: 10px 12px;
  border-radius: 2px;
  margin-top: 8px;
  font-size: 12px;
  color: #555;
}

.quote-header, .repost-info, .like-info, .search-info, .follow-info, .vote-info, .idle-info, .comment-context {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 6px;
  font-size: 11px;
  color: #666;
}

.icon-small {
  color: #999;
}
.icon-small.filled {
  color: #999; /* Keep icons neutral unless highlighted */
}

.search-query {
  font-family: 'JetBrains Mono', monospace;
  background: #F0F0F0;
  padding: 0 4px;
  border-radius: 2px;
}

.card-footer {
  margin-top: 12px;
  display: flex;
  justify-content: flex-end;
  font-size: 10px;
  color: #BBB;
  font-family: 'JetBrains Mono', monospace;
}

/* Waiting State */
.waiting-state {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
  color: #CCC;
  font-size: 12px;
  text-transform: uppercase;
  letter-spacing: 0.1em;
}

/* Replay Bar */
.replay-bar {
    background: #111;
    border-bottom: 1px solid #333;
    padding: 10px 20px;
    display: flex;
    align-items: center;
    gap: 20px;
    color: #fff;
}

.replay-toggle {
    display: flex;
    align-items: center;
    gap: 10px;
}

.replay-label {
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 1px;
    color: #f59e0b;
}

/* Switch Toggle */
.switch {
  position: relative;
  display: inline-block;
  width: 36px;
  height: 20px;
}
.switch input { opacity: 0; width: 0; height: 0; }
.slider {
  position: absolute;
  cursor: pointer;
  top: 0; left: 0; right: 0; bottom: 0;
  background-color: #333;
  transition: .4s;
  border-radius: 20px;
}
.slider:before {
  position: absolute;
  content: "";
  height: 16px;
  width: 16px;
  left: 2px;
  bottom: 2px;
  background-color: white;
  transition: .4s;
  border-radius: 50%;
}
input:checked + .slider { background-color: #f59e0b; }
input:checked + .slider:before { transform: translateX(16px); }

.replay-controls {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 10px;
}

.scrubber-container {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 10px;
}

.scrubber {
    flex: 1;
    cursor: pointer;
}

.scrubber-value {
    font-family: 'JetBrains Mono', monospace;
    font-size: 12px;
    color: #888;
    width: 100px;
    text-align: right;
}

.control-btn {
    background: none;
    border: 1px solid #333;
    color: #ccc;
    width: 28px;
    height: 28px;
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
}
.control-btn:hover { background: #222; color: #fff; }

.footer-left { display: flex; align-items: center; }
.footer-right { display: flex; align-items: center; }


.pulse-ring {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  border: 1px solid #EAEAEA;
  animation: ripple 2s infinite;
}

@keyframes ripple {
  0% { transform: scale(0.8); opacity: 1; border-color: #CCC; }
  100% { transform: scale(2.5); opacity: 0; border-color: #EAEAEA; }
}

/* Animation */
.timeline-item-enter-active,
.timeline-item-leave-active {
  transition: all 0.4s cubic-bezier(0.165, 0.84, 0.44, 1);
}

.timeline-item-enter-from {
  opacity: 0;
  transform: translateY(20px);
}

.timeline-item-leave-to {
  opacity: 0;
}

/* Logs */
.system-logs {
  background: #000;
  color: #DDD;
  padding: 16px;
  font-family: 'JetBrains Mono', monospace;
  border-top: 1px solid #222;
  flex-shrink: 0;
}

.log-header {
  display: flex;
  justify-content: space-between;
  border-bottom: 1px solid #333;
  padding-bottom: 8px;
  margin-bottom: 8px;
  font-size: 10px;
  color: #666;
}

.log-content {
  display: flex;
  flex-direction: column;
  gap: 4px;
  height: 100px;
  overflow-y: auto;
  padding-right: 4px;
}

.log-content::-webkit-scrollbar { width: 4px; }
.log-content::-webkit-scrollbar-thumb { background: #333; border-radius: 2px; }

.log-line {
  font-size: 11px;
  display: flex;
  gap: 12px;
  line-height: 1.5;
}

.log-time { color: #555; min-width: 75px; }
.log-msg { color: #BBB; word-break: break-all; }
.mono { font-family: 'JetBrains Mono', monospace; }

/* Loading spinner for button */
.loading-spinner-small {
  display: inline-block;
  width: 14px;
  height: 14px;
  border: 2px solid rgba(255, 255, 255, 0.3);
  border-top-color: #FFF;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
  margin-right: 6px;
}
.director-select {
    flex: 1;
    background: #000;
    border: 1px solid #333;
    color: #fff;
    padding: 8px 12px;
    border-radius: 4px;
    font-family: 'JetBrains Mono', monospace;
    font-size: 12px;
}
.director-btn.action {
    background: #059669;
    color: #fff;
    border: none;
}
.director-btn.action:hover:not(:disabled) {
    background: #10B981;
}


/* --- Omega Dashboard Styles --- */
.omega-grid {
    display: grid;
    grid-template-columns: 2fr 1.5fr 1fr;
    grid-template-rows: minmax(0, 1fr); /* Fill available height */
    gap: 16px;
    height: calc(100vh - 350px); /* Adjust based on header/tree/logs */
    min-height: 400px;
    margin-bottom: 20px;
}

.grid-col {
    background: rgba(0,0,0,0.2);
    border: 1px solid rgba(255,255,255,0.05);
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
}

.col-header {
    padding: 10px 15px;
    background: rgba(0,0,0,0.3);
    border-bottom: 1px solid rgba(255,255,255,0.05);
    display: flex;
    justify-content: space-between;
    align-items: center;
}

.col-title {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 1px;
    color: #888;
}

/* Feed Column */
.feed-container {
    flex: 1;
    overflow-y: auto;
    padding: 10px;
    display: flex;
    flex-direction: column;
    gap: 10px;
}

.feed-tools {
    flex: 1;
    margin-left: 20px;
}

.feed-search-input {
    width: 100%;
    background: transparent;
    border: 1px solid #444;
    border-radius: 4px;
    color: #fff;
    font-size: 12px;
    padding: 4px 8px;
}

/* Timeline Item Compact Override for Feed */
.timeline-item {
    background: rgba(255,255,255,0.03);
    border: 1px solid rgba(255,255,255,0.05);
    border-radius: 6px;
    padding: 10px;
}

.card-header.compact {
    display: flex;
    justify-content: space-between;
    margin-bottom: 5px;
}

.action-badge-mini {
    font-size: 9px;
    padding: 2px 6px;
    border-radius: 4px;
    background: #333;
    color: #ccc;
    text-transform: uppercase;
}

.card-body.compact {
    font-size: 13px;
    line-height: 1.4;
    color: #ddd;
    margin-bottom: 5px;
}

.card-footer.compact {
    font-size: 10px;
    color: #666;
    display: flex;
    justify-content: space-between;
}


/* Center Column (Control) */
.center-col {
    padding: 15px;
    gap: 20px;
}

.director-tools-container {
    display: flex;
    flex-direction: column;
    gap: 15px;
}

.tool-group {
    background: rgba(255,255,255,0.03);
    padding: 10px;
    border-radius: 6px;
    border: 1px solid rgba(255,255,255,0.05);
}

.tool-label {
    font-size: 10px;
    color: #888;
    margin-bottom: 8px;
    font-weight: 600;
    letter-spacing: 0.5px;
}

.director-btn.small {
    font-size: 11px;
    padding: 6px 12px;
}

.fork-row, .search-row {
    display: flex;
    gap: 8px;
}

.global-controls {
    margin-top: auto;
    display: flex;
    gap: 10px;
}

/* Right Column (Agents) */
.agent-list {
    flex: 1;
    overflow-y: auto;
    padding: 10px;
    display: flex;
    flex-direction: column;
    gap: 8px;
}

.agent-card-mini {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px;
    background: rgba(255,255,255,0.05);
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.2s;
}

.agent-card-mini:hover {
    background: rgba(255,255,255,0.1);
}

.agent-card-mini.active {
    border: 1px solid #00ff9d;
    background: rgba(0, 255, 157, 0.1);
}

.agent-avatar-mini {
    width: 24px;
    height: 24px;
    background: #444;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 10px;
    font-weight: bold;
    color: #fff;
}

.agent-details {
    flex: 1;
    min-width: 0;
}

.agent-name {
    font-size: 12px;
    font-weight: 600;
    color: #eee;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.agent-role {
    font-size: 10px;
    color: #888;
}

.interrogate-btn {
    background: transparent;
    border: none;
    color: #00ff9d;
    cursor: pointer;
    padding: 4px;
    opacity: 0.5;
}

.agent-card-mini:hover .interrogate-btn {
    opacity: 1;
}

</style>