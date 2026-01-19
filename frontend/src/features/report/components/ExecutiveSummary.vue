<template>
  <div class="executive-summary glass-card">
    <div class="summary-header">
      <div class="header-icon">📊</div>
      <h3 class="header-title">Executive Summary</h3>
    </div>
    
    <div class="metrics-grid">
      <!-- Main Verdict / Status -->
      <div class="metric-card main-metric">
        <div class="metric-label">Mission Status</div>
        <div class="metric-value" :class="statusClass">{{ statusText }}</div>
      </div>

      <!-- Impact Score -->
      <div class="metric-card">
        <div class="metric-label">Impact Score</div>
        <div class="metric-value highlight">{{ reportData.impactScore || 0 }}</div>
        <div class="metric-sub">/ 100</div>
      </div>

      <!-- Total Reach -->
      <div class="metric-card">
        <div class="metric-label">Est. Reach</div>
        <div class="metric-value">{{ formatNumber(reportData.totalReach) }}</div>
      </div>

      <!-- Sentiment -->
      <div class="metric-card">
        <div class="metric-label">Avg Sentiment</div>
        <div class="metric-value" :class="getSentimentClass(reportData.avgSentiment)">
          {{ formatSentiment(reportData.avgSentiment) }}
        </div>
      </div>

       <!-- Engagement -->
      <div class="metric-card">
        <div class="metric-label">Engagement</div>
        <div class="metric-value">{{ formatNumber(reportData.totalEngagement) }}</div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';

const props = defineProps<{
  reportData: any; // { status, impactScore, totalReach, avgSentiment, totalEngagement }
}>();

const statusText = computed(() => {
  if (props.reportData.status === 'completed') return 'MISSION COMPLETE';
  if (props.reportData.status === 'running') return 'IN PROGRESS';
  return 'PENDING';
});

const statusClass = computed(() => {
  if (props.reportData.status === 'completed') return 'text-success';
  if (props.reportData.status === 'running') return 'text-warning';
  return 'text-muted';
});

const formatNumber = (num: number) => {
  if (!num) return '0';
  if (num >= 1000000) return (num / 1000000).toFixed(1) + 'M';
  if (num >= 1000) return (num / 1000).toFixed(1) + 'K';
  return num.toString();
};

const formatSentiment = (val: number) => {
  if (val === undefined || val === null) return '-';
  const pct = Math.round(val * 100);
  return `${pct > 0 ? '+' : ''}${pct}%`;
};

const getSentimentClass = (val: number) => {
  if (val > 0.2) return 'text-success';
  if (val < -0.2) return 'text-danger';
  return 'text-warning'; // neutral
};
</script>

<style scoped>
.executive-summary {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.summary-header {
  display: flex;
  align-items: center;
  gap: 12px;
}

.header-icon {
  font-size: 24px;
}

.header-title {
  margin: 0;
  font-size: 18px;
  font-weight: 700;
  color: var(--text-main);
  text-transform: uppercase;
  letter-spacing: 1px;
}

.metrics-grid {
  display: grid;
  grid-template-columns: 2fr 1fr 1fr 1fr 1fr;
  gap: 16px;
}

.metric-card {
  background: rgba(255, 255, 255, 0.03);
  border-radius: 8px;
  padding: 16px;
  display: flex;
  flex-direction: column;
  justify-content: center;
  border: 1px solid rgba(255, 255, 255, 0.05);
}

.metric-label {
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--text-muted);
  margin-bottom: 6px;
}

.metric-value {
  font-size: 24px;
  font-weight: 700;
  color: var(--text-main);
  font-family: 'JetBrains Mono', monospace;
}

.metric-sub {
  font-size: 12px;
  color: var(--text-muted);
  margin-top: 2px;
}

.main-metric .metric-value {
  font-size: 20px;
}

.highlight {
  color: var(--primary);
  text-shadow: 0 0 20px rgba(255, 87, 34, 0.3);
}

.text-success { color: #4ade80; }
.text-warning { color: #facc15; }
.text-danger { color: #f87171; }
.text-muted { color: var(--text-muted); }

@media (max-width: 1024px) {
  .metrics-grid {
    grid-template-columns: 1fr 1fr 1fr;
  }
  .main-metric {
    grid-column: 1 / -1;
  }
}
</style>
