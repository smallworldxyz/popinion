<template>
  <div class="platform-breakdown">
    <div class="breakdown-header">
      <h3 class="section-title">Platform Analytics</h3>
      <div class="time-filter">
        <button v-for="period in timePeriods" :key="period.value" class="time-btn" :class="{ active: selectedPeriod === period.value }" @click="selectedPeriod = period.value">
          {{ period.label }}
        </button>
      </div>
    </div>

    <div class="chart-card sentiment-chart">
      <h4 class="chart-title">Sentiment Comparison</h4>
      <div class="sentiment-bars">
        <div class="sentiment-item">
          <div class="sentiment-label">
            <span class="platform-icon twitter">𝕏</span>
            <span class="platform-name">Twitter</span>
          </div>
          <div class="sentiment-bar-container">
            <div class="sentiment-bar twitter" :style="{ width: `${Math.abs(twitterSentiment)}%` }">
              <span class="sentiment-value">{{ twitterSentiment }}%</span>
            </div>
          </div>
        </div>
        <div class="sentiment-item">
          <div class="sentiment-label">
            <span class="platform-icon reddit">◉</span>
            <span class="platform-name">Reddit</span>
          </div>
          <div class="sentiment-bar-container">
            <div class="sentiment-bar reddit" :style="{ width: `${Math.abs(redditSentiment)}%` }">
              <span class="sentiment-value">{{ redditSentiment }}%</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <div class="chart-card timeline-chart">
      <h4 class="chart-title">Engagement Over Time</h4>
      <div class="timeline-minivisualization">
        <div v-for="(point, idx) in timelineData" :key="idx" class="timeline-bar" :style="{ height: `${(point.value / maxTimelineValue) * 100}%` }">
          <div class="bar-tooltip">Round {{ point.round }}: {{ point.value }}</div>
        </div>
      </div>
      <div class="timeline-axis">
        <span>Round 1</span>
        <span>Round {{ Math.floor(timelineData.length / 2) }}</span>
        <span>Round {{ timelineData.length }}</span>
      </div>
    </div>

    <div class="chart-card distribution-chart">
      <h4 class="chart-title">Action Distribution</h4>
      <div class="distribution-grid">
        <div v-for="action in actionDistribution" :key="action.type" class="distribution-item">
          <div class="distribution-icon" :style="{ background: action.color }">{{ action.icon }}</div>
          <div class="distribution-details">
            <span class="distribution-type">{{ action.type }}</span>
            <span class="distribution-count">{{ action.count }}</span>
          </div>
          <div class="distribution-bar">
            <div class="distribution-fill" :style="{ width: `${action.percentage}%`, background: action.color }"></div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed } from 'vue'

const props = defineProps({
  platformData: {
    type: Object,
    required: true
  }
})

const selectedPeriod = ref('all')
const timePeriods = [
  { label: 'All', value: 'all' },
  { label: '7D', value: '7d' },
  { label: '30D', value: '30d' }
]

const twitterSentiment = computed(() => Math.round((props.platformData?.twitterSentiment || 0.68) * 100))
const redditSentiment = computed(() => Math.round((props.platformData?.redditSentiment || 0.54) * 100))

const timelineData = computed(() => {
  const data = props.platformData?.timeline || []
  if (data.length > 0) return data
  return Array.from({ length: 24 }, (_, i) => ({ round: i + 1, value: Math.floor(Math.random() * 500) + 100 }))
})

const maxTimelineValue = computed(() => Math.max(...timelineData.value.map(p => p.value)))

const actionDistribution = computed(() => {
  const actions = props.platformData?.actions || { create_post: 1200, like_post: 2800, comment: 1500, share: 900 }
  const total = Object.values(actions).reduce((sum, val) => sum + val, 0)
  return [
    { type: 'Posts', icon: '📝', count: actions.create_post, percentage: (actions.create_post / total) * 100, color: '#ff5722' },
    { type: 'Likes', icon: '❤️', count: actions.like_post, percentage: (actions.like_post / total) * 100, color: '#ec4899' },
    { type: 'Comments', icon: '💬', count: actions.comment, percentage: (actions.comment / total) * 100, color: '#3b82f6' },
    { type: 'Shares', icon: '🔁', count: actions.share, percentage: (actions.share / total) * 100, color: '#10b981' }
  ]
})
</script>

<style scoped>
.platform-breakdown {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.breakdown-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}

