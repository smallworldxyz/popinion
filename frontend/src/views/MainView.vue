<template>
  <div class="main-view">
    <!-- Header -->
    <header class="app-header">
      <div class="header-left">
        <div class="brand" @click="router.push('/')">
          <span class="brand-logo">●</span> Popinion
        </div>
      </div>
      
      <div class="header-center">
        <div class="view-switcher">
          <button 
            v-for="mode in ['graph', 'split', 'workbench']" 
            :key="mode"
            class="switch-btn"
            :class="{ active: viewMode === mode }"
            @click="viewMode = mode"
            title="Change Layout"
          >
            {{ { graph: 'Graph Only', split: 'Split View', workbench: 'Workspace Only' }[mode] }}
          </button>
        </div>
      </div>

      <div class="header-right">
        <div class="workflow-status">
          <span class="step-label">Current Phase</span>
          <span class="step-value">{{ stepNames[currentStep - 1] }}</span>
        </div>
        <div class="settings-trigger" @click="showSettings = true" title="System Settings">
            <span class="icon">⚙️</span>
        </div>
        <div class="status-badge" :class="statusClass">
          {{ statusText }}
        </div>
      </div>
    </header>

    <!-- Main Content Area -->
    <main class="content-area">
      <!-- Left Panel: Graph -->
      <div class="panel-wrapper left" :style="leftPanelStyle">
        <GraphPanel 
          :graphData="graphData"
          :loading="graphLoading"
          :currentPhase="currentPhase"
          @refresh="refreshGraph"
          @toggle-maximize="toggleMaximize('graph')"
        />
      </div>

      <!-- Right Panel: Step Components -->
      <div class="panel-wrapper right" :style="rightPanelStyle">
        <!-- Step 1: Knowledge Mapping (formerly GraphBuild) -->
        <Step1GraphBuild 
          v-if="currentStep === 1"
          :currentPhase="currentPhase"
          :projectData="projectData"
          :ontologyProgress="ontologyProgress"
          :buildProgress="buildProgress"
          :graphData="graphData"
          :systemLogs="systemLogs"
          @next-step="handleNextStep"
        />
        <!-- Step 2: Simulation Setup (formerly EnvSetup) -->
        <Step2EnvSetup
          v-else-if="currentStep === 2"
          :projectData="projectData"
          :graphData="graphData"
          :systemLogs="systemLogs"
          @go-back="handleGoBack"
          @next-step="handleNextStep"
          @add-log="addLog"
        />
        <!-- Step 3: Simulation (Director Mode) -->
        <Step3Simulation
          v-else-if="currentStep === 3"
          :simulationId="projectData?.simulation_id"
          :maxRounds="projectData?.simulation_config?.max_rounds"
          :projectData="projectData"
          :graphData="graphData"
          :systemLogs="systemLogs"
          @go-back="handleGoBack"
          @next-step="handleNextStep"
          @add-log="addLog"

          @update-status="handleSimulationStatusUpdate"
          @switch-simulation="handleSwitchSimulation"
        />
        <!-- Step 4: Report Generation -->
        <Step4Report
          v-else-if="currentStep === 4"
          :simulationId="projectData?.simulation_id"
          :graphData="graphData"
          :projectData="projectData"
          :systemLogs="systemLogs"
          @go-back="handleGoBack"
          @next-step="handleNextStep"
          @add-log="addLog"
        />
        <!-- Step 5: Deep Interaction -->
        <Step5Interaction
          v-else-if="currentStep === 5"
          :simulationId="projectData?.simulation_id"
          :reportId="projectData?.report_id"
          :injectedKnowledge="projectData?.injected_knowledge"
          :simulationTags="projectData?.simulation_tags"
          :systemLogs="systemLogs"
          @add-log="addLog"
        />
      </div>
    </main>
    <SettingsModal :isOpen="showSettings" @close="showSettings = false" />
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import GraphPanel from '../components/GraphPanel.vue'
import SettingsModal from '../components/SettingsModal.vue'
import Step1GraphBuild from '../components/Step1GraphBuild.vue'
import Step2EnvSetup from '../components/Step2EnvSetup.vue'
import Step3Simulation from '../components/Step3Simulation.vue'
import Step4Report from '../components/Step4Report.vue'
import Step5Interaction from '../components/Step5Interaction.vue'
import { generateOntology, getProject, buildGraph, getTaskStatus, getGraphData } from '../api/graph'
import { getPendingUpload, clearPendingUpload } from '../store/pendingUpload'

