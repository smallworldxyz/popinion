<template>
  <div class="knowledge-workbench">
    <!-- Main Header -->
    <div class="workbench-header">
      <h2 class="workbench-title">📋 Knowledge Workbench</h2>
      <p class="workbench-subtitle">Capture insights and identify knowledge gaps</p>
    </div>

    <!-- Split View Container -->
    <div class="workbench-split">
      <!-- Left Panel: Captured Insights -->
      <div class="panel panel-left">
        <div class="panel-header">
          <h3>📋 Captured Insights</h3>
          <span class="panel-count">{{ highlights.length }}</span>
        </div>

        <!-- Empty State for Left Panel -->
        <div v-if="highlights.length === 0" class="empty-state">
          <div class="empty-icon">💡</div>
          <h4>No highlights yet</h4>
          <p>Select text in agent responses and click "Add to Knowledge" to capture insights.</p>
        </div>

        <!-- Highlights Content -->
        <div v-else class="panel-content">
          <!-- Tag Filter -->
          <div class="tag-filter">
            <button 
              class="filter-btn"
              :class="{ active: activeFilter === null }"
              @click="activeFilter = null"
            >All ({{ highlights.length }})</button>
            <button 
              v-for="tag in uniqueTags" 
              :key="tag"
              class="filter-btn"
              :class="{ active: activeFilter === tag }"
              @click="activeFilter = tag"
            >{{ tag }} ({{ getTagCount(tag) }})</button>
          </div>

          <!-- Highlights List -->
          <div class="items-list">
            <div 
              v-for="(highlight, idx) in filteredHighlights" 
              :key="highlight.id"
              class="item-card"
              :class="{ selected: selectedIds.has(highlight.id) }"
            >
              <div class="item-checkbox">
                <input 
                  type="checkbox" 
                  :checked="selectedIds.has(highlight.id)"
                  @change="toggleSelect(highlight.id)"
                />
              </div>
              <div class="item-content">
                <p class="item-text">"{{ highlight.content }}"</p>
                <div class="item-meta">
                  <span class="meta-source">{{ cleanAgentName(highlight.source.agent) }}</span>
                  <span class="meta-dot">•</span>
                  <span class="meta-context">{{ highlight.source.context }}</span>
                  <span class="meta-dot">•</span>
                  <span class="meta-time">{{ formatTime(highlight.createdAt) }}</span>
                </div>
                <div class="item-tags">
                  <span v-for="tag in highlight.tags" :key="tag" class="tag-pill">{{ tag }}</span>
                </div>
              </div>
              <button class="item-delete" @click="removeHighlight(idx)">🗑️</button>
            </div>
          </div>
        </div>
      </div>

      <!-- Right Panel: Knowledge Gaps -->
      <div class="panel panel-right">
        <div class="panel-header">
          <h3>❓ Knowledge Gaps</h3>
          <span class="panel-count">{{ gaps.length }}</span>
        </div>

        <!-- Gap Input Form -->
        <div class="gap-form">
          <textarea 
            v-model="newGapContent" 
            placeholder="Describe what's missing... (e.g., 'Labor unions typically oppose automation due to job security concerns')"
            class="gap-input"
          />
          <div class="gap-options">
            <select v-model="newGapAgent" class="gap-select">
              <option :value="null">Global (All Agents)</option>
              <option v-for="(agent, idx) in agents" :key="idx" :value="idx">
                {{ agent.name }}
              </option>
            </select>
            <select v-model="newGapTag" class="gap-select">
              <option v-for="tag in gapTags" :key="tag" :value="tag">{{ formatGapTag(tag) }}</option>
            </select>
          </div>
          <button @click="addGap" class="add-gap-btn" :disabled="!newGapContent.trim()">
            + Add Gap
          </button>
        </div>

        <!-- Gaps List -->
        <div class="panel-content">
          <div v-if="gaps.length === 0" class="empty-state small">
            <p>No gaps identified yet. Use this panel to note missing perspectives.</p>
          </div>
          <div v-else class="items-list">
            <div 
              v-for="gap in gaps" 
              :key="gap.id" 
              class="item-card gap-card"
              :class="{ selected: selectedGapIds.has(gap.id) }"
            >
              <div class="item-checkbox">
                <input 
                  type="checkbox" 
                  :checked="selectedGapIds.has(gap.id)"
                  @change="toggleGapSelect(gap.id)"
                />
              </div>
              <div class="item-content">
                <p class="item-text">{{ gap.content }}</p>
                <div class="item-meta">
                  <span class="meta-target">{{ gap.agentName || 'Global' }}</span>
                  <span class="meta-dot">•</span>
                  <span class="gap-tag-badge">{{ formatGapTag(gap.tag) }}</span>
                </div>
              </div>
              <button class="item-delete" @click="removeGap(gap.id)">🗑️</button>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Unified Footer -->
    <div class="workbench-footer" v-if="highlights.length > 0 || gaps.length > 0">
      <div class="selection-info">
        <span>{{ totalSelected }} selected</span>
        <span v-if="selectedIds.size > 0" class="selection-detail">({{ selectedIds.size }} highlights)</span>
        <span v-if="selectedGapIds.size > 0" class="selection-detail">({{ selectedGapIds.size }} gaps)</span>
      </div>
      <div class="footer-actions">
        <button 
          class="action-btn danger" 
          :disabled="totalSelected === 0"
          @click="deleteSelected"
        >
          🗑️ Delete Selected
        </button>
        <div class="export-dropdown">
          <button class="action-btn secondary" @click="showExportMenu = !showExportMenu">
            📤 Export ▼
          </button>
          <div v-if="showExportMenu" class="export-menu">
            <div class="export-notice">💡 Use JSON to import later</div>
            <button @click="doExport('json')">JSON (For Import)</button>
            <button @click="doExport('md')">Markdown (Read Only)</button>
            <button @click="doExport('txt')">Plain Text (Read Only)</button>
          </div>
        </div>
        <button 
          class="action-btn secondary"
          :disabled="highlights.length === 0 || isGeneratingSummary"
          @click="generateSummary"
        >
          {{ isGeneratingSummary ? '⏳ Generating...' : '📊 Summary' }}
        </button>
        <label class="action-btn secondary import-btn">
          📥 Import
          <input type="file" accept=".json" @change="handleImport" hidden />
        </label>
        <button 
          class="action-btn primary" 
          :disabled="totalSelected === 0"
          @click="showInjectModal = true"
        >
          💉 Inject Selected
        </button>
      </div>
    </div>

    <!-- Inject Modal -->
    <Transition name="modal">
      <div v-if="showInjectModal" class="modal-overlay" @click.self="showInjectModal = false">
        <div class="modal-content">
          <div class="modal-header">
            <h3>Inject Knowledge</h3>
            <button class="close-btn" @click="showInjectModal = false">×</button>
          </div>
          <div class="modal-body">
            <p class="inject-info">{{ totalSelected }} item(s) selected ({{ selectedIds.size }} highlights, {{ selectedGapIds.size }} gaps)</p>
            
            <div class="inject-options">
              <label class="inject-option">
                <input type="radio" v-model="injectTarget" value="single" />
                <span>Single Agent</span>
              </label>
              <select v-if="injectTarget === 'single'" v-model="selectedAgentIdx" class="agent-select">
                <option v-for="(agent, idx) in agents" :key="idx" :value="idx">
                  {{ agent.name }}
                </option>
              </select>
              
              <label class="inject-option">
                <input type="radio" v-model="injectTarget" value="all" />
                <span>All Panel Participants</span>
              </label>
              
              <label class="inject-option">
                <input type="radio" v-model="injectTarget" value="global" />
                <span>Global (All Agents)</span>
              </label>
            </div>
          </div>
          <div class="modal-footer">
            <button class="action-btn secondary" @click="showInjectModal = false">Cancel</button>
            <button class="action-btn primary" @click="confirmInject">Inject</button>
          </div>
        </div>
      </div>
    </Transition>

    <!-- Summary Modal -->
    <Transition name="fade">
      <div v-if="showSummaryModal" class="modal-overlay" @click.self="showSummaryModal = false">
        <div class="summary-modal" @click.stop>
          <div class="modal-header">
            <h3>📊 Knowledge Summary</h3>
            <button class="modal-close" @click="showSummaryModal = false">×</button>
          </div>
          <div class="modal-body summary-body">
            <div class="summary-stats">
              <div class="stat-card">
                <span class="stat-value">{{ highlights.length }}</span>
                <span class="stat-label">Highlights</span>
              </div>
              <div class="stat-card">
                <span class="stat-value">{{ gaps.length }}</span>
                <span class="stat-label">Gaps</span>
              </div>
              <div class="stat-card">
                <span class="stat-value">{{ uniqueAgentsCount }}</span>
                <span class="stat-label">Agents</span>
              </div>
            </div>
            
            <div class="summary-section" v-if="summaryByTag">
              <h4>By Category</h4>
              <div v-for="(items, tag) in summaryByTag" :key="tag" class="summary-group">
                <div class="group-header">
                  <span class="group-tag">{{ tag }}</span>
                  <span class="group-count">{{ items.length }}</span>
                </div>
                <ul class="group-items">
                  <li v-for="item in items.slice(0, 3)" :key="item.id">
                    <span class="item-agent">{{ item.source?.agent }}</span>
                    <span class="item-text">"{{ item.content?.substring(0, 80) }}..."</span>
                  </li>
                  <li v-if="items.length > 3" class="more-items">+{{ items.length - 3 }} more</li>
                </ul>
              </div>
            </div>
          </div>
          <div class="modal-footer">
            <button class="action-btn secondary" @click="copySummary">📋 Copy Summary</button>
            <button class="action-btn primary" @click="showSummaryModal = false">Close</button>
          </div>
        </div>
      </div>
    </Transition>
  </div>
