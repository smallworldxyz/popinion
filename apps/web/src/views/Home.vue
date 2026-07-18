<template>
  <div class="home-container">
    <!-- Top navigation bar -->
    <nav class="navbar">
      <div class="nav-brand">POPINION</div>
      <div class="nav-desc">Public Opinion Analysis</div>
      <div class="nav-links">
        <router-link to="/worlds" class="nav-btn">Worlds</router-link>
        <router-link to="/settings" class="nav-btn">⚙ Models</router-link>
        <a href="https://github.com/rithythul/popinion" target="_blank" class="nav-btn">
          GitHub <span class="arrow">↗</span>
        </a>
      </div>
    </nav>

    <div class="main-content">
      <!-- Hero: chat-native simulation prompt with reality-seed attachments -->
      <section class="hero-section">
        <div
          class="chat-box"
          :class="{ 'drag-over': isDragOver }"
          @dragover.prevent="handleDragOver"
          @dragleave.prevent="handleDragLeave"
          @drop.prevent="handleDrop"
        >
          <div class="chat-label">Sim Prompt</div>
          <textarea
            v-model="formData.simulationRequirement"
            class="chat-input"
            placeholder="Describe the scenario to simulate — a policy, a message, an event.
e.g. How will the public react to phasing out the national fuel subsidy over 12 months?"
            rows="5"
            :disabled="loading"
          ></textarea>

          <!-- reality seeds attached to the prompt -->
          <div v-if="files.length" class="seed-chips">
            <div v-for="(file, index) in files" :key="index" class="seed-chip">
              <span class="file-icon">📄</span>
              <span class="chip-name">{{ file.name }}</span>
              <button @click.stop="removeFile(index)" class="chip-x">×</button>
            </div>
          </div>

          <!-- Typed/pasted markdown as a reality seed: becomes an .md file at submit -->
          <div v-if="showText || pastedText" class="text-seed">
            <div class="text-seed-head">
              <span class="tg-icon">📝</span>
              <span class="text-seed-label">Markdown or plain text — pasted posts, notes, transcripts</span>
              <button class="chip-x" @click="clearText" :disabled="loading">×</button>
            </div>
            <textarea
              v-model="pastedText"
              class="text-seed-input"
              placeholder="Paste real opinions, posts, or notes here to ground the sim.

