<template>
  <div class="main-view">
    <!-- Premium Command Bar -->
    <header class="command-bar glass-panel">
      <div class="left-controls">
        <button class="icon-btn" @click="router.push('/')">←</button>
        <span class="mission-title">Mission Control / Operation {{ currentSimulationId ? currentSimulationId.substring(0,8) : 'UNK' }}</span>
      </div>
      
      <div class="center-controls">
        <div class="view-toggles">
           <button 
            v-for="mode in ['graph', 'split', 'workbench']" 
            :key="mode"
            class="toggle-chip"
            :class="{ active: viewMode === mode }"
            @click="viewMode = mode"
          >
            {{ mode.toUpperCase() }}
          </button>
        </div>
      </div>

      <div class="right-controls">
        <span class="status-indicator">
          <span class="pulse-dot" :class="currentStatus"></span>
          {{ statusText }}
        </span>
      </div>
    </header>

    <!-- Main Content Area -->
    <main class="content-area">
      <!-- Left Panel: Graph -->
      <div class="panel-wrapper left" :style="leftPanelStyle">
        <GraphPanel 
          :graphData="graphData"
          :loading="graphLoading"
          :currentPhase="3"
          :isSimulating="isSimulating"
          @refresh="refreshGraph"
          @toggle-maximize="toggleMaximize('graph')"
        />
      </div>

      <!-- Right Panel: Command HUD -->
      <div class="panel-wrapper right" :style="rightPanelStyle">
        <Step3Simulation
          :simulationId="currentSimulationId"
          :maxRounds="maxRounds"
          :minutesPerRound="minutesPerRound"
          :projectData="projectData"
          :graphData="graphData"
          :systemLogs="systemLogs"
          @go-back="handleGoBack"
          @next-step="handleNextStep"
          @add-log="addLog"
          @update-status="updateStatus"
        />
      </div>
    </main>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import GraphPanel from '../components/GraphPanel.vue'
import Step3Simulation from '../components/Step3Simulation.vue'
import { getProject, getGraphData } from '../api/graph'
import { getSimulation, getSimulationConfig, stopSimulation, closeSimulationEnv, getEnvStatus, injectEvent } from '../api/simulation'

const route = useRoute()
const router = useRouter()

// Props
const props = defineProps({
  simulationId: String
})

// Layout State
const viewMode = ref('split')

// Data State
const currentSimulationId = ref(route.params.simulationId)
const maxRounds = ref(route.query.maxRounds ? parseInt(route.query.maxRounds) : null)
const minutesPerRound = ref(30)
const projectData = ref(null)
const graphData = ref(null)
const graphLoading = ref(false)
const systemLogs = ref([])
const currentStatus = ref('processing')

// --- Computed Layout Styles ---
const leftPanelStyle = computed(() => {
  if (viewMode.value === 'graph') return { width: '100%', opacity: 1, transform: 'translateX(0)' }
  if (viewMode.value === 'workbench') return { width: '0%', opacity: 0, transform: 'translateX(-20px)' }
  return { width: '50%', opacity: 1, transform: 'translateX(0)' }
})

const rightPanelStyle = computed(() => {
  if (viewMode.value === 'workbench') return { width: '100%', opacity: 1, transform: 'translateX(0)' }
  if (viewMode.value === 'graph') return { width: '0%', opacity: 0, transform: 'translateX(20px)' }
  return { width: '50%', opacity: 1, transform: 'translateX(0)' }
})

// --- Status Computed ---
const statusText = computed(() => {
  if (currentStatus.value === 'error') return 'SYSTEM FAILURE'
  if (currentStatus.value === 'completed') return 'MISSION COMPLETE'
  return 'LIVE CONNECTION'
})

const isSimulating = computed(() => currentStatus.value === 'processing')

