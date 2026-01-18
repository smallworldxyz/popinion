<template>
  <div class="agent-explorer">
    <div class="explorer-header">
      <div class="search-bar">
        <i class="search-icon">🔍</i>
        <input 
          v-model="searchQuery" 
          type="text" 
          placeholder="Search agents by name, profession, or beliefs..."
          class="search-input"
        />
      </div>
      <div class="stats" v-if="agents.length">
        {{ filteredAgents.length }} / {{ agents.length }} Agents
      </div>
    </div>

    <div class="explorer-content">
      <div v-if="loading" class="loading-state">
        <div class="spinner"></div>
        <span>Loading agent population...</span>
      </div>

      <div v-else-if="filteredAgents.length === 0" class="empty-state">
        No agents found matching your search.
      </div>

      <div v-else class="agent-table-container">
        <table class="agent-table">
          <thead>
            <tr>
              <th @click="sortBy('name')">Name</th>
              <th @click="sortBy('age')">Age</th>
              <th @click="sortBy('profession')">Profession</th>
              <th>Bio Snippet</th>
              <th>Action</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="agent in filteredAgents" :key="agent.user_id" @click="selectAgent(agent)">
              <td class="name-col">
                <div class="agent-avatar">{{ getInitials(agent.name) }}</div>
                <span class="agent-name">{{ agent.name }}</span>
              </td>
              <td>{{ agent.age }}</td>
              <td>
                <span class="badge">{{ agent.profession }}</span>
              </td>
              <td class="bio-col">{{ truncate(agent.description || '', 80) }}</td>
              <td>
                <button class="view-btn" @click.stop="selectAgent(agent)">View Profile</button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- Agent Detail Modal -->
    <div v-if="selectedAgent" class="modal-overlay" @click="selectedAgent = null">
      <div class="modal-card" @click.stop>
        <button class="close-btn" @click="selectedAgent = null">×</button>
        
        <div class="agent-header">
          <div class="large-avatar">{{ getInitials(selectedAgent.name) }}</div>
          <div class="agent-identity">
            <h2>{{ selectedAgent.name }}</h2>
            <div class="tags">
              <span class="tag age">{{ selectedAgent.age }} years old</span>
              <span class="tag gender">{{ selectedAgent.gender }}</span>
              <span class="tag profession">{{ selectedAgent.profession }}</span>
            </div>
          </div>
        </div>

        <div class="agent-body">
          <section class="info-section">
            <h3>📖 Biography</h3>
            <p>{{ selectedAgent.description }}</p>
          </section>

          <section class="info-section">
            <h3>🧠 Core Beliefs & Values</h3>
            <div class="beliefs-list">
              <div v-for="(belief, idx) in parseList(selectedAgent.core_beliefs)" :key="idx" class="belief-item">
                {{ belief }}
              </div>
            </div>
          </section>
          
          <section class="info-section two-col">
            <div>
              <h3>🗣️ Communication Style</h3>
               <p>{{ selectedAgent.communication_style }}</p>
            </div>
            <div>
               <h3>🎯 Goals</h3>
               <div v-for="(goal, idx) in parseList(selectedAgent.goals)" :key="idx" class="list-item">
                 • {{ goal }}
               </div>
            </div>
          </section>
             
          <section class="info-section" v-if="selectedAgent.topics_of_interest">
            <h3>interests</h3>
            <div class="interests-tags">
               <span v-for="(interest, idx) in parseList(selectedAgent.topics_of_interest)" :key="idx" class="interest-tag">
                 {{ interest }}
               </span>
            </div>
          </section>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import { getProjectAgents } from '../api/simulation'

const props = defineProps({
  projectId: { type: String, required: true },
  simulationId: { type: String, default: null }
})

const agents = ref([])
const loading = ref(true)
const searchQuery = ref('')
const selectedAgent = ref(null)
const sortKey = ref('name')
const sortDesc = ref(false)

const loadAgents = async () => {
  loading.value = true
  try {
    const res = await getProjectAgents(props.projectId, props.simulationId)
    if (res.success) {
      agents.value = res.data || []
    }
  } catch (err) {
    console.error("Failed to load agents", err)
  } finally {
    loading.value = false
  }
}

const filteredAgents = computed(() => {
  let result = agents.value
  
  if (searchQuery.value) {
    const q = searchQuery.value.toLowerCase()
    result = result.filter(a => 
      (a.name && a.name.toLowerCase().includes(q)) ||
      (a.profession && a.profession.toLowerCase().includes(q)) ||
      (a.description && a.description.toLowerCase().includes(q))
    )
  }
  
  return result.sort((a, b) => {
    let valA = a[sortKey.value]
    let valB = b[sortKey.value]
    
    // Handle numeric age
    if (sortKey.value === 'age') {
        valA = parseInt(valA) || 0
        valB = parseInt(valB) || 0
    } else {
        valA = (valA || '').toString().toLowerCase()
        valB = (valB || '').toString().toLowerCase()
    }
    
    if (valA < valB) return sortDesc.value ? 1 : -1
    if (valA > valB) return sortDesc.value ? -1 : 1
    return 0
  })
})

const sortBy = (key) => {
  if (sortKey.value === key) {
    sortDesc.value = !sortDesc.value
  } else {
    sortKey.value = key
    sortDesc.value = false
  }
}

const selectAgent = (agent) => {
  selectedAgent.value = agent
}

