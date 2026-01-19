<template>
  <div class="influencer-card" :class="`risk-${riskLevel}`">
    <div class="risk-badge">
      <span class="risk-dot"></span>
      <span class="risk-label">{{ riskText }}</span>
    </div>

    <div class="agent-info">
      <div class="agent-avatar">
        <span>{{ agentInitials }}</span>
      </div>
      <div class="agent-details">
        <h4 class="agent-name">{{ influencer.name }}</h4>
        <p class="agent-handle">@{{ influencer.handle }}</p>
      </div>
    </div>

    <div class="influencer-metrics">
      <div class="metric-row">
        <span class="metric-label">Reach</span>
        <span class="metric-value">{{ formatNumber(influencer.reach) }}</span>
      </div>
      <div class="metric-row">
        <span class="metric-label">Actions</span>
        <span class="metric-value">{{ influencer.actionCount }}</span>
      </div>
      <div class="metric-row sentiment-row">
        <span class="metric-label">Sentiment</span>
        <span class="metric-value" :class="sentimentClass">{{ sentimentPercent }}%</span>
      </div>
    </div>

    <div class="platform-badges">
      <span v-if="influencer.onTwitter" class="platform-badge twitter">Twitter</span>
      <span v-if="influencer.onReddit" class="platform-badge reddit">Reddit</span>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'

const props = defineProps({
  influencer: {
    type: Object,
    required: true
  }
})

const riskLevel = computed(() => {
  const sentiment = props.influencer.sentiment || 0
  if (sentiment > 0.5) return 'low'
  if (sentiment < -0.5) return 'high'
  return 'medium'
})

const riskText = computed(() => {
  const level = riskLevel.value
  if (level === 'high') return 'HIGH RISK'
  if (level === 'medium') return 'NEUTRAL'
  return 'ALLY'
})

const agentInitials = computed(() => {
  const name = props.influencer.name || 'Agent'
  const parts = name.split(' ')
  if (parts.length >= 2) return parts[0][0] + parts[1][0]
  return name.substring(0, 2).toUpperCase()
})

const sentimentPercent = computed(() => {
  return Math.round((props.influencer.sentiment || 0) * 100)
})

const sentimentClass = computed(() => {
  const val = sentimentPercent.value
  if (val > 50) return 'positive'
  if (val < -50) return 'negative'
  return 'neutral'
})

const formatNumber = (num) => {
  if (num >= 1000000) return (num / 1000000).toFixed(1) + 'M'
  if (num >= 1000) return (num / 1000).toFixed(1) + 'K'
  return num.toString()
}
</script>

<style scoped>
.influencer-card {
  padding: 20px;
  background: var(--bg-surface, rgba(255, 255, 255, 0.05));
  /* backdrop-filter removed */
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 12px;
  transition: all 0.3s ease;
  position: relative;
  overflow: hidden;
}

.influencer-card::before {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  width: 4px;
  height: 100%;
  background: var(--risk-color);
  transition: width 0.3s ease;
}

.influencer-card:hover {
  border-color: var(--risk-color);
  transform: translateY(-2px);
}

.influencer-card:hover::before {
  width: 6px;
}

.influencer-card.risk-high {
  --risk-color: #ef4444;
}

.influencer-card.risk-medium {
  --risk-color: #f59e0b;
}

.influencer-card.risk-low {
  --risk-color: #22c55e;
}

.risk-badge {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 4px 12px;
  background: rgba(var(--risk-color-rgb), 0.1);
  border-radius: 12px;
  margin-bottom: 16px;
}

.risk-high .risk-badge {
  --risk-color-rgb: 239, 68, 68;
}

.risk-medium .risk-badge {
  --risk-color-rgb: 245, 158, 11;
}

.risk-low .risk-badge {
  --risk-color-rgb: 34, 197, 94;
}

.risk-dot {
  width: 6px;
  height: 6px;
  background: var(--risk-color);
  border-radius: 50%;
  animation: pulse 2s ease-in-out infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.6; transform: scale(1.2); }
}

.risk-label {
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 1px;
  color: var(--risk-color);
  text-transform: uppercase;
}

.agent-info {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 16px;
}

.agent-avatar {
  width: 48px;
  height: 48px;
  background: linear-gradient(135deg, rgba(255, 87, 34, 0.2), rgba(167, 139, 250, 0.2));
  border: 2px solid rgba(255, 87, 34, 0.3);
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 18px;
  font-weight: 700;
  font-family: 'JetBrains Mono', monospace;
  color: var(--text-main, #fff);
}

.agent-details {
  flex: 1;
  min-width: 0;
}

.agent-name {
  font-size: 16px;
  font-weight: 600;
  color: var(--text-main, #fff);
  margin: 0 0 4px 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.agent-handle {
  font-size: 13px;
  color: var(--text-secondary, rgba(255, 255, 255, 0.6));
  margin: 0;
  font-family: 'JetBrains Mono', monospace;
}

.influencer-metrics {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-bottom: 16px;
}

.metric-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 0;
  border-bottom: 1px solid rgba(255, 255, 255, 0.05);
}

.metric-row:last-child {
  border-bottom: none;
}

.metric-label {
  font-size: 13px;
  color: var(--text-secondary, rgba(255, 255, 255, 0.7));
}

.metric-value {
  font-size: 15px;
  font-weight: 600;
  font-family: 'JetBrains Mono', monospace;
  color: var(--text-main, #fff);
}

.sentiment-row .metric-value.positive {
  color: #22c55e;
}

.sentiment-row .metric-value.negative {
  color: #ef4444;
}

.sentiment-row .metric-value.neutral {
  color: #f59e0b;
}

.platform-badges {
  display: flex;
  gap: 8px;
}

.platform-badge {
  padding: 4px 10px;
  font-size: 11px;
  font-weight: 600;
  border-radius: 6px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.platform-badge.twitter {
  background: rgba(29, 161, 242, 0.1);
  color: #1da1f2;
  border: 1px solid rgba(29, 161, 242, 0.3);
}

.platform-badge.reddit {
  background: rgba(255, 69, 0, 0.1);
  color: #ff4500;
  border: 1px solid rgba(255, 69, 0, 0.3);
}
</style>
