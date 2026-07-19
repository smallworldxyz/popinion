<template>
  <div class="main-view">
    <!-- Shared app rail - home mode: no step indicator, no view switcher -->
    <AppSidebar />

    <main class="home-main">
      <div class="home-scroll">
        <div class="home-inner">
          <!-- Hero: the scenario composer is the focal point -->
          <header class="hero-head">
            <span class="eyebrow">Popinion · rehearse the future</span>
            <h1 class="hero-title">Describe a scenario to simulate.</h1>
            <p class="hero-lead">
              A policy, a message, an event - grounded in real evidence, then rehearsed
              across a population of personas before it meets the public.
            </p>
          </header>

          <!-- Chat-native simulation prompt with reality-seed attachments -->
          <div
            class="chat-box"
            :class="{ 'drag-over': isDragOver }"
            @dragover.prevent="handleDragOver"
            @dragleave.prevent="handleDragLeave"
            @drop.prevent="handleDrop"
          >
            <textarea
              v-model="formData.simulationRequirement"
              class="chat-input"
              placeholder="Describe the scenario to simulate - a policy, a message, an event.
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
                <span class="text-seed-label">Markdown or plain text - pasted posts, notes, transcripts</span>
                <button class="chip-x" @click="clearText" :disabled="loading">×</button>
              </div>
              <textarea
                v-model="pastedText"
                class="text-seed-input"
                placeholder="Paste real opinions, posts, or notes here to ground the sim.

Markdown is fine - it's treated exactly like an uploaded .md file."
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
                placeholder="@channel or t.me link - recent public posts ground the sim"
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
          </div>

          <!-- Model readiness - Models moved to the rail, so surface the gate here. -->
          <div v-if="modelStatus === null" class="model-note checking">Checking model…</div>
          <div v-else-if="!modelStatus.ready" class="model-note missing">
            <div class="model-note-title">No working model yet - Popinion needs a model to run.</div>
            <div class="model-note-reason">{{ modelStatus.reason }}</div>
            <button class="model-note-link" @click="openSettings">Set up a model →</button>
          </div>
          <div v-else class="model-note ready">
            <span class="ready-check">✓</span> Model ready
            <span class="mono ready-model">{{ modelStatus.model }}</span>
          </div>

          <!-- Concise value props - honest to what the engine does -->
          <div class="value-props">
            <div class="prop">
              <span class="prop-name">Grounded personas</span>
              <span class="prop-desc">Built from real evidence - no fabricated traits.</span>
            </div>
            <div class="prop">
              <span class="prop-name">Trust checks</span>
              <span class="prop-desc">Noise floor and ablation keep the signal honest.</span>
            </div>
            <div class="prop">
              <span class="prop-name">Rehearse the future</span>
              <span class="prop-desc">A/B scenarios with a significance test.</span>
            </div>
          </div>

          <!-- Slim pipeline hint: what happens after Start Engine -->
          <ol class="pipeline">
            <li><span class="mono p-num">01</span> Reality injection</li>
            <li><span class="mono p-num">02</span> Environment setup</li>
            <li><span class="mono p-num">03</span> Simulate</li>
            <li><span class="mono p-num">04</span> Report</li>
            <li><span class="mono p-num">05</span> Interact</li>
          </ol>
        </div>
      </div>
    </main>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { getLlmStatus } from '../api/settings'
import { openSettings } from '../settingsDrawer'
import AppSidebar from '../components/AppSidebar.vue'

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

// Typed markdown seed - sent as an .md file, which the backend already parses
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

// Computed attributes: canSubmit - at least one reality seed (file, text, or channel)
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
    ? parts.join(' + ') + ' - real data grounds the simulation'
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
/* Home shares the app shell: the rail sits beside a scrolling main column. */
.main-view {
  height: 100vh;
  display: flex;
  flex-direction: row;
  overflow: hidden;
  background: var(--color-bg);
  color: var(--color-text);
  font-family: var(--font-body);
}
.home-main {
  flex: 1;
  min-width: 0;
  overflow-y: auto;
}
.home-scroll {
  min-height: 100%;
  display: flex;
  justify-content: center;
  padding: 64px 40px 80px;
}
.home-inner {
  width: 100%;
  max-width: 820px;
}

/* Hero heading */
.hero-head { margin-bottom: 28px; }
.hero-title {
  margin: 12px 0 10px;
  font-family: var(--font-display);
  font-weight: 600;
  font-size: var(--fs-xl);
  line-height: 1.08;
  letter-spacing: -0.01em;
  color: var(--color-text);
}
.hero-lead {
  margin: 0;
  max-width: 60ch;
  font-size: var(--fs-md);
  line-height: 1.55;
  color: var(--color-text-muted);
}

/* Compose box - the hero input */
.chat-box {
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  padding: 18px 18px 14px;
  position: relative;
  transition: border-color 0.15s, box-shadow 0.15s;
}
.chat-box:focus-within {
  border-color: var(--color-accent);
  box-shadow: 0 0 0 3px var(--gold-soft);
}
.chat-box.drag-over {
  border-style: dashed;
  border-color: var(--color-accent);
}
.chat-input {
  width: 100%;
  border: none;
  outline: none;
  resize: vertical;
  font-family: var(--font-body);
  font-size: var(--fs-md);
  line-height: 1.55;
  color: var(--color-text);
  background: transparent;
  min-height: 120px;
}
.chat-input::placeholder { color: var(--color-text-muted); opacity: 0.75; }