const getInitials = (name) => {
  return name ? name.split(' ').map(n => n[0]).join('').substring(0,2).toUpperCase() : '?'
}

const truncate = (text, length) => {
  return text.length > length ? text.substring(0, length) + '...' : text
}

const parseList = (val) => {
    if (Array.isArray(val)) return val
    if (typeof val === 'string') return [val]
    return []
}

onMounted(() => {
  loadAgents()
})
</script>

<style scoped>
.agent-explorer {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: white;
  font-family: 'Space Grotesk', sans-serif;
}

.explorer-header {
  padding: 16px;
  border-bottom: 1px solid #eee;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.search-bar {
  position: relative;
  width: 300px;
}

.search-input {
  width: 100%;
  padding: 8px 12px 8px 32px;
  border: 1px solid #ddd;
  border-radius: 6px;
  font-size: 14px;
}

.search-icon {
  position: absolute;
  left: 8px;
  top: 50%;
  transform: translateY(-50%);
  font-style: normal;
  font-size: 14px;
}

.stats {
  font-size: 13px;
  color: #666;
  font-weight: 500;
}

.explorer-content {
  flex: 1;
  overflow: hidden;
  position: relative;
}

.loading-state, .empty-state {
  height: 100%;
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  color: #666;
  gap: 12px;
}

.spinner {
  width: 24px;
  height: 24px;
  border: 2px solid #ddd;
  border-top-color: #000;
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin { to { transform: rotate(360deg); } }

.agent-table-container {
  height: 100%;
  overflow-y: auto;
}

.agent-table {
  width: 100%;
  border-collapse: collapse;
}

.agent-table th {
  position: sticky;
  top: 0;
  background: #f9f9f9;
  text-align: left;
  padding: 12px 16px;
  font-size: 12px;
  font-weight: 600;
  color: #666;
  border-bottom: 1px solid #eee;
  cursor: pointer;
}

.agent-table th:hover {
  background: #f0f0f0;
}

.agent-table td {
  padding: 12px 16px;
  border-bottom: 1px solid #f5f5f5;
  font-size: 14px;
  cursor: pointer;
}

.agent-table tr:hover {
  background: #fcfcfc;
}

.name-col {
  display: flex;
  align-items: center;
  gap: 12px;
  font-weight: 600;
}

.agent-avatar {
  width: 32px;
  height: 32px;
  background: #000;
  color: #fff;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  font-weight: 700;
}

.badge {
  background: #f0f0f0;
  padding: 4px 8px;
  border-radius: 4px;
  font-size: 12px;
  color: #333;
}

.bio-col {
  color: #666;
  font-size: 13px;
  max-width: 300px;
}

.view-btn {
  padding: 4px 12px;
  border: 1px solid #ddd;
  background: white;
  border-radius: 4px;
  font-size: 12px;
  cursor: pointer;
}

.view-btn:hover {
  background: #f5f5f5;
  border-color: #ccc;
}

/* Modal */
.modal-overlay {
  position: fixed;
  top: 0; 
  left: 0;
  width: 100%;
  height: 100%;
  background: rgba(0,0,0,0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 2000;
  backdrop-filter: blur(2px);
}

.modal-card {
  background: white;
  width: 700px;
  max-height: 85vh;
  border-radius: 12px;
  box-shadow: 0 20px 40px rgba(0,0,0,0.2);
  display: flex;
  flex-direction: column;
  position: relative;
  overflow: hidden;
}

.close-btn {
  position: absolute;
  top: 16px;
  right: 16px;
  border: none;
  background: transparent;
  font-size: 24px;
  cursor: pointer;
  z-index: 10;
  color: #666;
}

.agent-header {
  padding: 24px;
  background: #f9f9f9;
  border-bottom: 1px solid #eee;
  display: flex;
  align-items: center;
  gap: 20px;
}

.large-avatar {
  width: 64px;
  height: 64px;
  background: #333;
  color: white;
  border-radius: 50%;
  font-size: 24px;
  font-weight: 700;
  display: flex;
  align-items: center;
  justify-content: center;
}

.agent-identity h2 {
  margin: 0 0 8px 0;
  font-size: 24px;
}

.tags {
  display: flex;
  gap: 8px;
}

.tag {
  font-size: 12px;
  padding: 4px 8px;
  border-radius: 4px;
  font-weight: 500;
}

.tag.age { background: #e3f2fd; color: #1565c0; }
.tag.gender { background: #f3e5f5; color: #7b1fa2; }
.tag.profession { background: #e8f5e9; color: #2e7d32; }

.agent-body {
  padding: 24px;
  overflow-y: auto;
}

.info-section {
  margin-bottom: 24px;
}

.info-section h3 {
  font-size: 14px;
  text-transform: uppercase;
  color: #999;
  letter-spacing: 1px;
  margin-bottom: 12px;
}

.info-section p {
  line-height: 1.6;
  color: #333;
}

.beliefs-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.belief-item {
  background: #fff8e1;
  padding: 8px 12px;
  border-left: 3px solid #ffc107;
  border-radius: 0 4px 4px 0;
  font-size: 14px;
}

.two-col {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 24px;
}

.list-item {
  margin-bottom: 6px;
  font-size: 14px;
}

.interests-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
}

.interest-tag {
    background: #f5f5f5;
    padding: 6px 12px;
    border-radius: 20px;
    font-size: 13px;
    color: #555;
    border: 1px solid #eee;
}
</style>
