<template>
  <div class="main-view">
    <!-- Header -->
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
          <span class="step-num">Step 5/5</span>
          <span class="step-name">DeepInteractive</span>
        </div>
        <div class="step-divider"></div>
        <span class="status-indicator" :class="statusClass">
          <span class="dot"></span>
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
          :currentPhase="5"
          :isSimulating="false"
          @refresh="refreshGraph"
          @toggle-maximize="toggleMaximize('graph')"
        />
      </div>

      <!-- Right Panel: Step5 DeepInteractive or Knowledge Pad -->
      <div class="panel-wrapper right" :style="rightPanelStyle">
        <!-- Use v-show instead of v-if to preserve state when switching views -->
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
          :reportId="currentReportId"
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
      </div>
    </main>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import GraphPanel from '../components/GraphPanel.vue'
import Step5Interaction from '../components/Step5Interaction.vue'
import KnowledgePad from '../components/KnowledgePad.vue'
import { getProject, getGraphData } from '../api/graph'
import { getSimulation, getSimulationProfilesRealtime } from '../api/simulation'
import { getReport } from '../api/report'

const route = useRoute()
const router = useRouter()

// Props
const props = defineProps({
  reportId: String
})

// Layout State - Default to Workbench view
const viewMode = ref('workbench')

// Data State
const currentReportId = ref(route.params.reportId)
const simulationId = ref(null)
const projectData = ref(null)
const graphData = ref(null)
const graphLoading = ref(false)
const systemLogs = ref([])
const currentStatus = ref('ready') // ready | processing | completed | error

// Knowledge Pad State
const knowledgePad = ref([])
const profiles = ref([])
const simulationTags = ref([]) // Hot topics from simulation
// Injected knowledge per agent: { 'global': [...], 'agent_0': [...], 'agent_1': [...] }
const injectedKnowledge = ref({})

// Computed for KnowledgePad agents dropdown
const agentsForInjection = computed(() => {
  return profiles.value.map((p, idx) => ({
    idx,
    name: cleanAgentName(p.username),
    username: p.username
  }))
})

// Helper for formatting agent names
const cleanAgentName = (name) => {
  if (!name) return 'Agent'
  return name
    .replace(/_\d+$/, '')
    .replace(/_/g, ' ')
    .split(' ')
    .map(w => w.charAt(0).toUpperCase() + w.slice(1).toLowerCase())
    .join(' ')
}

// --- Computed Layout Styles ---
const leftPanelStyle = computed(() => {
  if (viewMode.value === 'graph') return { width: '100%', opacity: 1, transform: 'translateX(0)' }
  if (viewMode.value === 'workbench' || viewMode.value === 'knowledge') return { width: '0%', opacity: 0, transform: 'translateX(-20px)' }
  return { width: '50%', opacity: 1, transform: 'translateX(0)' }
})

const rightPanelStyle = computed(() => {
  if (viewMode.value === 'workbench' || viewMode.value === 'knowledge') return { width: '100%', opacity: 1, transform: 'translateX(0)' }
  if (viewMode.value === 'graph') return { width: '0%', opacity: 0, transform: 'translateX(20px)' }
  return { width: '50%', opacity: 1, transform: 'translateX(0)' }
})

// --- Status Computed ---
const statusClass = computed(() => {
  return currentStatus.value
})

const statusText = computed(() => {
  if (currentStatus.value === 'error') return 'Error'
  if (currentStatus.value === 'completed') return 'Completed'
  if (currentStatus.value === 'processing') return 'Processing'
  return 'Ready'
})

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

// --- Knowledge Pad Methods ---
const addToKnowledge = (data) => {
  knowledgePad.value.push({
    id: crypto.randomUUID(),
    content: data.content,
    source: data.source,
    tags: data.tags || [],
    createdAt: Date.now()
  })
  addLog(`Added to Knowledge Pad: "${data.content.substring(0, 50)}..."`)
}

const removeHighlight = (idx) => {
  knowledgePad.value.splice(idx, 1)
}

const removeMultipleHighlights = (ids) => {
  knowledgePad.value = knowledgePad.value.filter(h => !ids.includes(h.id))
  addLog(`Deleted ${ids.length} highlight(s)`)
}