Markdown is fine — it's treated exactly like an uploaded .md file."
              rows="6"
              :disabled="loading"
            ></textarea>
          </div>

          <!-- Telegram channel as a reality seed: crawled server-side at build time -->
          <div v-if="showTelegram || telegramChannel" class="telegram-row">
            <span class="tg-icon">📡</span>
            <input
              v-model="telegramChannel"
              class="tg-input"
              placeholder="@channel or t.me link — recent public posts ground the sim"
              :disabled="loading"
            />
            <select v-model.number="telegramMaxPosts" class="tg-count" :disabled="loading">
              <option :value="25">25 posts</option>
              <option :value="50">50 posts</option>
              <option :value="100">100 posts</option>
              <option :value="200">200 posts</option>
            </select>
            <button class="chip-x" @click="clearTelegram" :disabled="loading">×</button>
          </div>

          <!-- First-run model gate: no working model → guide to Settings. -->
          <div v-if="modelStatus === null" class="model-note checking">Checking model…</div>
          <div v-else-if="!modelStatus.ready" class="model-note missing">
            <div class="model-note-title">No working model yet — Popinion needs a model to run.</div>
            <div class="model-note-reason">{{ modelStatus.reason }}</div>
            <router-link to="/settings" class="model-note-link">Set up a model →</router-link>
          </div>

          <div v-if="error" class="model-note missing">
            <div class="model-note-title">Couldn't start the engine.</div>
            <div class="model-note-reason">{{ error }}</div>
          </div>

          <div class="chat-toolbar">
            <button class="attach-btn" @click="triggerFileInput" :disabled="loading">
              📎 Reality seeds
            </button>
            <button class="attach-btn" @click="showText = !showText" :disabled="loading">
              📝 Write text
            </button>
            <button class="attach-btn" @click="showTelegram = !showTelegram" :disabled="loading">
              📡 Telegram
            </button>
            <span class="attach-hint">{{ seedHint }}</span>
            <button
              class="start-engine-btn"
              @click="startSimulation"
              :disabled="!canSubmit || loading"
            >
              <span v-if="!loading">Start Engine</span>
              <span v-else>Initializing…</span>
              <span class="btn-arrow">→</span>
            </button>
          </div>

          <input
            ref="fileInput"
            type="file"
            multiple
            accept=".pdf,.md,.txt"
            @change="handleFileSelect"
            style="display: none"
            :disabled="loading"
          />
          <div class="engine-badge">Engine: Popinion-V1.0</div>
        </div>
      </section>

      <!-- Dashboard section: Single column layout -->
      <section class="dashboard-section">
        <div class="info-panel">
          <div class="panel-header">
            <span class="status-dot">■</span> System Status
          </div>
          
          <h2 class="section-title">Ready</h2>
          <p class="section-desc">
            Prediction engine standing by. Describe a scenario above to run a simulation — attach reality seeds to ground it in real data.
          </p>
          
          <!-- Metric cards -->
          <div class="metrics-row">
            <div class="metric-card">
              <div class="metric-value">Graph-Grounded</div>
              <div class="metric-label">Personas from real evidence</div>
            </div>
            <div class="metric-card">
              <div class="metric-value">Trust Checks</div>
              <div class="metric-label">Noise floor + ablation</div>
            </div>
          </div>

          <!-- Capabilities Check -->
          <div class="capabilities-list">
             <div class="cap-item">
               <span class="cap-icon">✓</span>
               <span class="cap-text">Graph-grounded personas (no fabricated traits)</span>
             </div>
             <div class="cap-item">
               <span class="cap-icon">✓</span>
               <span class="cap-text">Direct simulation injection (World Agent)</span>
             </div>
             <div class="cap-item">
               <span class="cap-icon">✓</span>
               <span class="cap-text">Scenario A/B rehearsal with significance test</span>
             </div>
          </div>

          <!-- Simulation Steps (new section) -->
          <div class="steps-container">
            <div class="steps-header">
               <span class="diamond-icon">◇</span> Workflow Sequence
            </div>
            <div class="workflow-list">
              <div class="workflow-item">
                <span class="step-num">01</span>
                <div class="step-info">
                  <div class="step-title">Reality Injection</div>
                  <div class="step-desc">Crawl a Telegram channel or upload documents & build a knowledge graph from the real evidence</div>
                </div>
              </div>
              <div class="workflow-item">
                <span class="step-num">02</span>
                <div class="step-info">
                  <div class="step-title">Environment Setup</div>
                  <div class="step-desc">Entity relationship extraction & Profile generation & Agent configuration with simulation parameters</div>
                </div>
              </div>
              <div class="workflow-item">
                <span class="step-num">03</span>
                <div class="step-info">
                  <div class="step-title">Start Simulation</div>
                  <div class="step-desc">Multi-agent simulation over rounds with stance & sentiment captured at each action</div>
                </div>
              </div>
              <div class="workflow-item">
                <span class="step-num">04</span>
                <div class="step-info">
                  <div class="step-title">Report Generation</div>
                  <div class="step-desc">ReportAgent with rich toolset for deep interaction with post-simulation environment</div>
                </div>
              </div>
              <div class="workflow-item">
                <span class="step-num">05</span>
                <div class="step-info">
                  <div class="step-title">Deep Interaction</div>
                  <div class="step-desc">Chat with any agent in the simulation & Interact with ReportAgent</div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { getLlmStatus } from '../api/settings'
import { generateOntology } from '../api/graph'

const router = useRouter()

// Model readiness gate: Start Engine calls the LLM on the bulk slot, so on a
// fresh install (no local model, no key) we guide to Settings instead of
// letting the pipeline fail cryptically mid-build. null = still checking.
const modelStatus = ref(null)

const checkModel = async () => {
  try {
    modelStatus.value = (await getLlmStatus()).data
  } catch (e) {
    modelStatus.value = { ready: false, reason: 'Cannot reach the Popinion backend.' }
  }
}

const onFocus = () => {
  // Re-check when the window regains focus (e.g. after loading a model in LM Studio).
  if (modelStatus.value && !modelStatus.value.ready) checkModel()
}

onMounted(() => {
  checkModel()
  window.addEventListener('focus', onFocus)
})
onUnmounted(() => window.removeEventListener('focus', onFocus))

// formdata
const formData = ref({
  simulationRequirement: ''
})

// File list
const files = ref([])

// Typed markdown seed — sent as an .md file, which the backend already parses
const showText = ref(false)
const pastedText = ref('')

const clearText = () => {
  pastedText.value = ''
  showText.value = false
}

// Telegram channel seed
const showTelegram = ref(false)
const telegramChannel = ref('')
const telegramMaxPosts = ref(50)

const clearTelegram = () => {
  telegramChannel.value = ''
  showTelegram.value = false
}

// Status
const loading = ref(false)
const error = ref('')
const isDragOver = ref(false)