</template>

<script setup>
import { ref, computed } from 'vue'

const props = defineProps({
  highlights: {
    type: Array,
    default: () => []
  },
  gaps: {
    type: Array,
    default: () => []
  },
  agents: {
    type: Array,
    default: () => []
  }
})

const emit = defineEmits(['remove', 'remove-multiple', 'inject', 'export', 'import', 'add-gap', 'remove-gap', 'remove-gaps-multiple'])

// State for highlights
const activeFilter = ref(null)
const selectedIds = ref(new Set())
const showInjectModal = ref(false)
const showExportMenu = ref(false)
const injectTarget = ref('single')
const selectedAgentIdx = ref(0)

// State for gaps
const selectedGapIds = ref(new Set())
const newGapContent = ref('')
const newGapAgent = ref(null)
const newGapTag = ref('missing_perspective')

// Predefined gap tags
const gapTags = [
  'missing_perspective',
  'missing_stakeholder',
  'missing_relationship',
  'counter_argument',
  'blind_spot',
  'other'
]

// Summary Modal State
const showSummaryModal = ref(false)
const isGeneratingSummary = ref(false)

// Computed: Total selected count
const totalSelected = computed(() => selectedIds.value.size + selectedGapIds.value.size)

// Summary Computed Properties
const uniqueAgentsCount = computed(() => {
  const agents = new Set(props.highlights.map(h => h.source?.agent).filter(Boolean))
  return agents.size
})

