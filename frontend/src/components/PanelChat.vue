<template>
  <div class="panel-chat">
    <!-- Header -->
    <div class="panel-header">
      <div class="header-icon">
        <svg viewBox="0 0 24 24" width="24" height="24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"></path>
          <circle cx="9" cy="7" r="4"></circle>
          <path d="M23 21v-2a4 4 0 0 0-3-3.87M16 3.13a4 4 0 0 1 0 7.75"></path>
        </svg>
      </div>
      <div class="header-text">
        <h2>Panel Chat</h2>
        <span class="subtitle">Ask all {{ totalAgents }} agents at once</span>
      </div>
    </div>

    <!-- Question Input -->
    <div class="input-section">
      <label class="input-label">Your Question</label>
      <textarea
        v-model="question"
        class="question-input"
        placeholder="What do you think about this policy change?"
        rows="3"
        :disabled="isLoading"
      ></textarea>
      <button
        class="submit-btn"
        :disabled="!question.trim() || isLoading"
        @click="submitQuestion"
      >
        <span v-if="isLoading" class="loading-spinner"></span>
        <span v-else>Ask All Agents</span>
      </button>
    </div>

    <!-- Results -->
    <div v-if="result" class="results-section">
      <!-- Summary -->
      <div v-if="result.summary" class="summary-card">
        <h3>Summary</h3>
        <p>{{ result.summary }}</p>
      </div>

      <!-- Stance Distribution -->
      <div class="distribution-card">
        <h3>Stance Distribution</h3>
        <div class="stance-bars">
          <div class="stance-bar support">
            <div class="bar-fill" :style="{ width: `${result.stance_distribution?.support || 0}%` }"></div>
            <span class="bar-label">Support {{ result.stance_distribution?.support || 0 }}%</span>
          </div>
          <div class="stance-bar oppose">
            <div class="bar-fill" :style="{ width: `${result.stance_distribution?.oppose || 0}%` }"></div>
            <span class="bar-label">Oppose {{ result.stance_distribution?.oppose || 0 }}%</span>
          </div>
          <div class="stance-bar neutral">
            <div class="bar-fill" :style="{ width: `${result.stance_distribution?.neutral || 0}%` }"></div>
            <span class="bar-label">Neutral {{ result.stance_distribution?.neutral || 0 }}%</span>
          </div>
        </div>
      </div>

      <!-- View Toggle -->
      <div class="view-controls">
        <button
          :class="{ active: viewMode === 'stance' }"
          @click="viewMode = 'stance'"
        >By Stance</button>
        <button
          :class="{ active: viewMode === 'faction' }"
          @click="viewMode = 'faction'"
        >By Faction</button>
        <button
          :class="{ active: viewMode === 'all' }"
          @click="viewMode = 'all'"
        >All Responses</button>
      </div>

      <!-- By Stance View -->
      <div v-if="viewMode === 'stance'" class="grouped-view">
        <div v-for="stance in ['support', 'oppose', 'neutral']" :key="stance" class="stance-group">
          <h4 class="group-title" :class="stance">
            {{ stance.charAt(0).toUpperCase() + stance.slice(1) }}
            <span class="count">({{ result.by_stance?.[stance]?.length || 0 }})</span>
          </h4>
          <div class="response-cards">
            <div
              v-for="(resp, idx) in (result.by_stance?.[stance] || [])"
              :key="idx"
              class="response-card"
            >
              <div class="card-header">
                <span class="agent-name">{{ resp.agent_name }}</span>
                <span class="faction-tag">{{ resp.faction }}</span>
              </div>
              <p class="response-text">{{ resp.response }}</p>
            </div>
          </div>
        </div>
      </div>

      <!-- By Faction View -->
      <div v-if="viewMode === 'faction'" class="grouped-view">
        <div v-for="(responses, faction) in result.by_faction" :key="faction" class="faction-group">
          <h4 class="group-title faction">
            {{ faction }}
            <span class="count">({{ responses.length }})</span>
          </h4>
          <div class="response-cards">
            <div
              v-for="(resp, idx) in responses"
              :key="idx"
              class="response-card"
            >
              <div class="card-header">
                <span class="agent-name">{{ resp.agent_name }}</span>
                <span class="stance-tag" :class="resp.stance">{{ resp.stance }}</span>
              </div>
              <p class="response-text">{{ resp.response }}</p>
            </div>
          </div>
        </div>
      </div>

      <!-- All Responses View -->
      <div v-if="viewMode === 'all'" class="all-responses">
        <div
          v-for="(resp, idx) in result.responses"
          :key="idx"
          class="response-card full"
        >
          <div class="card-header">
            <span class="agent-name">{{ resp.agent_name }}</span>
            <span class="faction-tag">{{ resp.faction }}</span>
            <span class="stance-tag" :class="resp.stance">{{ resp.stance }}</span>
          </div>
          <p class="response-text">{{ resp.response }}</p>
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
import { ref } from 'vue'
import { panelChat } from '../api/simulation'

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
const isLoading = ref(false)
const result = ref(null)
const error = ref(null)
const viewMode = ref('stance')