// File input reference
const fileInput = ref(null)

// Computed attributes: canSubmit — at least one reality seed (file, text, or channel)
const canSubmit = computed(() => {
  return formData.value.simulationRequirement.trim() !== '' &&
    (files.value.length > 0 || pastedText.value.trim() !== '' || telegramChannel.value.trim() !== '') &&
    modelStatus.value?.ready === true
})

const seedHint = computed(() => {
  const parts = []
  if (files.value.length) parts.push(`${files.value.length} file${files.value.length > 1 ? 's' : ''}`)
  if (pastedText.value.trim()) parts.push('pasted text')
  if (telegramChannel.value.trim()) parts.push(`Telegram ${telegramChannel.value.trim()}`)
  return parts.length
    ? parts.join(' + ') + ' — real data grounds the simulation'
    : 'Attach PDF · MD · TXT, write text, or crawl Telegram'
})

// Trigger file selection
const triggerFileInput = () => {
  if (!loading.value) {
    fileInput.value?.click()
  }
}

// Process file selection
const handleFileSelect = (event) => {
  const selectedFiles = Array.from(event.target.files)
  addFiles(selectedFiles)
}

// Process drag related
const handleDragOver = (e) => {
  if (!loading.value) {
    isDragOver.value = true
  }
}

const handleDragLeave = (e) => {
  isDragOver.value = false
}

const handleDrop = (e) => {
  isDragOver.value = false
  if (loading.value) return
  
  const droppedFiles = Array.from(e.dataTransfer.files)
  addFiles(droppedFiles)
}

// Add files
const addFiles = (newFiles) => {
  const validFiles = newFiles.filter(file => {
    const ext = file.name.split('.').pop().toLowerCase()
    return ['pdf', 'md', 'txt'].includes(ext)
  })
  files.value.push(...validFiles)
}

// Remove file
const removeFile = (index) => {
  files.value.splice(index, 1)
}

// Scroll to bottom
const scrollToBottom = () => {
  window.scrollTo({
    top: document.body.scrollHeight,
    behavior: 'smooth'
  })
}

// Start Simulation: upload the reality seeds here, then land on the real
// project. Doing the call before navigation (rather than stashing the files in
// an in-memory module and jumping to /process/new) means a refresh can't strand
// the shell with no data to build from.
const startSimulation = async () => {
  if (!canSubmit.value || loading.value) return

  const channel = telegramChannel.value.trim()
  const typed = pastedText.value.trim()
  const seeds = typed
    ? [...files.value, new File([typed], 'written-note.md', { type: 'text/markdown' })]
    : files.value

  loading.value = true
  error.value = ''
  try {
    const formPayload = new FormData()
    seeds.forEach(f => formPayload.append('files', f))
    formPayload.append('simulation_requirement', formData.value.simulationRequirement)
    if (channel) {
      formPayload.append('telegram_channel', channel)
      formPayload.append('telegram_max_posts', String(telegramMaxPosts.value))
    }

    const res = await generateOntology(formPayload)
    if (res.success && res.data?.project_id) {
      router.push({ name: 'Process', params: { projectId: res.data.project_id } })
    } else {
      error.value = res.error || 'Failed to analyze the reality seeds.'
    }
  } catch (err) {
    error.value = err.response?.data?.error || err.message
  } finally {
    loading.value = false
  }
}
</script>

<style scoped>
.home-container {
  min-height: 100vh;
  background: var(--subtle);
  font-family: var(--font-sans);
  color: var(--text-strong);
}

/* Top navigation */
.navbar {
  height: 60px;
  background: var(--white);
  color: var(--text-strong);
  border-bottom: 1px solid var(--border);
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 16px;
  padding: 0 40px;
}
/* The tagline is decorative — drop it before it collides with the brand. */
@media (max-width: 760px) {
  .navbar { padding: 0 20px; }
  .nav-desc { display: none; }
}

.nav-brand {
  font-family: var(--font-serif);
  font-weight: 600;
  letter-spacing: 0.5px;
  font-size: 1.3rem;
  color: var(--text-strong);
}

.nav-links {
  display: flex;
  align-items: center;
}

.nav-desc {
  font-family: var(--font-sans);
  font-weight: 700;
  font-size: 0.7rem;
  letter-spacing: 0.14em;
  text-transform: uppercase;
  color: var(--text-muted);
}
.nav-btn {
  color: var(--text-strong);
  text-decoration: none;
  font-family: var(--font-sans);
  font-size: 0.85rem;
  font-weight: 500;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 7px 14px;
  border: 1px solid var(--border);
  border-radius: var(--r-sm);
  transition: all 0.15s;
}
.nav-btn + .nav-btn { margin-left: 10px; }
.nav-btn:hover { background: var(--navy); color: var(--on-dark); border-color: var(--navy); }
.arrow { font-family: var(--font-sans); }

