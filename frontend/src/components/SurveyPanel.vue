<template>
  <div class="survey-panel">
    <!-- Header -->
    <div class="survey-header">
      <div class="header-icon">
        <svg viewBox="0 0 24 24" width="24" height="24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M9 11l3 3L22 4"></path>
          <path d="M21 12v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11"></path>
        </svg>
      </div>
      <div class="header-text">
        <h2>Quick Survey</h2>
        <span class="subtitle">Poll agents with structured responses</span>
      </div>
    </div>

    <!-- Agent Selection -->
    <div class="agent-selection-section">
      <div class="selection-header">
        <label class="input-label">Survey Recipients</label>
        <div class="selection-actions">
          <button class="action-btn" @click="selectAllAgents" :disabled="isLoading">
            Select All
          </button>
          <button class="action-btn" @click="clearSelection" :disabled="isLoading || selectedAgents.size === 0">
            Clear
          </button>
        </div>
      </div>
      <div class="selection-display">
        <div v-if="selectedAgents.size === 0" class="no-selection">
          <span class="no-selection-text">No agents selected</span>
          <button class="select-agents-btn" @click="showSelectionModal = true" :disabled="isLoading">
            <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="12" cy="12" r="10"></circle>
              <line x1="12" y1="8" x2="12" y2="16"></line>
              <line x1="8" y1="12" x2="16" y2="12"></line>
            </svg>
            Select Agents
          </button>
        </div>
        <div v-else class="selected-agents-display">
          <div class="agent-chips">
            <span 
              v-for="idx in Array.from(selectedAgents).slice(0, 6)" 
              :key="idx" 
              class="agent-chip"
            >
              {{ getAgentName(idx) }}
            </span>
            <span v-if="selectedAgents.size > 6" class="agent-chip more-chip">
              +{{ selectedAgents.size - 6 }} more
            </span>
          </div>
          <button class="modify-btn" @click="showSelectionModal = true" :disabled="isLoading">
            Modify ({{ selectedAgents.size }}/{{ totalAgents }})
          </button>
        </div>
      </div>
    </div>

    <!-- Survey Setup -->
    <div class="setup-section">
      <div class="input-group">
        <label class="input-label">Survey Question</label>
        <textarea
          v-model="question"
          class="question-input"
          placeholder="Do you support the proposed economic policy?"
          rows="2"
          :disabled="isLoading"
        ></textarea>
      </div>

      <div class="options-row">
        <div class="input-group">
          <label class="input-label">Response Type</label>
          <div class="response-type-toggle">
            <button
              :class="{ active: responseType === 'likert' }"
              @click="responseType = 'likert'"
              :disabled="isLoading"
            >
              <span class="type-icon">📊</span>
              Likert Scale
            </button>
            <button
              :class="{ active: responseType === 'opinion' }"
              @click="responseType = 'opinion'"
              :disabled="isLoading"
            >
              <span class="type-icon">✅</span>
              Yes / No / Neutral
            </button>
          </div>
        </div>
      </div>

      <div class="preview-options">
        <span class="preview-label">Response options:</span>
        <div class="option-pills">
          <span v-for="opt in currentOptions" :key="opt" class="option-pill">{{ opt }}</span>
        </div>
      </div>

      <button
        class="submit-btn"
        :disabled="!question.trim() || isLoading || selectedAgents.size === 0"
        @click="runSurvey"
      >
        <span v-if="isLoading" class="loading-spinner"></span>
        <span v-else-if="selectedAgents.size === 0">⚠️ Select agents first</span>
        <span v-else>🗳️ Survey {{ selectedAgents.size }} Agent{{ selectedAgents.size > 1 ? 's' : '' }}</span>
      </button>
    </div>

    <!-- Agent Selection Modal -->
    <EntitySelectionModal
      :show="showSelectionModal"
      :entities="profilesAsEntities"
      :by-type="profilesByType"
      title="Select Survey Recipients"
      item-label="agents"
      :show-estimate="false"
      :select-all-by-default="false"
      @close="showSelectionModal = false"
      @confirm="handleAgentSelection"
    />

    <!-- Results -->
    <div v-if="result" class="results-section">
      <div class="results-header">
        <h3>Survey Results</h3>
        <span class="respondent-count">{{ result.total_respondents }} respondents</span>
      </div>

      <!-- Aggregated Results -->
      <div class="aggregated-card">
        <h4>Overall Response Distribution</h4>
        <div class="response-bars">
          <div 
            v-for="(data, option) in aggregatedData" 
            :key="option" 
            class="response-bar"
            :class="getOptionClass(option)"
          >
            <div class="bar-fill" :style="{ width: `${data.percentage}%` }"></div>
            <span class="bar-label">{{ option }} — {{ data.count }} ({{ data.percentage.toFixed(1) }}%)</span>
          </div>
        </div>
      </div>

      <!-- By Faction -->
      <div v-if="Object.keys(result.by_faction || {}).length > 0" class="faction-breakdown">
        <h4>Breakdown by Faction</h4>
        <div class="faction-cards">
          <div v-for="(responses, faction) in result.by_faction" :key="faction" class="faction-card">
            <div class="faction-header">
              <span class="faction-name">{{ faction }}</span>
              <span class="faction-count">{{ responses.length }} responses</span>
            </div>
            <div class="faction-responses">
              <div 
                v-for="(resp, idx) in responses.slice(0, 5)" 
                :key="idx" 
                class="faction-response"
              >
                <span class="agent-name">{{ resp.agent_name }}</span>
                <span class="agent-answer" :class="getOptionClass(resp.responses?.q1)">
                  {{ resp.responses?.q1 || 'N/A' }}
                </span>
              </div>
              <div v-if="responses.length > 5" class="more-responses">
                +{{ responses.length - 5 }} more
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Error -->
    <div v-if="error" class="error-message">
      <span>{{ error }}</span>
      <button @click="error = null">Dismiss</button>
    </div>
  </div>
