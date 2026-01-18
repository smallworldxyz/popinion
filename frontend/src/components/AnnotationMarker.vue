<template>
  <div class="annotation-marker" @click.stop="toggle">
    <div class="marker-icon" :class="{ 'has-content': hasContent }">
      <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 11.5a8.38 8.38 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.38 8.38 0 0 1-3.8-.9L3 21l1.9-5.7a8.38 8.38 0 0 1-.9-3.8 8.5 8.5 0 0 1 4.7-7.6 8.38 8.38 0 0 1 3.8-.9h.5a8.48 8.48 0 0 1 8 8v.5z"></path></svg>
    </div>
    
    <div v-if="isOpen" class="annotation-popover" @click.stop>
      <div class="popover-header">
        <span class="popover-title">CRITIQUE / NOTE</span>
        <button class="close-btn" @click="toggle">×</button>
      </div>
      <textarea 
        v-model="localContent" 
        class="annotation-input" 
        placeholder="Add your critique here..." 
        rows="3"
        ref="textareaRef"
      ></textarea>
      <div class="popover-footer">
        <button class="save-btn" @click="save" :disabled="isSaving">
            {{ isSaving ? 'SAVING...' : 'SAVE' }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, watch, nextTick } from 'vue'

const props = defineProps({
  initialContent: String,
  actionId: String,
  simulationId: String
})

const emit = defineEmits(['save', 'delete'])

const isOpen = ref(false)
const localContent = ref(props.initialContent || '')
const isSaving = ref(false)
const textareaRef = ref(null)

const hasContent = ref(!!props.initialContent)

watch(() => props.initialContent, (newVal) => {
  localContent.value = newVal || ''
  hasContent.value = !!newVal
})

const toggle = () => {
    isOpen.value = !isOpen.value
    if (isOpen.value) {
        nextTick(() => {
            textareaRef.value?.focus()
        })
    }
}

const save = async () => {
    isSaving.value = true
    try {
        await emit('save', {
            actionId: props.actionId,
            content: localContent.value
        })
        hasContent.value = !!localContent.value
        isOpen.value = false
    } finally {
        isSaving.value = false
    }
}
</script>

<style scoped>
.annotation-marker {
    position: relative;
    display: inline-block;
    cursor: pointer;
}

.marker-icon {
    color: #444;
    transition: color 0.2s;
    padding: 4px;
}
.marker-icon:hover { color: #888; }
.marker-icon.has-content { color: #f59e0b; } /* Amber for annotations */

.annotation-popover {
    position: absolute;
    right: 0;
    top: 24px;
    width: 250px;
    background: #1a1a1a;
    border: 1px solid #333;
    border-radius: 6px;
    padding: 10px;
    z-index: 1000;
    box-shadow: 0 4px 12px rgba(0,0,0,0.5);
}

.popover-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
}

.popover-title {
    font-size: 10px;
    font-weight: 700;
    color: #666;
    letter-spacing: 0.5px;
}

.close-btn {
    background: none;
    border: none;
    color: #666;
    font-size: 16px;
    cursor: pointer;
    line-height: 1;
}

.annotation-input {
    width: 100%;
    background: #000;
    border: 1px solid #333;
    color: #ccc;
    font-family: inherit;
    font-size: 12px;
    padding: 8px;
    border-radius: 4px;
    resize: vertical;
}

.popover-footer {
    display: flex;
    justify-content: flex-end;
    margin-top: 8px;
}

.save-btn {
    background: #f59e0b;
    color: #000;
    border: none;
    font-size: 10px;
    font-weight: 700;
    padding: 4px 10px;
    border-radius: 3px;
    cursor: pointer;
}
.save-btn:hover:not(:disabled) {
    opacity: 0.9;
}
</style>
