<template>
  <div class="create-view">
    <!-- Navbar (Simplified) -->
    <nav class="navbar glass-panel">
      <div class="nav-brand">
        <button class="back-btn" @click="goHome">←</button>
        <span class="brand-text">Initialize Simulation</span>
      </div>
    </nav>

    <main class="main-container u-container">
      
      <!-- Launch Header -->
      <div class="launch-header fade-in">
        <h1 class="hero-title">New Operation</h1>
        <p class="hero-subtitle">Define parameters for your future rehearsal.</p>
      </div>

      <div class="creation-panel glass-panel fade-in-up">
        
        <!-- Step 1: Intelligence -->
        <div class="input-section">
          <div class="section-label">
            <span class="step-num">01</span>
            <h3>Intel Baseline</h3>
          </div>
          <p class="section-desc">Upload core documents to ground the simulation in reality.</p>
          
          <div 
            class="upload-zone"
            :class="{ 'is-dragging': isDragOver, 'has-files': files.length > 0 }"
            @dragover.prevent="handleDragOver"
            @dragleave.prevent="handleDragLeave"
            @drop.prevent="handleDrop"
            @click="triggerFileInput"
          >
            <input ref="fileInput" type="file" multiple accept=".pdf,.md,.txt" @change="handleFileSelect" hidden :disabled="loading" />
            
            <div v-if="files.length === 0" class="upload-placeholder">
              <div class="upload-icon">📂</div>
              <span class="upload-text">Drop context files here</span>
            </div>

            <div v-else class="file-list">
              <div v-for="(file, index) in files" :key="index" class="file-chip">
                <span class="file-icon">📄</span>
                <span class="file-name">{{ file.name }}</span>
                <button @click.stop="removeFile(index)" class="remove-btn">×</button>
              </div>
              <div class="add-more">+ Add</div>
            </div>
          </div>
        </div>

        <!-- Step 2: Scenario -->
        <div class="input-section">
          <div class="section-label">
            <span class="step-num">02</span>
            <h3>Scenario Prompt</h3>
          </div>
          <p class="section-desc">Describe the triggering event or crisis.</p>
          <div class="textarea-wrapper">
             <textarea
              v-model="formData.simulationRequirement"
              class="glass-input"
              placeholder="Example: A new AI regulation bill is leaked, causing panic in the tech sector..."
              rows="4"
              :disabled="loading"
            ></textarea>
          </div>
        </div>

        <!-- Action Footer -->
        <div class="action-footer">
          <button 
            class="launch-btn"
            @click="startProject"
            :disabled="!canSubmit || loading"
          >
            <span v-if="loading" class="spinner-sm"></span>
            <span v-else>Initialize Engine</span>
          </button>
        </div>

      </div>

      <!-- Launch Checklist (Visual) -->
      <div class="launch-checklist">
        <div class="checklist-item" :class="{ active: files.length > 0 }">
          <div class="check-circle">{{ files.length > 0 ? '✓' : '1' }}</div>
          <span>Knowledge Graph</span>
        </div>
        <div class="checklist-line"></div>
        <div class="checklist-item" :class="{ active: formData.simulationRequirement.length > 10 }">
          <div class="check-circle">{{ formData.simulationRequirement.length > 10 ? '✓' : '2' }}</div>
          <span>Scenario Injection</span>
        </div>
        <div class="checklist-line"></div>
        <div class="checklist-item">
          <div class="check-circle">3</div>
          <span>Agent Allocation</span>
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
  return formData.value.simulationRequirement.trim().length > 0 && files.value.length > 0
})

// Actions
const goHome = () => router.push('/')

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
  loading.value = true
  
  // Simulate delay for "Initializing" feel
  setTimeout(() => {
    import('../store/pendingUpload.js').then(({ setPendingUpload }) => {
      setPendingUpload(files.value, formData.value.simulationRequirement)
      router.push({ name: 'Process', params: { projectId: 'new' } })
    })
  }, 800)
}
</script>

