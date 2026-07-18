<template>
  <div class="main-view">
    <!-- Header: one chrome for every altitude of the shell. -->
    <header class="app-header">
      <div class="header-left">
        <div class="brand" @click="router.push('/')">POPINION</div>
      </div>

      <div class="header-center">
        <div class="view-switcher">
          <button
            v-for="mode in ['graph', 'split', 'workbench', 'knowledge']"
            :key="mode"
            class="switch-btn"
            :class="{ active: viewMode === mode }"
            @click="viewMode = mode"
          >
            {{ { graph: 'Graph', split: 'Split View', workbench: 'Workbench', knowledge: '📋 Knowledge Pad' }[mode] }}
          </button>
        </div>
      </div>

      <div class="header-right">
        <div class="workflow-step">
          <span class="step-num">Step {{ stepNum }}/5</span>
          <span class="step-name">{{ stepName }}</span>
        </div>
        <div class="step-divider"></div>
        <span class="status-indicator" :class="statusClass">
          <span class="dot"></span>
          {{ statusText }}
        </span>
      </div>
    </header>

    <!-- Content: graph on the left, the active surface on the right. -->
    <main class="content-area">
      <div class="panel-wrapper left" :style="leftPanelStyle">
        <GraphPanel
          :graphData="graphData"
          :loading="graphLoading"
          :currentPhase="graphPhase"
          :isSimulating="isSimulating"
          @refresh="refreshGraph"
          @toggle-maximize="toggleMaximize('graph')"
        />
      </div>

      <div class="panel-wrapper right" :style="rightPanelStyle">
        <!-- Graph building -->
        <Step1GraphBuild
          v-if="surface === 'process' && currentStep === 1"
          :currentPhase="currentPhase"
          :projectData="projectData"
          :ontologyProgress="ontologyProgress"
          :buildProgress="buildProgress"
          :graphData="graphData"
          :systemLogs="systemLogs"
          @next-step="handleNextStep"
        />

        <!-- Environment setup -->
        <Step2EnvSetup
          v-else-if="surface === 'simulation' || (surface === 'process' && currentStep === 2)"
          :simulationId="simulationId"
          :projectData="projectData"
          :graphData="graphData"
          :systemLogs="systemLogs"
          @go-back="handleGoBack"
          @next-step="handleNextStep"
          @add-log="addLog"
          @update-status="updateStatus"
        />

        <!-- Start simulation -->
        <Step3Simulation
          v-else-if="surface === 'simulationRun'"
          :simulationId="simulationId"
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

        <!-- Report generation -->
        <Step4Report
          v-else-if="surface === 'report'"
          :reportId="reportId"
          :simulationId="simulationId"
          :systemLogs="systemLogs"
          @add-log="addLog"
          @update-status="updateStatus"
        />

        <!-- Deep interaction (+ Knowledge Pad) -->
        <template v-else-if="surface === 'interaction'">
          <KnowledgePad
            v-show="viewMode === 'knowledge'"
            :highlights="knowledgePad"
            :agents="agentsForInjection"
            @remove="removeHighlight"
            @remove-multiple="removeMultipleHighlights"
            @inject="handleInject"
            @export="exportKnowledgePad"
            @import="handleImportKnowledge"
          />
          <Step5Interaction
            v-show="viewMode !== 'knowledge'"
            :reportId="reportId"
            :simulationId="simulationId"
            :simulationTags="simulationTags"
            :injectedKnowledge="injectedKnowledge"
            :systemLogs="systemLogs"
            @add-log="addLog"
            @update-status="updateStatus"
            @add-to-knowledge="addToKnowledge"
            @remove-injection="handleRemoveInjection"
            @clear-injections="handleClearInjections"
          />
        </template>
      </div>
    </main>
  </div>
</template>

