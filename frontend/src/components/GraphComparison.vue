<template>
  <div class="graph-comparison">
    <div class="comparison-header">
      <h3>Merge Analysis</h3>
      <div class="stats" v-if="previewData">
        <div class="stat-item">
          <label>Source Entities</label>
          <span>{{ previewData.total_source_entities }}</span>
        </div>
        <div class="stat-item">
          <label>Target Entities</label>
          <span>{{ previewData.total_target_entities }}</span>
        </div>
        <div class="stat-item highlight">
          <label>Overlap</label>
          <span>{{ previewData.overlap_percentage.toFixed(1) }}%</span>
          <small>({{ previewData.overlapping_entities.length }} entities)</small>
        </div>
      </div>
    </div>

    <div class="overlaps-list" v-if="previewData">
      <div v-if="previewData.overlapping_entities.length === 0" class="empty-state">
        No overlaps detected. Graphs appear distinct.
      </div>
      
      <div 
        v-for="(item, index) in previewData.overlapping_entities" 
        :key="index"
        class="overlap-item"
      >
        <div class="entity-name">{{ item.entity_name }}</div>
        <div class="match-info">
          <span class="match-type">{{ item.match_type }}</span>
          <span class="confidence">Confidence: {{ (item.confidence * 100).toFixed(0) }}%</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
defineProps({
  previewData: {
    type: Object,
    required: true
  }
})
</script>

<style scoped>
.graph-comparison {
  display: flex;
  flex-direction: column;
  gap: 20px;
  height: 100%;
}

.comparison-header {
  border-bottom: 1px solid #eee;
  padding-bottom: 20px;
}

.stats {
  display: flex;
  gap: 20px;
  margin-top: 15px;
}

.stat-item {
  display: flex;
  flex-direction: column;
  gap: 5px;
  background: #f8f9fa;
  padding: 10px 15px;
  border-radius: 8px;
  min-width: 120px;
}

.stat-item.highlight {
  background: #e3f2fd;
  color: #1976d2;
}

.stat-item label {
  font-size: 0.8rem;
  color: #666;
  text-transform: uppercase;
  font-weight: 600;
}

.stat-item span {
  font-size: 1.5rem;
  font-weight: 700;
  font-family: 'JetBrains Mono', monospace;
}

.stat-item small {
  font-size: 11px;
  opacity: 0.8;
}

.overlaps-list {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding-right: 5px;
}

.overlap-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px;
  background: #fff;
  border: 1px solid #eee;
  border-radius: 6px;
  transition: all 0.2s;
}

.overlap-item:hover {
  border-color: #2196f3;
  background: #f5f9ff;
}

.entity-name {
  font-weight: 600;
  font-size: 14px;
}

.match-info {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  font-size: 12px;
}

.match-type {
  background: #e8f5e9;
  color: #2e7d32;
  padding: 2px 6px;
  border-radius: 4px;
  margin-bottom: 2px;
}

.confidence {
  color: #666;
}

.empty-state {
  text-align: center;
  padding: 40px;
  color: #999;
  font-style: italic;
  background: #f9f9f9;
  border-radius: 8px;
}
</style>
