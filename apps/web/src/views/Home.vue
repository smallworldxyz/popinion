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
          <div class="chat-label">>_ Sim Prompt</div>
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

// Start Simulation - Jump immediately, API call in Process page
const startSimulation = () => {
  if (!canSubmit.value || loading.value) return
  
  // Store pending upload data
  import('../store/pendingUpload.js').then(({ setPendingUpload }) => {
    const channel = telegramChannel.value.trim()
    const typed = pastedText.value.trim()
    const seeds = typed
      ? [...files.value, new File([typed], 'written-note.md', { type: 'text/markdown' })]
      : files.value
    setPendingUpload(
      seeds,
      formData.value.simulationRequirement,
      channel ? { channel, maxPosts: telegramMaxPosts.value } : null
    )

    // Jump immediately to Process page (use special flag for new items)
    router.push({
      name: 'Process',
      params: { projectId: 'new' }
    })
  })
}
</script>

<style scoped>
/* Global variables and Reset */
:root {
  --black: #000000;
  --white: #FFFFFF;
  --orange: #FF4500;
  --gray-light: #F5F5F5;
  --gray-text: #666666;
  --border: #E5E5E5;
  /* 
    Use Space Grotesk as main title font, JetBrains Mono as code/label font
    Ensure these Google Fonts are imported in index.html 
  */
  --font-mono: 'JetBrains Mono', monospace;
  --font-sans: 'Space Grotesk', -apple-system, sans-serif;
}

.home-container {
  min-height: 100vh;
  background: var(--white);
  font-family: var(--font-sans);
  color: var(--black);
}

/* Top navigation */
.navbar {
  height: 60px;
  background: var(--black);
  color: var(--white);
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
  font-family: var(--font-mono);
  font-weight: 800;
  letter-spacing: 1px;
  font-size: 1.2rem;
}

.nav-links {
  display: flex;
  align-items: center;
}

.github-link {
  color: var(--white);
  text-decoration: none;
  font-family: var(--font-mono);
  font-size: 0.9rem;
  font-weight: 500;
  display: flex;
  align-items: center;
  gap: 8px;
  transition: opacity 0.2s;
}

.github-link:hover {
  opacity: 0.8;
}

.nav-desc {
  font-family: var(--font-mono);
  font-size: 0.8rem;
  letter-spacing: 2px;
  text-transform: uppercase;
  opacity: 0.55;
}
.nav-btn {
  color: var(--white);
  text-decoration: none;
  font-family: var(--font-mono);
  font-size: 0.85rem;
  font-weight: 500;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 7px 14px;
  border: 1px solid rgba(255, 255, 255, 0.35);
  border-radius: 6px;
  transition: all 0.2s;
}
.nav-btn + .nav-btn { margin-left: 10px; }
.nav-btn:hover { background: var(--white); color: var(--black); }