const uniqueTagsCount = computed(() => {
  const tags = new Set(props.highlights.flatMap(h => h.tags || []))
  return tags.size
})

const summaryByTag = computed(() => {
  const grouped = {}
  props.highlights.forEach(h => {
    const tags = h.tags || ['Untagged']
    tags.forEach(tag => {
      if (!grouped[tag]) grouped[tag] = []
      grouped[tag].push(h)
    })
  })
  return grouped
})

// Computed for highlights
const uniqueTags = computed(() => {
  const tags = new Set()
  props.highlights.forEach(h => h.tags?.forEach(t => tags.add(t)))
  return Array.from(tags)
})

const filteredHighlights = computed(() => {
  if (!activeFilter.value) return props.highlights
  return props.highlights.filter(h => h.tags?.includes(activeFilter.value))
})

// Helpers
const cleanAgentName = (name) => {
  if (!name) return 'Agent'
  return name
    .replace(/_\d+$/, '')
    .replace(/_/g, ' ')
    .split(' ')
    .map(w => w.charAt(0).toUpperCase() + w.slice(1).toLowerCase())
    .join(' ')
}

const formatTime = (timestamp) => {
  if (!timestamp) return ''
  const date = new Date(timestamp)
  return date.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' })
}

const getTagCount = (tag) => {
  return props.highlights.filter(h => h.tags?.includes(tag)).length
}

const formatGapTag = (tag) => {
  return tag.replace(/_/g, ' ').replace(/\b\w/g, l => l.toUpperCase())
}

