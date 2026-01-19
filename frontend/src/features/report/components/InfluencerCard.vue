<template>
  <div class="influencer-card glass-card">
    <div class="card-header">
      <div class="rank-badge" v-if="rank">#{{ rank }}</div>
      <div class="avatar-wrapper">
        <div class="avatar">{{ influencer.name.charAt(0) }}</div>
        <div class="platform-indicators">
          <div v-if="influencer.onTwitter" class="platform-dot twitter" title="Active on Twitter"></div>
          <div v-if="influencer.onReddit" class="platform-dot reddit" title="Active on Reddit"></div>
        </div>
      </div>
      <div class="info-wrapper">
        <div class="name">{{ influencer.name }}</div>
        <div class="handle">@{{ influencer.handle || influencer.name.toLowerCase().replace(/\s/g, '_') }}</div>
      </div>
    </div>

    <div class="metrics-row">
      <div class="metric">
        <div class="label">Reach</div>
        <div class="value">{{ formatNumber(influencer.reach) }}</div>
      </div>
      <div class="metric">
        <div class="label">Actions</div>
        <div class="value">{{ influencer.actionCount }}</div>
      </div>
      <div class="metric">
        <div class="label">Sentiment</div>
        <div class="value" :class="getSentimentClass(influencer.sentiment)">
          {{ formatSentiment(influencer.sentiment) }}
        </div>
      </div>
    </div>

    <!-- Mini chart or recent action snippet could go here -->
    <div v-if="influencer.topQuote" class="quote-snippet">
      "{{ influencer.topQuote }}"
    </div>
  </div>
</template>

<script setup lang="ts">
const props = defineProps<{
  influencer: any;
  rank?: number;
}>();

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
  return 'text-warning';
};
</script>

<style scoped>
.influencer-card {
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 16px;
}

.card-header {
  display: flex;
  align-items: center;
  gap: 12px;
  position: relative;
}

.rank-badge {
  position: absolute;
  top: -8px;
  left: -8px;
  background: var(--bg-base);
  border: 1px solid var(--glass-border);
  color: var(--text-muted);
  font-size: 10px;
  padding: 2px 6px;
  border-radius: 4px;
}

.avatar-wrapper {
  position: relative;
}

.avatar {
  width: 48px;
  height: 48px;
  border-radius: 50%;
  background: linear-gradient(135deg, var(--primary), #a78bfa);
  color: white;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
  font-size: 20px;
  text-shadow: 0 2px 4px rgba(0,0,0,0.2);
}

.platform-indicators {
  position: absolute;
  bottom: -2px;
  right: -2px;
  display: flex;
  gap: 2px;
}

.platform-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  border: 2px solid var(--bg-surface);
}

.twitter { background: #1DA1F2; }
.reddit { background: #FF4500; }

.info-wrapper {
  display: flex;
  flex-direction: column;
}

.name {
  font-weight: 600;
  font-size: 15px;
  color: var(--text-main);
}

.handle {
  font-size: 12px;
  color: var(--text-muted);
}

.metrics-row {
  display: flex;
  justify-content: space-between;
  padding-top: 12px;
  border-top: 1px solid rgba(255,255,255,0.05);
}

.metric {
  display: flex;
  flex-direction: column;
  align-items: center;
}

.label {
  font-size: 10px;
  text-transform: uppercase;
  color: var(--text-muted);
}

.value {
  font-size: 14px;
  font-weight: 600;
  font-family: 'JetBrains Mono', monospace;
  color: var(--text-main);
}

.quote-snippet {
  font-size: 12px;
  font-style: italic;
  color: var(--text-secondary);
  background: rgba(255,255,255,0.03);
  padding: 8px;
  border-radius: 4px;
  border-left: 2px solid var(--primary);
}

.text-success { color: #4ade80; }
.text-warning { color: #facc15; }
.text-danger { color: #f87171; }
</style>
