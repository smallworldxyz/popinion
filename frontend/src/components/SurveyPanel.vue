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
        <span class="subtitle">Poll {{ totalAgents }} agents with structured responses</span>
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
        :disabled="!question.trim() || isLoading"
        @click="runSurvey"
      >
        <span v-if="isLoading" class="loading-spinner"></span>
        <span v-else>🗳️ Run Survey</span>
      </button>
    </div>

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

const props = defineProps({
  simulationId: {
    type: String,
    required: true
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
  if (!question.value.trim() || isLoading.value) return

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
    
    if (!createResponse.data?.success) {
      throw new Error(createResponse.data?.error || 'Failed to create survey')
    }

    const surveyId = createResponse.data.data.survey_id

    // Step 2: Deploy survey
    const deployResponse = await deploySurvey({
      simulation_id: props.simulationId,
      survey_id: surveyId,
      timeout: 300
    })

    if (deployResponse.data?.success && deployResponse.data?.data) {
      result.value = deployResponse.data.data
      emit('result', deployResponse.data.data)
    } else {
      throw new Error(deployResponse.data?.error || 'Survey deployment failed')
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