</template>

<script setup>
import { ref, computed } from 'vue'
import { createSurvey, deploySurvey } from '../api/simulation'
import EntitySelectionModal from './EntitySelectionModal.vue'

const props = defineProps({
  simulationId: {
    type: String,
    required: true
  },
  profiles: {
    type: Array,
    default: () => []
  },
  totalAgents: {
    type: Number,
    default: 0
  }
})

const emit = defineEmits(['result', 'error'])

// State
const question = ref('')
const responseType = ref('likert')
const isLoading = ref(false)
const result = ref(null)
const error = ref(null)

// Agent Selection State
const selectedAgents = ref(new Set())
const showSelectionModal = ref(false)

// Convert profiles to entities format for EntitySelectionModal
const profilesAsEntities = computed(() => {
  return props.profiles.map((p, idx) => ({
    uuid: idx,  // EntitySelectionModal uses 'uuid' for selection tracking
    id: idx,
    name: p.username || p.display_name || `Agent ${idx}`,
    type: p.source_entity_type || p.profession || 'Unknown'
  }))
})

// Group profiles by type for EntitySelectionModal
const profilesByType = computed(() => {
  const byType = {}
  props.profiles.forEach((p, idx) => {
    const type = p.source_entity_type || p.profession || 'Unknown'
    if (!byType[type]) byType[type] = []
    byType[type].push({
      uuid: idx,  // EntitySelectionModal uses 'uuid' for selection tracking
      id: idx,
      name: p.username || p.display_name || `Agent ${idx}`,
      type
    })
  })
  return byType
})

// Get agent name by index
const getAgentName = (idx) => {
  const profile = props.profiles[idx]
  if (!profile) return `Agent ${idx}`
  return profile.username || profile.display_name || `Agent ${idx}`
}

// Selection helpers
const selectAllAgents = () => {
  selectedAgents.value = new Set(props.profiles.map((_, idx) => idx))
}

const clearSelection = () => {
  selectedAgents.value = new Set()
}

const handleAgentSelection = (selectedIds) => {
  selectedAgents.value = new Set(selectedIds)
  showSelectionModal.value = false
}

// Options based on response type
const likertOptions = ['Strongly Agree', 'Agree', 'Neutral', 'Disagree', 'Strongly Disagree']
const opinionOptions = ['Agree', 'Disagree', 'Neutral']

const currentOptions = computed(() => {
  return responseType.value === 'likert' ? likertOptions : opinionOptions
})

// Computed: Aggregated data from result
const aggregatedData = computed(() => {
  if (!result.value?.aggregated?.q1) return {}
  return result.value.aggregated.q1
})

// Option styling class
const getOptionClass = (option) => {
  const optLower = (option || '').toLowerCase()
  if (optLower.includes('strongly agree')) return 'strongly-agree'
  if (optLower.includes('agree')) return 'agree'
  if (optLower.includes('strongly disagree')) return 'strongly-disagree'
  if (optLower.includes('disagree')) return 'disagree'
  if (optLower.includes('neutral')) return 'neutral'
  return ''
}

