<template>
  <div class="workbench-panel">
    <div class="scroll-container">
      <!-- Step 01: Analysis -->
      <div class="step-card" :class="{ 'active': currentPhase === 0, 'completed': currentPhase > 0 }">
        <div class="card-header">
          <div class="step-info">
            <span class="step-num">01</span>
            <span class="step-title">Topic Analysis</span>
          </div>
          <div class="step-status">
            <span v-if="currentPhase > 0" class="badge success">Analyzed</span>
            <span v-else-if="currentPhase === 0" class="badge processing">Analyzing...</span>
            <span v-else class="badge pending">Pending</span>
          </div>
        </div>
        
        <div class="card-content">
          <p class="description">
            Analyzing uploaded documents to extract key topics, entities, and relationships for the simulation.
          </p>

          <!-- Loading / Progress -->
          <div v-if="currentPhase === 0 && ontologyProgress" class="progress-section">
            <div class="spinner-sm"></div>
            <span>{{ ontologyProgress.message || 'Processing documents...' }}</span>
          </div>

          <!-- Detail Overlay -->
          <div v-if="selectedOntologyItem" class="ontology-detail-overlay">
            <div class="detail-header">
               <div class="detail-title-group">
                  <span class="detail-type-badge">{{ selectedOntologyItem.itemType === 'entity' ? 'TOPIC' : 'CONNECTION' }}</span>
                  <span class="detail-name">{{ selectedOntologyItem.name }}</span>
               </div>
               <button class="close-btn" @click="selectedOntologyItem = null">×</button>
            </div>
            <div class="detail-body">
               <div class="detail-desc">{{ selectedOntologyItem.description }}</div>
               
               <!-- Attributes -->
               <div class="detail-section" v-if="selectedOntologyItem.attributes?.length">
                  <span class="section-label">PROPERTIES</span>
                  <div class="attr-list">
                     <div v-for="attr in selectedOntologyItem.attributes" :key="attr.name" class="attr-item">
                        <span class="attr-name">{{ attr.name }}</span>
                        <span class="attr-desc">{{ attr.description }}</span>
                     </div>
                  </div>
               </div>

               <!-- Examples (Entity) -->
               <div class="detail-section" v-if="selectedOntologyItem.examples?.length">
                  <span class="section-label">EXAMPLES</span>
                  <div class="example-list">
                     <span v-for="ex in selectedOntologyItem.examples" :key="ex" class="example-tag">{{ ex }}</span>
                  </div>
               </div>

               <!-- Source/Target (Relation) -->
               <div class="detail-section" v-if="selectedOntologyItem.source_targets?.length">
                  <span class="section-label">CONNECTIONS</span>
                  <div class="conn-list">
                     <div v-for="(conn, idx) in selectedOntologyItem.source_targets" :key="idx" class="conn-item">
                        <span class="conn-node">{{ conn.source }}</span>
                        <span class="conn-arrow">→</span>
                        <span class="conn-node">{{ conn.target }}</span>
                     </div>
                  </div>
               </div>
            </div>
          </div>

          <div class="card-actions">
            <button v-if="!isEditingOntology" @click="isEditingOntology = true" class="action-btn secondary sm">Refine Topics</button>
            <template v-else>
              <button @click="cancelEditOntology" class="action-btn text sm">Cancel</button>
              <button @click="saveOntology" class="action-btn primary sm">Save Changes</button>
            </template>
          </div>

          <!-- Ontology Editor -->
          <div v-if="isEditingOntology" class="ontology-editor">
            <!-- Entity Types -->
            <div class="editor-section">
              <span class="tag-label">TOPICS / ENTITIES</span>
              <div v-for="(entity, index) in ontology.entity_types" :key="index" class="editor-item">
                <input v-model="entity.name" placeholder="Entity Name" />
                <button @click="removeEntityType(index)" class="remove-btn icon-only">-</button>
              </div>
              <button @click="addEntityType" class="add-btn text">+ Add Topic</button>
            </div>

            <!-- Relation Types -->
            <div class="editor-section">
              <span class="tag-label">RELATIONSHIPS</span>
              <div v-for="(relation, index) in ontology.edge_types" :key="index" class="editor-item">
                <input v-model="relation.name" placeholder="Relation Name" />
                <button @click="removeEdgeType(index)" class="remove-btn icon-only">-</button>
              </div>
              <button @click="addEdgeType" class="add-btn text">+ Add Relation</button>
            </div>
          </div>

          <!-- Generated Entity Tags -->
          <div v-if="projectData?.ontology?.entity_types && !isEditingOntology" class="tags-container" :class="{ 'dimmed': selectedOntologyItem }">
            <span class="tag-label">IDENTIFIED TOPICS</span>
            <div class="tags-list">
              <span 
                v-for="entity in projectData.ontology.entity_types" 
                :key="entity.name" 
                class="entity-tag clickable"
                @click="selectOntologyItem(entity, 'entity')"
              >
                {{ entity.name }}
              </span>
            </div>
          </div>
        </div>
      </div>

      <!-- Step 02: Graph Build -->
      <div class="step-card" :class="{ 'active': currentPhase === 1, 'completed': currentPhase > 1 }">
        <div class="card-header">
          <div class="step-info">
            <span class="step-num">02</span>
            <span class="step-title">Knowledge Graph Construction</span>
          </div>
          <div class="step-status">
            <span v-if="currentPhase > 1" class="badge success">Built</span>
            <span v-else-if="currentPhase === 1" class="badge processing">{{ buildProgress?.progress || 0 }}%</span>
            <span v-else class="badge pending">Pending</span>
          </div>
        </div>

        <div class="card-content">
          <p class="description">
            Building a structured knowledge graph to enable agents to reason about the context.
          </p>
          
          <!-- Stats Cards -->
          <div class="stats-grid">
            <div class="stat-card">
              <span class="stat-value">{{ graphStats.nodes }}</span>
              <span class="stat-label">Nodes</span>
            </div>
            <div class="stat-card">
              <span class="stat-value">{{ graphStats.edges }}</span>
              <span class="stat-label">Connections</span>
            </div>
            <div class="stat-card">
              <span class="stat-value">{{ graphStats.types }}</span>
              <span class="stat-label">Types</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Step 03: Complete -->
      <div class="step-card" :class="{ 'active': currentPhase === 2, 'completed': currentPhase >= 2 }">
        <div class="card-header">
          <div class="step-info">
            <span class="step-num">03</span>
            <span class="step-title">Ready for Simulation</span>
          </div>
          <div class="step-status">
            <span v-if="currentPhase >= 2" class="badge ready">Ready</span>
          </div>
        </div>
        
        <div class="card-content">
          <p class="description">Knowledge base is ready. Proceed to configure the simulation parameters.</p>
          <button 
            class="action-btn primary large" 
            :disabled="currentPhase < 2 || creatingSimulation"
            @click="handleEnterEnvSetup"
          >
            <span v-if="creatingSimulation">Creating Instance...</span>
            <span v-else>Configure Simulation ➝</span>
          </button>
        </div>
      </div>
    </div>

    <!-- Bottom Info / Logs -->
    <div class="system-logs">
      <div class="log-header">
        <span class="log-title">ACTIVITY LOG</span>
        <span class="log-id">{{ projectData?.project_id || 'IDLE' }}</span>
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
import { computed, ref, watch, nextTick, watchEffect } from 'vue'
import { useRouter } from 'vue-router'
import { createSimulation } from '../api/simulation'
import { updateOntology } from '../api/graph'
import { eventBus } from '../utils/eventBus'

