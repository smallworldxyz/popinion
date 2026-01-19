<template>
  <div class="dashboard-view">
    <!-- Navbar -->
    <nav class="navbar glass-panel">
      <div class="nav-brand">
        <div class="logo-circle">P</div>
        <span class="brand-text">Popinion Library</span>
      </div>
      <div class="nav-actions">
        <button class="icon-btn" @click="showSettings = true" title="Settings">
           ⚙️
        </button>
      </div>
    </nav>

    <!-- Main Content -->
    <main class="dashboard-content u-container">
      
      <!-- Header Section -->
      <header class="section-header fade-in">
        <div>
          <h1 class="page-title">Simulation Portfolio</h1>
          <p class="page-subtitle">Manage and review your rehearsal scenarios.</p>
        </div>
        <button class="primary-btn-glow" @click="createNew">
          <span class="plus">+</span> New Simulation
        </button>
      </header>
        
      <!-- Loading State -->
      <div v-if="loading" class="loading-state fade-in">
        <div class="spinner-pulse"></div>
        <p>Syncing portfolio...</p>
      </div>

      <!-- Empty State -->
      <div v-else-if="projects.length === 0" class="empty-state fade-in">
        <div class="empty-visual">📂</div>
        <h2>Your Library is Empty</h2>
        <p>Create your first simulation to begin rehearsing future scenarios.</p>
        <button class="primary-btn-glow" @click="createNew">Start New Project</button>
      </div>

      <!-- Project Grid -->
      <div v-else class="project-grid fade-in">
        <ProjectCard 
          v-for="project in projects" 
          :key="project.project_id" 
          :project="project"
          @open="openProject"
          @delete="confirmDelete"
        />
      </div>

    </main>

    <SettingsModal :isOpen="showSettings" @close="showSettings = false" />
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { getProjects, deleteProject } from '../api/project'
import ProjectCard from '../components/ProjectCard.vue'
import SettingsModal from '../components/SettingsModal.vue'

const router = useRouter()
const projects = ref([])
const loading = ref(true)
const showSettings = ref(false)

const loadProjects = async () => {
  loading.value = true
  const res = await getProjects()
  if (res.success) {
    projects.value = res.data
  }
  loading.value = false
}

const createNew = () => {
  router.push('/create')
}

const openProject = (projectId) => {
  router.push({ name: 'Process', params: { projectId } })
}

const confirmDelete = async (projectId) => {
  if (confirm('Are you sure you want to delete this simulation? This cannot be undone.')) {
    await deleteProject(projectId)
    await loadProjects()
  }
}

onMounted(() => {
  loadProjects()
})
</script>

<style scoped>
.dashboard-view {
  min-height: 100vh;
  padding-bottom: 60px;
}

/* Navbar */
.navbar {
  margin: 24px;
  padding: 0 24px;
  height: 64px;
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.nav-brand { display: flex; align-items: center; gap: 12px; }

.logo-circle {
  width: 32px; height: 32px;
  background: var(--primary);
  border-radius: 50%;
  display: flex; align-items: center; justify-content: center;
  font-weight: 800; color: white;
  box-shadow: 0 0 10px var(--primary-glow);
}

.brand-text { font-weight: 700; font-size: 16px; color: var(--text-main); }

.icon-btn {
  width: 40px; height: 40px;
  border-radius: var(--radius-md);
  color: var(--text-muted);
  font-size: 18px;
  transition: all 0.2s;
}

.icon-btn:hover { background: rgba(0,0,0,0.05); color: var(--text-main); }

/* Header */
.section-header {
  margin-top: 40px;
  margin-bottom: 32px;
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
}

.page-title {
  font-size: 32px;
  font-weight: 700;
  letter-spacing: -1px;
  background: linear-gradient(135deg, #000000 0%, #333333 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  margin-bottom: 8px;
}

.page-subtitle { color: var(--text-muted); font-size: 15px; }

/* Primary Button */
.primary-btn-glow {
  background: var(--primary);
  color: white;
  padding: 10px 20px;
  border-radius: var(--radius-md);
  font-weight: 600;
  font-size: 14px;
  display: flex;
  align-items: center;
  gap: 8px;
  transition: all 0.2s;
  box-shadow: 0 0 20px -5px var(--primary-glow);
}

.primary-btn-glow:hover {
  background: var(--primary-hover);
  transform: translateY(-2px);
  box-shadow: 0 5px 20px -5px var(--primary-glow);
}

/* Grid */
.project-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: 24px;
}

/* Loading */
.loading-state, .empty-state {
  text-align: center;
  padding: 80px 0;
  color: var(--text-muted);
}

.spinner-pulse {
  width: 48px; height: 48px;
  background: var(--primary);
  border-radius: 50%;
  margin: 0 auto 24px;
  animation: pulse-ring 1.5s cubic-bezier(0.215, 0.61, 0.355, 1) infinite;
}

@keyframes pulse-ring {
  0% { transform: scale(0.8); box-shadow: 0 0 0 0 rgba(255, 87, 34, 0.7); }
  70% { transform: scale(1); box-shadow: 0 0 0 20px rgba(255, 87, 34, 0); }
  100% { transform: scale(0.8); box-shadow: 0 0 0 0 rgba(255, 87, 34, 0); }
}

.empty-visual { font-size: 64px; margin-bottom: 24px; opacity: 0.2; filter: grayscale(1); }
.empty-state h2 { color: var(--text-main); margin-bottom: 8px; font-size: 20px; }
</style>
