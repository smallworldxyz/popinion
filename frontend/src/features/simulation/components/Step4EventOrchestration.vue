<template>
  <div class="step-card" :class="{ 'active': phase === 3, 'completed': phase > 3 }">
    <div class="card-header">
      <div class="step-info">
        <span class="step-num">04</span>
        <span class="step-title">Event Orchestration</span>
      </div>
      <div class="step-status">
        <span v-if="phase > 3" class="badge success">Completed</span>
        <span v-else-if="phase === 3" class="badge processing">Orchestrating</span>
        <span v-else class="badge pending">Wait</span>
      </div>
    </div>

    <div class="card-content">
      <p class="api-note">POST /api/simulation/prepare</p>
      <p class="description">
        Based on narrative direction, automatically generate initial activation events and hot topics.
      </p>

      <div v-if="config?.event_config" class="orchestration-content">
        <!-- NarrativeDirection -->
        <div class="narrative-box">
          <span class="box-label">Narrative Direction</span>
          <p class="narrative-text">{{ config.event_config.narrative_direction }}</p>
        </div>

        <!-- Initial posts stream -->
        <div class="initial-posts-section">
           <span class="box-label">Initial Activation Sequence ({{ config.event_config.initial_posts.length }})</span>
           <div class="posts-list">
             <div v-for="(post, idx) in config.event_config.initial_posts.slice(0, 3)" :key="idx" class="post-preview">
               <span class="post-role">{{ post.poster_type }}</span>
               <p>{{ post.content.substring(0, 100) }}...</p>
             </div>
           </div>
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
.narrative-box {
  background: rgba(var(--primary-rgb), 0.05);
  border: 1px solid rgba(var(--primary-rgb), 0.2);
  border-radius: 8px;
  padding: 16px;
  margin-bottom: 16px;
}

.box-label {
  display: block;
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  color: var(--primary);
  margin-bottom: 8px;
}

.narrative-text {
  font-size: 13px;
  line-height: 1.6;
}

.post-preview {
  background: var(--bg-tertiary);
  padding: 12px;
  border-radius: 6px;
  margin-bottom: 8px;
  font-size: 12px;
}

.post-role {
  font-weight: 600;
  color: var(--text-secondary);
  display: block;
  margin-bottom: 4px;
  font-size: 10px;
  text-transform: uppercase;
}
</style>
