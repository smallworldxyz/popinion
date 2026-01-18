<template>
  <div class="settings-modal-overlay" v-if="isOpen">
    <div class="settings-modal" v-outside-click="close">
      <!-- Header -->
      <div class="modal-header">
        <h3>System Settings</h3>
        <button class="close-btn" @click="close">×</button>
      </div>

      <!-- Content -->
      <div class="modal-body">
        <p class="section-desc">
          Select your AI Provider and enter your API Key.
        </p>

        <!-- Provider Selector -->
        <div class="setting-group">
            <label class="setting-label">AI Provider</label>
            <div class="provider-selector">
                <button 
                    v-for="p in providers" 
                    :key="p.id"
                    class="provider-btn"
                    :class="{ active: activeProvider === p.id }"
                    @click="activeProvider = p.id"
                >
                    {{ p.name }}
                </button>
            </div>
        </div>

        <!-- Dynamic Key Input -->
        <div class="setting-group">
          <label class="setting-label">{{ getLabelForProvider(activeProvider) }}</label>
          <div class="input-wrapper">
            <input 
              :type="showKey ? 'text' : 'password'" 
              v-model="apiKeys[activeProvider]"
              :placeholder="getPlaceholderForProvider(activeProvider)"
              class="setting-input"
            />
            <button class="toggle-visibility" @click="showKey = !showKey">
              {{ showKey ? 'Hide' : 'Show' }}
            </button>
          </div>
          <p class="help-text">{{ getHelpTextForProvider(activeProvider) }}</p>
        </div>
        
        <!-- Model Override (Advanced) -->
        <div class="setting-group">
            <label class="setting-label">Model Name (Optional)</label>
            <input 
                type="text" 
                v-model="customModels[activeProvider]" 
                :placeholder="getDefaultModel(activeProvider)"
                class="setting-input" 
            />
        </div>

      </div>

      <!-- Footer -->
      <div class="modal-footer">
        <button class="btn-cancel" @click="close">Cancel</button>
        <button class="btn-save" @click="saveSettings">
          {{ saved ? 'Saved!' : 'Save Changes' }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted, watch, reactive } from 'vue'

const props = defineProps({
    isOpen: {
        type: Boolean,
        default: false
    }
})

const emit = defineEmits(['close'])

const providers = [
    { id: 'openai', name: 'OpenAI' },
    { id: 'google', name: 'Gemini' },
    { id: 'anthropic', name: 'Claude' }
]

const activeProvider = ref('openai')
const apiKeys = reactive({
    openai: '',
    google: '',
    anthropic: ''
})
const customModels = reactive({
    openai: '',
    google: '',
    anthropic: ''
})

const showKey = ref(false)
const saved = ref(false)

// Init from localStorage
onMounted(() => {
    loadSettings()
})

watch(() => props.isOpen, (val) => {
    if (val) loadSettings()
})

const loadSettings = () => {
    activeProvider.value = localStorage.getItem('po_active_provider') || 'openai'
    
    apiKeys.openai = localStorage.getItem('po_openai_key') || ''
    apiKeys.google = localStorage.getItem('po_google_key') || ''
    apiKeys.anthropic = localStorage.getItem('po_anthropic_key') || ''
    
    customModels.openai = localStorage.getItem('po_openai_model') || ''
    customModels.google = localStorage.getItem('po_google_model') || ''
    customModels.anthropic = localStorage.getItem('po_anthropic_model') || ''
    
    saved.value = false
}

const saveSettings = () => {
    localStorage.setItem('po_active_provider', activeProvider.value)
    
    localStorage.setItem('po_openai_key', apiKeys.openai)
    localStorage.setItem('po_google_key', apiKeys.google)
    localStorage.setItem('po_anthropic_key', apiKeys.anthropic)
    
    localStorage.setItem('po_openai_model', customModels.openai)
    localStorage.setItem('po_google_model', customModels.google)
    localStorage.setItem('po_anthropic_model', customModels.anthropic)
    
    saved.value = true
    setTimeout(() => {
        close()
    }, 800)
}

const close = () => {
    emit('close')
}