/* Attached reality seeds */
.seed-chips { display: flex; flex-wrap: wrap; gap: 8px; margin: 10px 0 4px; }
.seed-chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  background: var(--color-bg);
  border: 1px solid var(--color-border);
  border-radius: 999px;
  padding: 4px 10px;
  font-size: var(--fs-xs);
  color: var(--color-text);
}
.chip-name { max-width: 220px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.chip-x {
  border: none;
  background: none;
  cursor: pointer;
  font-size: 1rem;
  line-height: 1;
  color: var(--color-text-muted);
}
.chip-x:hover { color: var(--stance-oppose-strong); }

.text-seed {
  margin: 10px 0 4px;
  padding: 8px 10px;
  background: var(--color-bg);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
}
.text-seed-head { display: flex; align-items: center; gap: 8px; }
.text-seed-label { flex: 1; font-size: var(--fs-xs); color: var(--color-text-muted); }
.text-seed-input {
  width: 100%;
  margin-top: 6px;
  border: none;
  outline: none;
  resize: vertical;
  background: transparent;
  font-family: var(--font-mono);
  font-size: var(--fs-xs);
  line-height: 1.5;
  color: var(--color-text);
  box-sizing: border-box;
}
.text-seed-input::placeholder { color: var(--color-text-muted); opacity: 0.7; }

.telegram-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 10px 0 4px;
  padding: 6px 10px;
  background: var(--color-bg);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
}
.tg-icon { font-size: 0.9rem; }
.tg-input {
  flex: 1;
  min-width: 0;
  border: none;
  outline: none;
  background: transparent;
  font-family: var(--font-mono);
  font-size: var(--fs-xs);
  color: var(--color-text);
}
.tg-input::placeholder { color: var(--color-text-muted); opacity: 0.7; }
.tg-count {
  border: 1px solid var(--color-border);
  background: var(--color-surface);
  border-radius: var(--radius-sm);
  font-family: var(--font-mono);
  font-size: var(--fs-2xs);
  padding: 3px 6px;
  color: var(--color-text-muted);
}

/* Toolbar: attach controls + primary CTA */
.chat-toolbar {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 10px;
  margin-top: 14px;
  padding-top: 14px;
  border-top: 1px solid var(--color-border);
}
.attach-btn {
  flex: none;
  font-family: var(--font-body);
  font-size: var(--fs-xs);
  font-weight: 500;
  padding: 8px 12px;
  border: 1px solid var(--color-border);
  background: transparent;
  color: var(--color-text);
  border-radius: var(--radius-sm);
  cursor: pointer;
  white-space: nowrap;
  transition: border-color 0.15s, color 0.15s;
}
.attach-btn:hover:not(:disabled) { border-color: var(--color-accent); color: var(--color-accent-text); }
.attach-btn:disabled { opacity: 0.4; cursor: not-allowed; }
.attach-hint { flex: 1; min-width: 120px; font-size: var(--fs-2xs); color: var(--color-text-muted); }

.start-engine-btn {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 10px 18px;
  border: 1px solid transparent;
  border-radius: var(--radius);
  background: var(--color-accent);
  color: var(--color-on-accent);
  font-family: var(--font-body);
  font-size: var(--fs-sm);
  font-weight: 600;
  line-height: 1;
  cursor: pointer;
  transition: background 0.15s, transform 0.15s;
}
.start-engine-btn:hover:not(:disabled) { background: var(--color-accent-hover); }
.start-engine-btn:active:not(:disabled) { transform: translateY(1px); }
.start-engine-btn:disabled {
  background: transparent;
  border-color: var(--color-border);
  color: var(--color-text-muted);
  cursor: not-allowed;
}
.btn-arrow { font-size: 1.05em; line-height: 1; }

/* Model readiness */
.model-note {
  margin-top: 14px;
  font-size: var(--fs-sm);
  line-height: 1.5;
}
.model-note.checking {
  font-family: var(--font-mono);
  font-size: var(--fs-2xs);
  color: var(--color-text-muted);
}
.model-note.ready {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  font-size: var(--fs-xs);
  color: var(--color-text-muted);
}
.ready-check { color: var(--stance-support); font-weight: 700; }
.ready-model { color: var(--color-text); }
.model-note.missing {
  border: 1px solid var(--color-border);
  border-left: 3px solid var(--color-accent);
  background: var(--color-surface);
  border-radius: var(--radius-sm);
  padding: 12px 14px;
}
.model-note-title { font-weight: 600; color: var(--color-text); }
.model-note-reason { color: var(--color-text-muted); margin-top: 2px; }
.model-note-link {
  display: inline-block;
  margin-top: 6px;
  padding: 0;
  border: none;
  background: none;
  cursor: pointer;
  font-family: var(--font-body);
  font-size: var(--fs-sm);
  color: var(--color-accent-text);
  font-weight: 600;
  text-decoration: underline;
  text-underline-offset: 3px;
}
.model-note-link:hover { color: var(--color-accent-hover); }

/* Concise value props */
.value-props {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 20px;
  margin-top: 44px;
  padding-top: 32px;
  border-top: 1px solid var(--color-border);
}
.prop { display: flex; flex-direction: column; gap: 6px; }
.prop-name {
  font-family: var(--font-display);
  font-weight: 600;
  font-size: var(--fs-md);
  color: var(--color-text);
}
.prop-desc { font-size: var(--fs-xs); line-height: 1.5; color: var(--color-text-muted); }

/* Slim pipeline hint */
.pipeline {
  display: flex;
  flex-wrap: wrap;
  gap: 8px 18px;
  margin: 28px 0 0;
  padding: 0;
  list-style: none;
}
.pipeline li {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  font-size: var(--fs-xs);
  color: var(--color-text-muted);
}
.p-num {
  font-size: var(--fs-2xs);
  font-weight: 600;
  color: var(--color-accent-text);
}

@media (max-width: 1000px) {
  .home-scroll { padding: 40px 24px 64px; }
  .value-props { grid-template-columns: 1fr; gap: 18px; }
}
</style>