// Run Survey
const runSurvey = async () => {
  if (!question.value.trim() || isLoading.value || selectedAgents.value.size === 0) return

  isLoading.value = true
  error.value = null
  result.value = null

  try {
    // Step 1: Create survey template
    const surveyData = {
      title: `Quick Survey - ${Date.now()}`,
      description: 'Quick poll survey',
      questions: [
        {
          question_text: question.value.trim(),
          question_type: responseType.value === 'likert' ? 'likert' : 'opinion_poll',
          options: currentOptions.value
        }
      ]
    }

    const createResponse = await createSurvey(surveyData)
    
    if (!createResponse?.success) {
      throw new Error(createResponse?.error || 'Failed to create survey')
    }

    const surveyId = createResponse.data.survey_id

    // Step 2: Deploy survey to selected agents
    const deployResponse = await deploySurvey({
      simulation_id: props.simulationId,
      survey_id: surveyId,
      agent_ids: Array.from(selectedAgents.value),
      timeout: 300
    })

    if (deployResponse?.success && deployResponse?.data) {
      result.value = deployResponse.data
      emit('result', deployResponse.data)
    } else {
      throw new Error(deployResponse?.error || 'Survey deployment failed')
    }
  } catch (err) {
    error.value = err.message || 'Failed to run survey'
    emit('error', err)
  } finally {
    isLoading.value = false
  }
}
</script>

