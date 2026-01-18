<template>
  <div class="home-layout">
    <nav class="navbar">
      <div class="nav-brand">
        <span class="brand-logo">●</span>
        <span class="brand-text">Popinion</span>
      </div>
      <div class="nav-actions">
        <!-- <router-link to="/library" class="nav-link">Library</router-link> -->
        <a href="https://github.com/rithythul/pubop" target="_blank" class="nav-link icon-link">
          GitHub <span>↗</span>
        </a>
      </div>
    </nav>

    <main class="main-container">
      <div class="hero-section">
        <h1 class="hero-title">Simulate Public Opinion</h1>
        <p class="hero-subtitle">
          Don't guess the future. Rehearse it.
        </p>
      </div>

      <div class="creation-card">
        <div class="card-header">
          <div class="step-indicator">New Simulation</div>
        </div>

        <div class="input-group">
          <label class="input-label">
            1. Knowledge Baseline
            <span class="label-hint">Upload documents (PDF, MD, TXT) to ground the simulation in reality.</span>
          </label>
          
          <div 
            class="upload-area"
            :class="{ 'is-dragging': isDragOver, 'has-files': files.length > 0 }"
            @dragover.prevent="handleDragOver"
            @dragleave.prevent="handleDragLeave"
            @drop.prevent="handleDrop"
            @click="triggerFileInput"
          >
            <input
              ref="fileInput"
              type="file"
              multiple
              accept=".pdf,.md,.txt"
              @change="handleFileSelect"
              hidden
              :disabled="loading"
            />
            
            <div v-if="files.length === 0" class="upload-placeholder">
              <div class="upload-icon-circle">
                <svg width="24" height="24" viewBox="0 0 24 24" fill="none" class="icon">
                  <path d="M12 4V20M20 12H4" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                </svg>
              </div>
              <span class="upload-text">Drop context files here</span>
              <span class="upload-subtext">or click to browse</span>
            </div>

            <div v-else class="file-preview-list">
              <div v-for="(file, index) in files" :key="index" class="file-chip">
                <span class="file-icon">doc</span>
                <span class="file-name">{{ file.name }}</span>
                <button @click.stop="removeFile(index)" class="remove-file-btn">×</button>
              </div>
              <div class="add-more-trigger">
                <span>+ Add more</span>
              </div>
            </div>
          </div>
        </div>

        <div class="input-group">
          <label class="input-label">
            2. Scenario Prompt
            <span class="label-hint">What situation should the agents react to?</span>
          </label>
          <div class="textarea-wrapper">
            <textarea
              v-model="formData.simulationRequirement"
              class="scenario-input"
              placeholder="e.g., A major tech company releases a new AI product that accidentally insults a user. The stock price drops 5%..."
              rows="4"
              :disabled="loading"
            ></textarea>
          </div>
        </div>

        <div class="action-footer">
          <button 
            class="primary-btn"
            @click="startProject"
            :disabled="!canSubmit || loading"
          >
            <span v-if="loading">Initializing...</span>
            <span v-else>Start Simulation Engine</span>
            <span v-if="!loading" class="btn-icon">→</span>
          </button>
        </div>
      </div>

      <!-- Feature Capabilities Grid -->
      <div class="features-grid">
        <div class="feature-item">
          <div class="feature-icon">🔍</div>
          <div class="feature-content">
            <h3>Graph-Powered Context</h3>
            <p>Converts flat documents into a Knowledge Graph for deep agent reasoning.</p>
          </div>
        </div>
        <div class="feature-item">
          <div class="feature-icon">🌍</div>
          <div class="feature-content">
            <h3>Multi-Platform Simulation</h3>
             <p>Simulates discourse across Twitter, Reddit, and News platforms simultaneously.</p>
          </div>
        </div>
        <div class="feature-item">
          <div class="feature-icon">🤖</div>
          <div class="feature-content">
            <h3>Cognitive Agents</h3>
            <p>Agents with memory, bias, and dynamic opinion evolution.</p>
          </div>
        </div>
      </div>
    </main>
  </div>
</template>

<script setup>
import { ref, computed } from 'vue'
import { useRouter } from 'vue-router'

const router = useRouter()

// Data
const formData = ref({ simulationRequirement: '' })
const files = ref([])
const loading = ref(false)
const isDragOver = ref(false)
const fileInput = ref(null)

// Computed
const canSubmit = computed(() => {
  return formData.value.simulationRequirement.trim() !== '' && files.value.length > 0
})

// Actions
const triggerFileInput = () => {
  if (!loading.value) fileInput.value?.click()
}

const handleFileSelect = (event) => {
  addFiles(Array.from(event.target.files))
}

const handleDragOver = () => { isDragOver.value = true }
const handleDragLeave = () => { isDragOver.value = false }
const handleDrop = (e) => {
  isDragOver.value = false
  if (!loading.value) addFiles(Array.from(e.dataTransfer.files))
}

const addFiles = (newFiles) => {
  const valid = newFiles.filter(f => ['pdf', 'md', 'txt'].includes(f.name.split('.').pop().toLowerCase()))
  files.value.push(...valid)
}

const removeFile = (index) => {
  files.value.splice(index, 1)
}