.section-title {
  font-size: 20px;
  font-weight: 700;
  color: var(--text-main, #fff);
  margin: 0;
}

.time-filter {
  display: flex;
  gap: 8px;
  background: rgba(255, 255, 255, 0.05);
  padding: 4px;
  border-radius: 8px;
}

.time-btn {
  padding: 6px 14px;
  font-size: 13px;
  font-weight: 600;
  font-family: 'JetBrains Mono', monospace;
  color: var(--text-secondary, rgba(255, 255, 255, 0.6));
  background: transparent;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s ease;
}

.time-btn:hover {
  color: var(--text-main, #fff);
  background: rgba(255, 255, 255, 0.05);
}

.time-btn.active {
  color: var(--text-main, #fff);
  background: rgba(255, 87, 34, 0.2);
}

.chart-card {
  padding: 24px;
  background: var(--bg-surface, rgba(255, 255, 255, 0.05));
  /* backdrop-filter removed */
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 12px;
}

.chart-title {
  font-size: 15px;
  font-weight: 600;
  color: var(--text-main, #fff);
  margin: 0 0 20px 0;
}

.sentiment-bars {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.sentiment-item {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.sentiment-label {
  display: flex;
  align-items: center;
  gap: 8px;
}

.platform-icon {
  font-size: 18px;
}

.platform-icon.twitter {
  color: #1da1f2;
}

.platform-icon.reddit {
  color: #ff4500;
}

.platform-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-main, #fff);
}

.sentiment-bar-container {
  position: relative;
  width: 100%;
  height: 32px;
  background: rgba(255, 255, 255, 0.05);
  border-radius: 6px;
  overflow: hidden;
}

.sentiment-bar {
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: flex-end;
  padding: 0 12px;
  border-radius: 6px;
  transition: width 0.6s cubic-bezier(0.4, 0, 0.2, 1);
}

.sentiment-bar.twitter {
  background: linear-gradient(90deg, rgba(29, 161, 242, 0.6), rgba(29, 161, 242, 0.8));
}

.sentiment-bar.reddit {
  background: linear-gradient(90deg, rgba(255, 69, 0, 0.6), rgba(255, 69, 0, 0.8));
}

.sentiment-value {
  font-size: 13px;
  font-weight: 700;
  font-family: 'JetBrains Mono', monospace;
  color: #fff;
  text-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
}

.timeline-minivisualization {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  height: 120px;
  gap: 2px;
  margin-bottom: 8px;
}

.timeline-bar {
  flex: 1;
  background: linear-gradient(180deg, rgba(255, 87, 34, 0.8), rgba(255, 87, 34, 0.4));
  border-radius: 2px 2px 0 0;
  position: relative;
  cursor: pointer;
  transition: all 0.2s ease;
}

.timeline-bar:hover {
  background: linear-gradient(180deg, rgba(167, 139, 250, 1), rgba(167, 139, 250, 0.6));
}

.bar-tooltip {
  position: absolute;
  bottom: 100%;
  left: 50%;
  transform: translateX(-50%);
  padding: 4px 8px;
  background: rgba(0, 0, 0, 0.9);
  color: #fff;
  font-size: 11px;
  font-family: 'JetBrains Mono', monospace;
  border-radius: 4px;
  white-space: nowrap;
  opacity: 0;
  pointer-events: none;
  transition: opacity 0.2s ease;
  margin-bottom: 4px;
}

.timeline-bar:hover .bar-tooltip {
  opacity: 1;
}

.timeline-axis {
  display: flex;
  justify-content: space-between;
  font-size: 11px;
  font-family: 'JetBrains Mono', monospace;
  color: var(--text-secondary, rgba(255, 255, 255, 0.5));
}

.distribution-grid {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.distribution-item {
  display: grid;
  grid-template-columns: auto 1fr auto;
  gap: 12px;
  align-items: center;
}

.distribution-icon {
  width: 36px;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 8px;
  font-size: 16px;
}

.distribution-details {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 80px;
}

.distribution-type {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-main, #fff);
}

.distribution-count {
  font-size: 12px;
  font-family: 'JetBrains Mono', monospace;
  color: var(--text-secondary, rgba(255, 255, 255, 0.6));
}

.distribution-bar {
  flex: 1;
  height: 8px;
  background: rgba(255, 255, 255, 0.05);
  border-radius: 4px;
  overflow: hidden;
}

.distribution-fill {
  height: 100%;
  border-radius: 4px;
  transition: width 0.6s cubic-bezier(0.4, 0, 0.2, 1);
}
</style>