/* Chat-native simulation prompt */
.chat-box {
  max-width: 1000px;
  margin: 40px auto 0;
  background: var(--white);
  border: 1px solid var(--border);
  border-radius: var(--r-lg);
  padding: 20px;
  position: relative;
  box-shadow: var(--elev-1);
}
.chat-box.drag-over { border-style: dashed; border-color: var(--emerald); background: rgba(12, 133, 119, .04); }
.chat-label {
  font-family: var(--font-sans);
  font-weight: 700;
  font-size: 0.7rem;
  letter-spacing: 0.14em;
  text-transform: uppercase;
  color: var(--text-muted);
  margin-bottom: 12px;
}
.chat-input {
  width: 100%;
  border: none;
  outline: none;
  resize: vertical;
  font-family: inherit;
  font-size: 1.05rem;
  line-height: 1.6;
  color: var(--text-strong);
  background: transparent;
  min-height: 120px;
}
.chat-input::placeholder { color: var(--text-muted); }
.seed-chips { display: flex; flex-wrap: wrap; gap: 8px; margin: 8px 0 4px; }
.seed-chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  background: var(--subtle);
  border: 1px solid var(--border);
  border-radius: var(--r-pill);
  padding: 4px 10px;
  font-size: 0.85rem;
}
.chip-name { max-width: 220px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.chip-x { border: none; background: none; cursor: pointer; font-size: 1rem; color: var(--text-muted); line-height: 1; }
.chip-x:hover { color: var(--coral-on-light); }
.text-seed {
  margin: 8px 0 4px;
  padding: 6px 10px;
  background: var(--subtle);
  border: 1px solid var(--border);
  border-radius: var(--r-md);
}
.text-seed-head { display: flex; align-items: center; gap: 8px; }
.text-seed-label { flex: 1; font-size: 0.85rem; color: var(--text-muted); }
.text-seed-input {
  width: 100%;
  margin-top: 6px;
  border: none;
  outline: none;
  resize: vertical;
  background: transparent;
  font-family: inherit;
  font-size: 0.9rem;
  line-height: 1.55;
  color: var(--text-strong);
  box-sizing: border-box;
}
.text-seed-input::placeholder { color: var(--text-muted); }
.telegram-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 8px 0 4px;
  padding: 6px 10px;
  background: var(--subtle);
  border: 1px solid var(--border);
  border-radius: var(--r-md);
}
.tg-icon { font-size: 0.9rem; }
.tg-input {
  flex: 1;
  border: none;
  outline: none;
  background: transparent;
  font-family: inherit;
  font-size: 0.9rem;
  color: var(--text-strong);
}
.tg-input::placeholder { color: var(--text-muted); }
.tg-count {
  border: 1px solid var(--border);
  background: var(--white);
  border-radius: var(--r-sm);
  font-family: inherit;
  font-size: 0.78rem;
  padding: 4px 8px;
  color: var(--text-strong);
}
.chat-toolbar {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 12px;
  margin-top: 14px;
  padding-top: 14px;
  border-top: 1px solid var(--border);
}
.attach-btn {
  flex: none;
  font-family: inherit;
  font-size: 0.9rem;
  font-weight: 500;
  padding: 8px 14px;
  border: 1px solid var(--border);
  background: var(--white);
  color: var(--text-strong);
  border-radius: var(--r-sm);
  cursor: pointer;
  white-space: nowrap;
}
.attach-btn:hover { background: var(--subtle); }
.attach-hint { flex: 1; font-size: 0.82rem; color: var(--text-muted); }
.chat-toolbar .start-engine-btn { width: auto; margin: 0; }
/* First-run model gate */
.model-note {
  margin-top: 12px;
  font-size: 0.88rem;
  line-height: 1.5;
}
.model-note.checking {
  font-size: 0.78rem;
  color: var(--text-muted);
}
.model-note.missing {
  border: 1px solid var(--border);
  border-left: 3px solid var(--coral);
  background: rgba(232, 106, 76, .06);
  border-radius: var(--r-sm);
  padding: 12px 14px;
}
.model-note-title { font-weight: 600; color: var(--text-strong); }
.model-note-reason { color: var(--text-muted); margin-top: 2px; }
.model-note-link {
  display: inline-block;
  margin-top: 6px;
  color: var(--coral-on-light);
  font-weight: 600;
  text-decoration: underline;
  text-underline-offset: 3px;
}
.model-note-link:hover { color: var(--coral); }