// Highlight Actions
const toggleSelect = (id) => {
  const newSet = new Set(selectedIds.value)
  if (newSet.has(id)) {
    newSet.delete(id)
  } else {
    newSet.add(id)
  }
  selectedIds.value = newSet
}

const removeHighlight = (idx) => {
  emit('remove', idx)
}

// Gap Actions
const toggleGapSelect = (id) => {
  const newSet = new Set(selectedGapIds.value)
  if (newSet.has(id)) {
    newSet.delete(id)
  } else {
    newSet.add(id)
  }
  selectedGapIds.value = newSet
}

const addGap = () => {
  if (!newGapContent.value.trim()) return
  
  const agentName = newGapAgent.value !== null 
    ? props.agents[newGapAgent.value]?.name || `Agent ${newGapAgent.value}`
    : null
  
  emit('add-gap', {
    content: newGapContent.value.trim(),
    agentIdx: newGapAgent.value,
    agentName: agentName,
    tag: newGapTag.value
  })
  
  // Reset form
  newGapContent.value = ''
  newGapAgent.value = null
  newGapTag.value = 'missing_perspective'
}

const removeGap = (id) => {
  emit('remove-gap', id)
}

// Combined Actions
const deleteSelected = () => {
  if (selectedIds.value.size > 0) {
    emit('remove-multiple', Array.from(selectedIds.value))
    selectedIds.value = new Set()
  }
  if (selectedGapIds.value.size > 0) {
    emit('remove-gaps-multiple', Array.from(selectedGapIds.value))
    selectedGapIds.value = new Set()
  }
}

const doExport = (format) => {
  showExportMenu.value = false
  
  let content = ''
  let filename = 'knowledge_workbench'
  let mimeType = 'text/plain'
  
  if (format === 'json') {
    content = JSON.stringify({ 
      exportedAt: new Date().toISOString(),
      highlights: props.highlights,
      gaps: props.gaps  // Include gaps in export
    }, null, 2)
    filename += '.json'
    mimeType = 'application/json'
  } else if (format === 'md') {
    content = '# Knowledge Workbench Export\n\n'
    content += `*Exported: ${new Date().toLocaleString()}*\n\n`
    
    // Highlights section
    content += '## Captured Insights\n\n'
    content += '---\n\n'
    props.highlights.forEach((h, i) => {
      content += `### ${i + 1}. ${cleanAgentName(h.source?.agent)}\n\n`
      content += `> ${h.content}\n\n`
      content += `*Context: ${h.source?.context || 'unknown'}*\n\n`
      if (h.tags?.length) content += `Tags: ${h.tags.join(', ')}\n\n`
      content += '---\n\n'
    })
    
    // Gaps section
    content += '## Knowledge Gaps\n\n'
    content += '---\n\n'
    props.gaps.forEach((g, i) => {
      content += `### ${i + 1}. ${formatGapTag(g.tag)}\n\n`
      content += `> ${g.content}\n\n`
      content += `*Target: ${g.agentName || 'Global'}*\n\n`
      content += '---\n\n'
    })
    
    filename += '.md'
  } else {
    content = 'KNOWLEDGE WORKBENCH EXPORT\n'
    content += '='.repeat(40) + '\n'
    content += `Exported: ${new Date().toLocaleString()}\n\n`
    
    content += 'CAPTURED INSIGHTS\n'
    content += '-'.repeat(40) + '\n'
    props.highlights.forEach((h, i) => {
      content += `[${i + 1}] ${cleanAgentName(h.source?.agent)} (${h.source?.context})\n`
      content += `"${h.content}"\n`
      if (h.tags?.length) content += `Tags: ${h.tags.join(', ')}\n`
      content += '\n'
    })
    
    content += '\nKNOWLEDGE GAPS\n'
    content += '-'.repeat(40) + '\n'
    props.gaps.forEach((g, i) => {
      content += `[${i + 1}] ${formatGapTag(g.tag)} → ${g.agentName || 'Global'}\n`
      content += `"${g.content}"\n\n`
    })
    
    filename += '.txt'
  }
  
  const blob = new Blob([content], { type: mimeType })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = filename
  a.click()
  URL.revokeObjectURL(url)
}