// Methods
const submitQuestion = async () => {
  if (!question.value.trim() || isLoading.value) return

  isLoading.value = true
  error.value = null
  result.value = null

  try {
    const response = await panelChat({
      simulation_id: props.simulationId,
      prompt: question.value.trim(),
      classify_stance: true,
      generate_summary: true,
      timeout: 300
    })

    if (response.success && response.data) {
      result.value = response.data
      emit('result', response.data)
    } else {
      throw new Error(response.error || 'Panel chat failed')
    }
  } catch (err) {
    error.value = err.message || 'Failed to get responses'
    emit('error', err)
  } finally {
    isLoading.value = false
  }
}
</script>

<style scoped>
.panel-chat {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
  padding: 1.5rem;
  background: var(--bg-secondary, #f8f9fa);
  border-radius: 12px;
}

.panel-header {
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
  background: linear-gradient(135deg, #6366f1, #8b5cf6);
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

.input-section {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
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
  border-color: #6366f1;
}

.submit-btn {
  align-self: flex-start;
  padding: 0.75rem 1.5rem;
  background: linear-gradient(135deg, #6366f1, #8b5cf6);
  border: none;
  border-radius: 8px;
  color: white;
  font-weight: 500;
  cursor: pointer;
  transition: transform 0.2s, box-shadow 0.2s;
}

.submit-btn:hover:not(:disabled) {
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(99, 102, 241, 0.3);
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

.results-section {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.summary-card, .distribution-card {
  background: white;
  padding: 1.25rem;
  border-radius: 10px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
}

.summary-card h3, .distribution-card h3 {
  margin: 0 0 0.75rem 0;
  font-size: 1rem;
  font-weight: 600;
  color: var(--text-primary, #1a1a1a);
}

.summary-card p {
  margin: 0;
  color: var(--text-secondary, #4b5563);
  line-height: 1.6;
}

.stance-bars {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.stance-bar {
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

.stance-bar.support .bar-fill { background: #10b981; }
.stance-bar.oppose .bar-fill { background: #ef4444; }
.stance-bar.neutral .bar-fill { background: #6b7280; }

.bar-label {
  position: absolute;
  left: 12px;
  top: 50%;
  transform: translateY(-50%);
  font-size: 0.875rem;
  font-weight: 500;
  color: var(--text-primary, #1a1a1a);
}

.view-controls {
  display: flex;
  gap: 0.5rem;
  padding: 0.25rem;
  background: var(--bg-tertiary, #e5e7eb);
  border-radius: 8px;
  width: fit-content;
}

.view-controls button {
  padding: 0.5rem 1rem;
  background: transparent;
  border: none;
  border-radius: 6px;
  font-size: 0.875rem;
  color: var(--text-secondary, #6b7280);
  cursor: pointer;
  transition: all 0.2s;
}

.view-controls button.active {
  background: white;
  color: var(--text-primary, #1a1a1a);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
}

.grouped-view, .all-responses {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.group-title {
  margin: 0 0 0.75rem 0;
  font-size: 0.875rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.group-title.support { color: #10b981; }
.group-title.oppose { color: #ef4444; }
.group-title.neutral { color: #6b7280; }
.group-title.faction { color: #6366f1; }

.group-title .count {
  font-weight: normal;
  opacity: 0.7;
}

.response-cards {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 1rem;
}

.response-card {
  background: white;
  padding: 1rem;
  border-radius: 8px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
}

.response-card.full {
  grid-column: 1 / -1;
}

.card-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 0.75rem;
}

.agent-name {
  font-weight: 600;
  color: var(--text-primary, #1a1a1a);
}

.faction-tag, .stance-tag {
  padding: 0.125rem 0.5rem;
  border-radius: 4px;
  font-size: 0.75rem;
  font-weight: 500;
}

.faction-tag {
  background: #e0e7ff;
  color: #4338ca;
}

.stance-tag.support { background: #d1fae5; color: #047857; }
.stance-tag.oppose { background: #fee2e2; color: #b91c1c; }
.stance-tag.neutral { background: #f3f4f6; color: #4b5563; }

.response-text {
  margin: 0;
  font-size: 0.875rem;
  color: var(--text-secondary, #4b5563);
  line-height: 1.5;
}

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