.engine-badge {
  position: absolute;
  top: 20px;
  right: 22px;
  font-size: 0.72rem;
  font-variant-numeric: tabular-nums;
  color: var(--text-muted);
}

/* Main content area */
.main-content {
  max-width: 1400px;
  margin: 0 auto;
  padding: 48px 40px 60px;
}

.hero-section {
  margin-bottom: 60px;
}

/* Dashboard Section: Single column */
.dashboard-section {
  border-top: 1px solid var(--border);
  padding-top: 60px;
}

.info-panel {
  max-width: 1000px;
  margin: 0 auto;
}

.panel-header {
  font-family: var(--font-sans);
  font-weight: 700;
  font-size: 0.7rem;
  letter-spacing: 0.14em;
  text-transform: uppercase;
  color: var(--text-muted);
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 20px;
}

.status-dot {
  color: var(--emerald);
  font-size: 0.8rem;
}

.section-title {
  font-family: var(--font-serif);
  font-size: 2.2rem;
  font-weight: 500;
  margin: 0 0 15px 0;
  color: var(--text-strong);
}

.section-desc {
  color: var(--text-muted);
  margin-bottom: 25px;
  line-height: 1.6;
  max-width: 640px;
}

.metrics-row {
  display: flex;
  gap: 20px;
  margin-bottom: 24px;
  flex-wrap: wrap;
}

.metric-card {
  background: var(--white);
  border: 1px solid var(--border);
  border-radius: var(--r-md);
  box-shadow: var(--elev-1);
  padding: 20px 24px;
  min-width: 200px;
}

.metric-value {
  font-family: var(--font-serif);
  font-size: 1.3rem;
  font-weight: 500;
  margin-bottom: 6px;
  color: var(--text-strong);
}

.metric-label {
  font-size: 0.85rem;
  color: var(--text-muted);
}

/* Simulation Steps */
.steps-container {
  background: var(--white);
  border: 1px solid var(--border);
  border-radius: var(--r-lg);
  box-shadow: var(--elev-1);
  padding: 30px;
  position: relative;
}

.steps-header {
  font-family: var(--font-sans);
  font-weight: 700;
  font-size: 0.7rem;
  letter-spacing: 0.14em;
  text-transform: uppercase;
  color: var(--text-muted);
  margin-bottom: 25px;
  display: flex;
  align-items: center;
  gap: 8px;
}

.diamond-icon {
  font-size: 1.2rem;
  line-height: 1;
  color: var(--emerald);
}

.workflow-list {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.workflow-item {
  display: flex;
  align-items: flex-start;
  gap: 20px;
}

.step-num {
  font-family: var(--font-serif);
  font-weight: 600;
  font-size: 1.1rem;
  font-variant-numeric: tabular-nums;
  color: var(--emerald);
  min-width: 28px;
}

.step-info {
  flex: 1;
}

.step-title {
  font-weight: 600;
  font-size: 1rem;
  margin-bottom: 4px;
  color: var(--text-strong);
}

.step-desc {
  font-size: 0.85rem;
  color: var(--text-muted);
  line-height: 1.55;
}

.start-engine-btn {
  background: var(--coral);
  color: var(--on-dark);
  border: none;
  padding: 9px 18px;
  font-family: inherit;
  font-weight: 600;
  font-size: 0.95rem;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  transition: background 0.15s ease, transform 0.15s ease;
  border-radius: var(--r-sm);
  box-shadow: var(--elev-1);
}
.start-engine-btn .btn-arrow { font-size: 1rem; }
.start-engine-btn:hover:not(:disabled) {
  background: var(--coral-on-light);
  transform: translateY(-1px);
}
.start-engine-btn:active:not(:disabled) { transform: translateY(0); }
.start-engine-btn:disabled {
  background: var(--subtle);
  color: var(--text-muted);
  cursor: not-allowed;
  box-shadow: none;
}

/* Capabilities List */
.capabilities-list {
  margin: 20px 0;
  display: flex;
  flex-direction: column;
  gap: 10px;
  background: var(--white);
  border: 1px solid var(--border);
  border-radius: var(--r-md);
  padding: 16px 18px;
  border-left: 3px solid var(--emerald);
}

.cap-item {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 0.9rem;
  color: var(--text-strong);
}

.cap-icon {
  color: var(--emerald);
  font-weight: bold;
}

/* Responsive */
@media (max-width: 1024px) {
  .main-content { padding: 40px 20px; }
  .info-panel { max-width: 100%; }
}
</style>