// Helpers
const getLabelForProvider = (id) => {
    const map = { openai: 'OpenAI API Key', google: 'Google AI Key', anthropic: 'Anthropic API Key' }
    return map[id] || 'API Key'
}

const getPlaceholderForProvider = (id) => {
    const map = { openai: 'sk-...', google: 'AIza...', anthropic: 'sk-ant-...' }
    return map[id] || ''
}

const getHelpTextForProvider = (id) => {
    const map = { 
        openai: 'Standard OpenAI models (gpt-4o, gpt-4o-mini).', 
        google: 'Gemini models (gemini/gemini-1.5-pro, gemini/gemini-1.5-flash).', 
        anthropic: 'Claude models (anthropic/claude-3-5-sonnet-20240620).' 
    }
    return map[id] || ''
}

const getDefaultModel = (id) => {
    const map = { 
        openai: 'gpt-4o-mini', 
        google: 'gemini/gemini-1.5-pro', 
        anthropic: 'anthropic/claude-3-5-sonnet-20240620' 
    }
    return map[id]
}

// Directive
const vOutsideClick = {
  mounted(el, binding) {
    el.clickOutsideEvent = function(event) {
      if (!(el === event.target || el.contains(event.target))) {
        binding.value(event, el);
      }
    };
    document.body.addEventListener('click', el.clickOutsideEvent);
  },
  unmounted(el) {
    document.body.removeEventListener('click', el.clickOutsideEvent);
  }
}
</script>

<style scoped>
.settings-modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    backdrop-filter: blur(4px);
}

.settings-modal {
    background: #1a1a1a;
    width: 480px;
    max-width: 90vw;
    border-radius: 12px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    box-shadow: 0 20px 50px rgba(0,0,0,0.5);
    display: flex;
    flex-direction: column;
    overflow: hidden;
}

.modal-header {
    padding: 20px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: #222;
}

.modal-header h3 {
    margin: 0;
    color: #fff;
    font-size: 16px;
    font-weight: 600;
}

.close-btn {
    background: none;
    border: none;
    color: #888;
    font-size: 24px;
    cursor: pointer;
    line-height: 1;
}

.close-btn:hover { color: #fff; }

.modal-body {
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 20px;
}

.section-desc {
    margin: 0;
    font-size: 13px;
    color: #888;
    line-height: 1.4;
}

.setting-group {
    display: flex;
    flex-direction: column;
    gap: 8px;
}

.setting-label {
    font-size: 12px;
    font-weight: 600;
    color: #ccc;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}

.provider-selector {
    display: flex;
    gap: 8px;
    background: #333;
    padding: 4px;
    border-radius: 6px;
}

.provider-btn {
    flex: 1;
    background: transparent;
    border: none;
    color: #888;
    padding: 8px;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    border-radius: 4px;
    transition: all 0.2s;
}

.provider-btn.active {
    background: #444;
    color: #fff;
    box-shadow: 0 2px 4px rgba(0,0,0,0.2);
}

.input-wrapper {
    display: flex;
    gap: 8px;
}

.setting-input {
    flex: 1;
    background: #333;
    border: 1px solid #444;
    color: #fff;
    padding: 10px;
    border-radius: 6px;
    font-family: monospace;
    font-size: 13px;
}

.setting-input:focus {
    outline: none;
    border-color: #00FF9D;
    background: #2a2a2a;
}

.toggle-visibility {
    background: #333;
    border: 1px solid #444;
    color: #ccc;
    padding: 0 12px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 12px;
}

.help-text {
    font-size: 11px;
    color: #666;
    margin: 0;
}

.modal-footer {
    padding: 15px 20px;
    background: #222;
    border-top: 1px solid rgba(255, 255, 255, 0.05);
    display: flex;
    justify-content: flex-end;
    gap: 10px;
}

.btn-cancel {
    background: transparent;
    border: 1px solid #444;
    color: #ccc;
    padding: 8px 16px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 13px;
}

.btn-save {
    background: #00FF9D;
    border: none;
    color: #000;
    font-weight: 600;
    padding: 8px 16px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 13px;
    transition: all 0.2s;
}

.btn-save:hover {
    background: #00cc7d;
}
</style>
