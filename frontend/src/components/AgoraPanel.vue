<template>
  <div class="agora-panel">
    <!-- Warning Banner (shown when debate is active) -->
    <div v-if="debateActive" class="agora-warning-banner">
      <span class="warning-icon">🔴</span>
      <span class="warning-text">Debate in Progress - Agent interactions limited</span>
      <div class="warning-actions">
        <button class="warning-btn pause" @click="pauseDebate" :disabled="debateState?.status === 'paused'">
          ⏸️ Pause
        </button>
        <button class="warning-btn stop" @click="stopDebate">
          ⏹️ Stop
        </button>
      </div>
    </div>

    <!-- Setup Section (when no active debate) -->
    <div v-if="!debateState" class="agora-setup">
      <div class="setup-header">
        <h3>🏛️ Agora - Structured Debate Arena</h3>
        <p class="setup-subtitle">Select a goal, pick agents, and start a moderated debate</p>
      </div>

      <!-- Status Banner (Option 2: Profile-based debates - no env needed) -->
      <div class="env-status-banner alive">
        <div class="env-status-content">
          <span class="env-status-text">
            ✅ Profile-based debates ready - Using stored agent personalities
          </span>
        </div>
      </div>

      <!-- Goal Selection -->
      <div class="setup-section">
        <label class="setup-label">Debate Goal</label>
        <div class="goal-grid">
          <button
            v-for="template in templates"
            :key="template.goal_type"
            class="goal-card"
            :class="{ active: selectedGoal === template.goal_type }"
            @click="selectedGoal = template.goal_type"
          >
            <span class="goal-icon">{{ getGoalIcon(template.goal_type) }}</span>
            <span class="goal-name">{{ template.name }}</span>
            <span class="goal-desc">{{ template.description }}</span>
          </button>
        </div>
      </div>

      <!-- Past Debates (V3) -->
      <div class="past-debates-section" v-if="pastDebates.length > 0">
        <button class="past-debates-toggle" @click="showPastDebates = !showPastDebates">
          📜 Past Debates ({{ pastDebates.length }})
          <span class="toggle-icon">{{ showPastDebates ? '▼' : '▶' }}</span>
        </button>
        <div v-if="showPastDebates" class="past-debates-list">
          <div 
            v-for="debate in pastDebates" 
            :key="debate.debate_id"
            class="past-debate-item"
            @click="loadDebate(debate.debate_id)"
          >
            <div class="past-debate-topic">{{ debate.topic.slice(0, 60) }}{{ debate.topic.length > 60 ? '...' : '' }}</div>
            <div class="past-debate-meta">
              <span class="status-badge" :class="debate.status">{{ debate.status }}</span>
              <span>{{ debate.turn_count }} turns</span>
              <span>Round {{ debate.current_round }}/{{ debate.max_rounds }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Topic Input -->
      <div class="setup-section">
        <label class="setup-label">Debate Topic</label>
        <textarea
          v-model="topic"
          class="topic-input"
          placeholder="Enter the topic or question to debate..."
          rows="2"
        ></textarea>
      </div>

      <!-- Agent Selection -->
      <div class="setup-section">
        <label class="setup-label">Select Debaters (min 2)</label>
        <div class="agent-chips">
          <button
            v-for="(agent, idx) in profiles"
            :key="idx"
            class="agent-chip"
            :class="{ selected: selectedAgents.has(idx) }"
            @click="toggleAgent(idx)"
          >
            <span class="chip-avatar">{{ (agent.username || 'A')[0] }}</span>
            <span class="chip-name">{{ agent.username }}</span>
          </button>
        </div>
        <p class="selection-count">{{ selectedAgents.size }} agents selected</p>
      </div>

      <!-- Options Row -->
      <div class="options-row">
        <div class="option-group">
          <label>Rounds</label>
          <select v-model.number="maxRounds">
            <option :value="3">3 rounds</option>
            <option :value="5">5 rounds</option>
            <option :value="7">7 rounds</option>
            <option :value="10">10 rounds</option>
          </select>
        </div>
        <div class="option-group">
          <label>Mode</label>
          <select v-model="debateMode">
            <option value="continuous">Continuous</option>
            <option value="review">Pause after each round</option>
          </select>
        </div>
        <div class="option-group">
          <label>Round Duration</label>
          <select v-model.number="roundDurationSeconds">
            <option :value="15">15 seconds</option>
            <option :value="30">30 seconds</option>
            <option :value="45">45 seconds</option>
            <option :value="60">60 seconds</option>
            <option :value="90">90 seconds</option>
          </select>
        </div>
      </div>

      <!-- Start Button -->
      <div class="start-section">
        <p class="start-warning">
          ⚠️ Starting a debate will temporarily limit new agent interactions. You can still view existing content and use the Knowledge Pad.
        </p>
        <button
          class="start-btn"
          :disabled="!canStart || isStarting"
          @click="startDebate"
        >
          <span v-if="isStarting">Starting...</span>
          <span v-else>🏛️ Start Debate</span>
        </button>
        <!-- Debug: Show what's missing -->
        <p v-if="!canStart" class="requirements-hint">
          <span v-if="!selectedGoal">⭕ Select a debate goal</span>
          <span v-else-if="!topic.trim()">⭕ Enter a debate topic</span>
          <span v-else-if="selectedAgents.size < 2">⭕ Select at least 2 agents ({{ selectedAgents.size }} selected)</span>
        </p>
      </div>
    </div>

    <!-- Active Debate View -->
    <div v-if="debateState" class="agora-active">
      <div class="agora-split">
        <!-- Left: Transcript (70%) -->
        <div class="debate-transcript">
          <div class="transcript-header">
            <h4>{{ debateState.topic }}</h4>
            <span class="round-indicator">
              Round {{ debateState.current_round }} / {{ debateState.max_rounds }}
            </span>
            <span class="status-badge" :class="debateState.status">
              {{ debateState.status }}
            </span>
          </div>

          <div class="transcript-content" ref="transcriptRef">
            <div v-if="debateState.turns.length === 0" class="transcript-empty">
              <div class="empty-status">
                <span class="pulse-dot"></span>
                <p>{{ isExecutingRound ? 'Agents are preparing their opening statements...' : 'Debate ready to begin.' }}</p>
                <button 
                  v-if="!isExecutingRound" 
                  class="mini-start-btn" 
                  @click="startTimedRound"
                >⚡ Start Timed Round</button>
              </div>
            </div>
            
            <!-- V2: Live streaming indicator with timer -->
            <div v-if="isStreaming" class="live-streaming-bar">
              <div class="timer-display">
                <span class="timer-icon">🕐</span>
                <span class="timer-value">{{ timeRemaining }}s</span>
              </div>
              <div class="speaker-indicator" v-if="currentSpeaker">
                <span class="pulse-dot live"></span>
                <span>{{ currentSpeaker }} is speaking...</span>
              </div>
              <div class="live-badge">LIVE</div>
            </div>
            
            <div v-else-if="isExecutingRound && debateState.turns.length > 0" class="generating-indicator">
              <span class="pulse-dot"></span>
              <span>Agents are responding to the latest points...</span>
            </div>
            <div
              v-for="turn in debateState.turns"
              :key="turn.turn_id"
              class="turn-card"
              :class="turn.stance_label"
              @mouseup="handleTextSelection"
            >
              <div class="turn-header">
                <span class="turn-agent">{{ turn.agent_name }}</span>
                <span class="turn-round">Round {{ turn.round_num }}</span>
                <span class="turn-stance" :class="turn.stance_label">
                  {{ turn.stance_label }}
                </span>
              </div>
              <div class="turn-content">{{ turn.response }}</div>
            </div>
          </div>

          <!-- Pivot Input (if paused) -->
          <div v-if="debateState.status === 'paused'" class="pivot-section">
            <input
              v-model="pivotTopic"
              class="pivot-input"
              placeholder="Optional: Enter a pivot topic to steer the discussion..."
            />
            <button class="continue-btn" @click="continueDebate">
              ▶️ Continue{{ pivotTopic ? ' with Pivot' : '' }}
            </button>
          </div>

          <!-- Auto-run next round (if continuous mode and running) -->
          <div v-if="debateState.status === 'running' && debateState.debate_mode === 'continuous' && !isExecutingRound" class="auto-continue">
            <button class="continue-btn" @click="startTimedRound">
              ▶️ Start Next Timed Round
            </button>
          </div>
        </div>

        <!-- Right: Command Center (30%) -->
        <div class="command-center">
          <h4>📊 Command Center</h4>

          <!-- Live Stats -->
          <div class="stats-card">
            <div class="stat-row">
              <span class="stat-label">Status</span>
              <span class="stat-value" :class="debateState.status">{{ debateState.status }}</span>
            </div>
            <div class="stat-row">
              <span class="stat-label">Turns</span>
              <span class="stat-value">{{ debateState.turns.length }}</span>
            </div>
            <div class="stat-row">
              <span class="stat-label">Agents</span>
              <span class="stat-value">{{ debateState.agent_ids.length }}</span>
            </div>
          </div>

          <!-- Stance Distribution -->
          <div class="stance-card">
            <h5>Stance Distribution</h5>
            <div class="stance-bars">
              <div class="stance-bar support" :style="{ width: stanceStats.support + '%' }">
                {{ stanceStats.support }}%
              </div>
              <div class="stance-bar neutral" :style="{ width: stanceStats.neutral + '%' }">
                {{ stanceStats.neutral }}%
              </div>
              <div class="stance-bar oppose" :style="{ width: stanceStats.oppose + '%' }">
                {{ stanceStats.oppose }}%
              </div>
            </div>
            <div class="stance-legend">
              <span class="legend-item support">Support</span>
              <span class="legend-item neutral">Neutral</span>
              <span class="legend-item oppose">Oppose</span>
            </div>
          </div>

          <!-- Stance Timeline Chart -->
          <div v-if="debateState.turns.length > 1" class="chart-card">
            <h5>📈 Stance Timeline</h5>
            <div class="stance-chart">
              <svg viewBox="0 0 300 120" class="chart-svg">
                <!-- Grid lines -->
                <line x1="30" y1="10" x2="30" y2="100" stroke="#333" stroke-width="1"/>
                <line x1="30" y1="55" x2="290" y2="55" stroke="#444" stroke-width="1" stroke-dasharray="4"/>
                <line x1="30" y1="100" x2="290" y2="100" stroke="#333" stroke-width="1"/>
                
                <!-- Y-axis labels -->
                <text x="5" y="15" fill="#888" font-size="8">+100</text>
                <text x="15" y="58" fill="#888" font-size="8">0</text>
                <text x="5" y="103" fill="#888" font-size="8">-100</text>
                
                <!-- Lines for each agent -->
                <polyline
                  v-for="(line, agentId) in stanceChartLines"
                  :key="agentId"
                  :points="line.points"
                  fill="none"
                  :stroke="line.color"
                  stroke-width="2"
                />
                
                <!-- Data points -->
                <circle
                  v-for="(point, idx) in stanceChartPoints"
                  :key="idx"
                  :cx="point.x"
                  :cy="point.y"
                  r="3"
                  :fill="point.color"
                />
              </svg>
              <div class="chart-legend">
                <span 
                  v-for="(name, id) in debateState.agent_names" 
                  :key="id"
                  class="chart-legend-item"
                  :style="{ color: getAgentColor(id) }"
                >● {{ name }}</span>
              </div>
            </div>
          </div>

          <!-- Summary (when completed/stopped) -->
          <div v-if="debateState.summary" class="summary-card">
            <h5>📋 Debate Summary</h5>
            <div class="summary-content" v-html="formattedSummary"></div>
          </div>

          <!-- V3: Selection Popup for Knowledge Pad -->
          <Teleport to="body">
            <div 
              v-if="selectionPopup" 
              class="agora-selection-popup"
              :style="{ left: selectionPopup.x + 'px', top: selectionPopup.y + 'px' }"
            >
              <button class="add-to-knowledge-btn" @click="openTagModal">
                📝 Add to Knowledge
              </button>
              <button class="close-popup-btn" @click="closeSelectionPopup">✕</button>
            </div>
          </Teleport>

          <!-- V3: Tag Modal for Knowledge Pad -->
          <Teleport to="body">
            <div v-if="showTagModal" class="agora-tag-modal-overlay" @click.self="closeTagModal">
              <div class="agora-tag-modal" @click.stop>
                <div class="tag-modal-header">
                  <h3>📋 Add to Knowledge Pad</h3>
                  <button class="tag-modal-close" @click="closeTagModal">×</button>
                </div>
                <div class="tag-modal-body">
                  <div class="tag-preview">
                    <p class="preview-text">"{{ pendingHighlight?.text?.substring(0, 150) }}{{ pendingHighlight?.text?.length > 150 ? '...' : '' }}"</p>
                    <p class="preview-source">From: {{ pendingHighlight?.source }}</p>
                  </div>
                  
                  <div class="tag-section">
                    <label class="tag-label">Select Tags:</label>
                    <div class="predefined-tags">
                      <button 
                        v-for="tag in predefinedTags" 
                        :key="tag"
                        class="tag-chip"
                        :class="{ selected: selectedTags.has(tag) }"
                        @click="toggleTag(tag)"
                      >{{ tag }}</button>
                    </div>
                  </div>
                  
                  <div class="custom-tag-section">
                    <label class="tag-label">Add Custom Tag:</label>
                    <div class="custom-tag-input">
                      <input 
                        v-model="customTagInput" 
                        placeholder="Type and press Enter"
                        @keydown.enter.prevent="addCustomTag"
                      />
                      <button class="add-tag-btn" @click="addCustomTag">+</button>
                    </div>
                  </div>
                  
                  <div v-if="selectedTags.size > 0" class="selected-tags-preview">
                    <span class="tags-label">Selected:</span>
                    <span v-for="tag in selectedTags" :key="tag" class="selected-tag">
                      {{ tag }}
                      <button @click="toggleTag(tag)">×</button>
                    </span>
                  </div>
                </div>
                <div class="tag-modal-footer">
                  <button class="tag-btn secondary" @click="closeTagModal">Cancel</button>
                  <button class="tag-btn primary" @click="confirmAddToKnowledge">
                    Add to Knowledge Pad
                  </button>
                </div>
              </div>
            </div>
          </Teleport>
          
          <!-- Round Summary Modal -->
          <Teleport to="body">
            <div v-if="showRoundSummary && currentRoundSummary" class="agora-tag-modal-overlay">
              <div class="agora-tag-modal round-summary-modal">
                <div class="tag-modal-header">
                  <h3>Round {{ currentRoundSummary.round }} Summary</h3>
                  <button class="tag-modal-close" @click="showRoundSummary = false">×</button>
                </div>
                <div class="tag-modal-body">
                  <div class="summary-section">
                    <p class="round-summary-text">{{ currentRoundSummary.summary }}</p>
                  </div>
                  
                  <div v-if="currentRoundSummary.key_points && currentRoundSummary.key_points.length > 0" class="key-points-section">
                    <span class="tag-label">Key Points:</span>
                    <ul class="round-key-points">
                      <li v-for="(point, idx) in currentRoundSummary.key_points" :key="idx">
                        {{ point }}
                      </li>
                    </ul>
                  </div>
                </div>
                <div class="tag-modal-footer">
                  <button class="tag-btn primary" @click="showRoundSummary = false">
                    Continue to Next Round
                  </button>
                </div>
              </div>
            </div>
          </Teleport>

          <!-- Actions -->
          <div class="action-buttons">
            <button
              v-if="debateState.status === 'running'"
              class="action-btn pause"
              @click="pauseDebate"
            >⏸️ Pause</button>
            <button
              v-if="debateState.status === 'paused'"
              class="action-btn resume"
              @click="resumeDebate"
            >▶️ Resume</button>
            <button
              v-if="['running', 'paused'].includes(debateState.status)"
              class="action-btn stop"
              :class="{ loading: isStopping }"
              :disabled="isStopping"
              @click="stopDebate"
            >{{ isStopping ? '⏳ Generating Summary...' : '⏹️ Stop' }}</button>
            <button
              v-if="['completed', 'stopped'].includes(debateState.status)"
              class="action-btn new"
              @click="resetDebate"
            >🔄 New Debate</button>
            <button
              v-if="['completed', 'stopped'].includes(debateState.status)"
              class="action-btn export"
              @click="exportDebate"
            >📥 Export JSON</button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, watch, onMounted, nextTick } from 'vue'
