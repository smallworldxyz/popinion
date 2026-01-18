<template>
  <div class="fusion-view">
    <!-- Header -->
    <nav class="navbar">
      <div class="nav-left">
        <div class="nav-brand" @click="router.push('/')">POPINION</div>
        <div class="nav-divider">/</div>
        <div class="nav-title">Graph Fusion</div>
      </div>
      <div class="nav-right">
        <button class="back-btn" @click="router.push('/')">Exit</button>
      </div>
    </nav>

    <div class="fusion-content">
      <!-- Project Selection Panel -->
      <div class="selection-panel card">
        <h3>Select Knowledge Graphs to Merge</h3>
        <p class="description">
          Choose two projects with existing knowledge graphs to analyze their intersections and discover new insights.
        </p>

        <div class="selectors-row">
          <div class="selector-group">
            <label>Source Graph (Base)</label>
            <select v-model="selectedSourceId">
              <option value="" disabled>Select a project...</option>
              <option 
                v-for="project in validProjects" 
                :key="project.project_id" 
                :value="project.project_id"
                :disabled="project.project_id === selectedTargetId"
              >
                {{ project.name || 'Unnamed Project' }} ({{ project.graph_id ? 'Ready' : 'No Graph' }})
              </option>
            </select>
          </div>

          <div class="merge-icon">→</div>

          <div class="selector-group">
            <label>Target Graph (Overlay)</label>
            <select v-model="selectedTargetId">
              <option value="" disabled>Select a project...</option>
              <option 
                v-for="project in validProjects" 
                :key="project.project_id" 
                :value="project.project_id"
                :disabled="project.project_id === selectedSourceId"
              >
                {{ project.name || 'Unnamed Project' }} ({{ project.graph_id ? 'Ready' : 'No Graph' }})
              </option>
            </select>
          </div>

          <button 
            class="analyze-btn" 
            :disabled="!canAnalyze || loading"
            @click="handleAnalyze"
          >
            {{ loading ? 'Analyzing...' : 'Analyze Overlap' }}
          </button>
        </div>
      </div>

      <!-- Results Area -->
      <div class="results-area card" v-if="comparisonResult || loading">
        <div v-if="loading" class="loading-state">
           <div class="spinner"></div>
           <p>Analyzing knowledge graph intersections...</p>
        </div>
        
        <GraphComparison 
          v-else-if="comparisonResult" 
          :previewData="comparisonResult" 
        />
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { listProjects, previewMerge } from '../api/graph'
import GraphComparison from '../components/GraphComparison.vue'

const router = useRouter()

const projects = ref([])
const loading = ref(false)
const selectedSourceId = ref('')
const selectedTargetId = ref('')
const comparisonResult = ref(null)

// Only show projects that have a graph_id
const validProjects = computed(() => {
  return projects.value.filter(p => p.graph_id)
})

const canAnalyze = computed(() => {
  return selectedSourceId.value && selectedTargetId.value
})

onMounted(async () => {
  try {
    const res = await listProjects()
    if (res.success) {
      projects.value = res.data.projects
    }
  } catch (err) {
    console.error("Failed to load projects", err)
  }
})

const handleAnalyze = async () => {
  if (!canAnalyze.value) return
  
  loading.value = true
  comparisonResult.value = null
  
  try {
    const sourceProject = projects.value.find(p => p.project_id === selectedSourceId.value)
    const targetProject = projects.value.find(p => p.project_id === selectedTargetId.value)
    
    const res = await previewMerge({
      source_graph_id: sourceProject.graph_id,
      target_graph_id: targetProject.graph_id
    })
    
    if (res.success) {
      comparisonResult.value = res.data
    }
  } catch (err) {
    alert("Analysis failed: " + err.message)
  } finally {
    loading.value = false
  }
}
</script>

<style scoped>
.fusion-view {
  min-height: 100vh;
  background: #f8f9fa;
  font-family: 'Space Grotesk', sans-serif;
  color: #333;
}

.navbar {
  height: 60px;
  background: #000;
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 24px;
}

.nav-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.nav-brand {
  font-family: 'JetBrains Mono', monospace;
  font-weight: 800;
  cursor: pointer;
}

.nav-divider {
  color: #666;
}

.nav-title {
  font-weight: 600;
}

.back-btn {
  background: transparent;
  border: 1px solid #333;
  color: #fff;
  padding: 6px 16px;
  border-radius: 4px;
  cursor: pointer;
}

.fusion-content {
  max-width: 1200px;
  margin: 40px auto;
  padding: 0 24px;
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.card {
  background: #fff;
  border: 1px solid #eaeaea;
  border-radius: 12px;
  padding: 24px;
  box-shadow: 0 4px 6px rgba(0,0,0,0.02);
}

.card h3 {
  margin: 0 0 8px 0;
  font-size: 1.25rem;
}

.description {
  color: #666;
  font-size: 0.9rem;
  margin-bottom: 24px;
}

.selectors-row {
  display: flex;
  align-items: flex-end;
  gap: 20px;
  flex-wrap: wrap;
}

.selector-group {
  flex: 1;
  min-width: 250px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.selector-group label {
  font-size: 0.8rem;
  font-weight: 600;
  color: #555;
  text-transform: uppercase;
}

.selector-group select {
  padding: 12px;
  border: 1px solid #ddd;
  border-radius: 6px;
  font-size: 0.95rem;
  background: #fff;
  font-family: inherit;
}

.merge-icon {
  font-size: 1.5rem;
  color: #999;
  padding-bottom: 10px;
}

.analyze-btn {
  background: #000;
  color: #fff;
  border: none;
  padding: 12px 24px;
  border-radius: 6px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s;
}

.analyze-btn:disabled {
  background: #ccc;
  cursor: not-allowed;
}

.analyze-btn:not(:disabled):hover {
  background: #333;
  transform: translateY(-1px);
}

.results-area {
  min-height: 300px;
}

.loading-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 200px;
  color: #666;
}

.spinner {
  width: 30px;
  height: 30px;
  border: 3px solid #f3f3f3;
  border-top: 3px solid #000;
  border-radius: 50%;
  animation: spin 1s linear infinite;
  margin-bottom: 15px;
}

@keyframes spin {
  0% { transform: rotate(0deg); }
  100% { transform: rotate(360deg); }
}
</style>