const handleInject = (data) => {
  const { target, agentIdx, highlights } = data
  const contents = highlights.map(h => h.content)
  
  if (target === 'single' && agentIdx !== null) {
    // Inject to specific agent
    const key = `agent_${agentIdx}`
    if (!injectedKnowledge.value[key]) {
      injectedKnowledge.value[key] = []
    }
    injectedKnowledge.value[key].push(...contents)
    addLog(`Injected ${contents.length} item(s) to agent #${agentIdx}`)
  } else if (target === 'all') {
    // Inject to all current panel participants (handled in Step5)
    if (!injectedKnowledge.value['panel_all']) {
      injectedKnowledge.value['panel_all'] = []
    }
    injectedKnowledge.value['panel_all'].push(...contents)
    addLog(`Injected ${contents.length} item(s) to all panel participants`)
  } else if (target === 'global') {
    // Inject globally to all agents
    if (!injectedKnowledge.value['global']) {
      injectedKnowledge.value['global'] = []
    }
    injectedKnowledge.value['global'].push(...contents)
    addLog(`Injected ${contents.length} item(s) globally`)
  }
  
  // Trigger reactivity
  injectedKnowledge.value = { ...injectedKnowledge.value }
}

const handleRemoveInjection = (data) => {
  const { target, index } = data
  
  if (target === 'global') {
    // Handle global + panel_all combined
    const globalItems = injectedKnowledge.value['global'] || []
    const panelItems = injectedKnowledge.value['panel_all'] || []
    const combinedLength = globalItems.length
    
    if (index < combinedLength) {
      // Remove from global
      injectedKnowledge.value['global'].splice(index, 1)
    } else {
      // Remove from panel_all
      injectedKnowledge.value['panel_all'].splice(index - combinedLength, 1)
    }
  } else {
    // Agent-specific
    const key = typeof target === 'number' ? `agent_${target}` : target
    if (injectedKnowledge.value[key]) {
      injectedKnowledge.value[key].splice(index, 1)
    }
  }
  
  // Trigger reactivity
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
  
  // Trigger reactivity
  injectedKnowledge.value = { ...injectedKnowledge.value }
  addLog(`Cleared injections for ${target}`)
}

const exportKnowledgePad = () => {
  const exportData = {
    simulationId: simulationId.value,
    highlights: knowledgePad.value,
    exportedAt: new Date().toISOString()
  }
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
  const { highlights, sourceSimulationId, exportedAt } = data
  
  // Merge imported highlights with existing ones
  // Assign new IDs to avoid conflicts
  const importedHighlights = highlights.map(h => ({
    ...h,
    id: crypto.randomUUID(), // New ID to avoid duplicates
    importedFrom: sourceSimulationId,
    originalId: h.id
  }))
  
  knowledgePad.value.push(...importedHighlights)
  addLog(`Imported ${importedHighlights.length} highlight(s) from ${sourceSimulationId || 'unknown source'}`)
}

// --- Layout Methods ---
const toggleMaximize = (target) => {
  if (viewMode.value === target) {
    viewMode.value = 'split'
  } else {
    viewMode.value = target
  }
}

// --- Data Logic ---
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

const loadReportData = async () => {
  try {
    addLog(`Loading report data: ${currentReportId.value}`)
    
    // Get report info to fetch simulation_id
    const reportRes = await getReport(currentReportId.value)
    if (reportRes.success && reportRes.data) {
      const reportData = reportRes.data
      simulationId.value = reportData.simulation_id
      
      // Load profiles for knowledge pad injection
      await loadProfiles()
      
      if (simulationId.value) {
        // Get simulation info
        const simRes = await getSimulation(simulationId.value)
        if (simRes.success && simRes.data) {
          const simData = simRes.data
          
          // Extract hot_topics for Knowledge Pad tagging
          if (simData.event_config?.hot_topics) {
            simulationTags.value = simData.event_config.hot_topics
            addLog(`Loaded ${simulationTags.value.length} simulation tags for Knowledge Pad`)
          }
          
          // Get project info
          if (simData.project_id) {
            const projRes = await getProject(simData.project_id)
            if (projRes.success && projRes.data) {
              projectData.value = projRes.data
              addLog(`Project loaded successfully: ${projRes.data.project_id}`)
              
              // get graph data
              if (projRes.data.graph_id) {
                await loadGraph(projRes.data.graph_id)
              }
            }
          }
        }
      }
    } else {
      addLog(`Failed to get report info: ${reportRes.error || 'Unknown error'}`)
    }
  } catch (err) {
    addLog(`Loading exception: ${err.message}`)
  }
}

const loadGraph = async (graphId) => {
  graphLoading.value = true
  
  try {
    const res = await getGraphData(graphId)
    if (res.success) {
      graphData.value = res.data
      addLog('Graph data loaded successfully')
    }
  } catch (err) {
    addLog(`Graph loading failed: ${err.message}`)
  } finally {
    graphLoading.value = false
  }
}