import {
  getAgoraTemplates,
  createAgoraDebate,
  executeAgoraRound,
  getAgoraStatus,
  pauseAgoraDebate,
  resumeAgoraDebate,
  stopAgoraDebate,
  getEnvStatus,
  startSimulation,
  getStreamAgoraRoundUrl,
  listAgoraDebates,
  getAgoraDebate
} from '../api/simulation'

const props = defineProps({
  simulationId: String,
  profiles: {
    type: Array,
    default: () => []
  }
})

const emit = defineEmits(['debate-started', 'debate-ended', 'add-log', 'add-to-knowledge'])

// Setup State
const templates = ref([])
const selectedGoal = ref('stress_test')
const topic = ref('')
const selectedAgents = ref(new Set())
const maxRounds = ref(5)
const debateMode = ref('continuous')
const turnTimeout = ref(60)  // Timeout per turn in seconds
const isStarting = ref(false)

// Environment State
const envAlive = ref(false)
const envChecking = ref(false)
const envStarting = ref(false)

// Debate State
const debateState = ref(null)
const pivotTopic = ref('')
const isExecutingRound = ref(false)
const transcriptRef = ref(null)

// V2: Timed Round State
const roundDurationSeconds = ref(30)
const timeRemaining = ref(0)
const isStreaming = ref(false)
const currentSpeaker = ref(null)
const timerInterval = ref(null)
const eventSourceRef = ref(null)
const isStopping = ref(false)