<style scoped>
.survey-panel {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
  padding: 1.5rem;
  background: var(--bg-secondary, #f8f9fa);
  border-radius: 12px;
}

.survey-header {
  display: flex;
  align-items: center;
  gap: 1rem;
}

.header-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 48px;
  height: 48px;
  background: linear-gradient(135deg, #10b981, #059669);
  border-radius: 12px;
  color: white;
}

.header-text h2 {
  margin: 0;
  font-size: 1.25rem;
  font-weight: 600;
  color: var(--text-primary, #1a1a1a);
}

.header-text .subtitle {
  font-size: 0.875rem;
  color: var(--text-secondary, #6b7280);
}

/* Agent Selection Section */
.agent-selection-section {
  background: white;
  border-radius: 10px;
  padding: 1rem;
  border: 1px solid var(--border-color, #e5e7eb);
}

.selection-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.75rem;
}

.selection-actions {
  display: flex;
  gap: 0.5rem;
}

.action-btn {
  padding: 0.25rem 0.75rem;
  background: transparent;
  border: 1px solid var(--border-color, #e5e7eb);
  border-radius: 6px;
  font-size: 0.75rem;
  color: var(--text-secondary, #6b7280);
  cursor: pointer;
  transition: all 0.2s;
}

.action-btn:hover:not(:disabled) {
  border-color: #10b981;
  color: #10b981;
}

.action-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.selection-display {
  min-height: 40px;
}

.no-selection {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.5rem;
  background: #f9fafb;
  border-radius: 8px;
  border: 1px dashed var(--border-color, #e5e7eb);
}

.no-selection-text {
  font-size: 0.875rem;
  color: var(--text-secondary, #9ca3af);
}

.select-agents-btn {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 1rem;
  background: linear-gradient(135deg, #10b981, #059669);
  border: none;
  border-radius: 6px;
  color: white;
  font-size: 0.875rem;
  font-weight: 500;
  cursor: pointer;
  transition: transform 0.2s, box-shadow 0.2s;
}

.select-agents-btn:hover:not(:disabled) {
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(16, 185, 129, 0.3);
}

.select-agents-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.selected-agents-display {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}

.agent-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  flex: 1;
}

.agent-chip {
  padding: 0.25rem 0.75rem;
  background: #ecfdf5;
  border: 1px solid #10b981;
  border-radius: 16px;
  font-size: 0.75rem;
  color: #047857;
  white-space: nowrap;
}

.agent-chip.more-chip {
  background: #f3f4f6;
  border-color: #d1d5db;
  color: #6b7280;
}

.modify-btn {
  padding: 0.5rem 1rem;
  background: white;
  border: 1px solid #10b981;
  border-radius: 6px;
  color: #047857;
  font-size: 0.8rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
  white-space: nowrap;
}

.modify-btn:hover:not(:disabled) {
  background: #ecfdf5;
}

.modify-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.setup-section {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.input-group {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.input-label {
  font-size: 0.875rem;
  font-weight: 500;
  color: var(--text-primary, #1a1a1a);
}

.question-input {
  padding: 1rem;
  border: 1px solid var(--border-color, #e5e7eb);
  border-radius: 8px;
  font-size: 1rem;
  resize: vertical;
  transition: border-color 0.2s;
}

.question-input:focus {
  outline: none;
  border-color: #10b981;
}

.response-type-toggle {
  display: flex;
  gap: 0.5rem;
}

.response-type-toggle button {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  padding: 0.75rem 1rem;
  background: white;
  border: 2px solid var(--border-color, #e5e7eb);
  border-radius: 8px;
  font-size: 0.875rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
}

.response-type-toggle button:hover:not(:disabled) {
  border-color: #10b981;
}

.response-type-toggle button.active {
  background: #ecfdf5;
  border-color: #10b981;
  color: #047857;
}

.response-type-toggle button:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.type-icon {
  font-size: 1rem;
}

.preview-options {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.75rem;
  background: white;
  border-radius: 8px;
}

.preview-label {
  font-size: 0.75rem;
  color: var(--text-secondary, #6b7280);
}

.option-pills {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.option-pill {
  padding: 0.25rem 0.75rem;
  background: #f3f4f6;
  border-radius: 12px;
  font-size: 0.75rem;
  color: var(--text-primary, #374151);
}

.submit-btn {
  align-self: flex-start;
  padding: 0.75rem 1.5rem;
  background: linear-gradient(135deg, #10b981, #059669);
  border: none;
  border-radius: 8px;
  color: white;
  font-weight: 500;
  font-size: 1rem;
  cursor: pointer;
  transition: transform 0.2s, box-shadow 0.2s;
}

.submit-btn:hover:not(:disabled) {
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(16, 185, 129, 0.3);
}

.submit-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.loading-spinner {
  display: inline-block;
  width: 16px;
  height: 16px;
  border: 2px solid white;
  border-top-color: transparent;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

/* Results */
.results-section {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}

.results-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.results-header h3 {
  margin: 0;
  font-size: 1.1rem;
  font-weight: 600;
}

.respondent-count {
  padding: 0.25rem 0.75rem;
  background: #e0e7ff;
  color: #4338ca;
  border-radius: 12px;
  font-size: 0.75rem;
  font-weight: 500;
}

.aggregated-card {
  background: white;
  padding: 1.25rem;
  border-radius: 10px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
}

.aggregated-card h4 {
  margin: 0 0 1rem 0;
  font-size: 0.875rem;
  font-weight: 600;
  color: var(--text-primary, #1a1a1a);
}

.response-bars {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.response-bar {
  position: relative;
  height: 32px;
  background: var(--bg-secondary, #f3f4f6);
  border-radius: 6px;
  overflow: hidden;
}

.bar-fill {
  height: 100%;
  border-radius: 6px;
  transition: width 0.5s ease;
}

.response-bar.strongly-agree .bar-fill { background: #059669; }
.response-bar.agree .bar-fill { background: #10b981; }
.response-bar.neutral .bar-fill { background: #6b7280; }
.response-bar.disagree .bar-fill { background: #f59e0b; }
.response-bar.strongly-disagree .bar-fill { background: #ef4444; }

.bar-label {
  position: absolute;
  left: 12px;
  top: 50%;
  transform: translateY(-50%);
  font-size: 0.8rem;
  font-weight: 500;
  color: var(--text-primary, #1a1a1a);
}

/* Faction Breakdown */
.faction-breakdown {
  margin-top: 0.5rem;
}

.faction-breakdown h4 {
  margin: 0 0 1rem 0;
  font-size: 0.875rem;
  font-weight: 600;
  color: var(--text-primary, #1a1a1a);
}

.faction-cards {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: 1rem;
}

.faction-card {
  background: white;
  padding: 1rem;
  border-radius: 8px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
}

.faction-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.75rem;
  padding-bottom: 0.5rem;
  border-bottom: 1px solid #e5e7eb;
}

.faction-name {
  font-weight: 600;
  color: #4338ca;
}

.faction-count {
  font-size: 0.75rem;
  color: var(--text-secondary, #6b7280);
}

.faction-responses {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.faction-response {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 0.8rem;
}

.agent-name {
  color: var(--text-primary, #374151);
}

.agent-answer {
  padding: 0.125rem 0.5rem;
  border-radius: 4px;
  font-size: 0.7rem;
  font-weight: 500;
}

.agent-answer.strongly-agree { background: #d1fae5; color: #047857; }
.agent-answer.agree { background: #d1fae5; color: #047857; }
.agent-answer.neutral { background: #f3f4f6; color: #4b5563; }
.agent-answer.disagree { background: #fef3c7; color: #92400e; }
.agent-answer.strongly-disagree { background: #fee2e2; color: #b91c1c; }

.more-responses {
  font-size: 0.75rem;
  color: var(--text-secondary, #6b7280);
  font-style: italic;
}

/* Error */
.error-message {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 1rem;
  background: #fee2e2;
  border-radius: 8px;
  color: #b91c1c;
}

.error-message button {
  padding: 0.25rem 0.75rem;
  background: #b91c1c;
  border: none;
  border-radius: 4px;
  color: white;
  cursor: pointer;
}
</style>