const refreshGraph = () => {
  if (projectData.value?.graph_id) {
    loadGraph(projectData.value.graph_id)
  }
}

// Watch route params
watch(() => route.params.reportId, (newId) => {
  if (newId && newId !== currentReportId.value) {
    currentReportId.value = newId
    loadReportData()
  }
}, { immediate: true })

onMounted(() => {
  addLog('InteractionView initialized')
  loadReportData()
  
  // Load pre-loaded knowledge from Step 2 (if any)
  const preloadedRaw = sessionStorage.getItem('preloadedKnowledge')
  if (preloadedRaw) {
    try {
      const preloaded = JSON.parse(preloadedRaw)
      if (Array.isArray(preloaded) && preloaded.length > 0) {
        // Add to knowledgePad with new IDs
        const imported = preloaded.map(h => ({
          ...h,
          id: crypto.randomUUID(),
          preloaded: true,
          originalId: h.id
        }))
        knowledgePad.value.push(...imported)
        
        // Also apply agent mappings to injectedKnowledge for auto-injection
        imported.forEach(h => {
          if (h.isGlobal || h.mappedAgentIdx === null) {
            // Add as global knowledge
            if (!injectedKnowledge.value['global']) {
              injectedKnowledge.value['global'] = []
            }
            injectedKnowledge.value['global'].push(h.content)
          } else if (h.mappedAgentIdx !== undefined) {
            // Add to specific agent
            const key = `agent_${h.mappedAgentIdx}`
            if (!injectedKnowledge.value[key]) {
              injectedKnowledge.value[key] = []
            }
            injectedKnowledge.value[key].push(h.content)
          }
        })
        
        // Trigger reactivity
        injectedKnowledge.value = { ...injectedKnowledge.value }
        
        const globalCount = imported.filter(h => h.isGlobal || h.mappedAgentIdx === null).length
        const mappedCount = imported.length - globalCount
        addLog(`Loaded ${imported.length} pre-loaded items (${mappedCount} agent-specific, ${globalCount} global)`)
        
        // Clear after loading to avoid re-loading on refresh
        sessionStorage.removeItem('preloadedKnowledge')
      }
    } catch (err) {
      addLog(`Failed to load pre-loaded knowledge: ${err.message}`)
    }
  }
})
</script>

<style scoped>
.main-view {
  height: 100vh;
  display: flex;
  flex-direction: column;
  background: #FFF;
  overflow: hidden;
  font-family: 'Space Grotesk', -apple-system, BlinkMacSystemFont, sans-serif;
}

/* Header */
.app-header {
  height: 60px;
  border-bottom: 1px solid #EAEAEA;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 24px;
  background: #FFF;
  z-index: 100;
  position: relative;
}

.header-center {
  position: absolute;
  left: 50%;
  transform: translateX(-50%);
}

.brand {
  font-family: 'JetBrains Mono', monospace;
  font-weight: 800;
  font-size: 18px;
  letter-spacing: 1px;
  cursor: pointer;
}

.view-switcher {
  display: flex;
  background: #F5F5F5;
  padding: 4px;
  border-radius: 6px;
  gap: 4px;
}

.switch-btn {
  border: none;
  background: transparent;
  padding: 6px 16px;
  font-size: 12px;
  font-weight: 600;
  color: #666;
  border-radius: 4px;
  cursor: pointer;
  transition: all 0.2s;
}

.switch-btn.active {
  background: #FFF;
  color: #000;
  box-shadow: 0 2px 4px rgba(0,0,0,0.05);
}

.header-right {
  display: flex;
  align-items: center;
  gap: 16px;
}

.workflow-step {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
}

.step-num {
  font-family: 'JetBrains Mono', monospace;
  font-weight: 700;
  color: #999;
}

.step-name {
  font-weight: 700;
  color: #000;
}

.step-divider {
  width: 1px;
  height: 14px;
  background-color: #E0E0E0;
}

.status-indicator {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  color: #666;
  font-weight: 500;
}

.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #CCC;
}

.status-indicator.ready .dot { background: #4CAF50; }
.status-indicator.processing .dot { background: #FF9800; animation: pulse 1s infinite; }
.status-indicator.completed .dot { background: #4CAF50; }
.status-indicator.error .dot { background: #F44336; }

@keyframes pulse { 50% { opacity: 0.5; } }

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
  border-right: 1px solid #EAEAEA;
}
</style>