// V3: Debate History
const pastDebates = ref([])
const showPastDebates = ref(false)

// V3: Text Selection for Knowledge Pad
const selectionPopup = ref(null)
const showTagModal = ref(false)
const pendingHighlight = ref(null)
const selectedTags = ref(new Set())
const customTagInput = ref('')
const predefinedTags = ['key_insight', 'quote', 'evidence', 'argument', 'counter_argument', 'consensus', 'disagreement']

// V3: Round Summaries
const showRoundSummary = ref(false)
const currentRoundSummary = ref(null)

// Computed
const debateActive = computed(() => {
  return debateState.value && ['running', 'paused'].includes(debateState.value.status)
})

// V2: Convert markdown summary to HTML
const formattedSummary = computed(() => {
  if (!debateState.value?.summary) return ''
  
  // Simple markdown-to-HTML conversion
  let html = debateState.value.summary
    // Headers
    .replace(/^### (.+)$/gm, '<h4>$1</h4>')
    .replace(/^## (.+)$/gm, '<h3>$1</h3>')
    // Bold
    .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
    // Tables (basic)
    .replace(/\| Position \| Count \|/g, '<table class="stance-table"><tr><th>Position</th><th>Count</th></tr>')
    .replace(/\|[-]+\|[-]+\|/g, '')
    .replace(/\| (Support|Oppose|Neutral) +\| (\d+) +\|/g, '<tr><td>$1</td><td>$2</td></tr>')
    // Horizontal rule
    .replace(/^---$/gm, '<hr/>')
    // Paragraphs (double newline)
    .replace(/\n\n/g, '</p><p>')
    // Single newlines to <br>
    .replace(/\n/g, '<br/>')
  
  // Close table if opened
  if (html.includes('<table')) {
    html = html.replace(/<hr\/>/g, '</table><hr/>')
  }
  
  return '<p>' + html + '</p>'
})

// Agent colors for chart
const agentColors = ['#3b82f6', '#ef4444', '#22c55e', '#f59e0b', '#ff5722', '#ec4899']
const getAgentColor = (agentId) => {
  const ids = Object.keys(debateState.value?.agent_names || {})
  const idx = ids.indexOf(String(agentId))
  return agentColors[idx % agentColors.length]
}

// Stance chart data - lines per agent
const stanceChartLines = computed(() => {
  if (!debateState.value?.turns?.length) return {}
  
  const lines = {}
  const agentIds = Object.keys(debateState.value.agent_names)
  const turns = debateState.value.turns
  const totalTurns = turns.length
  
  agentIds.forEach(agentId => {
    const agentTurns = turns.filter(t => String(t.agent_id) === String(agentId))
    if (agentTurns.length < 1) return
    
    const points = agentTurns.map((turn, idx) => {
      const turnIdx = turns.indexOf(turn)
      const x = 30 + ((turnIdx / (totalTurns - 1 || 1)) * 260)
      // Map stance_score (-100 to 100) to y (100 to 10)
      const y = 55 - (turn.stance_score * 0.45)
      return `${x},${y}`
    }).join(' ')
    
    lines[agentId] = {
      points,
      color: getAgentColor(agentId)
    }
  })
  
  return lines
})

// Individual points for chart
const stanceChartPoints = computed(() => {
  if (!debateState.value?.turns?.length) return []
  
  const points = []
  const turns = debateState.value.turns
  const totalTurns = turns.length
  
  turns.forEach((turn, idx) => {
    const x = 30 + ((idx / (totalTurns - 1 || 1)) * 260)
    const y = 55 - (turn.stance_score * 0.45)
    points.push({
      x,
      y,
      color: getAgentColor(turn.agent_id)
    })
  })
  
  return points
})

// Auto-trigger first round for new debates (V2: use timed rounds)
watch(debateState, (newState, oldState) => {
  if (newState && !oldState && newState.turns.length === 0 && !isExecutingRound.value) {
    console.log('[Agora] New debate detected with 0 turns, auto-triggering first timed round...')
    startTimedRound()
  }
})

const canStart = computed(() => {
  // Option 2: Profile-based debates don't require live environment
  const result = selectedGoal.value && topic.value.trim() && selectedAgents.value.size >= 2
  console.log('[Agora] canStart check:', { 
    selectedGoal: selectedGoal.value, 
    topicFilled: !!topic.value.trim(), 
    agentCount: selectedAgents.value.size,
    result 
  })
  return result
})

const stanceStats = computed(() => {
  if (!debateState.value || !debateState.value.turns.length) {
    return { support: 0, neutral: 100, oppose: 0 }
  }

  const turns = debateState.value.turns
  let support = 0, oppose = 0, neutral = 0

  turns.forEach(t => {
    if (t.stance_label === 'support') support++
    else if (t.stance_label === 'oppose') oppose++
    else neutral++
  })

  const total = turns.length
  return {
    support: Math.round((support / total) * 100),
    neutral: Math.round((neutral / total) * 100),
    oppose: Math.round((oppose / total) * 100)
  }
})

// Methods
const getGoalIcon = (goalType) => {
  const icons = {
    stress_test: '🎯',
    risk_id: '⚠️',
    stakeholder: '👥',
    competitive: '⚔️',
    consensus: '🤝',
    socratic: '🔍'
  }
  return icons[goalType] || '💬'
}

const toggleAgent = (idx) => {
  const newSet = new Set(selectedAgents.value)
  if (newSet.has(idx)) {
    newSet.delete(idx)
  } else {
    newSet.add(idx)
  }
  selectedAgents.value = newSet
}

const loadTemplates = async () => {
  // Fallback templates in case API fails
  const fallbackTemplates = [
    { goal_type: 'stress_test', name: 'Point-Counterpoint', description: 'Stress-test a decision' },
    { goal_type: 'risk_id', name: "Devil's Advocate", description: 'Identify risks & blind spots' },
    { goal_type: 'stakeholder', name: 'Stakeholder Caucus', description: 'Understand perspectives' },
    { goal_type: 'competitive', name: 'Red Team / Blue Team', description: 'Competitive analysis' },
    { goal_type: 'consensus', name: 'Consensus Building', description: 'Find middle ground' },
    { goal_type: 'socratic', name: 'Socratic Drill', description: 'Expose assumptions' }
  ]
  
  try {
    console.log('[Agora] Loading templates...')
    const response = await getAgoraTemplates()
    console.log('[Agora] Templates response:', response)
    if (response.success && response.data?.length > 0) {
      templates.value = response.data
      console.log('[Agora] Templates loaded:', templates.value.length)
    } else {
      console.warn('[Agora] API returned no templates, using fallback')
      templates.value = fallbackTemplates
    }
  } catch (error) {
    console.error('[Agora] Failed to load templates, using fallback:', error)
    templates.value = fallbackTemplates
  }
}

const startDebate = async () => {
  emit('add-log', '[Agora] Clicked Start Debate')
  
  const canStartVal = canStart.value
  console.log('[Agora] Attempting to start debate...', { 
    canStart: canStartVal, 
    isStarting: isStarting.value,
    topic: topic.value,
    agents: Array.from(selectedAgents.value)
  })

  if (!canStartVal || isStarting.value) {
    const reason = !canStartVal ? "requirements not met" : "already starting"
    console.warn(`[Agora] Cannot start debate: ${reason}`)
    emit('add-log', `[Agora] Cannot start: ${reason}`)
    return
  }

  isStarting.value = true
  emit('add-log', '[Agora] Starting debate creation...')
  
  try {
    const agentNames = {}
    selectedAgents.value.forEach(idx => {
      const profile = props.profiles[idx]
      // Try multiple name fields from profile
      const name = profile?.username || profile?.display_name || profile?.screen_name || profile?.name || `Agent_${idx}`
      agentNames[idx] = name
      console.log(`[Agora] Agent ${idx} name: ${name}`, profile)
    })

    const payload = {
      simulation_id: props.simulationId,
      topic: topic.value,
      goal_type: selectedGoal.value,
      agent_ids: Array.from(selectedAgents.value),
      agent_names: agentNames,
      max_rounds: maxRounds.value,
      debate_mode: debateMode.value,
      round_duration_seconds: roundDurationSeconds.value,
      moderator_mode: 'user_only'
    }

    console.log('[Agora] sending POST /api/simulation/agora/create', payload)
    const response = await createAgoraDebate(payload)
    console.log('[Agora] response status:', response.status || 'Success')

    if (response.success && response.data) {
      const state = response.data
      console.log('[Agora] Success! Debate ID:', state.debate_id)
      emit('add-log', `[Agora] Debate created: ${state.debate_id}`)
      
      // Forces re-render by setting to null first then the state
      debateState.value = null
      await nextTick()
      debateState.value = state
      
      console.log('[Agora] debateState set, transitioning UI...')
      emit('debate-started', state.debate_id)
    } else {
      const errorMsg = response.error || 'Unknown server error'
      console.error('[Agora] API Error:', errorMsg)
      emit('add-log', `[⚠️ Agora] Create failed: ${errorMsg}`)
    }
  } catch (error) {
    console.error('[Agora] Network/JS Exception:', error)
    emit('add-log', `[❌ Agora] Error: ${error.message}`)
  } finally {
    // Keep button disabled for 2s to prevent double-starts
    setTimeout(() => {
      isStarting.value = false
      console.log('[Agora] Ready for next action')
    }, 2000)
  }
}

const executeNextRound = async () => {
  if (!debateState.value || isExecutingRound.value) {
    console.warn('[Agora] Cannot execute round:', { 
      hasState: !!debateState.value, 
      isExecuting: isExecutingRound.value 
    })
    return
  }

  console.log('[Agora] Executing round...', debateState.value.current_round + 1)
  isExecutingRound.value = true
  try {
    const response = await executeAgoraRound(debateState.value.debate_id, {
      pivot_topic: pivotTopic.value
    })
    
    if (response.success && response.data) {
      // response.data contains { debate_id, round, turns, status, is_complete }
      const roundData = response.data
      
      // Merge new turns into state
      if (roundData.turns && Array.from(roundData.turns).length > 0) {
        // If the response turns are only for the current round, we should append them
        // But based on my check, we should probably fetch the FULL state to be safe
        // For now, we'll just update the debateState with the new data
        const newState = { ...debateState.value, ...roundData }
        debateState.value = newState
      } else {
        // If no turns in response, just update the state
        debateState.value = roundData
      }
      
      console.log('[Agora] Round complete, refreshing full state...')
      await refreshDebateState()
      
      // V3: Show round summary
      if (roundData.round_summary) {
        currentRoundSummary.value = roundData.round_summary
        showRoundSummary.value = true
      }
      
      pivotTopic.value = ''

      // Scroll to bottom
      await nextTick()
      if (transcriptRef.value) {
        transcriptRef.value.scrollTop = transcriptRef.value.scrollHeight
      }
    } else {
      console.error('[Agora] Round execution failed:', response.error)
    }
  } catch (error) {
    console.error('[Agora] Exception in executeNextRound:', error)
  } finally {
    isExecutingRound.value = false
    console.log('[Agora] isExecutingRound reset to false')
  }
}

/**
 * V2: Start a timed round with real-time SSE streaming
 * Agents exchange messages A→B→A→B for the specified duration
 */
const startTimedRound = async () => {
  if (!debateState.value || isStreaming.value) return
  
  const debateId = debateState.value.debate_id
  const duration = roundDurationSeconds.value
  
  console.log(`[Agora] Starting timed round for ${duration}s`)
  emit('add-log', `[Agora] Starting ${duration}s timed round...`)
  
  isStreaming.value = true
  isExecutingRound.value = true
  timeRemaining.value = duration
  
  // Start countdown timer
  timerInterval.value = setInterval(() => {
    if (timeRemaining.value > 0) {
      timeRemaining.value--
    } else {
      clearInterval(timerInterval.value)
    }
  }, 1000)
  
  try {
    // Use fetch with POST body for SSE (EventSource doesn't support POST body)
    const streamUrl = getStreamAgoraRoundUrl(debateId)
    const response = await fetch(streamUrl, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        round_duration_seconds: duration,
        pivot_topic: pivotTopic.value || undefined
      })
    })
    
    if (!response.ok) {
      throw new Error(`Stream failed: ${response.status}`)
    }
    
    const reader = response.body.getReader()
    const decoder = new TextDecoder()
    let buffer = ''
    
    while (true) {
      const { done, value } = await reader.read()
      if (done) break
      
      buffer += decoder.decode(value, { stream: true })
      
      // Process complete SSE messages
      const lines = buffer.split('\n')
      buffer = lines.pop() // Keep incomplete line in buffer
      
      for (const line of lines) {
        if (line.startsWith('data: ')) {
          try {
            const data = JSON.parse(line.slice(6))
            
            if (data._type === 'round_complete') {
              // Round completed
              console.log(`[Agora] Round ${data.round} complete: ${data.exchanges} exchanges in ${data.duration_seconds.toFixed(1)}s`)
              emit('add-log', `[Agora] Round ${data.round} complete (${data.exchanges} exchanges)`)
              
              // V3: Show round summary
              if (data.round_summary) {
                currentRoundSummary.value = data.round_summary
                showRoundSummary.value = true
              }
              
              // Update debate state
              await refreshDebateState()
            } else if (data._type === 'error') {
              console.error('[Agora] Stream error:', data.error)
              emit('add-log', `[⚠️ Agora] Error: ${data.error}`)
            } else {
              // Regular turn - add to transcript
              currentSpeaker.value = data.agent_name
              
              // Add turn to local state for immediate display
              if (!debateState.value.turns) {
                debateState.value.turns = []
              }
              debateState.value.turns.push(data)
              
              // Trigger reactivity
              debateState.value = { ...debateState.value }
              
              // Scroll to bottom
              await nextTick()
              if (transcriptRef.value) {
                transcriptRef.value.scrollTop = transcriptRef.value.scrollHeight
              }
            }
          } catch (e) {
            console.warn('[Agora] Failed to parse SSE data:', e)
          }
        }
      }
    }
    
    pivotTopic.value = ''
    
  } catch (error) {
    console.error('[Agora] Exception in startTimedRound:', error)
    emit('add-log', `[❌ Agora] Streaming error: ${error.message}`)
  } finally {
    isStreaming.value = false
    isExecutingRound.value = false
    currentSpeaker.value = null
    clearInterval(timerInterval.value)
    console.log('[Agora] Timed round complete')
  }
}