const route = useRoute()
const router = useRouter()

// Layout State
const viewMode = ref('split') // graph | split | workbench
const showSettings = ref(false)

// Step State
// Updated terminology
const currentStep = ref(1) 
const stepNames = ['Knowledge Mapping', 'Simulation Setup', 'Running Simulation', 'Report Generation', 'Deep Interaction']

// Data State
const currentProjectId = ref(route.params.projectId)
const loading = ref(false)
const graphLoading = ref(false)
const error = ref('')
const projectData = ref(null)
const graphData = ref(null)
const currentPhase = ref(-1) // -1: Upload, 0: Ontology, 1: Build, 2: Complete
const ontologyProgress = ref(null)
const buildProgress = ref(null)
const systemLogs = ref([])

// Polling timers
let pollTimer = null
let graphPollTimer = null

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
const statusClass = computed(() => {
  if (error.value) return 'error'
  if (currentPhase.value >= 2) return 'ready'
  return 'processing'
})

const statusText = computed(() => {
  if (error.value) return 'System Error'
  if (currentPhase.value >= 2) return 'Ready to Simulate'
  if (currentPhase.value === 1) return 'Building Knowledge Graph...'
  if (currentPhase.value === 0) return 'Analyzing Topic...'
  return 'Initializing...'
})