<style scoped>
.create-view {
  min-height: 100vh;
  background: radial-gradient(circle at 50% -20%, #1e1b4b 0%, var(--bg-app) 70%);
  display: flex; flex-direction: column;
}

/* Navbar */
.navbar {
  margin: 24px; padding: 0 24px; height: 64px;
  display: flex; align-items: center;
}
.nav-brand { display: flex; align-items: center; gap: 16px; }
.back-btn { 
  font-size: 24px; color: var(--text-muted); padding: 0 8px; 
  transition: color 0.2s;
}
.back-btn:hover { color: var(--primary); transform: translateX(-2px); }
.brand-text { font-weight: 600; color: var(--text-main); letter-spacing: 0.5px; }

/* Main Content */
.main-container {
  max-width: 800px;
  margin-top: 40px;
  padding-bottom: 80px;
}

.launch-header { text-align: center; margin-bottom: 40px; }
.hero-title { 
  font-size: 42px; font-weight: 800; letter-spacing: -1px; margin-bottom: 8px; 
  background: linear-gradient(135deg, #fff 0%, #a5b4fc 100%);
  -webkit-background-clip: text; -webkit-text-fill-color: transparent;
}
.hero-subtitle { color: var(--text-muted); font-size: 16px; }

/* Panel */
.creation-panel {
  padding: 40px;
  box-shadow: var(--shadow-lg);
  position: relative;
  overflow: hidden;
}

/* Glowing Border Top */
.creation-panel::before {
  content: ''; position: absolute; top: 0; left: 0; right: 0; height: 1px;
  background: linear-gradient(90deg, transparent, var(--primary), transparent);
  opacity: 0.5;
}

.input-section { margin-bottom: 40px; }

.section-label { display: flex; align-items: center; gap: 12px; margin-bottom: 8px; }
.step-num { 
  font-family: var(--font-mono); font-size: 12px; color: var(--primary); 
  background: rgba(255, 87, 34, 0.1); padding: 2px 6px; border-radius: 4px;
}
.section-label h3 { font-size: 18px; font-weight: 600; color: var(--text-main); }
.section-desc { margin-left: 36px; color: var(--text-muted); font-size: 14px; margin-bottom: 16px; }

/* Upload Zone */
.upload-zone {
  margin-left: 36px;
  border: 1px dashed var(--border-light);
  background: rgba(0,0,0,0.2);
  border-radius: var(--radius-md);
  padding: 32px;
  text-align: center;
  transition: all 0.2s;
  cursor: pointer;
}
.upload-zone:hover, .upload-zone.is-dragging {
  border-color: var(--primary);
  background: rgba(255, 87, 34, 0.05);
  box-shadow: inset 0 0 20px rgba(255, 87, 34, 0.1);
}

.upload-icon { font-size: 32px; margin-bottom: 8px; opacity: 0.7; }
.upload-text { color: var(--text-muted); font-weight: 500; }

.file-list { display: flex; flex-wrap: wrap; gap: 8px; justify-content: center; }
.file-chip {
  background: rgba(255,255,255,0.05); border: 1px solid var(--border-light);
  padding: 6px 12px; border-radius: 20px; font-size: 13px; color: var(--text-main);
  display: flex; align-items: center; gap: 8px;
}
.remove-btn { color: var(--text-muted); font-size: 18px; line-height: 1; margin-left: 4px; }
.remove-btn:hover { color: var(--error); }
.add-more { color: var(--primary); font-size: 13px; padding: 6px 12px; opacity: 0.8; }

/* Textarea */
.textarea-wrapper { margin-left: 36px; }
.glass-input {
  width: 100%; padding: 16px;
  background: rgba(0,0,0,0.2); border: 1px solid var(--border-light);
  border-radius: var(--radius-md); color: var(--text-main);
  font-family: var(--font-main); font-size: 14px; resize: vertical;
  transition: all 0.2s;
}
.glass-input:focus {
  border-color: var(--primary); outline: none;
  box-shadow: 0 0 0 1px var(--primary-glow);
  background: rgba(0,0,0,0.3);
}

/* Launch Button */
.launch-btn {
  width: 100%; padding: 16px;
  background: var(--primary); color: white;
  font-weight: 700; font-size: 16px; border-radius: var(--radius-md);
  letter-spacing: 0.5px; text-transform: uppercase;
  transition: all 0.2s;
  box-shadow: 0 0 20px var(--primary-glow);
}
.launch-btn:hover:not(:disabled) {
  transform: translateY(-2px);
  background: var(--primary-hover);
  box-shadow: 0 0 30px var(--primary-glow);
}
.launch-btn:disabled { opacity: 0.5; cursor: not-allowed; box-shadow: none; }

/* Checklist Animation */
.launch-checklist {
  margin-top: 60px; display: flex; align-items: center; justify-content: center; gap: 16px;
}
.checklist-item {
  display: flex; flex-direction: column; align-items: center; gap: 8px; opacity: 0.4; transition: all 0.3s;
}
.checklist-item.active { opacity: 1; transform: scale(1.05); }
.checklist-item.active .check-circle { 
  background: var(--success); border-color: var(--success); color: black;
  box-shadow: 0 0 15px var(--success-glow);
}

.check-circle {
  width: 32px; height: 32px; border-radius: 50%; border: 2px solid var(--text-muted);
  display: flex; align-items: center; justify-content: center; font-weight: 700;
  font-size: 14px; color: var(--text-main); transition: all 0.3s;
}
.checklist-line { width: 60px; height: 2px; background: var(--border-light); }
.checklist-item.active + .checklist-line { background: var(--success); }

/* Globals */
.fade-in-up { animation: fadeInUp 0.6s ease-out forwards; }
@keyframes fadeInUp { from { opacity: 0; transform: translateY(20px); } to { opacity: 1; transform: translateY(0); } }

.spinner-sm {
  width: 20px; height: 20px; border: 2px solid rgba(255,255,255,0.3); 
  border-top-color: white; border-radius: 50%; animation: spin 1s linear infinite;
  display: inline-block;
}
@keyframes spin { to { transform: rotate(360deg); } }
</style>
