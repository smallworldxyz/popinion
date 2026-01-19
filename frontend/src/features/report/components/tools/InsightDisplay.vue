<template>
  <div class="insight-display">
    <!-- Header -->
    <div class="insight-header">
      <div class="header-main">
        <div class="header-title">Deep Insight</div>
        <div class="header-stats">
          <span class="stat-item">
            <span class="stat-value">{{ result.stats.facts || result.facts.length }}</span>
            <span class="stat-label">Facts</span>
          </span>
          <span class="stat-divider">/</span>
          <span class="stat-item">
            <span class="stat-value">{{ result.stats.entities || result.entities.length }}</span>
            <span class="stat-label">Entities</span>
          </span>
          <span class="stat-divider">/</span>
          <span class="stat-item">
            <span class="stat-value">{{ result.stats.relationships || result.relations.length }}</span>
            <span class="stat-label">Relations</span>
          </span>
          <template v-if="resultLength">
            <span class="stat-divider">·</span>
            <span class="stat-size">{{ formatSize(resultLength) }}</span>
          </template>
        </div>
      </div>
      <div v-if="result.query" class="header-topic">{{ result.query }}</div>
      <div v-if="result.simulationRequirement" class="header-scenario">
        <span class="scenario-label">Prediction Scenario: </span>
        <span class="scenario-text">{{ result.simulationRequirement }}</span>
      </div>
    </div>

    <!-- Tab Navigation -->
    <div class="insight-tabs">
      <button 
        class="insight-tab" 
        :class="{ active: activeTab === 'facts' }"
        @click="activeTab = 'facts'"
      >
        <span class="tab-label">Current Key Memory ({{ result.facts.length }})</span>
      </button>
      <button 
        class="insight-tab" 
        :class="{ active: activeTab === 'entities' }"
        @click="activeTab = 'entities'"
      >
        <span class="tab-label">Core Entity ({{ result.entities.length }})</span>
      </button>
      <button 
        class="insight-tab" 
        :class="{ active: activeTab === 'relations' }"
        @click="activeTab = 'relations'"
      >
        <span class="tab-label">Relations Chain ({{ result.relations.length }})</span>
      </button>
      <button 
        v-if="result.subQueries.length > 0"
        class="insight-tab" 
        :class="{ active: activeTab === 'subqueries' }"
        @click="activeTab = 'subqueries'"
      >
        <span class="tab-label">Sub Problem ({{ result.subQueries.length }})</span>
      </button>
    </div>

    <!-- Tab Content -->
    <div class="insight-content">
      <!-- Facts Tab -->
      <div v-if="activeTab === 'facts' && result.facts.length > 0" class="facts-panel">
        <div class="panel-header">
          <span class="panel-title">Latest Key Facts Associated in Temporal Memory</span>
          <span class="panel-count">Total {{ result.facts.length }} items</span>
        </div>
        <div class="facts-list">
          <div 
            v-for="(fact, i) in displayedFacts" 
            :key="i"
            class="fact-item"
          >
            <span class="fact-number">{{ i + 1 }}</span>
            <div class="fact-content">{{ fact }}</div>
          </div>
        </div>
        <button 
          v-if="result.facts.length > INITIAL_SHOW_COUNT"
          class="expand-btn"
          @click="expandedFacts = !expandedFacts"
        >
          {{ expandedFacts ? 'Collapse ▲' : `Expand All ${result.facts.length} items ▼` }}
        </button>
      </div>

      <!-- Entities Tab -->
      <div v-if="activeTab === 'entities' && result.entities.length > 0" class="entities-panel">
        <div class="panel-header">
          <span class="panel-title">Core Entity</span>
          <span class="panel-count">Total {{ result.entities.length }} units</span>
        </div>
        <div class="entities-grid">
          <div 
            v-for="(entity, i) in displayedEntities" 
            :key="i"
            class="entity-tag"
            :title="entity.summary || ''"
          >
            <span class="entity-name">{{ entity.name }}</span>
            <span class="entity-type">{{ entity.type }}</span>
            <span v-if="entity.relatedFactsCount > 0" class="entity-fact-count">{{ entity.relatedFactsCount }} items</span>
          </div>
        </div>
        <button 
          v-if="result.entities.length > 12"
          class="expand-btn"
          @click="expandedEntities = !expandedEntities"
        >
          {{ expandedEntities ? 'Collapse ▲' : `Expand All ${result.entities.length} units ▼` }}
        </button>
      </div>

      <!-- Relations Tab -->
      <div v-if="activeTab === 'relations' && result.relations.length > 0" class="relations-panel">
        <div class="panel-header">
          <span class="panel-title">Relations Chain</span>
          <span class="panel-count">Total {{ result.relations.length }} items</span>
        </div>
        <div class="relations-list">
          <div 
            v-for="(rel, i) in displayedRelations" 
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
        <button 
          v-if="result.relations.length > INITIAL_SHOW_COUNT"
          class="expand-btn"
          @click="expandedRelations = !expandedRelations"
        >
          {{ expandedRelations ? 'Collapse ▲' : `Expand All ${result.relations.length} items ▼` }}
        </button>
      </div>

      <!-- SubQueries Tab -->
      <div v-if="activeTab === 'subqueries' && result.subQueries.length > 0" class="subqueries-panel">
        <div class="panel-header">
          <span class="panel-title">Drift Query Generating Analyze Sub Problem</span>
          <span class="panel-count">Total {{ result.subQueries.length }} units</span>
        </div>
        <div class="subqueries-list">
          <div 
            v-for="(sq, i) in result.subQueries" 
            :key="i"
            class="subquery-item"
          >
            <span class="subquery-number">Q{{ i + 1 }}</span>
            <div class="subquery-text">{{ sq }}</div>
          </div>
        </div>
      </div>

      <!-- Empty States -->
      <div v-if="activeTab === 'facts' && result.facts.length === 0" class="empty-state">No Current Key Memory</div>
      <div v-if="activeTab === 'entities' && result.entities.length === 0" class="empty-state">No Core Entity</div>
      <div v-if="activeTab === 'relations' && result.relations.length === 0" class="empty-state">No Relations Chain</div>
      
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue';

const props = defineProps<{
  result: any;
  resultLength?: number;
}>();

const activeTab = ref('facts');
const expandedFacts = ref(false);
const expandedEntities = ref(false);
const expandedRelations = ref(false);
const INITIAL_SHOW_COUNT = 5;

const displayedFacts = computed(() => {
  return expandedFacts.value ? props.result.facts : props.result.facts.slice(0, INITIAL_SHOW_COUNT);
});

const displayedEntities = computed(() => {
  return expandedEntities.value ? props.result.entities : props.result.entities.slice(0, 12);
});

const displayedRelations = computed(() => {
  return expandedRelations.value ? props.result.relations : props.result.relations.slice(0, INITIAL_SHOW_COUNT);
});

const formatSize = (length: number) => {
  if (!length) return '';
  if (length >= 1000) {
    return `${(length / 1000).toFixed(1)}k chars`;
  }
  return `${length} chars`;
};
</script>

<style scoped>
/* Scoped styles will be needed, borrowing from original logic or defining new ones */
/* Since original styles were likely global or in parent, I should define basic structure here */
.insight-display {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
/* ... styles ... */
</style>
