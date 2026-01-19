<template>
  <Transition name="modal">
    <div v-if="show" class="entity-modal-overlay" @click.self="$emit('close')">
      <div class="entity-modal">
        <!-- Header -->
        <div class="modal-header">
          <div class="header-info">
            <h2 class="modal-title">{{ title }}</h2>
            <span class="modal-subtitle">{{ selectedCount }} of {{ filteredEntities.length }} {{ itemLabel }} selected</span>
          </div>
          <button class="close-btn" @click="$emit('close')">×</button>
        </div>

        <!-- Search and Actions Bar -->
        <div class="actions-bar">
          <div class="search-box">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="11" cy="11" r="8"></circle>
              <path d="m21 21-4.35-4.35"></path>
            </svg>
            <input 
              type="text" 
              v-model="searchQuery" 
              placeholder="Search entities..."
              class="search-input"
            />
          </div>
          <div class="filter-container">
            <button class="action-btn small" @click="showFilter = !showFilter">Filter</button>
            <div v-if="showFilter" class="filter-dropdown">
              <label for="relationship-slider">Min Relationships: {{ relationshipCountFilter }}</label>
              <input
                type="range"
                id="relationship-slider"
                min="0"
                :max="maxRelationships"
                v-model="relationshipCountFilter"
              />
            </div>
          </div>
          <div class="bulk-actions">
            <button class="action-btn small" @click="selectAll">Select All (Filtered)</button>
            <button class="action-btn small secondary" @click="deselectAll">Deselect All</button>
          </div>
        </div>

        <!-- Entity Groups by Type -->
        <div class="entity-groups">
          <div 
            v-for="(group, typeName) in groupedEntities" 
            :key="typeName" 
            class="entity-group"
          >
            <!-- Group Header -->
            <div class="group-header" @click="toggleGroup(typeName)">
              <div class="group-info">
                <span class="expand-icon">{{ expandedGroups[typeName] ? '▼' : '▶' }}</span>
                <span class="group-name">{{ typeName }}</span>
                <span class="group-count">({{ group.length }})</span>
                <span class="selected-count">{{ getGroupSelectedCount(typeName) }} selected</span>
              </div>
              <div class="group-actions" @click.stop>
                <button class="mini-btn" @click="selectGroup(typeName)">All</button>
                <button class="mini-btn" @click="deselectGroup(typeName)">None</button>
              </div>
            </div>

            <!-- Group Content -->
            <Transition name="expand">
              <div v-if="expandedGroups[typeName]" class="group-content">
                <div 
                  v-for="entity in group"
                  :key="entity.uuid"
                  class="entity-item"
                  :class="{ selected: selectedIds.has(entity.uuid) }"
                  @click="toggleEntity(entity.uuid)"
                >
                  <input 
                    type="checkbox" 
                    :checked="selectedIds.has(entity.uuid)"
                    class="entity-checkbox"
                    @click.stop
                    @change="toggleEntity(entity.uuid)"
                  />
                  <div class="entity-info">
                    <span class="entity-name">{{ entity.name }}</span>
                    <span class="entity-summary">{{ truncateSummary(entity.summary) }}</span>
                  </div>
                  <span class="relationship-count" :title="`${entity.relationship_count} relationships`">
                    {{ entity.relationship_count }} rels
                  </span>
                </div>
              </div>
            </Transition>
          </div>
        </div>

        <!-- Footer -->
        <div class="modal-footer">
          <div v-if="showEstimate" class="selection-summary">
            <span class="summary-text">
              <strong>{{ selectedCount }}</strong> agents will be created
            </span>
            <span class="estimate-text">
              ~{{ selectedCount }} LLM calls for profile generation
            </span>
          </div>
          <div v-else class="selection-summary">
            <span class="summary-text">
              <strong>{{ selectedCount }}</strong> {{ itemLabel }} selected
            </span>
          </div>
          <div class="footer-actions">
            <button class="action-btn secondary" @click="$emit('close')">Cancel</button>
            <button 
              class="action-btn primary" 
              @click="confirmSelection"
              :disabled="selectedCount === 0"
            >
              Confirm Selection ({{ selectedCount }})
            </button>
          </div>
        </div>
      </div>
    </div>
  </Transition>
</template>

<script setup>
import { ref, computed, watch } from 'vue'

const props = defineProps({
  show: Boolean,
  entities: {
    type: Array,
    default: () => []
  },
  byType: {
    type: Object,
    default: () => ({})
  },
  title: {
    type: String,
    default: 'Select Agents for Simulation'
  },
  itemLabel: {
    type: String,
    default: 'entities'
  },
  showEstimate: {
    type: Boolean,
    default: true
  },
  selectAllByDefault: {
    type: Boolean,
    default: true
  }
})