const router = useRouter()

const props = defineProps({
  currentPhase: { type: Number, default: 0 },
  projectData: Object,
  ontologyProgress: Object,
  buildProgress: Object,
  graphData: Object,
  systemLogs: { type: Array, default: () => [] }
})

const emit = defineEmits(['next-step'])

const selectedOntologyItem = ref(null)
const logContent = ref(null)
const creatingSimulation = ref(false)

// Ontology Editing
const isEditingOntology = ref(false)
const ontology = ref({ entity_types: [], edge_types: [] })

const deepClone = (obj) => JSON.parse(JSON.stringify(obj))

watchEffect(() => {
  if (props.projectData?.ontology) {
    ontology.value = deepClone(props.projectData.ontology)
  }
})

// Enter Environment Setup - Create simulation and navigate
const handleEnterEnvSetup = async () => {
  if (!props.projectData?.project_id || !props.projectData?.graph_id) {
    console.error('Missing project or graph info')
    return
  }
  
  creatingSimulation.value = true
  
  try {
    const res = await createSimulation({
      project_id: props.projectData.project_id,
      graph_id: props.projectData.graph_id,
      enable_twitter: true,
      enable_reddit: true
    })
    
    if (res.success && res.data?.simulation_id) {
      emit('next-step')
      router.push({
        name: 'Process',
        params: { projectId: props.projectData.project_id },
        query: { simId: res.data.simulation_id }
      })
      emit('next-step')
    } else {
      console.error('Failed to create simulation:', res.error)
      alert('Failed to create simulation: ' + (res.error || 'Unknown error'))
    }
  } catch (err) {
    console.error('Simulation creation error:', err)
    alert('Simulation creation error: ' + err.message)
  } finally {
    creatingSimulation.value = false
  }
}

