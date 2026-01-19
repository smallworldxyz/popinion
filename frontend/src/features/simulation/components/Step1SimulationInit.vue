<template>
  <div class="step-card" :class="{ 'active': phase === 0, 'completed': phase > 0 }">
    <div class="card-header">
      <div class="step-info">
        <span class="step-num">01</span>
        <span class="step-title">Simulation Setup</span>
      </div>
      <div class="step-status">
        <span v-if="phase > 0" class="badge success">Completed</span>
        <span v-else class="badge processing">Initializing</span>
      </div>
    </div>

    <div class="card-content">
      <p class="api-note">POST /api/simulation/create</p>
      <p class="description">
        Create new simulation instance, fetch simulation world parameter template
      </p>

      <div class="info-card">
        <div class="info-row">
          <span class="info-label">Simulation ID</span>
          <span class="info-value mono">{{ simulationId || 'Pending...' }}</span>
        </div>
        <div class="info-row">
          <span class="info-label">Task ID</span>
          <span class="info-value mono">{{ taskId || 'Async TaskCompleted' }}</span>
        </div>
      </div>

      <!-- Actions -->
      <div v-if="phase === 0 && simulationId" class="action-group">
        <button 
          class="action-btn secondary"
          @click="$emit('open-ingestion')"
        >
          🌍 Import Reality Data
        </button>

        <button 
          class="action-btn secondary"
          @click="$emit('select-agents')"
        >
          🎯 Select Agents (Optional)
        </button>

        <button 
          class="action-btn primary"
          @click="$emit('start-preparation')"
        >
          ▶ Start Preparation (All Entities)
        </button>
        <span class="action-hint">Choose specific entities OR start with all {{ expectedTotal || '' }} entities</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import '../styles/envSetup.css';

defineProps<{
  phase: number
  taskId: string | null
  simulationId: string | undefined
  expectedTotal: number | null
  loading?: boolean
}>();

defineEmits(['select-agents', 'start-preparation', 'open-ingestion']);
</script>

<style scoped>
.action-btn {
  padding: 8px 16px;
  border-radius: 6px;
  font-weight: 600;
  cursor: pointer;
  border: none;
  font-size: 13px;
  transition: all 0.2s;
}

.action-btn.primary {
  background: var(--primary);
  color: white;
}

.action-btn.primary:hover {
  background: var(--primary-dark);
}

.action-btn.secondary {
  background: var(--bg-tertiary);
  color: var(--text-secondary);
  border: 1px solid var(--border-color);
}

.action-btn.secondary:hover {
  background: var(--bg-hover);
}
</style>
