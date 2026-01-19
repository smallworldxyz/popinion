<template>
  <div 
    class="project-card" 
    @mouseenter="isHovered = true" 
    @mouseleave="isHovered = false"
    @click="$emit('open', project.project_id)"
  >
    <!-- Status Badge (Top Right) -->
    <div class="status-badge" :class="getStatusClass(project.status)">
      <div class="status-dot"></div>
      {{ formatStatus(project.status) }}
    </div>

    <!-- Main Visual / Initials -->
    <div class="card-visual">
      <div class="project-initials">{{ getInitials(project.name) }}</div>
    </div>

    <!-- Content -->
    <div class="card-content">
      <h3 class="project-title" :title="project.name">{{ truncate(project.name || 'Untitled Project', 40) }}</h3>
      <div class="meta-info">
        <span class="date">{{ formatDate(project.created_at) }}</span>
      </div>
    </div>

    <!-- Hover Actions Overlay -->
    <div class="card-actions" :class="{ 'visible': isHovered }">
      <button class="action-btn play" @click.stop="$emit('open', project.project_id)">
        <span class="icon">▶</span> Open
      </button>
      <button class="action-btn delete" @click.stop="$emit('delete', project.project_id)">
        <span class="icon">🗑️</span>
      </button>
    </div>
  </div>
</template>

<script setup>
import { ref } from 'vue'

const props = defineProps({
  project: Object
})

const isHovered = ref(false)

const getInitials = (name) => {
  return (name || 'U').substring(0, 2).toUpperCase()
}

const formatStatus = (status) => {
  const map = {
    'created': 'Draft',
    'ontology_generated': 'Analyzed',
    'graph_building': 'Building',
    'graph_completed': 'Ready',
    'simulation_running': 'Running',
    'simulation_completed': 'Done'
  }
  return map[status] || status
}

const getStatusClass = (status) => {
  if (['simulation_completed', 'graph_completed'].includes(status)) return 'status-success'
  if (['simulation_running', 'graph_building'].includes(status)) return 'status-active'
  return 'status-neutral'
}

const formatDate = (isoString) => {
  if (!isoString) return ''
  return new Date(isoString).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' })
}

const truncate = (text, len) => {
  if (!text) return ''
  return text.length > len ? text.substring(0, len) + '...' : text
}
</script>

<style scoped>
.project-card {
  position: relative;
  background: var(--bg-card);
  border: 1px solid var(--border-light);
  border-radius: var(--radius-lg);
  overflow: hidden;
  height: 240px;
  display: flex;
  flex-direction: column;
  cursor: pointer;
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  /* backdrop-filter removed */
}

.project-card:hover {
  background: var(--bg-card-hover);
  border-color: var(--border-hover);
  transform: translateY(-4px);
  box-shadow: var(--shadow-glow);
}

/* Visual Section (Initials) */
.card-visual {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg-surface);
}

.project-initials {
  font-size: 48px;
  font-weight: 800;
  color: var(--text-muted);
  opacity: 0.2;
  letter-spacing: -2px;
}

/* Status Badge */
.status-badge {
  position: absolute;
  top: 12px;
  right: 12px;
  padding: 4px 10px;
  border-radius: var(--radius-full);
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  display: flex;
  align-items: center;
  gap: 6px;
  background: rgba(255,255,255,0.9);
  border: 1px solid var(--border);
  color: var(--text-secondary);
  box-shadow: var(--shadow-sm);
}

.status-dot {
  width: 6px; height: 6px;
  border-radius: 50%;
  background: var(--text-muted);
}

.status-success { color: var(--success); border-color: rgba(16, 185, 129, 0.2); }
.status-success .status-dot { background: var(--success); box-shadow: 0 0 8px var(--success-glow); }

.status-active { color: var(--active); border-color: rgba(245, 158, 11, 0.2); }
.status-active .status-dot { background: var(--active); }

/* Content Section */
.card-content {
  padding: 20px;
  border-top: 1px solid var(--border-light);
  background: var(--bg-elevated);
}

.project-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--text-main);
  margin-bottom: 4px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.meta-info {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.date {
  font-size: 12px;
  color: var(--text-muted);
}

/* Actions Overlay */
.card-actions {
  position: absolute;
  bottom: 0; left: 0; right: 0;
  padding: 20px;
  display: flex;
  gap: 12px;
  background: linear-gradient(0deg, var(--bg-app) 0%, rgba(255,255,255,0.9) 100%);
  transform: translateY(100%);
  transition: transform 0.2s cubic-bezier(0.4, 0, 0.2, 1);
}

.visible {
  transform: translateY(0);
}

.action-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 8px 16px;
  border-radius: var(--radius-md);
  font-size: 13px;
  font-weight: 600;
  transition: all 0.2s;
}

.action-btn.play {
  flex: 1;
  background: var(--primary);
  color: white;
  box-shadow: 0 0 15px -5px var(--primary-glow);
}

.action-btn.play:hover {
  background: var(--primary-hover);
  transform: translateY(-1px);
}

.action-btn.delete {
  background: rgba(239, 68, 68, 0.1);
  color: var(--error);
  padding: 8px;
}

.action-btn.delete:hover {
  background: rgba(239, 68, 68, 0.2);
}
</style>
