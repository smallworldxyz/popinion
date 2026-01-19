<template>
  <div class="step-card" :class="{ 'active': phase === 1, 'completed': phase > 1 }">
    <div class="card-header">
      <div class="step-info">
        <span class="step-num">02</span>
        <span class="step-title">Generate Agent Profiles</span>
      </div>
      <div class="step-status">
        <span v-if="phase > 1" class="badge success">Completed</span>
        <span v-else-if="phase === 1" class="badge processing">{{ prepareProgress }}%</span>
        <span v-else class="badge pending">Wait</span>
      </div>
    </div>

    <div class="card-content">
      <p class="api-note">POST /api/simulation/prepare</p>
      <p class="description">
        Based on graph structure and entity attributes, automatically generate detailed social media personas (style, interested topics, activity patterns)
      </p>

      <div class="status-card" v-if="currentStage || progressMessage">
        <div class="status-header">
          <span class="status-title">Profile Generation Status</span>
          <span class="status-stage">{{ currentStage }}</span>
        </div>
        <div class="progress-bar">
          <div class="progress-fill" :style="{ width: prepareProgress + '%' }"></div>
        </div>
        <p class="status-message">{{ progressMessage }}</p>
        
        <div class="stat-card">
          <span class="stat-value">{{ totalTopicsCount }}</span>
          <span class="stat-label">Identified Topics</span>
        </div>
      </div>

      <!-- Profiles List Preview -->
      <div v-if="profiles.length > 0" class="profiles-preview">
        <div class="preview-header">
          <span class="preview-title">Generated Agent Profiles</span>
          <button class="action-btn sm" @click="$emit('explore')">
            🔍 Explore Agent Population
          </button>
        </div>
        
        <div class="profiles-list">
          <div 
            v-for="(profile, idx) in displayProfiles" 
            :key="idx" 
            class="profile-card"
            @click="$emit('select-profile', profile)"
          >
            <div class="profile-header">
              <span class="profile-username">@{{ profile.username }}</span>
              <span class="profile-role" :title="profile.role_description">{{ profile.role_description?.substring(0, 15) }}...</span>
            </div>
            
            <div class="profile-topics">
              <span 
                v-for="(topic, tIdx) in profile.interested_topics?.slice(0, 3)" 
                :key="tIdx" 
                class="topic-tag"
              >
                {{ topic }}
              </span>
              <span v-if="(profile.interested_topics?.length || 0) > 3" class="topic-more">
                +{{ (profile.interested_topics?.length || 0) - 3 }}
              </span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import '../styles/envSetup.css';
import type { SimulationProfile } from '../types/envSetup';

const props = defineProps<{
  phase: number
  prepareProgress: number
  currentStage: string
  progressMessage: string
  profiles: SimulationProfile[]
  displayProfiles: SimulationProfile[]
  totalTopicsCount: number
}>();

defineEmits(['explore', 'select-profile']);
</script>

<style scoped>
.status-card {
  background: var(--bg-tertiary);
  border-radius: 8px;
  padding: 16px;
  margin-bottom: 24px;
}

.status-header {
  display: flex;
  justify-content: space-between;
  margin-bottom: 12px;
}

.status-title {
  font-weight: 600;
  font-size: 13px;
}

.status-stage {
  font-size: 12px;
  color: var(--primary);
  font-family: monospace;
}

.progress-bar {
  height: 6px;
  background: var(--border-color);
  border-radius: 3px;
  overflow: hidden;
  margin-bottom: 8px;
}

.progress-fill {
  height: 100%;
  background: var(--primary);
  transition: width 0.3s ease;
}

.status-message {
  font-size: 12px;
  color: var(--text-secondary);
  margin-bottom: 12px;
}

.stat-card {
  display: inline-flex;
  flex-direction: column;
  background: rgba(var(--primary-rgb), 0.1);
  padding: 8px 16px;
  border-radius: 6px;
}

.stat-value {
  font-size: 18px;
  font-weight: 700;
  color: var(--primary);
}

.stat-label {
  font-size: 10px;
  text-transform: uppercase;
  color: var(--text-secondary);
}

.preview-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.preview-title {
  font-size: 14px;
  font-weight: 600;
}

.profiles-list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 12px;
}

.profile-card {
  background: var(--bg-tertiary);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  padding: 12px;
  cursor: pointer;
  transition: all 0.2s;
}

.profile-card:hover {
  transform: translateY(-2px);
  border-color: var(--primary);
}

.profile-header {
  margin-bottom: 8px;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.profile-username {
  font-weight: 600;
  font-size: 13px;
  color: var(--primary);
}

.profile-role {
  font-size: 10px;
  background: rgba(255, 255, 255, 0.1);
  padding: 2px 4px;
  border-radius: 4px;
}

.profile-topics {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.topic-tag {
  font-size: 9px;
  background: var(--bg-secondary);
  padding: 2px 6px;
  border-radius: 4px;
  color: var(--text-secondary);
}

.topic-more {
  font-size: 9px;
  color: var(--text-tertiary);
}

.action-btn.sm {
  padding: 4px 12px;
  font-size: 11px;
  background: transparent;
  border: 1px solid var(--border-color);
  color: var(--text-secondary);
  border-radius: 4px;
  cursor: pointer;
}

.action-btn.sm:hover {
  border-color: var(--primary);
  color: var(--primary);
}
</style>