const addEntityType = () => {
  ontology.value.entity_types.push({ name: '', attributes: [], examples: [] })
}

const removeEntityType = (index) => {
  ontology.value.entity_types.splice(index, 1)
}

const addEdgeType = () => {
  ontology.value.edge_types.push({ name: '', source_targets: [] })
}

const removeEdgeType = (index) => {
  ontology.value.edge_types.splice(index, 1)
}

const saveOntology = async () => {
  if (!props.projectData?.project_id) return

  try {
    eventBus.showLoading('Saving changes...')
    const res = await updateOntology(props.projectData.project_id, ontology.value)

    if (res.success) {
      if (props.projectData) {
        props.projectData.ontology = deepClone(ontology.value)
      }
      isEditingOntology.value = false
    } else {
      alert('Failed to save: ' + (res.error || 'Unknown error'))
    }
  } catch (err) {
    alert('Error saving: ' + err.message)
  } finally {
    eventBus.hideLoading()
  }
}

const cancelEditOntology = () => {
  if (props.projectData?.ontology) {
    ontology.value = deepClone(props.projectData.ontology)
  }
  isEditingOntology.value = false
}

const selectOntologyItem = (item, type) => {
  selectedOntologyItem.value = { ...item, itemType: type }
}

const graphStats = computed(() => {
  const nodes = props.graphData?.node_count || props.graphData?.nodes?.length || 0
  const edges = props.graphData?.edge_count || props.graphData?.edges?.length || 0
  const types = props.projectData?.ontology?.entity_types?.length || 0
  return { nodes, edges, types }
})

// Auto-scroll logs
watch(() => props.systemLogs.length, () => {
  nextTick(() => {
    if (logContent.value) {
      logContent.value.scrollTop = logContent.value.scrollHeight
    }
  })
})
</script>

<style scoped>
.workbench-panel {
  height: 100%;
  background-color: var(--bg-surface);
  display: flex;
  flex-direction: column;
  position: relative;
  overflow: hidden;
}

