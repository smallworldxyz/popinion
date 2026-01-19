<template>
  <div class="quick-search-display">
    <!-- Header -->
    <div class="quicksearch-header">
      <div class="header-main">
        <div class="header-title">Quick Search</div>
        <div class="header-stats">
          <span class="stat-item">
            <span class="stat-value">{{ result.count || result.facts.length }}</span>
            <span class="stat-label">Results</span>
          </span>
          <template v-if="resultLength">
            <span class="stat-divider">·</span>
            <span class="stat-size">{{ formatSize(resultLength) }}</span>
          </template>
        </div>
      </div>
      <div v-if="result.query" class="header-query">
        <span class="query-label">Search: </span>
        <span class="query-text">{{ result.query }}</span>
      </div>
    </div>

    <!-- Tab Navigation -->
    <div v-if="showTabs" class="quicksearch-tabs">
      <button 
        class="quicksearch-tab" 
        :class="{ active: activeTab === 'facts' }"
        @click="activeTab = 'facts'"
      >
        <span class="tab-label">Facts ({{ result.facts.length }})</span>
      </button>
      <button 
        v-if="hasEdges"
        class="quicksearch-tab" 
        :class="{ active: activeTab === 'edges' }"
        @click="activeTab = 'edges'"
      >
        <span class="tab-label">Relations ({{ result.edges.length }})</span>
      </button>
      <button 
        v-if="hasNodes"
        class="quicksearch-tab" 
        :class="{ active: activeTab === 'nodes' }"
        @click="activeTab = 'nodes'"
      >
        <span class="tab-label">Nodes ({{ result.nodes.length }})</span>
      </button>
    </div>

    <!-- Tab Content -->
    <div class="quicksearch-content">
      <!-- Facts -->
      <div v-if="activeTab === 'facts'" class="facts-panel">
        <div class="panel-header">
          <span class="panel-title">Related Facts</span>
          <span class="panel-count">Total {{ result.facts.length }} items</span>
        </div>
        <div v-if="result.facts.length > 0" class="facts-list">
          <div 
            v-for="(fact, i) in displayedFacts" 
            :key="i"
            class="fact-item"
          >
            <span class="fact-number">{{ i + 1 }}</span>
            <div class="fact-content">{{ fact }}</div>
          </div>
        </div>
        <div v-else class="empty-state">No related facts</div>
        
        <button 
          v-if="result.facts.length > INITIAL_SHOW_COUNT"
          class="expand-btn"
          @click="expandedFacts = !expandedFacts"
        >
          {{ expandedFacts ? 'Collapse ▲' : `Expand All ${result.facts.length} items ▼` }}
        </button>
      </div>

      <!-- Edges -->
      <div v-if="activeTab === 'edges' && hasEdges" class="edges-panel">
        <div class="panel-header">
           <span class="panel-title">Related Relations</span>
           <span class="panel-count">Total {{ result.edges.length }} items</span>
        </div>
        <div class="relations-list">
           <div 
            v-for="(rel, i) in result.edges" 
            :key="i"
            class="relation-item"
          >
            <span class="rel-source">{{ rel.source }}</span>
            <span class="rel-arrow">
              <span class="rel-line"></span>
              <span class="rel-label">{{ rel.relation }}</span>
              <span class="rel-line"></span>
            </span>
            <span class="rel-target">{{ rel.target }}</span>
          </div>
        </div>
      </div>

      <!-- Nodes -->
      <div v-if="activeTab === 'nodes' && hasNodes" class="nodes-panel">
        <div class="panel-header">
           <span class="panel-title">Related Nodes</span>
           <span class="panel-count">Total {{ result.nodes.length }} units</span>
        </div>
        <div class="entities-grid">
           <div 
            v-for="(node, i) in result.nodes" 
            :key="i"
            class="entity-tag"
          >
            <span class="entity-name">{{ node.name }}</span>
            <span v-if="node.type" class="entity-type">{{ node.type }}</span>
          </div>
        </div>
      </div>

    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue';

const props = defineProps<{
  // Using 'any' as parsed structure is complex
  result: any;
  resultLength?: number; 
}>();

const activeTab = ref('facts');
const expandedFacts = ref(false);
const INITIAL_SHOW_COUNT = 5;

const hasEdges = computed(() => props.result.edges && props.result.edges.length > 0);
const hasNodes = computed(() => props.result.nodes && props.result.nodes.length > 0);
const showTabs = computed(() => hasEdges.value || hasNodes.value);

const displayedFacts = computed(() => {
  return expandedFacts.value ? props.result.facts : props.result.facts.slice(0, INITIAL_SHOW_COUNT);
});

const formatSize = (length: number) => {
  if (!length) return '';
  if (length >= 1000) return `${(length / 1000).toFixed(1)}k chars`;
  return `${length} chars`;
};
</script>

<style scoped>
.quick-search-display {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
/* ... styles ... */
</style>
