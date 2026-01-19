<template>
  <div v-if="isOpen" class="green-room-overlay">
    <div class="green-room-modal">
      <!-- Header -->
      <div class="modal-header">
        <div class="header-left">
          <div class="recording-dot"></div>
          <span class="header-title">THE GREEN ROOM</span>
          <span class="header-subtitle"> // AGENT INTERROGATION PROTOCOL</span>
        </div>
        <button class="close-btn" @click="close">
          <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
        </button>
      </div>

      <div class="modal-body">
        <!-- Left: Chat Interface -->
        <div class="chat-section">
          <div class="chat-feed" ref="chatFeed">
            <div v-if="history.length === 0" class="empty-state">
              <span class="empty-icon">🕵️</span>
              <h3>Agent Isolation Complete</h3>
              <p>You are now connected directly to {{ agentName }}. </p>
              <p class="sub-hint">The simulation is strictly monitored. Ask your questions.</p>
            </div>

            <div v-for="(msg, idx) in history" :key="idx" class="chat-message" :class="msg.role">
              <div class="msg-avatar">
                {{ msg.role === 'user' ? 'YOU' : 'AGT' }}
              </div>
              <div class="msg-content">
                <div class="msg-author">{{ msg.role === 'user' ? 'Director' : agentName }}</div>
                <div class="msg-text">{{ msg.text }}</div>
                <div class="msg-meta">{{ msg.timestamp }}</div>
              </div>
            </div>

            <div v-if="isLoading" class="chat-message agent loading">
              <div class="msg-avatar">AGT</div>
              <div class="msg-content">
                <div class="typing-indicator">
                  <span></span><span></span><span></span>
                </div>
              </div>
            </div>
          </div>

          <div class="chat-input-area">
            <input 
              v-model="inputMessage" 
              @keyup.enter="sendMessage"
              type="text" 
              placeholder="Enter interrogation query..."
              :disabled="isLoading"
              ref="inputRef"
            />
            <button class="send-btn" @click="sendMessage" :disabled="!inputMessage.trim() || isLoading">
              SEND
            </button>
          </div>
        </div>

        <!-- Right: Agent Profile -->
        <div class="profile-section">
          <div class="profile-card">
            <div class="profile-header-sm">
                <div class="avatar-placeholder">{{ agentInitials }}</div>
                <div class="profile-identity">
                    <div class="p-name">{{ agentName }}</div>
                    <div class="p-id">ID: {{ agent?.id }}</div>
                </div>
            </div>

            <div class="profile-details">
                <div class="detail-row">
                    <span class="label">Role</span>
                    <span class="value">{{ agent?.profession || 'Unknown' }}</span>
                </div>
                <div class="detail-row">
                    <span class="label">Stance</span>
                    <span class="value" :class="agent?.stance">{{ agent?.stance || 'Neutral' }}</span>
                </div>
                 <div class="detail-row">
                    <span class="label">Personality</span>
                    <span class="value tag">{{ agent?.personality || 'Balanced' }}</span>
                </div>
            </div>

            <div class="bio-section">
                <div class="section-label">BIO SUMMARY</div>
                <p class="bio-text">{{ agent?.bio || 'No biography available.' }}</p>
            </div>

             <div class="suggestions-section">
                <div class="section-label">SUGGESTED PROBES</div>
                <div class="suggestion-chips">
                    <button v-for="q in suggestedQuestions" :key="q" @click="useQuestion(q)" class="chip">
                        {{ q }}
                    </button>
                </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, watch, nextTick } from 'vue'
import { interviewAgent } from '../api/simulation'

const props = defineProps({
  isOpen: Boolean,
  simulationId: String,
  agent: Object
})

const emit = defineEmits(['close'])

const inputMessage = ref('')
const isLoading = ref(false)
const history = ref([])
const chatFeed = ref(null)
const inputRef = ref(null)

const agentName = computed(() => props.agent?.name || `Agent ${props.agent?.id}`)
const agentInitials = computed(() => agentName.value.substring(0, 2).toUpperCase())

const suggestedQuestions = [
    "What is your primary goal right now?",
    "Why do you support this viewpoint?",
    "Who do you trust the most?",
    "What would change your mind?"
]

