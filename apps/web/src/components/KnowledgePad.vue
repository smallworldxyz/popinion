<template>
  <div class="knowledge-pad">
    <div class="pad-header">
      <h2 class="pad-title">📋 Knowledge Pad</h2>
      <p class="pad-subtitle">Capture and inject insights from conversations</p>
    </div>

    <div v-if="notice" class="pad-notice" :class="notice.kind">{{ notice.msg }}</div>

    <!-- Empty State -->
    <div v-if="highlights.length === 0" class="empty-state">
      <div class="empty-icon">💡</div>
      <h3>No highlights yet</h3>
      <p>Select text in agent responses and click "Add to Knowledge" to capture insights.</p>
    </div>

    <!-- Highlights List -->
    <div v-else class="highlights-list">
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

      <!-- Highlights -->
      <div 
        v-for="(highlight, idx) in filteredHighlights" 
        :key="highlight.id"
        class="highlight-card"
        :class="{ selected: selectedIds.has(highlight.id) }"
      >
        <div class="highlight-checkbox">
          <input 
            type="checkbox" 
            :checked="selectedIds.has(highlight.id)"
            @change="toggleSelect(highlight.id)"
          />
        </div>
        <div class="highlight-content">
          <p class="highlight-text">"{{ highlight.content }}"</p>
          <div class="highlight-meta">
            <span class="meta-source">{{ cleanAgentName(highlight.source.agent) }}</span>
            <span class="meta-dot">•</span>
            <span class="meta-context">{{ highlight.source.context }}</span>
            <span class="meta-dot">•</span>
            <span class="meta-time">{{ formatTime(highlight.createdAt) }}</span>
          </div>
          <div class="highlight-tags">
            <span v-for="tag in highlight.tags" :key="tag" class="tag-pill">{{ tag }}</span>
          </div>
        </div>
        <button class="highlight-delete" @click="removeHighlight(idx)">🗑️</button>
      </div>
    </div>

    <!-- Actions Footer -->
    <div v-if="highlights.length > 0" class="pad-footer">
      <div class="selection-info">
        <span>{{ selectedIds.size }} selected</span>
      </div>
      <div class="footer-actions">
        <button 
          class="action-btn danger" 
          :disabled="selectedIds.size === 0"
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
          :disabled="selectedIds.size === 0"
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
            <p class="inject-info">{{ selectedIds.size }} highlight(s) selected</p>
            
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
                <span class="stat-label">Total Highlights</span>
              </div>
              <div class="stat-card">
                <span class="stat-value">{{ uniqueAgentsCount }}</span>
                <span class="stat-label">Agents</span>
              </div>
              <div class="stat-card">
                <span class="stat-value">{{ uniqueTagsCount }}</span>
                <span class="stat-label">Tags</span>
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
  agents: {
    type: Array,
    default: () => []
  }
})

const emit = defineEmits(['remove', 'remove-multiple', 'inject', 'export', 'import'])

// State
const activeFilter = ref(null)
const selectedIds = ref(new Set())
const showInjectModal = ref(false)
const showExportMenu = ref(false)
const injectTarget = ref('single')
const selectedAgentIdx = ref(0)

// Summary Modal State
const showSummaryModal = ref(false)
const isGeneratingSummary = ref(false)

// Inline feedback instead of a native alert() dialog.
const notice = ref(null)
let noticeTimer = null
const flash = (msg, kind = 'error') => {
  notice.value = { msg, kind }
  clearTimeout(noticeTimer)
  noticeTimer = setTimeout(() => { notice.value = null }, 4000)
}

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

// Computed
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

// Actions
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