const refreshDebateState = async () => {
  if (!debateState.value) return

  try {
    const response = await getAgoraStatus(debateState.value.debate_id)
    if (response.success) {
      debateState.value = response.data
    }
  } catch (error) {
    console.error('Failed to refresh debate state:', error)
  }
}

const pauseDebate = async () => {
  if (!debateState.value) return

  try {
    const response = await pauseAgoraDebate(debateState.value.debate_id)
    if (response.success) {
      debateState.value = response.data
      emit('add-log', 'Debate paused')
    }
  } catch (error) {
    console.error('Failed to pause debate:', error)
  }
}

const resumeDebate = async () => {
  if (!debateState.value) return

  try {
    const response = await resumeAgoraDebate(debateState.value.debate_id)
    if (response.success) {
      debateState.value = response.data
      emit('add-log', 'Debate resumed')
    }
  } catch (error) {
    console.error('Failed to resume debate:', error)
  }
}

const continueDebate = async () => {
  await resumeDebate()
  await startTimedRound()
}

const stopDebate = async () => {
  if (!debateState.value || isStopping.value) return

  isStopping.value = true
  emit('add-log', '[Agora] Stopping debate and generating summary...')
  
  try {
    const response = await stopAgoraDebate(debateState.value.debate_id, {
      generate_summary: true
    })
    if (response.success) {
      debateState.value = response.data
      emit('debate-ended', debateState.value.debate_id)
      emit('add-log', '[Agora] Debate stopped successfully')
    }
  } catch (error) {
    console.error('Failed to stop debate:', error)
    emit('add-log', '[Agora] Error stopping debate: ' + error.message)
  } finally {
    isStopping.value = false
  }
}