const emit = defineEmits(['close', 'confirm'])

// State
const searchQuery = ref('')
const selectedIds = ref(new Set())
const expandedGroups = ref({})
const showFilter = ref(false)
const relationshipCountFilter = ref(0)

// Find the max relationship count to set the slider's max value
const maxRelationships = computed(() => {
  if (props.entities.length === 0) return 100;
  return Math.max(...props.entities.map(e => e.relationship_count || 0));
});

const filteredEntities = computed(() => {
  return props.entities.filter(entity => {
    const passesRelationshipFilter = (entity.relationship_count || 0) >= relationshipCountFilter.value;
    
    if (!searchQuery.value) {
      return passesRelationshipFilter;
    }
    
    const query = searchQuery.value.toLowerCase();
    const passesSearchFilter = entity.name.toLowerCase().includes(query) || (entity.summary && entity.summary.toLowerCase().includes(query));
    
    return passesRelationshipFilter && passesSearchFilter;
  });
});


// Initialize: optionally select all entities and expand first few groups
watch(() => props.entities, (newEntities) => {
  if (newEntities.length > 0) {
    // Select all by default only if prop is true
    if (props.selectAllByDefault) {
      selectedIds.value = new Set(newEntities.map(e => e.uuid))
    }
  } else {
    selectedIds.value = new Set()  // Start with none selected
  }
  
  // Expand first 3 groups by default
  const types = Object.keys(props.byType)
  types.slice(0, 3).forEach(type => {
    expandedGroups.value[type] = true
  })
}, { immediate: true })

// Computed
const selectedCount = computed(() => selectedIds.value.size)

const groupedEntities = computed(() => {
  const groups = {}
  for (const entity of filteredEntities.value) {
    const type = entity.type || 'Unknown'
    if (!groups[type]) groups[type] = []
    groups[type].push(entity)
  }
  // Sort groups by count descending
  return Object.fromEntries(
    Object.entries(groups).sort((a, b) => b[1].length - a[1].length)
  )
})

// Methods
const toggleGroup = (typeName) => {
  expandedGroups.value[typeName] = !expandedGroups.value[typeName]
}

const toggleEntity = (uuid) => {
  if (selectedIds.value.has(uuid)) {
    selectedIds.value.delete(uuid)
  } else {
    selectedIds.value.add(uuid)
  }
  // Trigger reactivity
  selectedIds.value = new Set(selectedIds.value)
}

const selectAll = () => {
  selectedIds.value = new Set(filteredEntities.value.map(e => e.uuid))
}

const deselectAll = () => {
  selectedIds.value = new Set()
}

const selectGroup = (typeName) => {
  const group = groupedEntities.value[typeName] || []
  group.forEach(e => selectedIds.value.add(e.uuid))
  selectedIds.value = new Set(selectedIds.value)
}

const deselectGroup = (typeName) => {
  const group = groupedEntities.value[typeName] || []
  group.forEach(e => selectedIds.value.delete(e.uuid))
  selectedIds.value = new Set(selectedIds.value)
}

const getGroupSelectedCount = (typeName) => {
  const group = groupedEntities.value[typeName] || []
  return group.filter(e => selectedIds.value.has(e.uuid)).length
}

const filteredGroupEntities = (group) => {
  if (!searchQuery.value) return group
  const query = searchQuery.value.toLowerCase()
  return group.filter(e => 
    e.name.toLowerCase().includes(query) || 
    (e.summary && e.summary.toLowerCase().includes(query))
  )
}

const truncateSummary = (summary) => {
  if (!summary) return ''
  return summary.length > 60 ? summary.slice(0, 60) + '...' : summary
}

const confirmSelection = () => {
  emit('confirm', Array.from(selectedIds.value))
}
</script>

<style scoped>
.entity-modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.7);
  /* backdrop-filter removed */
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.entity-modal {
  background: #1a1a2e;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 16px;
  width: 90%;
  max-width: 800px;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 20px 24px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
}

.header-info {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.modal-title {
  font-size: 18px;
  font-weight: 600;
  color: #fff;
  margin: 0;
}

.modal-subtitle {
  font-size: 13px;
  color: rgba(255, 255, 255, 0.5);
}

.close-btn {
  width: 32px;
  height: 32px;
  border: none;
  background: rgba(255, 255, 255, 0.1);
  border-radius: 8px;
  color: rgba(255, 255, 255, 0.6);
  font-size: 20px;
  cursor: pointer;
  transition: all 0.2s;
}

.close-btn:hover {
  background: rgba(255, 255, 255, 0.2);
  color: #fff;
}

.actions-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px 24px;
  gap: 16px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.05);
}

.search-box {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
  max-width: 300px;
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 8px;
  padding: 8px 12px;
}