// --- Helpers ---
const addLog = (msg) => {
  const time = new Date().toLocaleTimeString('en-US', { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit' })
  systemLogs.value.push({ time, msg })
  if (systemLogs.value.length > 100) systemLogs.value.shift()
}

const handleSimulationStatusUpdate = (status) => {
  // Update UI status based on simulation state
  if (status === 'processing') {
    // maybe update global status
  }
}

const handleSwitchSimulation = (newSimulationId) => {
  addLog(`Switching to simulation branch: ${newSimulationId}`)
  if (projectData.value) {
    projectData.value.simulation_id = newSimulationId
  }
}

// --- Layout Methods ---
const toggleMaximize = (target) => {
  viewMode.value = viewMode.value === target ? 'split' : target
}

const handleNextStep = (params = {}) => {
  if (currentStep.value < 5) {
    currentStep.value++
    addLog(`Transitioning to: ${stepNames[currentStep.value - 1]}`)
  }
}

const handleGoBack = () => {
  if (currentStep.value > 1) {
    currentStep.value--
    addLog(`Returned to: ${stepNames[currentStep.value - 1]}`)
  }
}

// --- Data Logic (Kept mostly purely functional, just updated logs) ---

const initProject = async () => {
  if (currentProjectId.value === 'new') {
    await handleNewProject()
  } else {
    await loadProject()
  }
}

const handleNewProject = async () => {
  const pending = getPendingUpload()
  if (!pending.isPending || pending.files.length === 0) {
    error.value = 'No context files found.'
    alert('Session expired. Please start over.')
    router.replace('/')
    return
  }
  
  try {
    loading.value = true
    currentPhase.value = 0
    ontologyProgress.value = { message: 'Analyzing documents...' }
    addLog('Project initialized. Starting document analysis...')
    
    const formData = new FormData()
    pending.files.forEach(f => formData.append('files', f))
    formData.append('simulation_requirement', pending.simulationRequirement)
    
    const res = await generateOntology(formData)
    if (res.success) {
      clearPendingUpload()
      currentProjectId.value = res.data.project_id
      projectData.value = res.data
      
      router.replace({ name: 'Process', params: { projectId: res.data.project_id } })
      ontologyProgress.value = null
      addLog(`Topic analysis complete. Project ID: ${res.data.project_id}`)
      await startBuildGraph()
    } else {
      error.value = res.error || 'Analysis failed'
      addLog(`Error: ${error.value}`)
    }
  } catch (err) {
    error.value = err.message
    addLog(`System Error: ${err.message}`)
  } finally {
    loading.value = false
  }
}

const loadProject = async () => {
  try {
    loading.value = true
    const res = await getProject(currentProjectId.value)
    if (res.success) {
      projectData.value = res.data
      updatePhaseByStatus(res.data.status)
      addLog(`Project loaded. Status: ${res.data.status}`)
      
      if (res.data.status === 'ontology_generated' && !res.data.graph_id) {
        await startBuildGraph()
      } else if (res.data.status === 'graph_building' && res.data.graph_build_task_id) {
        currentPhase.value = 1
        startPollingTask(res.data.graph_build_task_id)
        startGraphPolling()
      } else if (res.data.status === 'graph_completed' && res.data.graph_id) {
        currentPhase.value = 2
        await loadGraph(res.data.graph_id)
      }
    } else {
      error.value = res.error
    }
  } catch (err) {
    error.value = err.message
  } finally {
    loading.value = false
  }
}

const updatePhaseByStatus = (status) => {
  switch (status) {
    case 'created':
    case 'ontology_generated': currentPhase.value = 0; break;
    case 'graph_building': currentPhase.value = 1; break;
    case 'graph_completed': currentPhase.value = 2; break;
    case 'failed': error.value = 'Project failed'; break;
  }
}

const startBuildGraph = async () => {
  try {
    currentPhase.value = 1
    buildProgress.value = { progress: 0, message: 'Starting knowledge extraction...' }
    addLog('Building Knowledge Graph...')
    
    const res = await buildGraph({ project_id: currentProjectId.value })
    if (res.success) {
      startGraphPolling()
      startPollingTask(res.data.task_id)
    } else {
      error.value = res.error
      addLog(`Build Error: ${res.error}`)
    }
  } catch (err) {
    error.value = err.message
  }
}

const startGraphPolling = () => {
  fetchGraphData()
  graphPollTimer = setInterval(fetchGraphData, 10000)
}

const fetchGraphData = async () => {
  try {
    const projRes = await getProject(currentProjectId.value)
    if (projRes.success && projRes.data.graph_id) {
      const gRes = await getGraphData(projRes.data.graph_id)
      if (gRes.success) {
        graphData.value = gRes.data
        const nodeCount = gRes.data.node_count || gRes.data.nodes?.length || 0
        addLog(`Knowledge Graph updated: ${nodeCount} nodes`)
      }
    }
  } catch (err) {
    // Silent fail for polling
  }
}

const startPollingTask = (taskId) => {
  pollTaskStatus(taskId)
  pollTimer = setInterval(() => pollTaskStatus(taskId), 2000)
}

const pollTaskStatus = async (taskId) => {
  try {
    const res = await getTaskStatus(taskId)
    if (res.success) {
      const task = res.data
      if (task.message && task.message !== buildProgress.value?.message) {
        addLog(task.message)
      }
      buildProgress.value = { progress: task.progress || 0, message: task.message }
      
      if (task.status === 'completed') {
        addLog('Knowledge Graph construction complete.')
        stopPolling()
        stopGraphPolling()
        currentPhase.value = 2
        const projRes = await getProject(currentProjectId.value)
        if (projRes.success && projRes.data.graph_id) {
            projectData.value = projRes.data
            await loadGraph(projRes.data.graph_id)
        }
      } else if (task.status === 'failed') {
        stopPolling()
        error.value = task.error
        addLog(`Build Failed: ${task.error}`)
      }
    }
  } catch (e) {
    console.error(e)
  }
}

const loadGraph = async (graphId) => {
  graphLoading.value = true
  try {
    const res = await getGraphData(graphId)
    if (res.success) {
      graphData.value = res.data
    }
  } catch (e) {
    addLog(`Error loading graph: ${e.message}`)
  } finally {
    graphLoading.value = false
  }
}

const refreshGraph = () => {
  if (projectData.value?.graph_id) {
    loadGraph(projectData.value.graph_id)
  }
}

const stopPolling = () => {
  if (pollTimer) {
    clearInterval(pollTimer)
    pollTimer = null
  }
}

const stopGraphPolling = () => {
  if (graphPollTimer) {
    clearInterval(graphPollTimer)
    graphPollTimer = null
  }
}

onMounted(() => {
  initProject()
})

onUnmounted(() => {
  stopPolling()
  stopGraphPolling()
})
</script>

<style scoped>
.main-view {
  height: 100vh;
  display: flex;
  flex-direction: column;
  background: var(--bg-app);
  overflow: hidden;
}

/* Header */
.app-header {
  height: var(--header-height);
  border-bottom: 1px solid var(--border-light);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 24px;
  background: var(--bg-surface);
  z-index: 100;
  box-shadow: var(--shadow-sm);
}

.brand {
  font-family: var(--font-sans);
  font-weight: 700;
  font-size: 16px;
  letter-spacing: -0.5px;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--text-main);
}

.brand-logo {
  color: var(--primary);
  font-size: 14px;
}

.header-center {
  position: absolute;
  left: 50%;
  transform: translateX(-50%);
}

.view-switcher {
  display: flex;
  background: var(--bg-subtle);
  padding: 4px;
  border-radius: var(--radius-md);
  gap: 4px;
}

.switch-btn {
  padding: 6px 12px;
  font-size: 13px;
  font-weight: 500;
  color: var(--text-muted);
  border-radius: var(--radius-sm);
  transition: all 0.2s;
}

.switch-btn:hover {
  color: var(--text-main);
}

.switch-btn.active {
  background: var(--bg-surface);
  color: var(--text-main);
  box-shadow: var(--shadow-sm);
  font-weight: 600;
}

.header-right {
  display: flex;
  align-items: center;
  gap: 24px;
}

.settings-trigger {
    cursor: pointer;
    opacity: 0.6;
    transition: opacity 0.2s;
    font-size: 18px;
}
.settings-trigger:hover {
    opacity: 1;
}

.workflow-status {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
}

.step-label {
  color: var(--text-faint);
  font-weight: 500;
  text-transform: uppercase;
  font-size: 11px;
  letter-spacing: 0.5px;
}

.step-value {
  color: var(--text-main);
  font-weight: 600;
}

.status-badge {
  font-size: 12px;
  font-weight: 600;
  padding: 4px 10px;
  border-radius: var(--radius-full);
  background: var(--bg-subtle);
  color: var(--text-muted);
}

.status-badge.processing {
  background: rgba(255, 69, 0, 0.1);
  color: var(--primary);
  animation: pulse 2s infinite;
}

.status-badge.ready {
  background: #ECFDF5;
  color: var(--status-success);
}

.status-badge.error {
  background: #FEF2F2;
  color: var(--status-error);
}

@keyframes pulse { 0% { opacity: 1; } 50% { opacity: 0.6; } 100% { opacity: 1; } }

/* Content */
.content-area {
  flex: 1;
  display: flex;
  position: relative;
  overflow: hidden;
}

.panel-wrapper {
  height: 100%;
  overflow: hidden;
  transition: width 0.4s cubic-bezier(0.25, 0.8, 0.25, 1), opacity 0.3s ease, transform 0.3s ease;
  will-change: width, opacity, transform;
}

.panel-wrapper.left {
  border-right: 1px solid var(--border-light);
  background: var(--bg-subtle);
}

.panel-wrapper.right {
  background: var(--bg-surface);
}
</style>
