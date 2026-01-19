<template>
  <div class="panorama-display">
    <!-- Header -->
    <div class="panorama-header">
      <div class="header-main">
        <div class="header-title">Panorama Search</div>
        <div class="header-stats">
          <span class="stat-item">
            <span class="stat-value">{{ result.stats.nodes }}</span>
            <span class="stat-label">Nodes</span>
          </span>
          <span class="stat-divider">/</span>
          <span class="stat-item">
            <span class="stat-value">{{ result.stats.edges }}</span>
            <span class="stat-label">Edges</span>
          </span>
          <template v-if="resultLength">
            <span class="stat-divider">·</span>
            <span class="stat-size">{{ formatSize(resultLength) }}</span>
          </template>
        </div>
      </div>
      <div v-if="result.query" class="header-topic">{{ result.query }}</div>
    </div>

    <!-- Tab Navigation -->
    <div class="panorama-tabs">
      <button 
        class="panorama-tab" 
        :class="{ active: activeTab === 'active' }"
        @click="activeTab = 'active'"
      >
        <span class="tab-label">Current Valid Memory ({{ result.activeFacts.length }})</span>
      </button>
      <button 
        class="panorama-tab" 
        :class="{ active: activeTab === 'historical' }"
        @click="activeTab = 'historical'"
      >
        <span class="tab-label">History Memory ({{ result.historicalFacts.length }})</span>
      </button>
      <button 
        class="panorama-tab" 
        :class="{ active: activeTab === 'entities' }"
        @click="activeTab = 'entities'"
      >
        <span class="tab-label">Entities Involved ({{ result.entities.length }})</span>
      </button>
    </div>

    <!-- Tab Content -->
    <div class="panorama-content">
      <!-- Active Facts -->
      <div v-if="activeTab === 'active'" class="facts-panel active-facts">
        <div class="panel-header">
          <span class="panel-title">Current Valid Memory</span>
          <span class="panel-count">Total {{ result.activeFacts.length }} items</span>
        </div>
        
        <div v-if="result.activeFacts.length > 0" class="facts-list">
          <div 
            v-for="(fact, i) in displayedActive" 
            :key="i"
            class="fact-item active"
          >
            <span class="fact-number">{{ i + 1 }}</span>
            <div class="fact-content">{{ fact }}</div>
          </div>
        </div>
        <div v-else class="empty-state">No Current Valid Memory</div>
        
        <button 
          v-if="result.activeFacts.length > INITIAL_SHOW_COUNT"
          class="expand-btn"
          @click="expandedActive = !expandedActive"
        >
          {{ expandedActive ? 'Collapse ▲' : `Expand All ${result.activeFacts.length} items ▼` }}
        </button>
      </div>

      <!-- Historical Facts -->
      <div v-if="activeTab === 'historical'" class="facts-panel historical-facts">
        <div class="panel-header">
          <span class="panel-title">History Memory</span>
          <span class="panel-count">Total {{ result.historicalFacts.length }} items</span>
        </div>

        <div v-if="result.historicalFacts.length > 0" class="facts-list">
          <div 
            v-for="(fact, i) in displayedHistorical" 
            :key="i"
            class="fact-item historical"
          >
            <span class="fact-number">{{ i + 1 }}</span>
            <div class="fact-content">
              <template v-if="getTimeMatch(fact)">
                <span class="fact-time">{{ getTimeMatch(fact)[1] }}</span>
                <span class="fact-text">{{ getTimeMatch(fact)[2] }}</span>
              </template>
              <template v-else>
                <span class="fact-text">{{ fact }}</span>
              </template>
            </div>
          </div>
        </div>
        <div v-else class="empty-state">No History Memory</div>

        <button 
          v-if="result.historicalFacts.length > INITIAL_SHOW_COUNT"
          class="expand-btn"
          @click="expandedHistorical = !expandedHistorical"
        >
          {{ expandedHistorical ? 'Collapse ▲' : `Expand All ${result.historicalFacts.length} items ▼` }}
        </button>
      </div>

      <!-- Entities -->
      <div v-if="activeTab === 'entities'" class="entities-panel">
        <div class="panel-header">
          <span class="panel-title">Entities Involved</span>
          <span class="panel-count">Total {{ result.entities.length }} units</span>
        </div>

        <div v-if="result.entities.length > 0" class="entities-grid">
           <div 
            v-for="(entity, i) in displayedEntities" 
            :key="i"
            class="entity-tag"
          >
            <span class="entity-name">{{ entity.name }}</span>
            <span v-if="entity.type" class="entity-type">{{ entity.type }}</span>
          </div>
        </div>
        <div v-else class="empty-state">No Entities Involved</div>

        <button 
          v-if="result.entities.length > 8"
          class="expand-btn"
          @click="expandedEntities = !expandedEntities"
        >
           {{ expandedEntities ? 'Collapse ▲' : `Expand All ${result.entities.length} units ▼` }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue';

const props = defineProps<{
  // Using 'any' for result as the structure is complex and validated in parsers
  result: any; 
  resultLength?: number;
}>();

const activeTab = ref('active');
const expandedActive = ref(false);
const expandedHistorical = ref(false);
const expandedEntities = ref(false);
const INITIAL_SHOW_COUNT = 5;

const displayedActive = computed(() => {
  return expandedActive.value ? props.result.activeFacts : props.result.activeFacts.slice(0, INITIAL_SHOW_COUNT);
});

const displayedHistorical = computed(() => {
  return expandedHistorical.value ? props.result.historicalFacts : props.result.historicalFacts.slice(0, INITIAL_SHOW_COUNT);
});

const displayedEntities = computed(() => {
  return expandedEntities.value ? props.result.entities : props.result.entities.slice(0, 8);
});

const formatSize = (length: number) => {
  if (!length) return '';
  if (length >= 1000) return `${(length / 1000).toFixed(1)}k chars`;
  return `${length} chars`;
};

const getTimeMatch = (fact: string) => {
  return fact.match(/^\[(.+?)\]\s*(.*)$/);
};
</script>

<style scoped>
.panorama-display {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
/* ... additional styles ... */
</style>