const confirmInject = () => {
  // Collect selected highlights
  const selectedHighlights = props.highlights.filter(h => selectedIds.value.has(h.id))
  
  // Collect selected gaps (treated as knowledge to inject)
  const selectedGaps = props.gaps.filter(g => selectedGapIds.value.has(g.id))
  
  // Combine into single injection payload
  const combinedHighlights = [
    ...selectedHighlights,
    ...selectedGaps.map(g => ({
      id: g.id,
      content: g.content,
      source: { agent: 'User (Knowledge Gap)', context: 'gap' },
      tags: [g.tag],
      isGap: true
    }))
  ]
  
  emit('inject', {
    target: injectTarget.value,
    agentIdx: injectTarget.value === 'single' ? selectedAgentIdx.value : null,
    highlights: combinedHighlights
  })
  
  showInjectModal.value = false
  selectedIds.value = new Set()
  selectedGapIds.value = new Set()
}

const handleImport = async (event) => {
  const file = event.target.files[0]
  if (!file) return
  
  // Validate file type
  if (!file.name.endsWith('.json')) {
    alert('Only JSON files can be imported. Please export as JSON if you want to import later.')
    event.target.value = ''
    return
  }
  
  try {
    const text = await file.text()
    const data = JSON.parse(text)
    
    // Validate structure - support both old format (highlights only) and new format (highlights + gaps)
    if (!data.highlights || !Array.isArray(data.highlights)) {
      alert('Invalid Knowledge file. Missing highlights array.')
      event.target.value = ''
      return
    }
    
    // Emit the imported data to parent for processing
    emit('import', {
      highlights: data.highlights,
      gaps: data.gaps || [],  // Support new format with gaps
      sourceSimulationId: data.simulationId || null,
      exportedAt: data.exportedAt || null
    })
    
    // Reset file input
    event.target.value = ''
  } catch (err) {
    alert(`Failed to parse JSON file: ${err.message}`)
    event.target.value = ''
  }
}

// Summary methods
const generateSummary = () => {
  isGeneratingSummary.value = true
  setTimeout(() => {
    isGeneratingSummary.value = false
    showSummaryModal.value = true
  }, 300)
}

const copySummary = () => {
  let summary = `# Knowledge Workbench Summary\n\n`
  summary += `**Highlights:** ${props.highlights.length}\n`
  summary += `**Gaps:** ${props.gaps.length}\n`
  summary += `**Agents:** ${uniqueAgentsCount.value}\n\n`
  
  summary += `## Captured Insights\n\n`
  Object.entries(summaryByTag.value).forEach(([tag, items]) => {
    summary += `### ${tag} (${items.length})\n`
    items.forEach(item => {
      summary += `- **${item.source?.agent || 'Unknown'}:** "${item.content?.substring(0, 100)}..."\n`
    })
    summary += '\n'
  })
  
  summary += `## Knowledge Gaps\n\n`
  props.gaps.forEach(g => {
    summary += `- **${formatGapTag(g.tag)}** → ${g.agentName || 'Global'}: "${g.content}"\n`
  })
  
  navigator.clipboard.writeText(summary)
  alert('Summary copied to clipboard!')
}
</script>

<style scoped>
.knowledge-workbench {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: #FAFAFA;
  overflow: hidden;
}

.workbench-header {
  padding: 20px 24px;
  border-bottom: 1px solid #E5E7EB;
  background: #FFF;
}

.workbench-title {
  font-size: 18px;
  font-weight: 700;
  margin: 0 0 4px 0;
  color: #111;
}

.workbench-subtitle {
  font-size: 12px;
  color: #6B7280;
  margin: 0;
}

/* Split View */
.workbench-split {
  flex: 1;
  display: flex;
  overflow: hidden;
}

.panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.panel-left {
  border-right: 1px solid #E5E7EB;
}

.panel-header {
  padding: 14px 20px;
  border-bottom: 1px solid #E5E7EB;
  background: #F9FAFB;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.panel-header h3 {
  margin: 0;
  font-size: 13px;
  font-weight: 600;
  color: #374151;
}

.panel-count {
  font-size: 11px;
  font-weight: 600;
  background: #E5E7EB;
  color: #6B7280;
  padding: 2px 8px;
  border-radius: 10px;
}

.panel-content {
  flex: 1;
  overflow-y: auto;
}

/* Empty State */
.empty-state {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 40px 20px;
  text-align: center;
}

.empty-state.small {
  padding: 20px;
}