// --- Helpers ---
const addLog = (msg) => {
  const time = new Date().toLocaleTimeString('en-US', { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit' }) + '.' + new Date().getMilliseconds().toString().padStart(3, '0')
  systemLogs.value.push({ time, msg })
  if (systemLogs.value.length > 200) {
    systemLogs.value.shift()
  }
}

const updateStatus = (status) => {
  currentStatus.value = status
}

// --- Layout Methods ---
const toggleMaximize = (target) => {
  viewMode.value = viewMode.value === target ? 'split' : target
}

const handleGoBack = async () => {
  addLog('Terminating session...')
  stopGraphRefresh()
  try {
     await stopSimulation({ simulation_id: currentSimulationId.value })
  } catch (e) {
      console.warn(e)
  }
  router.push({ name: 'Simulation', params: { simulationId: currentSimulationId.value } })
}

const handleNextStep = () => {
    // Handled by child
}

// --- Data Logic ---
const loadSimulationData = async () => {
  try {
    addLog(`Establishing Uplink: ${currentSimulationId.value}`)
    const simRes = await getSimulation(currentSimulationId.value)
    if (simRes.success && simRes.data) {
      const simData = simRes.data
      
      try {
        const configRes = await getSimulationConfig(currentSimulationId.value)
        if (configRes.success && configRes.data?.time_config?.minutes_per_round) {
          minutesPerRound.value = configRes.data.time_config.minutes_per_round
        }
      } catch (configErr) { }
      
      if (simData.project_id) {
        const projRes = await getProject(simData.project_id)
        if (projRes.success && projRes.data) {
          projectData.value = projRes.data
          if (projRes.data.graph_id) {
            await loadGraph(projRes.data.graph_id)
          }
        }
      }
    }
  } catch (err) {
    addLog(`Uplink Error: ${err.message}`)
  }
}

const loadGraph = async (graphId) => {
  if (!isSimulating.value) graphLoading.value = true
  try {
    const res = await getGraphData(graphId)
    if (res.success) graphData.value = res.data
  } catch (err) { 
      // Silent 
  } finally {
    graphLoading.value = false
  }
}

const refreshGraph = () => {
  if (projectData.value?.graph_id) loadGraph(projectData.value.graph_id)
}

let graphRefreshTimer = null
const startGraphRefresh = () => {
  if (graphRefreshTimer) return
  graphRefreshTimer = setInterval(refreshGraph, 30000)
}

const stopGraphRefresh = () => {
  if (graphRefreshTimer) {
    clearInterval(graphRefreshTimer)
    graphRefreshTimer = null
  }
}

watch(isSimulating, (newValue) => {
  if (newValue) startGraphRefresh()
  else stopGraphRefresh()
}, { immediate: true })

onMounted(() => {
  addLog('Command Center Initialized')
  loadSimulationData()
})

onUnmounted(() => {
  stopGraphRefresh()
})
</script>

<style scoped>
.main-view {
  height: 100vh;
  display: flex;
  flex-direction: column;
  background: var(--bg-app);
  overflow: hidden;
  color: var(--text-main);
}

/* Command Bar */
.command-bar {
  height: 60px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 24px;
  z-index: 100;
  border-bottom: 1px solid var(--border-light);
  background: var(--bg-surface);
}

.left-controls { display: flex; align-items: center; gap: 16px; }
.mission-title { font-family: var(--font-mono); font-size: 13px; color: var(--text-muted); letter-spacing: 1px; }

.icon-btn { color: var(--text-muted); font-size: 18px; transition: color 0.2s; }
.icon-btn:hover { color: var(--text-main); }

/* View Toggles */
.view-toggles {
  display: flex;
  background: rgba(0,0,0,0.2);
  padding: 4px;
  border-radius: var(--radius-sm);
  gap: 2px;
}

.toggle-chip {
  padding: 6px 14px;
  font-size: 11px;
  font-weight: 700;
  color: var(--text-muted);
  border-radius: 4px;
  transition: all 0.2s;
}

.toggle-chip:hover { color: var(--text-main); }
.toggle-chip.active {
  background: var(--bg-surface);
  color: var(--primary);
  box-shadow: 0 1px 3px rgba(0,0,0,0.3);
}

/* Status */
.status-indicator {
  display: flex; align-items: center; gap: 8px;
  font-family: var(--font-mono); font-size: 12px; color: var(--success);
  text-transform: uppercase;
  background: rgba(16, 185, 129, 0.1);
  padding: 6px 12px; border-radius: 20px;
  border: 1px solid rgba(16, 185, 129, 0.2);
}

.pulse-dot {
  width: 8px; height: 8px; border-radius: 50%;
  background: var(--success);
  box-shadow: 0 0 8px var(--success);
}

.pulse-dot.processing { animation: pulse 1s infinite; }
@keyframes pulse { 50% { opacity: 0.4; } }

/* Content */
.content-area {
  flex: 1;
  display: flex;
  position: relative;
  overflow: hidden;
  background: radial-gradient(circle at 50% 50%, #1e1b4b 0%, #000 100%);
}

.panel-wrapper {
  height: 100%;
  overflow: hidden;
  transition: width 0.4s cubic-bezier(0.25, 0.8, 0.25, 1), opacity 0.3s ease, transform 0.3s ease;
  will-change: width, opacity, transform;
}

.panel-wrapper.left { border-right: 1px solid var(--border-light); background: rgba(0,0,0,0.2); }
.panel-wrapper.right { background: transparent; }
</style>
