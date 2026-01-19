<template>
  <div class="executive-summary">
    <div class="mission-status" :class="`status-${missionStatus}`">
      <div class="status-icon">
        <svg v-if="missionStatus === 'completed'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path>
          <polyline points="22 4 12 14.01 9 11.01"></polyline>
        </svg>
      </div>
      <div class="status-text">
        <span class="status-label">MISSION STATUS</span>
        <span class="status-value">{{ statusText }}</span>
      </div>
    </div>

    <div class="metrics-grid">
      <div class="metric-card impact-score">
        <div class="metric-header">
          <span class="metric-icon">🎯</span>
          <span class="metric-label">Impact Score</span>
        </div>
        <div class="metric-value-row">
          <span class="metric-value">{{ impactScore }}</span>
          <span class="metric-max">/100</span>
        </div>
        <div class="progress-bar">
          <div class="progress-fill" :style="{ width: `${impactScore}%` }"></div>
        </div>
      </div>

      <div class="metric-card">
        <div class="metric-header">
          <span class="metric-icon">📡</span>
          <span class="metric-label">Total Reach</span>
        </div>
        <div class="metric-value-row">
          <span class="metric-value">{{ formatNumber(totalReach) }}</span>
        </div>
        <div class="metric-subtext">impressions</div>
      </div>

      <div class="metric-card sentiment">
        <div class="metric-header">
          <span class="metric-icon">💭</span>
          <span class="metric-label">Avg Sentiment</span>
        </div>
        <div class="metric-value-row">
          <span class="metric-value" :class="sentimentClass">{{ sentimentPercent }}%</span>
        </div>
        <div class="sentiment-bar">
          <div class="sentiment-fill" :class="sentimentClass" :style="{ width: `${Math.abs(sentimentPercent)}%` }"></div>
        </div>
      </div>

      <div class="metric-card">
        <div class="metric-header">
          <span class="metric-icon">⚡</span>
          <span class="metric-label">Engagement</span>
        </div>
        <div class="metric-value-row">
          <span class="metric-value">{{ formatNumber(totalEngagement) }}</span>
        </div>
        <div class="metric-subtext">actions</div>
      </div>
    </div>

    <div class="platform-split">
      <div class="platform-item twitter">
        <div class="platform-header">
          <span class="platform-name">Twitter</span>
          <span class="platform-percent">{{ twitterPercent }}%</span>
        </div>
        <div class="platform-bar">
          <div class="platform-fill" :style="{ width: `${twitterPercent}%` }"></div>
        </div>
      </div>
      <div class="platform-item reddit">
        <div class="platform-header">
          <span class="platform-name">Reddit</span>
          <span class="platform-percent">{{ redditPercent }}%</span>
        </div>
        <div class="platform-bar">
          <div class="platform-fill" :style="{ width: `${redditPercent}%` }"></div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'

const props = defineProps({
  reportData: {
    type: Object,
    required: true
  }
})

const missionStatus = computed(() => props.reportData?.status || 'completed')
const statusText = computed(() => {
  const status = missionStatus.value
  if (status === 'completed') return 'COMPLETED'
  if (status === 'running') return 'IN PROGRESS'
  return 'PENDING'
})

const impactScore = computed(() => props.reportData?.impactScore || 87)
const totalReach = computed(() => props.reportData?.totalReach || 1200000)
const sentimentValue = computed(() => props.reportData?.avgSentiment || 0.62)
const sentimentPercent = computed(() => Math.round(sentimentValue.value * 100))
const sentimentClass = computed(() => {
  const val = sentimentPercent.value
  if (val > 50) return 'positive'
  if (val < -50) return 'negative'
  return 'neutral'
})

const totalEngagement = computed(() => props.reportData?.totalEngagement || 45000)
const twitterActions = computed(() => props.reportData?.twitterActions || 28000)
const redditActions = computed(() => props.reportData?.redditActions || 17000)
const totalActions = computed(() => twitterActions.value + redditActions.value)
const twitterPercent = computed(() => Math.round((twitterActions.value / totalActions.value) * 100))
const redditPercent = computed(() => Math.round((redditActions.value / totalActions.value) * 100))

const formatNumber = (num) => {
  if (num >= 1000000) return (num / 1000000).toFixed(1) + 'M'
  if (num >= 1000) return (num / 1000).toFixed(1) + 'K'
  return num.toString()
}
</script>