// Scroll to bottom watcher
watch(() => history.value.length, () => {
    nextTick(() => {
        if (chatFeed.value) {
            chatFeed.value.scrollTop = chatFeed.value.scrollHeight
        }
    })
})

const useQuestion = (q) => {
    inputMessage.value = q
    inputRef.value?.focus()
}

const sendMessage = async () => {
    if (!inputMessage.value.trim() || isLoading.value) return

    const question = inputMessage.value
    inputMessage.value = ''
    
    // Add User Message
    history.value.push({
        role: 'user',
        text: question,
        timestamp: new Date().toLocaleTimeString()
    })

    isLoading.value = true

    try {
        const res = await interviewAgent({
            simulation_id: props.simulationId,
            agent_id: props.agent.id,
            prompt: question,
            platform: 'reddit' // Default to reddit for now, or make configurable
        })

        if (res.success && res.data && res.data.result) {
            const reply = res.data.result.response || res.data.result.result?.response || "No response."
            
             // Add Agent Message
            history.value.push({
                role: 'agent',
                text: reply,
                timestamp: new Date().toLocaleTimeString()
            })
        } else {
             history.value.push({
                role: 'agent',
                text: "[CONNECTION ERROR] Agent failed to respond.",
                timestamp: new Date().toLocaleTimeString()
            })
        }
    } catch (e) {
        console.error("Interview failed", e)
         history.value.push({
            role: 'agent',
            text: `[SYSTEM ERROR] ${e.message}`,
            timestamp: new Date().toLocaleTimeString()
        })
    } finally {
        isLoading.value = false
        nextTick(() => {
             inputRef.value?.focus()
        })
    }
}

const close = () => {
    emit('close')
    history.value = [] // clear history on close? or keep it? user preference. clearing for now.
}
</script>

<style scoped>
.green-room-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.85);
  /* backdrop-filter removed */
  z-index: 9999;
  display: flex;
  align-items: center;
  justify-content: center;
}

.green-room-modal {
  width: 900px;
  height: 600px;
  max-width: 95vw;
  max-height: 90vh;
  background: #111;
  border: 1px solid #333;
  border-radius: 8px;
  display: flex;
  flex-direction: column;
  box-shadow: 0 0 50px rgba(0, 255, 128, 0.1);
  overflow: hidden;
}

/* Header */
.modal-header {
  height: 50px;
  border-bottom: 1px solid #333;
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0 20px;
  background: #0A0A0A;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 10px;
}

.recording-dot {
  width: 8px;
  height: 8px;
  background: #ff3b30;
  border-radius: 50%;
  box-shadow: 0 0 8px #ff3b30;
  animation: pulse 2s infinite;
}

@keyframes pulse {
  0% { opacity: 1; }
  50% { opacity: 0.5; }
  100% { opacity: 1; }
}

.header-title {
  color: #fff;
  font-weight: 700;
  letter-spacing: 1px;
}

.header-subtitle {
  color: #666;
  font-family: 'JetBrains Mono', monospace;
  font-size: 12px;
}