.empty-icon {
  font-size: 32px;
  margin-bottom: 12px;
}

.empty-state h4 {
  font-size: 14px;
  font-weight: 600;
  color: #374151;
  margin: 0 0 6px 0;
}

.empty-state p {
  font-size: 12px;
  color: #9CA3AF;
  max-width: 240px;
  margin: 0;
}

/* Tag Filter */
.tag-filter {
  padding: 12px 16px;
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
  background: #FFF;
  border-bottom: 1px solid #E5E7EB;
}

.filter-btn {
  padding: 5px 10px;
  font-size: 11px;
  border: 1px solid #E5E7EB;
  border-radius: 14px;
  background: #FFF;
  color: #6B7280;
  cursor: pointer;
  transition: all 0.2s;
}

.filter-btn.active {
  background: #6366F1;
  border-color: #6366F1;
  color: #FFF;
}

/* Items List */
.items-list {
  padding: 8px;
}

.item-card {
  display: flex;
  gap: 10px;
  padding: 12px;
  background: #FFF;
  border: 1px solid #E5E7EB;
  border-radius: 8px;
  margin-bottom: 8px;
  transition: all 0.2s;
}

.item-card:hover {
  border-color: #D1D5DB;
}

.item-card.selected {
  background: #EEF2FF;
  border-color: #A5B4FC;
}

.item-card.gap-card {
  border-left: 3px solid #F59E0B;
}

.item-checkbox {
  padding-top: 2px;
}

.item-content {
  flex: 1;
  min-width: 0;
}

.item-text {
  font-size: 13px;
  line-height: 1.5;
  color: #374151;
  margin: 0 0 6px 0;
  font-style: italic;
}

.gap-card .item-text {
  font-style: normal;
}

.item-meta {
  display: flex;
  gap: 5px;
  font-size: 10px;
  color: #9CA3AF;
  margin-bottom: 6px;
  flex-wrap: wrap;
}

.meta-source, .meta-target {
  font-weight: 600;
  color: #6B7280;
}

.item-tags {
  display: flex;
  gap: 4px;
  flex-wrap: wrap;
}

.tag-pill {
  padding: 2px 6px;
  font-size: 9px;
  background: #E5E7EB;
  border-radius: 8px;
  color: #4B5563;
}

.gap-tag-badge {
  background: #FEF3C7;
  color: #92400E;
  padding: 1px 6px;
  border-radius: 4px;
  font-weight: 500;
}

.item-delete {
  background: none;
  border: none;
  cursor: pointer;
  opacity: 0;
  transition: opacity 0.2s;
  font-size: 14px;
}

.item-card:hover .item-delete {
  opacity: 1;
}

/* Gap Form */
.gap-form {
  padding: 14px;
  border-bottom: 1px solid #E5E7EB;
  background: #FFF;
}

.gap-input {
  width: 100%;
  min-height: 70px;
  padding: 10px;
  border: 1px solid #E5E7EB;
  border-radius: 6px;
  resize: vertical;
  font-size: 12px;
  font-family: inherit;
  margin-bottom: 10px;
}

.gap-input:focus {
  outline: none;
  border-color: #6366F1;
}

.gap-options {
  display: flex;
  gap: 8px;
  margin-bottom: 10px;
}

.gap-select {
  flex: 1;
  padding: 7px 10px;
  border: 1px solid #E5E7EB;
  border-radius: 5px;
  font-size: 11px;
  background: #FFF;
}

.add-gap-btn {
  width: 100%;
  padding: 9px;
  background: #6366F1;
  color: white;
  border: none;
  border-radius: 5px;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.2s;
}

.add-gap-btn:hover:not(:disabled) {
  background: #4F46E5;
}

.add-gap-btn:disabled {
  background: #D1D5DB;
  cursor: not-allowed;
}

/* Footer */
.workbench-footer {
  padding: 14px 20px;
  border-top: 1px solid #E5E7EB;
  background: #FFF;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.selection-info {
  font-size: 12px;
  color: #6B7280;
  display: flex;
  gap: 6px;
}

.selection-detail {
  color: #9CA3AF;
}

.footer-actions {
  display: flex;
  gap: 10px;
}

.action-btn {
  padding: 8px 16px;
  font-size: 12px;
  font-weight: 600;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s;
}

