<template>
  <div class="step-card" :class="{ 'active': phase === 2, 'completed': phase > 2 }">
    <div class="card-header">
      <div class="step-info">
        <span class="step-num">03</span>
        <span class="step-title">World Configuration</span>
      </div>
      <div class="step-status">
        <span v-if="phase > 2" class="badge success">Completed</span>
        <span v-else-if="phase === 2" class="badge processing">Generating</span>
        <span v-else class="badge pending">Wait</span>
      </div>
    </div>

    <div class="card-content">
      <p class="api-note">POST /api/simulation/prepare</p>
      <p class="description">
        LLM intelligently configures world time flow, recommendation algorithms, individual active hours, and platform parameters.
      </p>

      <!-- Config Preview -->
      <div v-if="config" class="config-detail-panel">
        <div class="config-block">
          <div class="config-grid">
            <div class="config-item">
              <span class="config-item-label">Simulation Duration</span>
              <span class="config-item-value">{{ config.time_config?.total_simulation_hours || '-' }} Hours</span>
            </div>
            <div class="config-item">
              <span class="config-item-label">Duration Per Round</span>
              <span class="config-item-value">{{ config.time_config?.minutes_per_round || '-' }} minutes</span>
            </div>
          </div>
        </div>

        <!-- Reasoning -->
        <div v-if="config.generation_reasoning" class="config-block">
           <div class="config-block-header">
             <span class="config-block-title">AI Logic</span>
           </div>
           <p class="reasoning-text">{{ config.generation_reasoning }}</p>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import '../styles/envSetup.css';
import type { SimulationConfig } from '../types/envSetup';

defineProps<{
  phase: number
  config: SimulationConfig | null
}>();
</script>

<style scoped>
.config-detail-panel {
  background: var(--bg-tertiary);
  border-radius: 8px;
  padding: 16px;
  font-size: 13px;
}

.config-block {
  margin-bottom: 16px;
}

.config-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 16px;
}

.config-item {
  display: flex;
  flex-direction: column;
}

.config-item-label {
  font-size: 11px;
  color: var(--text-tertiary);
  margin-bottom: 4px;
}

.config-item-value {
  font-weight: 500;
  color: var(--text-primary);
}

.config-block-header {
  margin-bottom: 8px;
  border-bottom: 1px solid var(--border-color);
  padding-bottom: 4px;
}

.config-block-title {
  font-weight: 600;
  font-size: 12px;
  color: var(--text-secondary);
}

.reasoning-text {
  font-style: italic;
  color: var(--text-secondary);
  line-height: 1.5;
}
</style>