.scroll-container {
  flex: 1;
  overflow-y: auto;
  padding: 24px;
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.step-card {
  background: var(--bg-surface);
  border-radius: var(--radius-md);
  padding: 24px;
  box-shadow: var(--shadow-sm);
  border: 1px solid var(--border-light);
  transition: all 0.3s ease;
  position: relative;
}

.step-card.active {
  border-color: var(--primary);
  box-shadow: var(--shadow-md);
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.step-info {
  display: flex;
  align-items: center;
  gap: 12px;
}

.step-num {
  font-family: var(--font-mono);
  font-size: 14px;
  font-weight: 700;
  color: var(--text-faint);
  background: var(--bg-subtle);
  padding: 4px 8px;
  border-radius: 4px;
}

.step-card.active .step-num {
  color: var(--primary);
  background: rgba(255, 69, 0, 0.1);
}

.step-title {
  font-weight: 600;
  font-size: 16px;
  color: var(--text-main);
}

.badge {
  font-size: 11px;
  padding: 4px 10px;
  border-radius: 12px;
  font-weight: 600;
  text-transform: uppercase;
}

.badge.success { background: #E8F5E9; color: #2E7D32; }
.badge.processing { background: rgba(255, 69, 0, 0.1); color: var(--primary); }
.badge.ready { background: #E0F2F1; color: #00897B; }
.badge.pending { background: var(--bg-subtle); color: var(--text-muted); }

.description {
  font-size: 14px;
  color: var(--text-muted);
  line-height: 1.5;
  margin-bottom: 20px;
}

/* Actions */
.card-actions {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  margin-top: 20px;
}

.action-btn {
  padding: 8px 16px;
  border-radius: var(--radius-sm);
  font-weight: 500;
  font-size: 13px;
  transition: all 0.2s;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
}

.action-btn.sm { padding: 6px 12px; font-size: 12px; }
.action-btn.large { padding: 12px 24px; font-size: 14px; width: 100%; }

.action-btn.primary {
  background: var(--primary);
  color: white;
  border: 1px solid var(--primary);
}

.action-btn.primary:hover {
  background: var(--primary-hover);
}

.action-btn.secondary {
  background: white;
  color: var(--text-main);
  border: 1px solid var(--border-light);
}

.action-btn.secondary:hover {
  border-color: var(--text-muted);
}

.action-btn.text {
  background: transparent;
  color: var(--text-muted);
  border: none;
}

.action-btn.text:hover {
  color: var(--text-main);
}

.action-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

/* Tags */
.tags-container {
  margin-top: 16px;
  padding-top: 16px;
  border-top: 1px solid var(--border-subtle);
}

.tag-label {
  display: block;
  font-size: 11px;
  color: var(--text-faint);
  margin-bottom: 12px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.tags-list {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.entity-tag {
  background: var(--bg-subtle);
  border: 1px solid var(--border-light);
  padding: 6px 12px;
  border-radius: var(--radius-full);
  font-size: 12px;
  color: var(--text-main);
  transition: all 0.2s;
}

.entity-tag.clickable:hover {
  border-color: var(--primary);
  color: var(--primary);
  cursor: pointer;
}

/* Stats */
.stats-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 16px;
  background: var(--bg-subtle);
  padding: 20px;
  border-radius: var(--radius-md);
}

.stat-card {
  text-align: center;
}

.stat-value {
  display: block;
  font-size: 24px;
  font-weight: 700;
  color: var(--text-main);
  font-family: var(--font-mono);
}

.stat-label {
  font-size: 11px;
  color: var(--text-muted);
  text-transform: uppercase;
  margin-top: 4px;
  display: block;
}

/* Overlay & Editor Styles */
.ontology-detail-overlay {
    position: absolute;
    top: 20px;
    left: 20px;
    right: 20px;
    bottom: 20px;
    background: rgba(255, 255, 255, 0.98);
    backdrop-filter: blur(8px);
    z-index: 10;
    border: 1px solid var(--border-light);
    box-shadow: var(--shadow-lg);
    border-radius: var(--radius-md);
    display: flex;
    flex-direction: column;
}

.detail-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border-light);
}

.detail-type-badge {
    background: var(--text-main);
    color: white;
    font-size: 10px;
    padding: 2px 6px;
    border-radius: 4px;
    margin-right: 8px;
}

.detail-name {
    font-weight: 700;
    font-size: 16px;
}

.detail-body {
    padding: 24px;
    overflow-y: auto;
}

.detail-desc {
    margin-bottom: 24px;
    color: var(--text-muted);
    font-size: 14px;
    line-height: 1.6;
}

/* System Logs */
.system-logs {
  background: var(--bg-subtle);
  border-top: 1px solid var(--border-light);
  padding: 16px;
  flex-basis: 200px;
  display: flex;
  flex-direction: column;
}

.log-header {
  display: flex;
  justify-content: space-between;
  margin-bottom: 12px;
  font-size: 11px;
}

.log-title {
  font-weight: 700;
  color: var(--text-muted);
}

.log-content {
  flex: 1;
  overflow-y: auto;
  font-family: var(--font-mono);
  font-size: 11px;
  color: var(--text-muted);
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.log-time {
  color: var(--text-faint);
  margin-right: 8px;
}

.spinner-sm {
  width: 14px;
  height: 14px;
  border: 2px solid var(--primary-light);
  border-top-color: var(--primary);
  border-radius: 50%;
  animation: spin 1s linear infinite;
  display: inline-block;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}
</style>