const startProject = () => {
  if (!canSubmit.value || loading.value) return
  
  import('../store/pendingUpload.js').then(({ setPendingUpload }) => {
    setPendingUpload(files.value, formData.value.simulationRequirement)
    router.push({ name: 'Process', params: { projectId: 'new' } })
  })
}
</script>

<style scoped>
.home-layout {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  background: radial-gradient(circle at 50% 0%, #FFF 0%, #FAFAFA 100%);
}

.navbar {
  height: var(--header-height);
  padding: 0 40px;
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.brand-logo {
  color: var(--primary);
  font-size: 20px;
  margin-right: 8px;
}

.brand-text {
  font-weight: 700;
  font-size: 18px;
  letter-spacing: -0.5px;
  color: var(--text-main);
}

.nav-link {
  color: var(--text-muted);
  text-decoration: none;
  font-weight: 500;
  font-size: 14px;
  transition: color 0.2s;
}

.nav-link:hover {
  color: var(--text-main);
}

.main-container {
  flex: 1;
  max-width: 800px;
  margin: 0 auto;
  padding: 60px 20px;
  width: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
}

.hero-section {
  text-align: center;
  margin-bottom: 48px;
}

.hero-title {
  font-size: 48px;
  font-weight: 800;
  letter-spacing: -1.5px;
  line-height: 1.1;
  margin-bottom: 16px;
  background: linear-gradient(180deg, var(--text-main) 0%, #444 100%);
  -webkit-background-clip: text;
  background-clip: text;
  -webkit-text-fill-color: transparent;
}

.hero-subtitle {
  font-size: 18px;
  color: var(--text-muted);
  font-weight: 400;
}

/* Card Styling */
.creation-card {
  width: 100%;
  background: var(--bg-surface);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-lg);
  border: 1px solid var(--border-light);
  overflow: hidden;
  padding: 32px;
  margin-bottom: 60px;
}

.card-header {
  margin-bottom: 24px;
  border-bottom: 1px solid var(--border-subtle);
  padding-bottom: 16px;
}

.step-indicator {
  font-size: 12px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 1px;
  color: var(--primary);
}

.input-group {
  margin-bottom: 24px;
}

.input-label {
  display: block;
  font-size: 14px;
  font-weight: 600;
  color: var(--text-main);
  margin-bottom: 8px;
}

.label-hint {
  font-weight: 400;
  color: var(--text-muted);
  margin-left: 8px;
  font-size: 13px;
}

/* Upload Area */
.upload-area {
  border: 2px dashed var(--border-light);
  border-radius: var(--radius-md);
  padding: 32px;
  text-align: center;
  cursor: pointer;
  transition: all 0.2s;
  background: var(--bg-subtle);
}

.upload-area:hover, .upload-area.is-dragging {
  border-color: var(--primary);
  background: rgba(255, 69, 0, 0.02);
}

.upload-icon-circle {
  width: 48px;
  height: 48px;
  background: var(--bg-surface);
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  margin: 0 auto 12px;
  color: var(--primary);
  box-shadow: var(--shadow-sm);
}

.upload-text {
  display: block;
  font-weight: 500;
  color: var(--text-main);
}

.upload-subtext {
  font-size: 13px;
  color: var(--text-muted);
}

.file-preview-list {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  justify-content: center;
}

.file-chip {
  background: var(--bg-surface);
  border: 1px solid var(--border-light);
  padding: 6px 12px;
  border-radius: var(--radius-full);
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  box-shadow: var(--shadow-sm);
}

.file-icon {
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
  color: var(--text-faint);
}

.remove-file-btn {
  color: var(--text-muted);
  font-size: 16px;
  line-height: 1;
}

.remove-file-btn:hover {
  color: var(--status-error);
}

/* Textarea */
.textarea-wrapper {
  position: relative;
}

.scenario-input {
  width: 100%;
  padding: 16px;
  border: 1px solid var(--border-light);
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-main);
  resize: vertical;
  transition: border-color 0.2s;
  font-size: 14px;
  line-height: 1.6;
}

.scenario-input:focus {
  border-color: var(--primary);
  box-shadow: 0 0 0 3px rgba(255, 69, 0, 0.1);
}

/* Actions */
.action-footer {
  margin-top: 32px;
}

.primary-btn {
  width: 100%;
  background: var(--primary);
  color: white;
  padding: 16px;
  border-radius: var(--radius-md);
  font-weight: 600;
  font-size: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  transition: all 0.2s;
}

.primary-btn:hover:not(:disabled) {
  background: var(--primary-hover);
  transform: translateY(-1px);
  box-shadow: var(--shadow-md);
}

.primary-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

/* Features Grid */
.features-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 24px;
  width: 100%;
}

.feature-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
}

.feature-icon {
  font-size: 24px;
  margin-bottom: 12px;
  background: var(--bg-surface);
  width: 48px;
  height: 48px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--border-subtle);
}

.feature-content h3 {
  font-size: 14px;
  font-weight: 600;
  margin-bottom: 4px;
  color: var(--text-main);
}

.feature-content p {
  font-size: 13px;
  color: var(--text-muted);
  max-width: 200px;
}

@media (max-width: 768px) {
  .features-grid {
    grid-template-columns: 1fr;
  }
}
</style>