/* Chat-native simulation prompt */
.chat-box {
  max-width: 1000px;
  margin: 40px auto 0;
  background: var(--white);
  border: 1px solid var(--black);
  border-radius: 14px;
  padding: 20px;
  position: relative;
  box-shadow: 0 4px 24px rgba(0, 0, 0, 0.06);
}
.chat-box.drag-over { border-style: dashed; background: #f6f8ff; }
.chat-label {
  font-family: var(--font-mono);
  font-size: 0.8rem;
  letter-spacing: 1px;
  color: #6b7280;
  margin-bottom: 10px;
}
.chat-input {
  width: 100%;
  border: none;
  outline: none;
  resize: vertical;
  font-family: inherit;
  font-size: 1.05rem;
  line-height: 1.6;
  color: var(--black);
  background: transparent;
  min-height: 120px;
}
.chat-input::placeholder { color: #9ca3af; }
.seed-chips { display: flex; flex-wrap: wrap; gap: 8px; margin: 8px 0 4px; }
.seed-chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  background: #f3f4f6;
  border: 1px solid #e5e7eb;
  border-radius: 999px;
  padding: 4px 10px;
  font-size: 0.85rem;
}
.chip-name { max-width: 220px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.chip-x { border: none; background: none; cursor: pointer; font-size: 1rem; color: #9ca3af; line-height: 1; }
.chip-x:hover { color: #dc2626; }
.text-seed {
  margin: 8px 0 4px;
  padding: 6px 10px;
  background: #f3f4f6;
  border: 1px solid #e5e7eb;
  border-radius: 10px;
}
.text-seed-head { display: flex; align-items: center; gap: 8px; }
.text-seed-label { flex: 1; font-size: 0.85rem; color: var(--gray-text); }
.text-seed-input {
  width: 100%;
  margin-top: 6px;
  border: none;
  outline: none;
  resize: vertical;
  background: transparent;
  font-family: var(--font-mono);
  font-size: 0.85rem;
  line-height: 1.5;
  color: var(--black);
  box-sizing: border-box;
}
.text-seed-input::placeholder { color: #9ca3af; }
.telegram-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 8px 0 4px;
  padding: 6px 10px;
  background: #f3f4f6;
  border: 1px solid #e5e7eb;
  border-radius: 10px;
}
.tg-icon { font-size: 0.9rem; }
.tg-input {
  flex: 1;
  border: none;
  outline: none;
  background: transparent;
  font-family: var(--font-mono);
  font-size: 0.85rem;
  color: var(--black);
}
.tg-input::placeholder { color: #9ca3af; }
.tg-count {
  border: 1px solid #e5e7eb;
  background: #fff;
  border-radius: 6px;
  font-family: var(--font-mono);
  font-size: 0.78rem;
  padding: 3px 6px;
  color: #374151;
}
.chat-toolbar {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 12px;
  margin-top: 14px;
  padding-top: 14px;
  border-top: 1px solid #eee;
}
.attach-btn {
  flex: none;
  font-family: inherit;
  font-size: 0.9rem;
  padding: 8px 14px;
  border: 1px solid #d1d5db;
  background: #fff;
  border-radius: 8px;
  cursor: pointer;
  white-space: nowrap;
}
.attach-btn:hover { background: #f9fafb; }
.attach-hint { flex: 1; font-size: 0.82rem; color: #9ca3af; }
.chat-toolbar .start-engine-btn { width: auto; margin: 0; }
/* First-run model gate */
.model-note {
  margin-top: 12px;
  font-size: 0.88rem;
  line-height: 1.5;
}
.model-note.checking {
  font-family: var(--font-mono);
  font-size: 0.75rem;
  color: #9ca3af;
}
.model-note.missing {
  border: 1px solid #f0dcc0;
  border-left: 3px solid #d97706;
  background: #fdf8f0;
  border-radius: 8px;
  padding: 12px 14px;
}
.model-note-title { font-weight: 600; color: #78350f; }
.model-note-reason { color: #92672a; margin-top: 2px; }
.model-note-link {
  display: inline-block;
  margin-top: 6px;
  color: #000;
  font-weight: 600;
  text-decoration: underline;
  text-underline-offset: 3px;
}
.model-note-link:hover { color: #d97706; }

.engine-badge {
  position: absolute;
  top: 18px;
  right: 20px;
  font-family: var(--font-mono);
  font-size: 0.72rem;
  color: #9ca3af;
}

.arrow {
  font-family: sans-serif;
}

/* Main content area */
.main-content {
  max-width: 1400px;
  margin: 0 auto;
  padding: 60px 40px;
}

/* Hero section: Console box */
.hero-section {
  margin-bottom: 60px;
}

.hero-section .console-box {
  max-width: 1000px;
  margin: 0 auto;
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
  font-family: var(--font-mono);
  font-size: 0.8rem;
  color: #999;
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 20px;
}

.status-dot {
  color: var(--orange);
  font-size: 0.8rem;
}

.section-title {
  font-size: 2rem;
  font-weight: 700;
  margin: 0 0 15px 0;
}

.section-desc {
  color: var(--gray-text);
  margin-bottom: 25px;
  line-height: 1.6;
}

.metrics-row {
  display: flex;
  gap: 20px;
  margin-bottom: 15px;
}

.metric-card {
  border: 1px solid var(--border);
  padding: 20px 30px;
  min-width: 150px;
}

.metric-value {
  font-family: var(--font-mono);
  font-size: 1.8rem;
  font-weight: 700;
  margin-bottom: 5px;
}

.metric-label {
  font-size: 0.85rem;
  color: #999;
}

/* Simulation Steps */
.steps-container {
  border: 1px solid var(--border);
  padding: 30px;
  position: relative;
}

.steps-header {
  font-family: var(--font-mono);
  font-size: 0.8rem;
  color: #999;
  margin-bottom: 25px;
  display: flex;
  align-items: center;
  gap: 8px;
}

.diamond-icon {
  font-size: 1.2rem;
  line-height: 1;
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
  font-family: var(--font-mono);
  font-weight: 700;
  color: var(--black);
  opacity: 0.3;
}

.step-info {
  flex: 1;
}

.step-title {
  font-weight: 700;
  font-size: 1rem;
  margin-bottom: 4px;
}

.step-desc {
  font-size: 0.85rem;
  color: var(--gray-text);
}


.console-box {
  border: 1px solid #CCC; /* Outer solid line */
  padding: 8px; /* Inner padding creates double border effect */
}

.console-section {
  padding: 20px;
}

.console-section.btn-section {
  padding-top: 0;
}

.console-header {
  display: flex;
  justify-content: space-between;
  margin-bottom: 15px;
  font-family: var(--font-mono);
  font-size: 0.75rem;
  color: #666;
}

.upload-zone {
  border: 1px dashed #CCC;
  height: 200px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all 0.3s;
  background: #FAFAFA;
}

.upload-zone:hover {
  background: #F0F0F0;
  border-color: #999;
}

.upload-placeholder {
  text-align: center;
}

.upload-icon {
  width: 40px;
  height: 40px;
  border: 1px solid #DDD;
  display: flex;
  align-items: center;
  justify-content: center;
  margin: 0 auto 15px;
  color: #999;
}

.upload-title {
  font-weight: 700;
  font-size: 0.9rem;
  margin-bottom: 5px;
}

.upload-hint {
  font-family: var(--font-mono);
  font-size: 0.75rem;
  color: #999;
}

.file-list {
  width: 100%;
  padding: 15px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.file-item {
  display: flex;
  align-items: center;
  background: var(--white);
  padding: 8px 12px;
  border: 1px solid #EEE;
  font-family: var(--font-mono);
  font-size: 0.85rem;
}

.file-name {
  flex: 1;
  margin: 0 10px;
}

.remove-btn {
  background: none;
  border: none;
  cursor: pointer;
  font-size: 1.2rem;
  color: #999;
}

.console-divider {
  display: flex;
  align-items: center;
  margin: 10px 0;
}

.console-divider::before,
.console-divider::after {
  content: '';
  flex: 1;
  height: 1px;
  background: #EEE;
}

.console-divider span {
  padding: 0 15px;
  font-family: var(--font-mono);
  font-size: 0.7rem;
  color: #BBB;
  letter-spacing: 1px;
}

.input-wrapper {
  position: relative;
  border: 1px solid #DDD;
  background: #FAFAFA;
}

.code-input {
  width: 100%;
  border: none;
  background: transparent;
  padding: 20px;
  font-family: var(--font-mono);
  font-size: 0.9rem;
  line-height: 1.6;
  resize: vertical;
  outline: none;
  min-height: 150px;
}

.model-badge {
  position: absolute;
  bottom: 10px;
  right: 15px;
  font-family: var(--font-mono);
  font-size: 0.7rem;
  color: #AAA;
}

.start-engine-btn {
  width: 100%;
  background: var(--black);
  color: var(--white);
  border: none;
  padding: 20px;
  font-family: var(--font-mono);
  font-weight: 700;
  font-size: 1.1rem;
  display: flex;
  justify-content: space-between;
  align-items: center;
  cursor: pointer;
  transition: all 0.3s ease;
  letter-spacing: 1px;
  position: relative;
  overflow: hidden;
}

/* Clickable status (not disabled) */
.start-engine-btn:not(:disabled) {
  background: var(--black);
  border: 1px solid var(--black);
  animation: pulse-border 2s infinite;
}

.start-engine-btn:hover:not(:disabled) {
  background: var(--orange);
  border-color: var(--orange);
  transform: translateY(-2px);
}

.start-engine-btn:active:not(:disabled) {
  transform: translateY(0);
}

.start-engine-btn:disabled {
  background: #E5E5E5;
  color: #999;
  cursor: not-allowed;
  transform: none;
  border: 1px solid #E5E5E5;
}

/* Guide animation: Subtle border pulse */
@keyframes pulse-border {
  0% { box-shadow: 0 0 0 0 rgba(0, 0, 0, 0.2); }
  70% { box-shadow: 0 0 0 6px rgba(0, 0, 0, 0); }
  100% { box-shadow: 0 0 0 0 rgba(0, 0, 0, 0); }
}

/* Capabilities List (New) */
.capabilities-list {
  margin: 20px 0;
  display: flex;
  flex-direction: column;
  gap: 10px;
  background: #f8f9fa;
  padding: 15px;
  border-left: 3px solid var(--orange);
}

.cap-item {
  display: flex;
  align-items: center;
  gap: 10px;
  font-family: var(--font-mono);
  font-size: 0.85rem;
  color: #555;
}

.cap-icon {
  color: var(--orange);
  font-weight: bold;
}

/* Responsive */
@media (max-width: 1024px) {
  .main-content {
    padding: 40px 20px;
  }
  
  .hero-section .console-box {
    max-width: 100%;
  }
  
  .info-panel {
    max-width: 100%;
  }
}
</style>