.search-box svg {
  color: rgba(255, 255, 255, 0.4);
  flex-shrink: 0;
}

.search-input {
  flex: 1;
  background: none;
  border: none;
  color: #fff;
  font-size: 14px;
  outline: none;
}

.search-input::placeholder {
  color: rgba(255, 255, 255, 0.3);
}

.filter-container {
  position: relative;
}

.filter-dropdown {
  position: absolute;
  top: 100%;
  right: 0;
  background: #2c2c54;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 8px;
  padding: 16px;
  margin-top: 8px;
  z-index: 10;
  color: white;
  width: 250px;
}

.filter-dropdown label {
  display: block;
  margin-bottom: 8px;
  font-size: 12px;
}

.filter-dropdown input[type="range"] {
  width: 100%;
}

.bulk-actions {
  display: flex;
  gap: 8px;
}

.action-btn {
  padding: 8px 16px;
  border: none;
  border-radius: 8px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
}

.action-btn.small {
  padding: 6px 12px;
  font-size: 12px;
}

.action-btn.primary {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: #fff;
}

.action-btn.primary:hover {
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(102, 126, 234, 0.4);
}

.action-btn.primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
  transform: none;
}

.action-btn.secondary {
  background: rgba(255, 255, 255, 0.1);
  color: rgba(255, 255, 255, 0.7);
}

.action-btn.secondary:hover {
  background: rgba(255, 255, 255, 0.15);
  color: #fff;
}

.entity-groups {
  flex: 1;
  overflow-y: auto;
  padding: 16px 24px;
}

.entity-group {
  margin-bottom: 8px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 10px;
  overflow: hidden;
}

.group-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
  background: rgba(255, 255, 255, 0.03);
  cursor: pointer;
  transition: background 0.2s;
}

.group-header:hover {
  background: rgba(255, 255, 255, 0.06);
}

.group-info {
  display: flex;
  align-items: center;
  gap: 8px;
}

.expand-icon {
  font-size: 10px;
  color: rgba(255, 255, 255, 0.4);
  width: 12px;
}

.group-name {
  font-weight: 600;
  color: #fff;
}

.group-count {
  color: rgba(255, 255, 255, 0.4);
  font-size: 13px;
}

.selected-count {
  font-size: 12px;
  color: #667eea;
  margin-left: 8px;
}

.group-actions {
  display: flex;
  gap: 4px;
}

.mini-btn {
  padding: 4px 8px;
  font-size: 11px;
  border: 1px solid rgba(255, 255, 255, 0.15);
  background: transparent;
  color: rgba(255, 255, 255, 0.6);
  border-radius: 4px;
  cursor: pointer;
  transition: all 0.2s;
}

.mini-btn:hover {
  background: rgba(255, 255, 255, 0.1);
  color: #fff;
}

.group-content {
  border-top: 1px solid rgba(255, 255, 255, 0.05);
}

.entity-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 16px;
  cursor: pointer;
  transition: background 0.2s;
  border-bottom: 1px solid rgba(255, 255, 255, 0.03);
}

.entity-item:last-child {
  border-bottom: none;
}

.entity-item:hover {
  background: rgba(255, 255, 255, 0.03);
}

.entity-item.selected {
  background: rgba(102, 126, 234, 0.1);
}

.entity-checkbox {
  width: 16px;
  height: 16px;
  accent-color: #667eea;
  cursor: pointer;
}

.entity-info {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.entity-name {
  font-weight: 500;
  color: #fff;
  font-size: 14px;
}

.entity-summary {
  font-size: 12px;
  color: rgba(255, 255, 255, 0.4);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.relationship-count {
  font-size: 11px;
  color: rgba(255, 255, 255, 0.3);
  background: rgba(255, 255, 255, 0.05);
  padding: 2px 6px;
  border-radius: 4px;
  white-space: nowrap;
}

.modal-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px 24px;
  border-top: 1px solid rgba(255, 255, 255, 0.1);
  background: rgba(0, 0, 0, 0.2);
}

.selection-summary {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.summary-text {
  color: #fff;
  font-size: 14px;
}

.estimate-text {
  color: rgba(255, 255, 255, 0.4);
  font-size: 12px;
}

.footer-actions {
  display: flex;
  gap: 12px;
}

/* Transitions */
.modal-enter-active,
.modal-leave-active {
  transition: all 0.3s ease;
}

.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}

.modal-enter-from .entity-modal,
.modal-leave-to .entity-modal {
  transform: scale(0.95) translateY(20px);
}

.expand-enter-active,
.expand-leave-active {
  transition: all 0.2s ease;
  overflow: hidden;
}

.expand-enter-from,
.expand-leave-to {
  opacity: 0;
  max-height: 0;
}

.expand-enter-to,
.expand-leave-from {
  max-height: 500px;
}
</style>