const doExport = (format) => {
  showExportMenu.value = false
  
  let content = ''
  let filename = 'knowledge_pad'
  let mimeType = 'text/plain'
  
  if (format === 'json') {
    content = JSON.stringify({ 
      exportedAt: new Date().toISOString(),
      highlights: props.highlights 
    }, null, 2)
    filename += '.json'
    mimeType = 'application/json'
  } else if (format === 'md') {
    content = '# Knowledge Pad Export\n\n'
    content += `*Exported: ${new Date().toLocaleString()}*\n\n`
    content += '---\n\n'
    props.highlights.forEach((h, i) => {
      content += `## ${i + 1}. ${cleanAgentName(h.source?.agent)}\n\n`
      content += `> ${h.content}\n\n`
      content += `*Context: ${h.source?.context || 'unknown'}*\n\n`
      if (h.tags?.length) content += `Tags: ${h.tags.join(', ')}\n\n`
      content += '---\n\n'
    })
    filename += '.md'
  } else {
    content = 'KNOWLEDGE PAD EXPORT\n'
    content += '='.repeat(40) + '\n'
    content += `Exported: ${new Date().toLocaleString()}\n\n`
    props.highlights.forEach((h, i) => {
      content += `[${i + 1}] ${cleanAgentName(h.source?.agent)} (${h.source?.context})\n`
      content += `"${h.content}"\n`
      if (h.tags?.length) content += `Tags: ${h.tags.join(', ')}\n`
      content += '\n'
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

const deleteSelected = () => {
  if (selectedIds.value.size === 0) return
  emit('remove-multiple', Array.from(selectedIds.value))
  selectedIds.value = new Set()
}

const confirmInject = () => {
  const selectedHighlights = props.highlights.filter(h => selectedIds.value.has(h.id))
  emit('inject', {
    target: injectTarget.value,
    agentIdx: injectTarget.value === 'single' ? selectedAgentIdx.value : null,
    highlights: selectedHighlights
  })
  showInjectModal.value = false
  selectedIds.value = new Set()
}

const handleImport = async (event) => {
  const file = event.target.files[0]
  if (!file) return
  
  // Validate file type
  if (!file.name.endsWith('.json')) {
    flash('Only JSON files can be imported. Export as JSON first.')
    event.target.value = ''
    return
  }
  
  try {
    const text = await file.text()
    const data = JSON.parse(text)
    
    // Validate structure
    if (!data.highlights || !Array.isArray(data.highlights)) {
      flash('Invalid Knowledge Pad file: missing highlights array.')
      event.target.value = ''
      return
    }
    
    // Emit the imported data to parent for processing
    emit('import', {
      highlights: data.highlights,
      sourceSimulationId: data.simulationId || null,
      exportedAt: data.exportedAt || null
    })
    
    // Reset file input
    event.target.value = ''
  } catch (err) {
    flash(`Failed to parse JSON file: ${err.message}`)
    event.target.value = ''
  }
}

// Summary methods
const generateSummary = () => {
  isGeneratingSummary.value = true
  // Small delay to show loading state
  setTimeout(() => {
    isGeneratingSummary.value = false
    showSummaryModal.value = true
  }, 300)
}

const copySummary = () => {
  // Build text summary
  let summary = `# Knowledge Pad Summary\n\n`
  summary += `**Total Highlights:** ${props.highlights.length}\n`
  summary += `**Agents:** ${uniqueAgentsCount.value}\n`
  summary += `**Tags:** ${uniqueTagsCount.value}\n\n`
  
  summary += `## By Category\n\n`
  Object.entries(summaryByTag.value).forEach(([tag, items]) => {
    summary += `### ${tag} (${items.length})\n`
    items.forEach(item => {
      summary += `- **${item.source?.agent || 'Unknown'}:** "${item.content?.substring(0, 100)}..."\n`
    })
    summary += '\n'
  })
  
  navigator.clipboard.writeText(summary)
  flash('Summary copied to clipboard.', 'ok')
}
</script>

<style scoped>
.knowledge-pad {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: #FAFAFA;
  overflow: hidden;
}
.pad-notice {
  margin: 8px 16px 0;
  padding: 8px 12px;
  border-radius: 8px;
  font-size: 13px;
  line-height: 1.4;
}
.pad-notice.error { background: #fee2e2; color: #b91c1c; }
.pad-notice.ok { background: #d1fae5; color: #065f46; }

.pad-header {
  padding: 24px;
  border-bottom: 1px solid #E5E7EB;
  background: #FFF;
}

.pad-title {
  font-size: 20px;
  font-weight: 700;
  margin: 0 0 4px 0;
  color: #111;
}

.pad-subtitle {
  font-size: 13px;
  color: #6B7280;
  margin: 0;
}

/* Empty State */
.empty-state {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 40px;
  text-align: center;
}

.empty-icon {
  font-size: 48px;
  margin-bottom: 16px;
}

.empty-state h3 {
  font-size: 16px;
  font-weight: 600;
  color: #374151;
  margin: 0 0 8px 0;
}

.empty-state p {
  font-size: 13px;
  color: #9CA3AF;
  max-width: 280px;
}

/* Tag Filter */
.tag-filter {
  padding: 16px 24px;
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  background: #FFF;
  border-bottom: 1px solid #E5E7EB;
}

.filter-btn {
  padding: 6px 12px;
  font-size: 12px;
  border: 1px solid #E5E7EB;
  border-radius: 16px;
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

/* Highlights List */
.highlights-list {
  flex: 1;
  overflow-y: auto;
}

.highlight-card {
  display: flex;
  gap: 12px;
  padding: 16px 24px;
  background: #FFF;
  border-bottom: 1px solid #F3F4F6;
  transition: background 0.2s;
}

.highlight-card:hover {
  background: #F9FAFB;
}

.highlight-card.selected {
  background: #EEF2FF;
}

.highlight-checkbox {
  padding-top: 2px;
}

.highlight-content {
  flex: 1;
}

.highlight-text {
  font-size: 14px;
  line-height: 1.6;
  color: #374151;
  margin: 0 0 8px 0;
  font-style: italic;
}

.highlight-meta {
  display: flex;
  gap: 6px;
  font-size: 11px;
  color: #9CA3AF;
  margin-bottom: 8px;
}

.meta-source {
  font-weight: 600;
  color: #6B7280;
}

.highlight-tags {
  display: flex;
  gap: 6px;
}

.tag-pill {
  padding: 2px 8px;
  font-size: 10px;
  background: #E5E7EB;
  border-radius: 10px;
  color: #4B5563;
}

.highlight-delete {
  background: none;
  border: none;
  cursor: pointer;
  opacity: 0;
  transition: opacity 0.2s;
}

.highlight-card:hover .highlight-delete {
  opacity: 1;
}

/* Footer */
.pad-footer {
  padding: 16px 24px;
  border-top: 1px solid #E5E7EB;
  background: #FFF;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.selection-info {
  font-size: 13px;
  color: #6B7280;
}

.footer-actions {
  display: flex;
  gap: 12px;
}

.action-btn {
  padding: 10px 20px;
  font-size: 13px;
  font-weight: 600;
  border-radius: 8px;
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
}

.export-menu button {
  display: block;
  width: 100%;
  padding: 10px 16px;
  border: none;
  background: none;
  text-align: left;
  font-size: 13px;
  color: #374151;
  cursor: pointer;
  transition: background 0.15s;
}

.export-menu button:hover {
  background: #F3F4F6;
}

.export-notice {
  padding: 8px 16px;
  font-size: 11px;
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
  border-radius: 16px;
  width: 400px;
  max-width: 90vw;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.2);
}

.modal-header {
  padding: 20px 24px;
  border-bottom: 1px solid #E5E7EB;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.modal-header h3 {
  font-size: 16px;
  font-weight: 600;
  margin: 0;
}

.close-btn {
  background: none;
  border: none;
  font-size: 24px;
  color: #9CA3AF;
  cursor: pointer;
}

.modal-body {
  padding: 24px;
}

.inject-info {
  font-size: 13px;
  color: #6B7280;
  margin: 0 0 16px 0;
}

.inject-options {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.inject-option {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  cursor: pointer;
}

.agent-select {
  margin-left: 24px;
  padding: 8px 12px;
  border: 1px solid #E5E7EB;
  border-radius: 6px;
  font-size: 13px;
}

.modal-footer {
  padding: 16px 24px;
  border-top: 1px solid #E5E7EB;
  display: flex;
  justify-content: flex-end;
  gap: 12px;
}

/* Summary Modal */
.summary-modal {
  width: 560px;
  max-width: 95vw;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  background: #FFF;
  border-radius: 16px;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.25);
}

.summary-body {
  overflow-y: auto;
}

.summary-stats {
  display: flex;
  gap: 12px;
  margin-bottom: 20px;
}

.stat-card {
  flex: 1;
  padding: 16px;
  background: linear-gradient(135deg, #F0F9FF 0%, #E0F2FE 100%);
  border-radius: 12px;
  text-align: center;
}

.stat-value {
  display: block;
  font-size: 24px;
  font-weight: 700;
  color: #0369A1;
}

.stat-label {
  font-size: 11px;
  color: #64748B;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.summary-section h4 {
  font-size: 13px;
  font-weight: 600;
  color: #374151;
  margin-bottom: 12px;
}

.summary-group {
  margin-bottom: 16px;
  padding: 12px;
  background: #F9FAFB;
  border-radius: 8px;
}

.group-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}

.group-tag {
  font-weight: 600;
  font-size: 12px;
  color: #4338CA;
  background: #E0E7FF;
  padding: 3px 10px;
  border-radius: 12px;
}

.group-count {
  font-size: 11px;
  color: #9CA3AF;
}

.group-items {
  margin: 0;
  padding-left: 16px;
  font-size: 12px;
}

.group-items li {
  margin-bottom: 6px;
  color: #6B7280;
}

.item-agent {
  font-weight: 600;
  color: #374151;
  margin-right: 6px;
}

.item-text {
  font-style: italic;
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