<style scoped>
.executive-summary {
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.mission-status {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 20px 24px;
  background: var(--bg-surface, rgba(255, 255, 255, 0.05));
  /* backdrop-filter removed */
  border: 1px solid rgba(255, 87, 34, 0.2);
  border-radius: 12px;
  transition: all 0.3s ease;
}

.mission-status.status-completed {
  border-color: rgba(34, 197, 94, 0.3);
  background: rgba(34, 197, 94, 0.05);
}

.status-icon {
  width: 48px;
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(34, 197, 94, 0.1);
  border-radius: 50%;
  color: #22c55e;
}

.status-icon svg {
  width: 24px;
  height: 24px;
}

.status-text {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.status-label {
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 1px;
  color: var(--text-secondary, rgba(255, 255, 255, 0.6));
  text-transform: uppercase;
}

.status-value {
  font-size: 24px;
  font-weight: 700;
  font-family: 'JetBrains Mono', monospace;
  color: var(--text-main, #fff);
}

.metrics-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 16px;
}

.metric-card {
  padding: 20px;
  background: var(--bg-surface, rgba(255, 255, 255, 0.05));
  /* backdrop-filter removed */
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 12px;
  transition: all 0.3s ease;
}

.metric-card:hover {
  border-color: rgba(255, 87, 34, 0.3);
  transform: translateY(-2px);
}

.metric-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
}

.metric-icon {
  font-size: 20px;
}

.metric-label {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-secondary, rgba(255, 255, 255, 0.7));
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.metric-value-row {
  display: flex;
  align-items: baseline;
  gap: 4px;
  margin-bottom: 8px;
}

.metric-value {
  font-size: 36px;
  font-weight: 700;
  font-family: 'JetBrains Mono', monospace;
  color: var(--text-main, #fff);
  line-height: 1;
}

.metric-max {
  font-size: 18px;
  font-weight: 500;
  color: var(--text-secondary, rgba(255, 255, 255, 0.5));
}

.metric-subtext {
  font-size: 12px;
  color: var(--text-secondary, rgba(255, 255, 255, 0.6));
}

.progress-bar {
  width: 100%;
  height: 8px;
  background: rgba(255, 255, 255, 0.1);
  border-radius: 4px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  background: linear-gradient(90deg, #ff5722, #a78bfa);
  border-radius: 4px;
  transition: width 0.6s cubic-bezier(0.4, 0, 0.2, 1);
}

.metric-value.positive {
  color: #22c55e;
}

.metric-value.negative {
  color: #ef4444;
}

.metric-value.neutral {
  color: #f59e0b;
}

.sentiment-bar {
  width: 100%;
  height: 8px;
  background: rgba(255, 255, 255, 0.1);
  border-radius: 4px;
  overflow: hidden;
}

.sentiment-fill {
  height: 100%;
  border-radius: 4px;
  transition: width 0.6s cubic-bezier(0.4, 0, 0.2, 1);
}

.sentiment-fill.positive {
  background: linear-gradient(90deg, #10b981, #22c55e);
}

.sentiment-fill.negative {
  background: linear-gradient(90deg, #dc2626, #ef4444);
}

.sentiment-fill.neutral {
  background: linear-gradient(90deg, #d97706, #f59e0b);
}

.platform-split {
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 20px;
  background: var(--bg-surface, rgba(255, 255, 255, 0.05));
  /* backdrop-filter removed */
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 12px;
}

.platform-item {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.platform-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.platform-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-main, #fff);
}

.platform-percent {
  font-size: 14px;
  font-weight: 600;
  font-family: 'JetBrains Mono', monospace;
  color: var(--text-secondary, rgba(255, 255, 255, 0.7));
}

.platform-bar {
  width: 100%;
  height: 6px;
  background: rgba(255, 255, 255, 0.1);
  border-radius: 3px;
  overflow: hidden;
}

.platform-item.twitter .platform-fill {
  background: linear-gradient(90deg, #1da1f2, #60b7f6);
}

.platform-item.reddit .platform-fill {
  background: linear-gradient(90deg, #ff4500, #ff6a33);
}

.platform-fill {
  height: 100%;
  border-radius: 3px;
  transition: width 0.6s cubic-bezier(0.4, 0, 0.2, 1);
}
</style>