<script setup>
// One shell for every altitude of a World: graph building, environment setup,
// the run, its report, and the deep-interaction surface are states of this
// component, keyed off the route name, not five near-identical files.
import { ref, computed, watch, onUnmounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import GraphPanel from '../components/GraphPanel.vue'
import Step1GraphBuild from '../components/Step1GraphBuild.vue'
import Step2EnvSetup from '../components/Step2EnvSetup.vue'
import Step3Simulation from '../components/Step3Simulation.vue'
import Step4Report from '../components/Step4Report.vue'
import Step5Interaction from '../components/Step5Interaction.vue'
import KnowledgePad from '../components/KnowledgePad.vue'
import { getProject, buildGraph, getTaskStatus, getGraphData } from '../api/graph'
import {
  getSimulation, getSimulationConfig, getSimulationProfilesRealtime,
  stopSimulation, closeSimulationEnv, getEnvStatus,
} from '../api/simulation'
import { getReport } from '../api/report'

const route = useRoute()
const router = useRouter()

// Which of the five altitudes this shell is currently showing.
const SURFACE_BY_ROUTE = {
  Process: 'process',
  Simulation: 'simulation',
  SimulationRun: 'simulationRun',
  Report: 'report',
  Interaction: 'interaction',
}
const surface = computed(() => SURFACE_BY_ROUTE[route.name] || 'process')

// --- Shared state ---
const viewMode = ref('split')
const projectData = ref(null)
const graphData = ref(null)
const graphLoading = ref(false)
const systemLogs = ref([])
const currentStatus = ref('processing') // processing | completed | error | ready
const error = ref('')

// process build state
const currentStep = ref(1)
const currentPhase = ref(-1) // -1: upload, 0: ontology, 1: build, 2: complete
const ontologyProgress = ref(null)
const buildProgress = ref(null)

// run state
const simulationId = ref(null)
const reportId = ref(null)
const maxRounds = ref(null)
const minutesPerRound = ref(30)

// interaction / knowledge-pad state
const knowledgePad = ref([])
const profiles = ref([])
const simulationTags = ref([])
const injectedKnowledge = ref({})

const stepNames = ['Graph Building', 'Environment Setup', 'Start Simulation', 'Report Generating', 'Deep Interactive']

// Polling timers, cleared on every re-init and on unmount.
let pollTimer = null
let graphPollTimer = null
let graphRefreshTimer = null

// --- Header meta ---
const stepNum = computed(() => {
  if (surface.value === 'process') return currentStep.value
  return { simulation: 2, simulationRun: 3, report: 4, interaction: 5 }[surface.value]
})
// The interaction route historically labelled its header "DeepInteractive".
const stepName = computed(() =>
  surface.value === 'interaction' ? 'DeepInteractive' : stepNames[stepNum.value - 1])

const statusClass = computed(() => {
  if (surface.value === 'process') {
    if (error.value) return 'error'
    return currentPhase.value >= 2 ? 'completed' : 'processing'
  }
  return currentStatus.value
})

const statusText = computed(() => {
  if (surface.value === 'process') {
    if (error.value) return 'Error'
    if (currentPhase.value >= 2) return 'Ready'
    if (currentPhase.value === 1) return 'Building Graph'
    if (currentPhase.value === 0) return 'Generating Ontology'
    return 'Initializing'
  }
  if (currentStatus.value === 'error') return 'Error'
  if (surface.value === 'simulation') return currentStatus.value === 'completed' ? 'Ready' : 'Preparing'
  if (surface.value === 'simulationRun') return currentStatus.value === 'completed' ? 'Completed' : 'Running'
  if (surface.value === 'report') return currentStatus.value === 'completed' ? 'Completed' : 'Generating'
  // interaction
  if (currentStatus.value === 'completed') return 'Completed'
  if (currentStatus.value === 'processing') return 'Processing'
  return 'Ready'
})

const isSimulating = computed(() => surface.value === 'simulationRun' && currentStatus.value === 'processing')

const graphPhase = computed(() => {
  if (surface.value === 'process') return currentPhase.value
  return { simulation: 2, simulationRun: 3, report: 4, interaction: 5 }[surface.value]
})

// --- Layout ---
const hidesGraph = (m) => m === 'workbench' || (surface.value === 'interaction' && m === 'knowledge')

const leftPanelStyle = computed(() => {
  if (viewMode.value === 'graph') return { width: '100%', opacity: 1, transform: 'translateX(0)' }
  if (hidesGraph(viewMode.value)) return { width: '0%', opacity: 0, transform: 'translateX(-20px)' }
  return { width: '50%', opacity: 1, transform: 'translateX(0)' }
})

const rightPanelStyle = computed(() => {
  if (hidesGraph(viewMode.value)) return { width: '100%', opacity: 1, transform: 'translateX(0)' }
  if (viewMode.value === 'graph') return { width: '0%', opacity: 0, transform: 'translateX(20px)' }
  return { width: '50%', opacity: 1, transform: 'translateX(0)' }
})

// --- Helpers ---
const addLog = (msg) => {
  const now = new Date()
  const time = now.toLocaleTimeString('en-US', { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit' })
    + '.' + now.getMilliseconds().toString().padStart(3, '0')
  systemLogs.value.push({ time, msg })
  if (systemLogs.value.length > 200) systemLogs.value.shift()
}

const updateStatus = (status) => { currentStatus.value = status }

const toggleMaximize = (target) => {
  viewMode.value = viewMode.value === target ? 'split' : target
}

const loadGraph = async (graphId, { quiet = false } = {}) => {
  if (!quiet) graphLoading.value = true
  try {
    const res = await getGraphData(graphId)
    if (res.success) {
      graphData.value = res.data
      if (!quiet) addLog('Graph data loaded successfully')
    }
  } catch (err) {
    addLog(`Graph loading failed: ${err.message}`)
  } finally {
    graphLoading.value = false
  }
}

const refreshGraph = () => {
  const gid = projectData.value?.graph_id
  if (gid) loadGraph(gid, { quiet: isSimulating.value })
}

// --- Navigation between surfaces ---
const handleNextStep = (params = {}) => {
  if (surface.value === 'process') {
    if (currentStep.value < 5) {
      currentStep.value++
      addLog(`Entering Step ${currentStep.value}: ${stepNames[currentStep.value - 1]}`)
    }
    return
  }
  if (surface.value === 'simulation') {
    addLog('Enter Step 3: Start Simulation')
    const routeParams = { name: 'SimulationRun', params: { simulationId: simulationId.value } }
    if (params.maxRounds) routeParams.query = { maxRounds: params.maxRounds }
    router.push(routeParams)
    return
  }
  if (surface.value === 'simulationRun') {
    // Step3Simulation navigates to the report itself; this is a backstop.
    addLog('Entering Step 4: Report generation')
  }
}

const handleGoBack = async () => {
  if (surface.value === 'process') {
    if (currentStep.value > 1) {
      currentStep.value--
      addLog(`Going back to Step ${currentStep.value}: ${stepNames[currentStep.value - 1]}`)
    }
    return
  }
  if (surface.value === 'simulation') {
    if (projectData.value?.project_id) {
      router.push({ name: 'Process', params: { projectId: projectData.value.project_id } })
    } else {
      router.push('/')
    }
    return
  }
  if (surface.value === 'simulationRun') {
    // Close a running simulation before dropping back to Step 2.
    addLog('Preparing to go back to Step 2, closing simulation...')
    stopGraphRefresh()
    try {
      const envStatusRes = await getEnvStatus({ simulation_id: simulationId.value })
      if (envStatusRes.success && envStatusRes.data?.env_alive) {
        addLog('Closing simulation environment...')
        try {
          await closeSimulationEnv({ simulation_id: simulationId.value, timeout: 10 })
          addLog('✓ Simulation environment closed')
        } catch (closeErr) {
          addLog('Failed to close simulation environment, attempting force stop...')
          try {
            await stopSimulation({ simulation_id: simulationId.value })
            addLog('✓ Simulation force stopped')
          } catch (stopErr) {
            addLog(`Force stop failed: ${stopErr.message}`)
          }
        }
      } else if (isSimulating.value) {
        addLog('Stopping simulation process...')
        try {
          await stopSimulation({ simulation_id: simulationId.value })
          addLog('✓ Simulation stopped')
        } catch (err) {
          addLog(`Failed to stop simulation: ${err.message}`)
        }
      }
    } catch (err) {
      addLog(`Failed to check simulation status: ${err.message}`)
    }
    router.push({ name: 'Simulation', params: { simulationId: simulationId.value } })
  }
}

// --- process: graph build ---
const loadProject = async (projectId) => {
  try {
    graphLoading.value = false
    addLog(`Loading project ${projectId}...`)
    const res = await getProject(projectId)
    if (!res.success) {
      error.value = res.error
      addLog(`Error loading project: ${res.error}`)
      return
    }
    projectData.value = res.data
    updatePhaseByStatus(res.data.status)
    addLog(`Project loaded. Status: ${res.data.status}`)

    if (res.data.status === 'ontology_generated' && !res.data.graph_id) {
      await startBuildGraph(projectId)
    } else if (res.data.status === 'graph_building' && res.data.graph_build_task_id) {
      currentPhase.value = 1
      startPollingTask(projectId, res.data.graph_build_task_id)
      startGraphPolling(projectId)
    } else if (res.data.status === 'graph_completed' && res.data.graph_id) {
      currentPhase.value = 2
      await loadGraph(res.data.graph_id)
    }
  } catch (err) {
    error.value = err.message
    addLog(`Exception in loadProject: ${err.message}`)
  }
}

const updatePhaseByStatus = (status) => {
  switch (status) {
    case 'created':
    case 'ontology_generated': currentPhase.value = 0; break
    case 'graph_building': currentPhase.value = 1; break
    case 'graph_completed': currentPhase.value = 2; break
    case 'failed': error.value = 'Project failed'; break
  }
}

const startBuildGraph = async (projectId) => {
  try {
    currentPhase.value = 1
    buildProgress.value = { progress: 0, message: 'Starting build...' }
    addLog('Initiating graph build...')
    const res = await buildGraph({ project_id: projectId })
    if (res.success) {
      addLog(`Graph build task started. Task ID: ${res.data.task_id}`)
      startGraphPolling(projectId)
      startPollingTask(projectId, res.data.task_id)
    } else {
      error.value = res.error
      addLog(`Error starting build: ${res.error}`)
    }
  } catch (err) {
    error.value = err.message
    addLog(`Exception in startBuildGraph: ${err.message}`)
  }
}

const startGraphPolling = (projectId) => {
  addLog('Started polling for graph data...')
  fetchGraphData(projectId)
  graphPollTimer = setInterval(() => fetchGraphData(projectId), 10000)
}

const fetchGraphData = async (projectId) => {
  try {
    const projRes = await getProject(projectId)
    if (projRes.success && projRes.data.graph_id) {
      const gRes = await getGraphData(projRes.data.graph_id)
      if (gRes.success) {
        graphData.value = gRes.data
        const nodeCount = gRes.data.node_count || gRes.data.nodes?.length || 0
        const edgeCount = gRes.data.edge_count || gRes.data.edges?.length || 0
        addLog(`Graph data refreshed. Nodes: ${nodeCount}, Edges: ${edgeCount}`)
      }
    }
  } catch (err) {
    console.warn('Graph fetch error:', err)
  }
}

const startPollingTask = (projectId, taskId) => {
  pollTaskStatus(projectId, taskId)
  pollTimer = setInterval(() => pollTaskStatus(projectId, taskId), 2000)
}

const pollTaskStatus = async (projectId, taskId) => {
  try {
    const res = await getTaskStatus(taskId)
    if (!res.success) return
    const task = res.data
    if (task.message && task.message !== buildProgress.value?.message) addLog(task.message)
    buildProgress.value = { progress: task.progress || 0, message: task.message }

    if (task.status === 'completed') {
      addLog('Graph build task completed.')
      stopPolling()
      stopGraphPolling()
      currentPhase.value = 2
      const projRes = await getProject(projectId)
      if (projRes.success && projRes.data.graph_id) {
        projectData.value = projRes.data
        await loadGraph(projRes.data.graph_id)
      }
    } else if (task.status === 'failed') {
      stopPolling()
      error.value = task.error
      addLog(`Graph build task failed: ${task.error}`)
    }
  } catch (e) {
    console.error(e)
  }
}

const stopPolling = () => {
  if (pollTimer) { clearInterval(pollTimer); pollTimer = null }
}
const stopGraphPolling = () => {
  if (graphPollTimer) { clearInterval(graphPollTimer); graphPollTimer = null; addLog('Graph polling stopped.') }
}

// --- simulation / simulationRun: load a run ---
const loadSimulationData = async ({ withConfig = false, stopFirst = false } = {}) => {
  try {
    if (stopFirst) await checkAndStopRunningSimulation()
    addLog(`Loading simulation data: ${simulationId.value}`)
    const simRes = await getSimulation(simulationId.value)
    if (!simRes.success || !simRes.data) {
      addLog(`Loading simulation data failed: ${simRes.error || 'Unknown error'}`)
      return
    }
    const simData = simRes.data
    if (withConfig) {
      try {
        const configRes = await getSimulationConfig(simulationId.value)
        if (configRes.success && configRes.data?.time_config?.minutes_per_round) {
          minutesPerRound.value = configRes.data.time_config.minutes_per_round
          addLog(`Time configuration: ${minutesPerRound.value} minutes per round`)
        }
      } catch (configErr) {
        addLog(`Failed to get time configuration, using default: ${minutesPerRound.value} minutes/round`)
      }
    }
    if (simData.project_id) {
      const projRes = await getProject(simData.project_id)
      if (projRes.success && projRes.data) {
        projectData.value = projRes.data
        addLog(`Project loaded successfully: ${projRes.data.project_id}`)
        if (projRes.data.graph_id) await loadGraph(projRes.data.graph_id)
      }
    }
  } catch (err) {
    addLog(`Loading error: ${err.message}`)
  }
}

const checkAndStopRunningSimulation = async () => {
  if (!simulationId.value) return
  try {
    const envStatusRes = await getEnvStatus({ simulation_id: simulationId.value })
    if (envStatusRes.success && envStatusRes.data?.env_alive) {
      addLog('Detected running simulation environment, closing...')
      try {
        const closeRes = await closeSimulationEnv({ simulation_id: simulationId.value, timeout: 10 })
        if (closeRes.success) {
          addLog('✓ Simulation environment closed')
        } else {
          addLog(`Failed to close simulation environment: ${closeRes.error || 'Unknown error'}`)
          await forceStopSimulation()
        }
      } catch (closeErr) {
        addLog(`Close simulation environment error: ${closeErr.message}`)
        await forceStopSimulation()
      }
    } else {
      const simRes = await getSimulation(simulationId.value)
      if (simRes.success && simRes.data?.status === 'running') {
        addLog('Detected simulation status as Running, stopping...')
        await forceStopSimulation()
      }
    }
  } catch (err) {
    console.warn('Check simulation status failed:', err)
  }
}

const forceStopSimulation = async () => {
  try {
    const stopRes = await stopSimulation({ simulation_id: simulationId.value })
    addLog(stopRes.success ? '✓ Simulation force stopped' : `Force stop simulation failed: ${stopRes.error || 'Unknown error'}`)
  } catch (err) {
    addLog(`Force stop simulation error: ${err.message}`)
  }
}

const startGraphRefresh = () => {
  if (graphRefreshTimer) return
  addLog('Enable graph real-time refresh (30s)')
  graphRefreshTimer = setInterval(refreshGraph, 30000)
}
const stopGraphRefresh = () => {
  if (graphRefreshTimer) { clearInterval(graphRefreshTimer); graphRefreshTimer = null; addLog('Stop graph real-time refresh') }
}

// --- report / interaction: load a report chain ---
const loadReportData = async ({ withProfiles = false } = {}) => {
  try {
    addLog(`Loading report data: ${reportId.value}`)
    const reportRes = await getReport(reportId.value)
    if (!reportRes.success || !reportRes.data) {
      addLog(`Failed to get report info: ${reportRes.error || 'Unknown error'}`)
      return
    }
    simulationId.value = reportRes.data.simulation_id
    if (withProfiles) await loadProfiles()
    if (simulationId.value) {
      const simRes = await getSimulation(simulationId.value)
      if (simRes.success && simRes.data) {
        const simData = simRes.data
        if (withProfiles && simData.event_config?.hot_topics) {
          simulationTags.value = simData.event_config.hot_topics
          addLog(`Loaded ${simulationTags.value.length} simulation tags for Knowledge Pad`)
        }
        if (simData.project_id) {
          const projRes = await getProject(simData.project_id)
          if (projRes.success && projRes.data) {
            projectData.value = projRes.data
            addLog(`Project loaded successfully: ${projRes.data.project_id}`)
            if (projRes.data.graph_id) await loadGraph(projRes.data.graph_id)
          }
        }
      }
    }
  } catch (err) {
    addLog(`Loading exception: ${err.message}`)
  }
}

const loadProfiles = async () => {
  if (!simulationId.value) return
  try {
    const res = await getSimulationProfilesRealtime(simulationId.value)
    if (res.success && res.data?.profiles) {
      profiles.value = res.data.profiles
      addLog(`Loaded ${profiles.value.length} agent profiles for injection`)
    }
  } catch (err) {
    addLog(`Failed to load profiles: ${err.message}`)
  }
}

// --- Knowledge Pad (interaction surface) ---
const cleanAgentName = (name) => {
  if (!name) return 'Agent'
  return name.replace(/_\d+$/, '').replace(/_/g, ' ').split(' ')
    .map(w => w.charAt(0).toUpperCase() + w.slice(1).toLowerCase()).join(' ')
}
const agentsForInjection = computed(() =>
  profiles.value.map((p, idx) => ({ idx, name: cleanAgentName(p.username), username: p.username })))

const addToKnowledge = (data) => {
  knowledgePad.value.push({ id: crypto.randomUUID(), content: data.content, source: data.source, tags: data.tags || [], createdAt: Date.now() })
  addLog(`Added to Knowledge Pad: "${data.content.substring(0, 50)}..."`)
}
const removeHighlight = (idx) => { knowledgePad.value.splice(idx, 1) }
const removeMultipleHighlights = (ids) => {
  knowledgePad.value = knowledgePad.value.filter(h => !ids.includes(h.id))
  addLog(`Deleted ${ids.length} highlight(s)`)
}
const handleInject = (data) => {
  const { target, agentIdx, highlights } = data
  const contents = highlights.map(h => h.content)
  if (target === 'single' && agentIdx !== null) {
    const key = `agent_${agentIdx}`
    if (!injectedKnowledge.value[key]) injectedKnowledge.value[key] = []
    injectedKnowledge.value[key].push(...contents)
    addLog(`Injected ${contents.length} item(s) to agent #${agentIdx}`)
  } else if (target === 'all') {
    if (!injectedKnowledge.value['panel_all']) injectedKnowledge.value['panel_all'] = []
    injectedKnowledge.value['panel_all'].push(...contents)
    addLog(`Injected ${contents.length} item(s) to all panel participants`)
  } else if (target === 'global') {
    if (!injectedKnowledge.value['global']) injectedKnowledge.value['global'] = []
    injectedKnowledge.value['global'].push(...contents)
    addLog(`Injected ${contents.length} item(s) globally`)
  }
  injectedKnowledge.value = { ...injectedKnowledge.value }
}
const handleRemoveInjection = (data) => {
  const { target, index } = data
  if (target === 'global') {
    const globalItems = injectedKnowledge.value['global'] || []
    const combinedLength = globalItems.length
    if (index < combinedLength) injectedKnowledge.value['global'].splice(index, 1)
    else injectedKnowledge.value['panel_all'].splice(index - combinedLength, 1)
  } else {
    const key = typeof target === 'number' ? `agent_${target}` : target
    if (injectedKnowledge.value[key]) injectedKnowledge.value[key].splice(index, 1)
  }
  injectedKnowledge.value = { ...injectedKnowledge.value }
}
const handleClearInjections = (data) => {
  const { target } = data
  if (target === 'global') {
    injectedKnowledge.value['global'] = []
    injectedKnowledge.value['panel_all'] = []
  } else {
    const key = typeof target === 'number' ? `agent_${target}` : target
    injectedKnowledge.value[key] = []
  }
  injectedKnowledge.value = { ...injectedKnowledge.value }
  addLog(`Cleared injections for ${target}`)
}
const exportKnowledgePad = () => {
  const exportData = { simulationId: simulationId.value, highlights: knowledgePad.value, exportedAt: new Date().toISOString() }
  const blob = new Blob([JSON.stringify(exportData, null, 2)], { type: 'application/json' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `knowledge_pad_${simulationId.value || 'export'}.json`
  a.click()
  URL.revokeObjectURL(url)
  addLog('Knowledge Pad exported')
}
const handleImportKnowledge = (data) => {
  const { highlights, sourceSimulationId } = data
  const imported = highlights.map(h => ({ ...h, id: crypto.randomUUID(), importedFrom: sourceSimulationId, originalId: h.id }))
  knowledgePad.value.push(...imported)
  addLog(`Imported ${imported.length} highlight(s) from ${sourceSimulationId || 'unknown source'}`)
}

const loadPreloadedKnowledge = () => {
  const preloadedRaw = sessionStorage.getItem('preloadedKnowledge')
  if (!preloadedRaw) return
  try {
    const preloaded = JSON.parse(preloadedRaw)
    if (Array.isArray(preloaded) && preloaded.length > 0) {
      const imported = preloaded.map(h => ({ ...h, id: crypto.randomUUID(), preloaded: true, originalId: h.id }))
      knowledgePad.value.push(...imported)
      imported.forEach(h => {
        if (h.isGlobal || h.mappedAgentIdx === null) {
          if (!injectedKnowledge.value['global']) injectedKnowledge.value['global'] = []
          injectedKnowledge.value['global'].push(h.content)
        } else if (h.mappedAgentIdx !== undefined) {
          const key = `agent_${h.mappedAgentIdx}`
          if (!injectedKnowledge.value[key]) injectedKnowledge.value[key] = []
          injectedKnowledge.value[key].push(h.content)
        }
      })
      injectedKnowledge.value = { ...injectedKnowledge.value }
      const globalCount = imported.filter(h => h.isGlobal || h.mappedAgentIdx === null).length
      const mappedCount = imported.length - globalCount
      addLog(`Loaded ${imported.length} pre-loaded items (${mappedCount} agent-specific, ${globalCount} global)`)
      sessionStorage.removeItem('preloadedKnowledge')
    }
  } catch (err) {
    addLog(`Failed to load pre-loaded knowledge: ${err.message}`)
  }
}

// --- Init: run whenever the shell's route changes altitude. ---
const resetTimers = () => { stopPolling(); stopGraphPolling(); stopGraphRefresh() }

const resetState = () => {
  resetTimers()
  systemLogs.value = []
  projectData.value = null
  graphData.value = null
  graphLoading.value = false
  error.value = ''
  currentStep.value = 1
  currentPhase.value = -1
  ontologyProgress.value = null
  buildProgress.value = null
  simulationId.value = null
  reportId.value = null
  maxRounds.value = null
  minutesPerRound.value = 30
  knowledgePad.value = []
  profiles.value = []
  simulationTags.value = []
  injectedKnowledge.value = {}
}

const init = async () => {
  resetState()
  switch (surface.value) {
    case 'process':
      viewMode.value = 'split'
      currentStatus.value = 'processing'
      addLog('Project view initialized.')
      await loadProject(route.params.projectId)
      break
    case 'simulation':
      viewMode.value = 'split'
      currentStatus.value = 'processing'
      simulationId.value = route.params.simulationId
      addLog('Environment setup initializing')
      await loadSimulationData({ stopFirst: true })
      break
    case 'simulationRun':
      viewMode.value = 'split'
      currentStatus.value = 'processing'
      simulationId.value = route.params.simulationId
      maxRounds.value = route.query.maxRounds ? parseInt(route.query.maxRounds) : null
      if (maxRounds.value) addLog(`Custom simulation rounds: ${maxRounds.value}`)
      addLog('Run surface initialized')
      await loadSimulationData({ withConfig: true })
      // The run starts live; the watcher only covers later transitions.
      if (isSimulating.value) startGraphRefresh()
      break
    case 'report':
      viewMode.value = 'workbench'
      currentStatus.value = 'processing'
      reportId.value = route.params.reportId
      addLog('Report surface initialized')
      await loadReportData()
      break
    case 'interaction':
      viewMode.value = 'workbench'
      currentStatus.value = 'ready'
      reportId.value = route.params.reportId
      addLog('Interaction surface initialized')
      await loadReportData({ withProfiles: true })
      loadPreloadedKnowledge()
      break
  }
}

// The run surface auto-refreshes the graph while the simulation is live.
watch(isSimulating, (live) => {
  if (surface.value !== 'simulationRun') return
  if (live) startGraphRefresh()
  else stopGraphRefresh()
})

// Re-initialize on any altitude/param change (the shell instance is reused
// across these routes, so onMounted alone would not re-run).
watch(() => route.fullPath, init, { immediate: true })

onUnmounted(resetTimers)
</script>

<style scoped>
.main-view {
  height: 100vh;
  display: flex;
  flex-direction: column;
  background: var(--white);
  overflow: hidden;
  font-family: var(--font-sans);
}

/* Header — clean Riverbase toolbar */
.app-header {
  height: 60px;
  border-bottom: 1px solid var(--border);
  box-shadow: var(--elev-1);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 120px 0 24px;
  background: var(--white);
  z-index: 100;
  position: relative;
}

.header-center {
  position: absolute;
  left: 50%;
  transform: translateX(-50%);
}

.brand {
  font-family: var(--font-serif);
  font-weight: 500;
  font-size: 20px;
  letter-spacing: 0.5px;
  color: var(--text-strong);
  cursor: pointer;
}

.view-switcher {
  display: flex;
  background: var(--subtle);
  padding: 4px;
  border-radius: var(--r-pill);
  gap: 4px;
}

.switch-btn {
  border: none;
  background: transparent;
  padding: 6px 16px;
  font-size: 12px;
  font-weight: 600;
  color: var(--text-muted);
  border-radius: var(--r-pill);
  cursor: pointer;
  transition: all 0.2s;
}

.switch-btn:hover {
  color: var(--text-strong);
}

.switch-btn.active {
  background: var(--white);
  color: var(--emerald);
  box-shadow: var(--elev-1);
}

.header-right {
  display: flex;
  align-items: center;
  gap: 16px;
}

.workflow-step {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 14px;
}

.step-num {
  font-family: var(--font-sans);
  font-variant-numeric: tabular-nums;
  font-weight: 700;
  font-size: 10px;
  letter-spacing: 0.14em;
  text-transform: uppercase;
  color: var(--text-muted);
}

.step-name {
  font-weight: 700;
  color: var(--text-strong);
}

.step-divider {
  width: 1px;
  height: 14px;
  background-color: var(--border);
}

.status-indicator {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  color: var(--text-muted);
  font-weight: 500;
}

.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--text-muted);
}

.status-indicator.ready .dot { background: var(--emerald); }
.status-indicator.processing .dot { background: var(--coral); animation: pulse 1s infinite; }
.status-indicator.completed .dot { background: var(--emerald); }
.status-indicator.error .dot { background: var(--coral-on-light); }

@keyframes pulse { 50% { opacity: 0.5; } }

/* Content */
.content-area {
  flex: 1;
  display: flex;
  position: relative;
  overflow: hidden;
  background: var(--subtle);
}

.panel-wrapper {
  height: 100%;
  overflow: hidden;
  transition: width 0.4s cubic-bezier(0.25, 0.8, 0.25, 1), opacity 0.3s ease, transform 0.3s ease;
  will-change: width, opacity, transform;
}

.panel-wrapper.left {
  border-right: 1px solid var(--border);
}
</style>