const resetDebate = () => {
  debateState.value = null
  topic.value = ''
  selectedAgents.value = new Set()
  pivotTopic.value = ''
}

// Export debate as JSON
const exportDebate = () => {
  if (!debateState.value) return
  
  const exportData = {
    debate_id: debateState.value.debate_id,
    topic: debateState.value.topic,
    goal_type: debateState.value.goal_type,
    participants: debateState.value.agent_names,
    status: debateState.value.status,
    rounds: debateState.value.current_round,
    max_rounds: debateState.value.max_rounds,
    turns: debateState.value.turns.map(t => ({
      round: t.round_num,
      agent_id: t.agent_id,
      agent_name: t.agent_name,
      response: t.response,
      stance_score: t.stance_score,
      stance_label: t.stance_label
    })),
    stance_history: debateState.value.stance_history,
    summary: debateState.value.summary,
    exported_at: new Date().toISOString()
  }
  
  const blob = new Blob([JSON.stringify(exportData, null, 2)], { type: 'application/json' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `debate_${debateState.value.debate_id}_${Date.now()}.json`
  document.body.appendChild(a)
  a.click()
  document.body.removeChild(a)
  URL.revokeObjectURL(url)
  
  emit('add-log', '[Agora] Debate exported to JSON')
}

// V3: Text Selection for Knowledge Pad
const handleTextSelection = (event) => {
  const selection = window.getSelection()
  const text = selection.toString().trim()
  
  if (text.length < 10) {
    selectionPopup.value = null
    return
  }
  
  // Get position for popup
  const range = selection.getRangeAt(0)
  const rect = range.getBoundingClientRect()
  
  selectionPopup.value = {
    text,
    x: rect.left + (rect.width / 2),
    y: rect.top - 10
  }
}

// Open tag modal from selection popup
const openTagModal = () => {
  if (!selectionPopup.value) return
  pendingHighlight.value = {
    text: selectionPopup.value.text,
    source: `Agora Debate: ${debateState.value?.topic || 'Unknown topic'}`
  }
  showTagModal.value = true
  selectedTags.value = new Set()
  customTagInput.value = ''
  selectionPopup.value = null
}

// Close tag modal
const closeTagModal = () => {
  showTagModal.value = false
  pendingHighlight.value = null
  window.getSelection().removeAllRanges()
}

// Toggle tag selection
const toggleTag = (tag) => {
  if (selectedTags.value.has(tag)) {
    selectedTags.value.delete(tag)
  } else {
    selectedTags.value.add(tag)
  }
  selectedTags.value = new Set(selectedTags.value) // trigger reactivity
}

// Add custom tag
const addCustomTag = () => {
  const tag = customTagInput.value.trim().toLowerCase().replace(/\s+/g, '_')
  if (tag && !selectedTags.value.has(tag)) {
    selectedTags.value.add(tag)
    selectedTags.value = new Set(selectedTags.value)
    customTagInput.value = ''
  }
}

// Confirm and add to Knowledge Pad
const confirmAddToKnowledge = () => {
  if (!pendingHighlight.value) return
  
  emit('add-to-knowledge', {
    content: pendingHighlight.value.text,
    source: pendingHighlight.value.source,
    tags: Array.from(selectedTags.value)
  })
  
  emit('add-log', `[Agora] Added to Knowledge: "${pendingHighlight.value.text.substring(0, 40)}..." with ${selectedTags.value.size} tag(s)`)
  closeTagModal()
}

const closeSelectionPopup = () => {
  selectionPopup.value = null
}

// V3: Load past debates for this simulation
const loadPastDebates = async () => {
  if (!props.simulationId) return
  
  try {
    const response = await listAgoraDebates(props.simulationId)
    if (response.success) {
      pastDebates.value = response.data || []
      console.log(`[Agora] Loaded ${pastDebates.value.length} past debates`)
    }
  } catch (error) {
    console.error('[Agora] Failed to load past debates:', error)
  }
}

// V3: Load a specific debate from history
const loadDebate = async (debateId) => {
  try {
    emit('add-log', `[Agora] Loading debate ${debateId}...`)
    const response = await getAgoraDebate(debateId)
    if (response.success) {
      debateState.value = response.data
      showPastDebates.value = false
      emit('add-log', `[Agora] Loaded debate: ${response.data.topic}`)
    }
  } catch (error) {
    console.error('[Agora] Failed to load debate:', error)
    emit('add-log', `[Agora] Error: ${error.message}`)
  }
}

// Environment functions
const checkEnvStatus = async () => {
  if (!props.simulationId) {
    console.warn('[Agora] No simulation ID')
    return
  }
  
  envChecking.value = true
  try {
    console.log('[Agora] Checking env status for:', props.simulationId)
    const response = await getEnvStatus({ simulation_id: props.simulationId })
    console.log('[Agora] Env status response:', response)
    if (response.success) {
      envAlive.value = response.data?.env_alive === true
      console.log('[Agora] Env alive:', envAlive.value)
    }
  } catch (error) {
    console.error('[Agora] Failed to check env status:', error)
    envAlive.value = false
  } finally {
    envChecking.value = false
  }
}

const restartEnv = async () => {
  if (!props.simulationId) return
  
  envStarting.value = true
  emit('add-log', 'Starting simulation environment...')
  
  try {
    console.log('[Agora] Starting simulation environment...')
    const response = await startSimulation({
      simulation_id: props.simulationId,
      max_rounds: 1  // Minimum 1 round required by API, but we just need env to start
    })
    
    console.log('[Agora] Start response:', response)
    
    if (response.success) {
      emit('add-log', 'Environment started successfully!')
      // Wait a moment for env to fully initialize
      await new Promise(resolve => setTimeout(resolve, 2000))
      // Recheck status
      await checkEnvStatus()
    } else {
      emit('add-log', 'Failed to start environment: ' + (response.error || 'Unknown error'))
    }
  } catch (error) {
    console.error('[Agora] Failed to start env:', error)
    emit('add-log', 'Error starting environment: ' + error.message)
  } finally {
    envStarting.value = false
  }
}

// Lifecycle
onMounted(() => {
  loadTemplates()
  checkEnvStatus()
  loadPastDebates()
})

// Watch for simulationId changes (it might be set after mount)
watch(() => props.simulationId, (newId, oldId) => {
  console.log('[Agora] simulationId changed:', oldId, '->', newId)
  if (newId && newId !== oldId) {
    checkEnvStatus()
  }
}, { immediate: true })
</script>

<style scoped>
.agora-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: linear-gradient(135deg, #1a1a2e 0%, #16213e 100%);
  color: #e4e4e7;
}

/* Environment Status Banner */
.env-status-banner {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  border-radius: 10px;
  margin-bottom: 20px;
  transition: all 0.3s;
}

.env-status-banner.alive {
  background: linear-gradient(90deg, rgba(34, 197, 94, 0.15), rgba(34, 197, 94, 0.05));
  border: 1px solid rgba(34, 197, 94, 0.3);
}

.env-status-banner.dead {
  background: linear-gradient(90deg, rgba(251, 191, 36, 0.15), rgba(251, 191, 36, 0.05));
  border: 1px solid rgba(251, 191, 36, 0.3);
}

.env-status-content {
  flex: 1;
}

.env-status-text {
  font-size: 13px;
  color: #d4d4d8;
}

.env-start-btn {
  padding: 8px 16px;
  background: linear-gradient(135deg, #f59e0b, #d97706);
  border: none;
  border-radius: 8px;
  color: white;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
}

.env-start-btn:hover:not(:disabled) {
  transform: translateY(-1px);
  box-shadow: 0 2px 8px rgba(245, 158, 11, 0.3);
}

.env-start-btn:disabled {
  opacity: 0.6;
  cursor: wait;
}

/* V3: Past Debates Section */
.past-debates-section {
  margin-bottom: 16px;
}

.past-debates-toggle {
  width: 100%;
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 14px;
  background: rgba(124, 58, 237, 0.1);
  border: 1px solid rgba(124, 58, 237, 0.3);
  border-radius: 8px;
  color: #c4b5fd;
  cursor: pointer;
  font-size: 13px;
  font-weight: 500;
  transition: all 0.2s;
}

.past-debates-toggle:hover {
  background: rgba(124, 58, 237, 0.2);
}

.toggle-icon {
  font-size: 10px;
}

.past-debates-list {
  margin-top: 8px;
  max-height: 200px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.past-debate-item {
  padding: 10px 12px;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s;
}

.past-debate-item:hover {
  background: rgba(124, 58, 237, 0.15);
  border-color: rgba(124, 58, 237, 0.3);
}

.past-debate-topic {
  font-size: 12px;
  color: #e5e5e5;
  margin-bottom: 4px;
}

.past-debate-meta {
  display: flex;
  gap: 10px;
  font-size: 10px;
  color: #71717a;
}

.status-badge {
  padding: 1px 6px;
  border-radius: 3px;
  font-size: 9px;
  text-transform: uppercase;
  font-weight: 600;
}

.status-badge.running { background: #22c55e; color: white; }
.status-badge.paused { background: #f59e0b; color: white; }
.status-badge.completed { background: #3b82f6; color: white; }
.status-badge.stopped { background: #71717a; color: white; }

.env-refresh-btn {
  padding: 8px;
  background: rgba(255, 255, 255, 0.1);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 6px;
  cursor: pointer;
  font-size: 14px;
  transition: all 0.2s;
}

.env-refresh-btn:hover {
  background: rgba(255, 255, 255, 0.15);
}

/* Warning Banner */
.agora-warning-banner {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  background: linear-gradient(90deg, #dc262622, #dc262611);
  border-bottom: 1px solid #dc262644;
}

.warning-icon {
  animation: pulse 1.5s infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}

.warning-text {
  flex: 1;
  font-size: 14px;
  color: #fca5a5;
}

.warning-actions {
  display: flex;
  gap: 8px;
}

.warning-btn {
  padding: 6px 12px;
  border-radius: 6px;
  border: none;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.2s;
}

.warning-btn.pause {
  background: #3b82f6;
  color: white;
}

.warning-btn.stop {
  background: #dc2626;
  color: white;
}

/* Setup Section */
.agora-setup {
  padding: 24px;
  overflow-y: auto;
}

.setup-header {
  margin-bottom: 24px;
}

.setup-header h3 {
  font-size: 20px;
  margin: 0 0 8px 0;
  color: #f4f4f5;
}

.setup-subtitle {
  font-size: 14px;
  color: #a1a1aa;
  margin: 0;
}

.setup-section {
  margin-bottom: 20px;
}

.setup-label {
  display: block;
  font-size: 13px;
  font-weight: 500;
  color: #d4d4d8;
  margin-bottom: 10px;
}

/* Goal Grid */
.goal-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 12px;
}

.goal-card {
  display: flex;
  flex-direction: column;
  padding: 14px;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 10px;
  cursor: pointer;
  transition: all 0.2s;
  text-align: left;
}

.goal-card:hover {
  background: rgba(255, 255, 255, 0.06);
  border-color: rgba(255, 255, 255, 0.15);
}

.goal-card.active {
  background: rgba(59, 130, 246, 0.15);
  border-color: #3b82f6;
}

.goal-icon {
  font-size: 20px;
  margin-bottom: 6px;
}

.goal-name {
  font-size: 13px;
  font-weight: 500;
  color: #f4f4f5;
  margin-bottom: 4px;
}

.goal-desc {
  font-size: 11px;
  color: #a1a1aa;
  line-height: 1.4;
}

/* Topic Input */
.topic-input {
  width: 100%;
  padding: 12px;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 8px;
  color: #f4f4f5;
  font-size: 14px;
  resize: none;
}

.topic-input:focus {
  outline: none;
  border-color: #3b82f6;
}

/* Agent Chips */
.agent-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  max-height: 150px;
  overflow-y: auto;
}

.agent-chip {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 10px;
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 20px;
  cursor: pointer;
  transition: all 0.2s;
}

.agent-chip:hover {
  background: rgba(255, 255, 255, 0.1);
}

.agent-chip.selected {
  background: rgba(34, 197, 94, 0.2);
  border-color: #22c55e;
}

.chip-avatar {
  width: 20px;
  height: 20px;
  background: linear-gradient(135deg, #3b82f6, #ff5722);
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 10px;
  font-weight: 600;
}

.chip-name {
  font-size: 12px;
  color: #e4e4e7;
}

.selection-count {
  font-size: 12px;
  color: #71717a;
  margin-top: 8px;
}

/* Options Row */
.options-row {
  display: flex;
  gap: 16px;
  margin-bottom: 20px;
}

.option-group {
  flex: 1;
}

.option-group label {
  display: block;
  font-size: 12px;
  color: #a1a1aa;
  margin-bottom: 6px;
}

.option-group select {
  width: 100%;
  padding: 8px 12px;
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 6px;
  color: #e4e4e7;
  font-size: 13px;
}

/* Start Section */
.start-section {
  margin-top: 24px;
  padding-top: 20px;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
}

.start-warning {
  font-size: 12px;
  color: #fbbf24;
  background: rgba(251, 191, 36, 0.1);
  padding: 10px;
  border-radius: 6px;
  margin-bottom: 16px;
}

.start-btn {
  width: 100%;
  padding: 14px;
  background: linear-gradient(135deg, #3b82f6, #ff5722);
  border: none;
  border-radius: 10px;
  color: white;
  font-size: 15px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
}

.start-btn:hover:not(:disabled) {
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(59, 130, 246, 0.3);
}

.start-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.requirements-hint {
  margin-top: 12px;
  font-size: 12px;
  color: #f59e0b;
  text-align: center;
}

/* Active Debate */
.agora-active {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.agora-split {
  flex: 1;
  display: flex;
  overflow: hidden;
}

/* Transcript (70%) */
.debate-transcript {
  flex: 0 0 70%;
  display: flex;
  flex-direction: column;
  border-right: 1px solid rgba(255, 255, 255, 0.08);
}

.transcript-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 16px;
  background: rgba(0, 0, 0, 0.2);
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
}

.transcript-header h4 {
  flex: 1;
  margin: 0;
  font-size: 14px;
  color: #f4f4f5;
}

.round-indicator {
  font-size: 12px;
  color: #a1a1aa;
  background: rgba(255, 255, 255, 0.05);
  padding: 4px 10px;
  border-radius: 12px;
}

.status-badge {
  font-size: 11px;
  padding: 4px 8px;
  border-radius: 4px;
  text-transform: uppercase;
}

.status-badge.running { background: #22c55e22; color: #22c55e; }
.status-badge.paused { background: #f59e0b22; color: #f59e0b; }
.status-badge.completed { background: #3b82f622; color: #3b82f6; }
.status-badge.stopped { background: #71717a22; color: #71717a; }

.transcript-content {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
}

.transcript-empty {
  text-align: center;
  color: #71717a;
  padding: 60px 40px;
}

.empty-status {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
}

.pulse-dot {
  width: 10px;
  height: 10px;
  background-color: #3b82f6;
  border-radius: 50%;
  animation: pulse-ring 1.5s infinite;
}

@keyframes pulse-ring {
  0% { transform: scale(0.8); box-shadow: 0 0 0 0 rgba(59, 130, 246, 0.7); }
  70% { transform: scale(1); box-shadow: 0 0 0 10px rgba(59, 130, 246, 0); }
  100% { transform: scale(0.8); box-shadow: 0 0 0 0 rgba(59, 130, 246, 0); }
}

.generating-indicator {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px;
  background: rgba(59, 130, 246, 0.1);
  border: 1px dashed rgba(59, 130, 246, 0.3);
  border-radius: 8px;
  margin-bottom: 16px;
  font-size: 13px;
  color: #93c5fd;
}

/* V2: Live Streaming Bar */
.live-streaming-bar {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 12px 16px;
  background: linear-gradient(135deg, rgba(239, 68, 68, 0.15), rgba(236, 72, 153, 0.15));
  border: 1px solid rgba(239, 68, 68, 0.3);
  border-radius: 8px;
  margin-bottom: 16px;
  font-size: 13px;
}

.timer-display {
  display: flex;
  align-items: center;
  gap: 6px;
  font-weight: 600;
  color: #fca5a5;
}

.timer-icon {
  font-size: 16px;
}

.timer-value {
  font-size: 18px;
  font-family: 'JetBrains Mono', monospace;
  color: #f87171;
}

.speaker-indicator {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
  color: #e5e5e5;
}

.pulse-dot.live {
  background: #ef4444;
  box-shadow: 0 0 8px #ef4444;
}

.live-badge {
  padding: 4px 10px;
  background: #ef4444;
  color: white;
  font-size: 10px;
  font-weight: 700;
  border-radius: 4px;
  letter-spacing: 0.1em;
  animation: pulse-glow 1.5s infinite;
}

@keyframes pulse-glow {
  0%, 100% { box-shadow: 0 0 5px #ef4444; }
  50% { box-shadow: 0 0 15px #ef4444, 0 0 25px rgba(239, 68, 68, 0.4); }
}

.mini-start-btn {
  padding: 8px 16px;
  background: #3b82f6;
  border: none;
  border-radius: 6px;
  color: white;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
}

.turn-card {
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 10px;
  padding: 14px;
  margin-bottom: 12px;
}

.turn-card.support { border-left: 3px solid #22c55e; }
.turn-card.oppose { border-left: 3px solid #ef4444; }
.turn-card.neutral { border-left: 3px solid #71717a; }

.turn-header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 10px;
}

.turn-agent {
  font-weight: 500;
  color: #f4f4f5;
}

.turn-round {
  font-size: 11px;
  color: #71717a;
}

.turn-stance {
  font-size: 10px;
  padding: 2px 6px;
  border-radius: 4px;
  margin-left: auto;
}

.turn-stance.support { background: #22c55e22; color: #22c55e; }
.turn-stance.oppose { background: #ef444422; color: #ef4444; }
.turn-stance.neutral { background: #71717a22; color: #71717a; }

.turn-content {
  font-size: 13px;
  color: #d4d4d8;
  line-height: 1.6;
}

/* Pivot Section */
.pivot-section {
  display: flex;
  gap: 10px;
  padding: 16px;
  background: rgba(251, 191, 36, 0.05);
  border-top: 1px solid rgba(251, 191, 36, 0.2);
}

.pivot-input {
  flex: 1;
  padding: 10px;
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 6px;
  color: #f4f4f5;
  font-size: 13px;
}

.continue-btn {
  padding: 10px 16px;
  background: #22c55e;
  border: none;
  border-radius: 6px;
  color: white;
  font-size: 13px;
  cursor: pointer;
}

.auto-continue {
  padding: 16px;
  text-align: center;
}

/* Command Center (30%) */
.command-center {
  flex: 0 0 30%;
  padding: 16px;
  overflow-y: auto;
  background: rgba(0, 0, 0, 0.15);
}

.command-center h4 {
  margin: 0 0 16px 0;
  font-size: 14px;
  color: #f4f4f5;
}

.stats-card, .stance-card, .summary-card {
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 8px;
  padding: 12px;
  margin-bottom: 12px;
}

.stat-row {
  display: flex;
  justify-content: space-between;
  padding: 6px 0;
}

.stat-label {
  font-size: 12px;
  color: #a1a1aa;
}

.stat-value {
  font-size: 12px;
  font-weight: 500;
  color: #f4f4f5;
}

.stat-value.running { color: #22c55e; }
.stat-value.paused { color: #f59e0b; }

/* Stance Bars */
.stance-card h5 {
  margin: 0 0 10px 0;
  font-size: 12px;
  color: #d4d4d8;
}

.stance-bars {
  display: flex;
  height: 24px;
  border-radius: 4px;
  overflow: hidden;
  background: rgba(255, 255, 255, 0.05);
}

.stance-bar {
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 10px;
  color: white;
  transition: width 0.3s;
}

.stance-bar.support { background: #22c55e; }
.stance-bar.neutral { background: #71717a; }
.stance-bar.oppose { background: #ef4444; }

.stance-legend {
  display: flex;
  gap: 12px;
  margin-top: 8px;
  justify-content: center;
}

/* Stance Timeline Chart */
.chart-card {
  background: rgba(255, 255, 255, 0.03);
  border-radius: 8px;
  padding: 12px;
  margin-top: 12px;
}

.chart-card h5 {
  margin: 0 0 8px 0;
  font-size: 12px;
  color: #a1a1aa;
}

.stance-chart {
  width: 100%;
}

.chart-svg {
  width: 100%;
  height: auto;
  background: rgba(0, 0, 0, 0.2);
  border-radius: 4px;
}

.chart-legend {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 8px;
  font-size: 10px;
}

.chart-legend-item {
  font-weight: 500;
}

.legend-item {
  font-size: 10px;
  display: flex;
  align-items: center;
  gap: 4px;
}

.legend-item::before {
  content: '';
  width: 8px;
  height: 8px;
  border-radius: 2px;
}

.legend-item.support::before { background: #22c55e; }
.legend-item.neutral::before { background: #71717a; }
.legend-item.oppose::before { background: #ef4444; }

/* Summary */
.summary-card h5 {
  margin: 0 0 8px 0;
  font-size: 12px;
  color: #d4d4d8;
}

.summary-content {
  font-size: 13px;
  color: #d4d4d8;
  line-height: 1.6;
}

.summary-content h3 {
  font-size: 15px;
  color: #f4f4f5;
  margin: 16px 0 8px 0;
  padding-bottom: 4px;
  border-bottom: 1px solid rgba(255,255,255,0.1);
}

.summary-content h4 {
  font-size: 14px;
  color: #e5e5e5;
  margin: 12px 0 6px 0;
}

.summary-content strong {
  color: #f4f4f5;
}

.summary-content p {
  margin: 8px 0;
}

.summary-content hr {
  border: none;
  border-top: 1px solid rgba(255,255,255,0.1);
  margin: 16px 0;
}

.summary-content .stance-table {
  width: 100%;
  margin: 8px 0;
  border-collapse: collapse;
  font-size: 12px;
}

.summary-content .stance-table th,
.summary-content .stance-table td {
  padding: 6px 10px;
  text-align: left;
  border: 1px solid rgba(255,255,255,0.1);
}

.summary-content .stance-table th {
  background: rgba(255,255,255,0.05);
  color: #a1a1aa;
}

/* Action Buttons */
.action-buttons {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: 16px;
}

.action-btn {
  padding: 10px;
  border: none;
  border-radius: 6px;
  font-size: 13px;
  cursor: pointer;
  transition: all 0.2s;
}

.action-btn.pause { background: #3b82f6; color: white; }
.action-btn.resume { background: #22c55e; color: white; }
.action-btn.stop { background: #dc2626; color: white; }
.action-btn.stop.loading { background: #92400e; cursor: wait; }
.action-btn.new { background: rgba(255, 255, 255, 0.1); color: #e4e4e7; }
.action-btn.export { background: #7c3aed; color: white; }

.action-btn:disabled {
  opacity: 0.7;
  cursor: not-allowed;
}

/* V3: Selection Popup for Knowledge Pad */
.agora-selection-popup {
  position: fixed;
  transform: translateX(-50%) translateY(-100%);
  background: linear-gradient(135deg, #7c3aed 0%, #a855f7 100%);
  border-radius: 8px;
  padding: 6px 10px;
  display: flex;
  gap: 6px;
  align-items: center;
  box-shadow: 0 4px 20px rgba(124, 58, 237, 0.4);
  z-index: 10000;
  animation: popupFadeIn 0.2s ease-out;
}

@keyframes popupFadeIn {
  from { opacity: 0; transform: translateX(-50%) translateY(-80%); }
  to { opacity: 1; transform: translateX(-50%) translateY(-100%); }
}

.add-to-knowledge-btn {
  background: rgba(255, 255, 255, 0.2);
  color: white;
  border: none;
  padding: 6px 12px;
  border-radius: 6px;
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  white-space: nowrap;
  transition: all 0.2s;
}

.add-to-knowledge-btn:hover {
  background: rgba(255, 255, 255, 0.3);
}

.close-popup-btn {
  background: transparent;
  color: rgba(255, 255, 255, 0.7);
  border: none;
  padding: 4px 8px;
  font-size: 12px;
  cursor: pointer;
  border-radius: 50%;
  transition: all 0.2s;
}

.close-popup-btn:hover {
  color: white;
  background: rgba(255, 255, 255, 0.2);
}

/* V3: Tag Modal for Knowledge Pad */
.agora-tag-modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.7);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10001;
}

.agora-tag-modal {
  background: #1e1e2e;
  border-radius: 12px;
  width: 90%;
  max-width: 450px;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
  border: 1px solid rgba(124, 58, 237, 0.3);
}

.tag-modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px 20px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
}

.tag-modal-header h3 {
  margin: 0;
  font-size: 16px;
  color: white;
}

.tag-modal-close {
  background: transparent;
  border: none;
  color: #a1a1aa;
  font-size: 24px;
  cursor: pointer;
  padding: 0;
  line-height: 1;
}

.tag-modal-close:hover { color: white; }

.tag-modal-body {
  padding: 20px;
}

.tag-preview {
  background: rgba(124, 58, 237, 0.1);
  border-radius: 8px;
  padding: 12px;
  margin-bottom: 16px;
}

.preview-text {
  color: #e5e5e5;
  font-size: 13px;
  font-style: italic;
  margin: 0 0 8px 0;
  line-height: 1.5;
}

.preview-source {
  color: #a1a1aa;
  font-size: 11px;
  margin: 0;
}

.tag-section, .custom-tag-section {
  margin-bottom: 16px;
}

.tag-label {
  display: block;
  color: #a1a1aa;
  font-size: 12px;
  margin-bottom: 8px;
}

.predefined-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.tag-chip {
  background: rgba(255, 255, 255, 0.1);
  border: 1px solid rgba(255, 255, 255, 0.2);
  color: #e5e5e5;
  padding: 6px 12px;
  border-radius: 16px;
  font-size: 11px;
  cursor: pointer;
  transition: all 0.2s;
}

.tag-chip:hover {
  background: rgba(124, 58, 237, 0.2);
  border-color: rgba(124, 58, 237, 0.4);
}

.tag-chip.selected {
  background: #7c3aed;
  border-color: #7c3aed;
  color: white;
}

.custom-tag-input {
  display: flex;
  gap: 8px;
}

.custom-tag-input input {
  flex: 1;
  background: rgba(0, 0, 0, 0.3);
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-radius: 6px;
  padding: 8px 12px;
  color: white;
  font-size: 12px;
}

.custom-tag-input input:focus {
  outline: none;
  border-color: #7c3aed;
}

.add-tag-btn {
  background: #7c3aed;
  border: none;
  color: white;
  padding: 8px 14px;
  border-radius: 6px;
  font-size: 16px;
  cursor: pointer;
}

.selected-tags-preview {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  align-items: center;
}

.tags-label {
  color: #a1a1aa;
  font-size: 11px;
}

.selected-tag {
  background: #7c3aed;
  color: white;
  padding: 4px 8px;
  border-radius: 12px;
  font-size: 11px;
  display: flex;
  align-items: center;
  gap: 4px;
}

.selected-tag button {
  background: transparent;
  border: none;
  color: rgba(255, 255, 255, 0.7);
  cursor: pointer;
  font-size: 12px;
  padding: 0;
}

.tag-modal-footer {
  display: flex;
  gap: 10px;
  justify-content: flex-end;
  padding: 16px 20px;
  border-top: 1px solid rgba(255, 255, 255, 0.1);
}

.tag-btn {
  padding: 10px 20px;
  border-radius: 8px;
  font-size: 13px;
  cursor: pointer;
  border: none;
  font-weight: 500;
}

.tag-btn.secondary {
  background: rgba(255, 255, 255, 0.1);
  color: #a1a1aa;
}

.tag-btn.primary {
  background: #7c3aed;
  color: white;
}

.tag-btn:hover {
  filter: brightness(1.1);
}

/* V3: Round Summary Modal Specifics */
.round-summary-modal {
  max-width: 550px;
}

.round-summary-text {
  color: #e5e5e5;
  font-size: 15px;
  line-height: 1.6;
  margin-bottom: 24px;
}

.round-key-points {
  margin: 12px 0 0 0;
  padding-left: 20px;
  color: #d1d1d6;
}

.round-key-points li {
  font-size: 13px;
  margin-bottom: 8px;
  line-height: 1.4;
}

.key-points-section {
  background: rgba(124, 58, 237, 0.05);
  padding: 16px;
  border-radius: 8px;
  border: 1px solid rgba(124, 58, 237, 0.1);
}
</style>