.close-btn {
  background: none;
  border: none;
  color: #666;
  cursor: pointer;
}
.close-btn:hover { color: #fff; }

/* Body */
.modal-body {
  flex: 1;
  display: flex;
  overflow: hidden;
}

/* Chat Section */
.chat-section {
  flex: 1;
  display: flex;
  flex-direction: column;
  border-right: 1px solid #333;
  background: #0e0e0e;
}

.chat-feed {
  flex: 1;
  padding: 20px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 20px;
}

/* Scrollbar */
.chat-feed::-webkit-scrollbar { width: 6px; }
.chat-feed::-webkit-scrollbar-thumb { background: #333; border-radius: 3px; }

.empty-state {
  text-align: center;
  margin-top: 100px;
  opacity: 0.5;
}
.empty-icon { font-size: 40px; display: block; margin-bottom: 10px; }
.empty-state h3 { color: #fff; margin: 0 0 5px 0; }
.empty-state p { color: #888; font-size: 14px; margin: 0; }
.sub-hint { font-size: 12px !important; color: #555 !important; margin-top: 5px !important; }

.chat-message {
  display: flex;
  gap: 15px;
  max-width: 85%;
}

.chat-message.user {
  align-self: flex-end;
  flex-direction: row-reverse;
}

.chat-message.agent {
  align-self: flex-start;
}

.msg-avatar {
  width: 36px;
  height: 36px;
  background: #222;
  color: #888;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 10px;
  font-weight: 700;
  font-family: 'JetBrains Mono', monospace;
  border-radius: 4px;
}
.chat-message.user .msg-avatar { background: #333; color: #fff; }
.chat-message.agent .msg-avatar { background: #1a2e22; color: #4ade80; border: 1px solid #225533; }

.msg-content {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.chat-message.user .msg-content { align-items: flex-end; }

.msg-author {
  font-size: 11px;
  color: #666;
  text-transform: uppercase;
}

.msg-text {
  background: #111;
  border: 1px solid #333;
  color: #ccc;
  padding: 10px 14px;
  border-radius: 8px;
  font-size: 14px;
  line-height: 1.5;
}
.chat-message.user .msg-text { background: #222; border-color: #444; color: #fff; }
.chat-message.agent .msg-text { background: #0f1a14; border-color: #1b4d2e; color: #d1fae5; }

.msg-meta {
  font-size: 10px;
  color: #444;
}

/* Typing Indicator */
.typing-indicator span {
  display: inline-block;
  width: 6px;
  height: 6px;
  background: #4ade80;
  border-radius: 50%;
  margin: 0 2px;
  animation: typing 1.4s infinite ease-in-out both;
}
.typing-indicator span:nth-child(1) { animation-delay: -0.32s; }
.typing-indicator span:nth-child(2) { animation-delay: -0.16s; }

@keyframes typing {
  0%, 80%, 100% { transform: scale(0); opacity: 0.5; }
  40% { transform: scale(1); opacity: 1; }
}

/* Input Area */
.chat-input-area {
  padding: 20px;
  border-top: 1px solid #333;
  display: flex;
  gap: 10px;
  background: #111;
}

.chat-input-area input {
  flex: 1;
  background: #000;
  border: 1px solid #333;
  color: #fff;
  padding: 10px 14px;
  border-radius: 4px;
  font-family: inherit;
}
.chat-input-area input:focus { outline: none; border-color: #666; }

.send-btn {
  background: #fff;
  color: #000;
  border: none;
  padding: 0 20px;
  font-weight: 700;
  border-radius: 4px;
  cursor: pointer;
  transition: opacity 0.2s;
}
.send-btn:disabled { opacity: 0.5; cursor: not-allowed; }
.send-btn:hover:not(:disabled) { opacity: 0.9; }

/* Profile Section */
.profile-section {
  width: 280px;
  background: #0A0A0A;
  padding: 20px;
  border-left: 1px solid #333;
}

.profile-header-sm {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 20px;
}

.avatar-placeholder {
    width: 48px;
    height: 48px;
    background: #222;
    border: 1px solid #333;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 18px;
    color: #fff;
    font-weight: bold;
}

.p-name { color: #fff; font-weight: 600; font-size: 14px; }
.p-id { color: #555; font-size: 11px; font-family: 'JetBrains Mono', monospace; }

.profile-details {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-bottom: 20px;
    padding-bottom: 20px;
    border-bottom: 1px solid #222;
}

.detail-row {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
}

.detail-row .label { color: #666; }
.detail-row .value { color: #bbb; }
.detail-row .value.tag { background: #222; padding: 2px 6px; border-radius: 4px; }

.section-label {
    font-size: 10px;
    color: #444;
    font-weight: 700;
    margin-bottom: 8px;
    letter-spacing: 1px;
}

.bio-text {
    font-size: 12px;
    color: #888;
    line-height: 1.5;
    margin-bottom: 20px;
}

.suggestions-section {
    margin-top: auto;
}

.suggestion-chips {
    display: flex;
    flex-direction: column;
    gap: 8px;
}

.chip {
    background: #151515;
    border: 1px solid #333;
    color: #aaa;
    padding: 8px 12px;
    text-align: left;
    font-size: 11px;
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.2s;
}
.chip:hover {
    background: #222;
    color: #fff;
    border-color: #555;
    transform: translateX(2px);
}
</style>