.action-btn.secondary {
  background: #FFF;
  border: 1px solid #E5E7EB;
  color: #374151;
}

.action-btn.secondary:hover {
  background: #F3F4F6;
}

.action-btn.primary {
  background: linear-gradient(135deg, #6366F1 0%, #8B5CF6 100%);
  border: none;
  color: #FFF;
}

.action-btn.primary:hover {
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(99, 102, 241, 0.3);
}

.action-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
  transform: none;
}

.action-btn.danger {
  background: #FFF;
  border: 1px solid #FCA5A5;
  color: #DC2626;
}

.action-btn.danger:hover:not(:disabled) {
  background: #FEE2E2;
  border-color: #F87171;
}

/* Export Dropdown */
.export-dropdown {
  position: relative;
}

.export-menu {
  position: absolute;
  bottom: 100%;
  left: 0;
  margin-bottom: 4px;
  background: #FFF;
  border: 1px solid #E5E7EB;
  border-radius: 8px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  overflow: hidden;
  min-width: 140px;
  z-index: 10;
}

.export-menu button {
  display: block;
  width: 100%;
  padding: 10px 14px;
  border: none;
  background: none;
  text-align: left;
  font-size: 12px;
  color: #374151;
  cursor: pointer;
  transition: background 0.15s;
}

.export-menu button:hover {
  background: #F3F4F6;
}

.export-notice {
  padding: 8px 14px;
  font-size: 10px;
  color: #92400E;
  background: #FEF3C7;
  border-bottom: 1px solid #FDE68A;
}

.import-btn {
  cursor: pointer;
}

/* Modal */
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal-content {
  background: #FFF;
  border-radius: 12px;
  width: 400px;
  max-width: 90vw;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.2);
}

.modal-header {
  padding: 18px 22px;
  border-bottom: 1px solid #E5E7EB;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.modal-header h3 {
  font-size: 15px;
  font-weight: 600;
  margin: 0;
}

.close-btn, .modal-close {
  background: none;
  border: none;
  font-size: 22px;
  color: #9CA3AF;
  cursor: pointer;
}

.modal-body {
  padding: 22px;
}

.inject-info {
  font-size: 12px;
  color: #6B7280;
  margin: 0 0 14px 0;
}

.inject-options {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.inject-option {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  cursor: pointer;
}

.agent-select {
  margin-left: 22px;
  padding: 7px 10px;
  border: 1px solid #E5E7EB;
  border-radius: 5px;
  font-size: 12px;
}

.modal-footer {
  padding: 14px 22px;
  border-top: 1px solid #E5E7EB;
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}

/* Summary Modal */
.summary-modal {
  width: 520px;
  max-width: 95vw;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  background: #FFF;
  border-radius: 12px;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.25);
}

.summary-body {
  overflow-y: auto;
}

.summary-stats {
  display: flex;
  gap: 10px;
  margin-bottom: 18px;
}

.stat-card {
  flex: 1;
  padding: 14px;
  background: linear-gradient(135deg, #F0F9FF 0%, #E0F2FE 100%);
  border-radius: 10px;
  text-align: center;
}

.stat-value {
  display: block;
  font-size: 22px;
  font-weight: 700;
  color: #0369A1;
}

.stat-label {
  font-size: 10px;
  color: #64748B;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.summary-section h4 {
  font-size: 12px;
  font-weight: 600;
  color: #374151;
  margin-bottom: 10px;
}

.summary-group {
  margin-bottom: 14px;
  padding: 10px;
  background: #F9FAFB;
  border-radius: 6px;
}

.group-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 6px;
}

.group-tag {
  font-weight: 600;
  font-size: 11px;
  color: #4338CA;
  background: #E0E7FF;
  padding: 2px 8px;
  border-radius: 10px;
}

.group-count {
  font-size: 10px;
  color: #9CA3AF;
}

.group-items {
  margin: 0;
  padding-left: 14px;
  font-size: 11px;
}

.group-items li {
  margin-bottom: 4px;
  color: #6B7280;
}

.item-agent {
  font-weight: 600;
  color: #374151;
  margin-right: 4px;
}

.more-items {
  color: #9CA3AF;
  font-style: italic;
}

/* Transitions */
.modal-enter-active,
.modal-leave-active,
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s;
}

.modal-enter-from,
.modal-leave-to,
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
