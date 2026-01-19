<template>
  <div class="modal-overlay" v-if="visible">
    <div class="modal-content">
      <div class="modal-header">
        <h3>Live Sentiment Ingestion</h3>
        <button class="close-btn" @click="$emit('close')">×</button>
      </div>
      
      <div class="modal-body">
        <p class="description">
          Ingest real-world posts from RSS feeds and social channels to seed your simulation.
        </p>

        <!-- Configuration -->
        <div class="form-group">
          <label>Keywords (Optional)</label>
          <input 
            v-model="keywordsStr" 
            placeholder="e.g. inflation, marcos, economy" 
            class="input-field" 
          />
          <span class="hint">Comma separated topics to filter for</span>
        </div>

        <div class="form-group">
          <label>Sources</label>
          <div class="sources-list">
            <div 
              v-for="source in defaultSources" 
              :key="source" 
              class="source-item"
            >
              <input 
                type="checkbox" 
                :id="source" 
                :value="source" 
                v-model="selectedSources"
              >
              <label :for="source">{{ formatSource(source) }}</label>
            </div>
          </div>
        </div>

        <!-- Custom Source Input (Hidden for now to keep simple) -->
        
        <!-- Results Preview -->
        <div v-if="ingestionResult" class="results-preview">
          <div class="result-stats">
            <span class="stat success">✓ {{ ingestionResult.total_ingested }} posts ingested</span>
            <span class="stat error" v-if="ingestionResult.errors?.length">{{ ingestionResult.errors.length }} errors</span>
          </div>
          
          <div class="sample-posts">
            <h4>Latest Ingested Posts:</h4>
            <div v-for="post in ingestionResult.sample_data" :key="post.post_id" class="post-card">
              <div class="post-meta">
                <span class="post-source">{{ post.author_name }}</span>
                <span class="post-date">{{ new Date(post.created_at).toLocaleDateString() }}</span>
              </div>
              <p class="post-content">{{ truncate(post.content, 150) }}</p>
            </div>
          </div>
        </div>

      </div>

      <div class="modal-footer">
        <div class="loading-indicator" v-if="loading">
          <span class="spinner"></span> Processing...
        </div>
        <button class="btn secondary" @click="$emit('close')" :disabled="loading">Cancel</button>
        <button class="btn primary" @click="handleIngest" :disabled="loading">
          {{ ingestionResult ? 'Ingest More' : 'Start Ingestion' }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { fetchIngestionDefaults, startIngestion, type IngestionResult } from '../../../api/ingestion';

const props = defineProps<{
  visible: boolean
}>();

const emit = defineEmits(['close', 'ingested']);

const keywordsStr = ref('');
const defaultSources = ref<string[]>([]);
const selectedSources = ref<string[]>([]);
const loading = ref(false);
const ingestionResult = ref<IngestionResult | null>(null);

const formatSource = (url: string) => {
  try {
    const u = new URL(url);
    return u.hostname.replace('www.', '');
  } catch {
    return url;
  }
};

const truncate = (str: string, n: number) => {
  return (str.length > n) ? str.slice(0, n-1) + '...' : str;
};

const loadDefaults = async () => {
  try {
    const res = await fetchIngestionDefaults();
    if (res.success) {
      defaultSources.value = res.data;
      selectedSources.value = [...res.data]; // Select all by default
    }
  } catch (err) {
    console.error("Failed to load defaults", err);
  }
};

const handleIngest = async () => {
  loading.value = true;
  ingestionResult.value = null;
  
  try {
    const keywords = keywordsStr.value.split(',').map(s => s.trim()).filter(Boolean);
    
    const res = await startIngestion({
      sources: selectedSources.value,
      keywords: keywords.length ? keywords : undefined,
      limit: 10
    });
    
    if (res.success && res.data) {
      ingestionResult.value = res.data;
      emit('ingested', res.data);
    }
  } catch (err) {
    console.error("Ingestion failed", err);
  } finally {
    loading.value = false;
  }
};

onMounted(() => {
  loadDefaults();
});
</script>

<style scoped>
.modal-overlay {
  position: fixed;
  top: 0; left: 0; right: 0; bottom: 0;
  background: rgba(0,0,0,0.7);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 2000;
  backdrop-filter: blur(4px);
}

.modal-content {
  background: var(--bg-primary);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  width: 600px;
  max-width: 90%;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  box-shadow: 0 20px 50px rgba(0,0,0,0.5);
}

.modal-header {
  padding: 16px 20px;
  border-bottom: 1px solid var(--border-color);
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.modal-header h3 {
  margin: 0;
  color: var(--text-primary);
}

.close-btn {
  background: none;
  border: none;
  font-size: 24px;
  color: var(--text-secondary);
  cursor: pointer;
}

.modal-body {
  padding: 20px;
  overflow-y: auto;
}

.description {
  color: var(--text-secondary);
  font-size: 0.9em;
  margin-bottom: 20px;
}

.form-group {
  margin-bottom: 20px;
}

.form-group label {
  display: block;
  font-weight: 500;
  margin-bottom: 8px;
  color: var(--text-primary);
}

.input-field {
  width: 100%;
  padding: 8px 12px;
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
  border-radius: 6px;
  color: var(--text-primary);
}

.hint {
  display: block;
  font-size: 0.8em;
  color: var(--text-tertiary);
  margin-top: 4px;
}

.sources-list {
  background: var(--bg-secondary);
  border: 1px solid var(--border-color);
  border-radius: 6px;
  padding: 10px;
  max-height: 150px;
  overflow-y: auto;
}

.source-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px;
}

.source-item:hover {
  background: var(--bg-tertiary);
}

.results-preview {
  margin-top: 20px;
  padding-top: 20px;
  border-top: 1px solid var(--border-color);
}

.result-stats {
  display: flex;
  gap: 15px;
  margin-bottom: 10px;
}

.stat.success { color: var(--success); }
.stat.error { color: var(--error); }

.post-card {
  background: var(--bg-secondary);
  padding: 10px;
  border-radius: 6px;
  margin-bottom: 8px;
  font-size: 0.9em;
}

.post-meta {
  display: flex;
  justify-content: space-between;
  color: var(--text-tertiary);
  font-size: 0.8em;
  margin-bottom: 4px;
}

.modal-footer {
  padding: 16px 20px;
  border-top: 1px solid var(--border-color);
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  align-items: center;
}

.btn {
  padding: 8px 16px;
  border-radius: 6px;
  cursor: pointer;
  border: none;
  font-weight: 500;
}

.btn.primary { background: var(--primary); color: white; }
.btn.primary:disabled { opacity: 0.5; cursor: not-allowed; }
.btn.secondary { background: var(--bg-tertiary); color: var(--text-secondary); }

.loading-indicator {
  color: var(--text-secondary);
  font-size: 0.9em;
  margin-right: auto;
}
</style>
