<template>
  <div class="settings">
    <header class="head">
      <h1>Model Settings</h1>
      <p class="sub">
        Choose where each part of Popinion runs its LLM calls. The <b>bulk</b> slot handles the
        high-volume work (entity extraction, the simulation loop); the <b>boost</b> slot handles
        the low-volume, quality-sensitive work (reports, persona synthesis). Local providers need
        no key.
      </p>
      <router-link class="back" to="/">← Back</router-link>
      <div v-if="ready" class="ready-line" :class="ready.ready ? 'ready-ok' : 'ready-warn'">
        <template v-if="ready.ready">Model ready ✓ — {{ ready.model }}</template>
        <template v-else>Not ready — {{ ready.reason }}</template>
      </div>
    </header>

    <div class="cards">
      <div v-for="c in cards" :key="c.key" class="card">
        <h3>{{ c.title }}</h3>
        <p class="card-hint">{{ c.hint }}</p>

        <label>Provider</label>
        <select class="input" @change="pickProvider(c.state, $event)">
          <option value="">— pick a provider —</option>
          <option v-for="d in providers.detected" :key="d.id" :value="d.id">● {{ d.label }}</option>
          <option v-for="p in providers.presets" :key="p.id" :value="p.id">{{ p.label }}</option>
        </select>

        <label>Base URL</label>
        <input class="input" v-model="c.state.base_url" placeholder="https://…/v1" />

        <label>Model</label>
        <input class="input" v-model="c.state.model" placeholder="model name" />

        <!-- LM Studio: manage local models (list → load/unload → download). -->
        <div v-if="isLmStudio(c.state)" class="lms">
          <div class="lms-head">
            <span>LM Studio models</span>
            <button class="mini" @click="refreshLms">↻</button>
          </div>
          <div v-if="!lmModels.length" class="lms-empty">No models downloaded yet.</div>
          <div v-for="m in lmModels" :key="m.id" class="lms-row" :class="{ sel: c.state.model === m.id }">
            <span class="lms-name" @click="c.state.model = m.id" title="Use for this slot">{{ m.id }}</span>
            <span class="badge" :class="m.state">{{ m.state === 'loaded' ? 'loaded' : 'idle' }}</span>
            <button v-if="m.state !== 'loaded'" class="mini" :disabled="busy[m.id]" @click="loadModel(m.id)">
              {{ busy[m.id] ? '…' : 'Load' }}
            </button>
            <button v-else class="mini" :disabled="busy[m.id]" @click="unloadModel(m.id)">
              {{ busy[m.id] ? '…' : 'Unload' }}
            </button>
          </div>
          <div class="lms-dl">
            <input class="input" v-model="dlModel" placeholder="download hub id, e.g. openai/gpt-oss-20b" />
            <button class="mini" :disabled="dlBusy" @click="downloadModel">{{ dlBusy ? '…' : 'Get' }}</button>
          </div>
          <div v-if="dlStatus" class="lms-status" :class="{ err: dlStatus.err }">{{ dlStatus.msg }}</div>
        </div>

        <label>{{ c.state.has_key ? 'API key (leave blank to keep current)' : 'API key (optional for local)' }}</label>
        <input class="input" type="password" v-model="c.state.key"
               :placeholder="c.state.has_key ? '•••••••• saved' : ''" />

        <div class="row">
          <button class="test" @click="testSlot(c.key)">Test</button>
          <span v-if="status[c.key]" :class="status[c.key].ok ? 'ok' : 'err'">{{ status[c.key].msg }}</span>
        </div>
      </div>
    </div>

    <div class="actions">
      <button class="save" :disabled="saving" @click="save">{{ saving ? 'Saving…' : 'Save settings' }}</button>
      <span v-if="savedMsg" class="saved">{{ savedMsg }}</span>
    </div>
  </div>
</template>

<script setup>
import { reactive, ref, onMounted } from 'vue'
import {
  getLlmSettings, getLlmStatus, updateLlmSettings, testLlm, getProviders,
  lmsModels, lmsLoad, lmsUnload, lmsDownload, lmsDownloadStatus,
} from '../api/settings'

const bulk = reactive({ base_url: '', model: '', has_key: false, key: '' })
const boost = reactive({ base_url: '', model: '', has_key: false, key: '' })
const cards = [
  { key: 'bulk', title: 'Bulk model', hint: 'High-volume: extraction, ontology, the simulation loop', state: bulk },
  { key: 'boost', title: 'Boost model', hint: 'Quality: reports, persona synthesis (falls back to bulk if left blank)', state: boost },
]
const providers = reactive({ detected: [], presets: [] })
const status = reactive({ bulk: null, boost: null })
const saving = ref(false)
const savedMsg = ref('')

// Readiness of the saved bulk slot (what Start Engine uses); null while probing.
const ready = ref(null)
async function refreshReady() {
  try { ready.value = (await getLlmStatus()).data } catch (e) { ready.value = null }
}

function pickProvider(state, e) {
  const p = [...providers.detected, ...providers.presets].find(x => x.id === e.target.value)
  if (p) { state.base_url = p.base_url; state.model = '' }
}

// ---- LM Studio model management ----
const lmModels = ref([])
const busy = reactive({})
const dlModel = ref('')
const dlBusy = ref(false)
const dlStatus = ref(null)

const isLmStudio = (state) => (state.base_url || '').includes(':1234')

async function refreshLms() {
  try {
    const r = await lmsModels()
    lmModels.value = r.data.models || []
  } catch (e) { lmModels.value = [] }
}
async function loadModel(id) {
  busy[id] = true
  try {
    const r = await lmsLoad(id)
    if (!r.data.ok) dlStatus.value = { err: true, msg: r.data.error || 'load failed' }
  } finally { busy[id] = false; await refreshLms(); refreshReady() }
}
async function unloadModel(id) {
  busy[id] = true
  try { await lmsUnload(id) } finally { busy[id] = false; await refreshLms(); refreshReady() }
}
async function downloadModel() {
  const id = dlModel.value.trim()
  if (!id) return
  dlBusy.value = true
  dlStatus.value = { err: false, msg: 'starting…' }
  try {
    const r = await lmsDownload(id)
    pollDownload(r.data.task_id)
  } catch (e) {
    dlBusy.value = false
    dlStatus.value = { err: true, msg: 'failed to start' }
  }
}
async function pollDownload(taskId) {
  try {
    const t = (await lmsDownloadStatus(taskId)).data
    if (t.status === 'completed') {
      dlStatus.value = { err: false, msg: 'downloaded ✓' }
      dlBusy.value = false; dlModel.value = ''; await refreshLms()
    } else if (t.status === 'failed') {
      dlStatus.value = { err: true, msg: t.error || 'download failed' }
      dlBusy.value = false
    } else {
      dlStatus.value = { err: false, msg: t.message || 'downloading…' }
      setTimeout(() => pollDownload(taskId), 3000)
    }
  } catch (e) {
    dlStatus.value = { err: true, msg: 'status check failed' }
    dlBusy.value = false
  }
}

// Every success response is `{ success: true, data: <payload> }` — read `.data`.
onMounted(async () => {
  try {
    const p = (await getProviders()).data
    providers.detected = p.detected || []
    providers.presets = p.presets || []
  } catch (e) { /* detection is best-effort */ }
  const cur = (await getLlmSettings()).data
  Object.assign(bulk, cur.bulk, { key: '' })
  Object.assign(boost, cur.boost, { key: '' })
  refreshLms() // best-effort; only shown when a slot uses LM Studio
  refreshReady()
})

async function testSlot(which) {
  const s = which === 'bulk' ? bulk : boost
  status[which] = { ok: false, msg: 'testing…' }
  try {
    const r = (await testLlm({ base_url: s.base_url, model: s.model, api_key: s.key })).data
    status[which] = r.ok
      ? { ok: true, msg: 'works ✓' }
      : { ok: false, msg: (r.error || 'failed').slice(0, 80) }
  } catch (e) {
    status[which] = { ok: false, msg: 'request failed' }
  }
}

// Only send api_key when the user typed one, or to clear it for a local provider.
function payload(s) {
  const out = { base_url: s.base_url, model: s.model }
  if (s.key) out.api_key = s.key
  else if (!s.has_key) out.api_key = ''
  return out
}

async function save() {
  saving.value = true
  savedMsg.value = ''
  try {
    const r = (await updateLlmSettings({ bulk: payload(bulk), boost: payload(boost) })).data
    Object.assign(bulk, r.bulk, { key: '' })
    Object.assign(boost, r.boost, { key: '' })
    savedMsg.value = 'Saved — new provider is live.'
    refreshReady()
  } catch (e) {
    savedMsg.value = 'Save failed: ' + (e.message || 'error')
  } finally {
    saving.value = false
  }
}
</script>

<style scoped>
.settings { max-width: 900px; margin: 0 auto; padding: 32px 20px; }
.head h1 { margin: 0 0 8px; }
.sub { color: #555; line-height: 1.5; max-width: 720px; }
.back { display: inline-block; margin-top: 8px; color: #2563eb; text-decoration: none; }
.ready-line { margin-top: 10px; font-size: 13px; padding: 6px 10px; border-radius: 8px; display: inline-block; }
.ready-ok { color: #065f46; background: #d1fae5; }
.ready-warn { color: #92400e; background: #fef3c7; }
.cards { display: grid; grid-template-columns: 1fr 1fr; gap: 20px; margin: 24px 0; }
@media (max-width: 720px) { .cards { grid-template-columns: 1fr; } }
.card { border: 1px solid #e5e7eb; border-radius: 12px; padding: 18px; background: #fff; }
.card h3 { margin: 0 0 4px; }
.card-hint { color: #6b7280; font-size: 13px; margin: 0 0 14px; }
.card label { display: block; font-size: 12px; color: #374151; margin: 12px 0 4px; font-weight: 600; }
.input { width: 100%; padding: 8px 10px; border: 1px solid #d1d5db; border-radius: 8px; font-size: 14px; box-sizing: border-box; font-family: inherit; }
.row { display: flex; align-items: center; gap: 10px; margin-top: 14px; }
.test { padding: 6px 14px; border: 1px solid #d1d5db; background: #f9fafb; border-radius: 8px; cursor: pointer; }
.ok { color: #059669; font-size: 13px; }
.err { color: #dc2626; font-size: 13px; }
.actions { display: flex; align-items: center; gap: 14px; }
.save { padding: 10px 22px; background: #2563eb; color: #fff; border: none; border-radius: 8px; cursor: pointer; font-size: 15px; }
.save:disabled { opacity: .6; cursor: default; }
.saved { color: #059669; }

/* LM Studio model manager */
.lms { margin-top: 12px; border: 1px dashed #d1d5db; border-radius: 8px; padding: 10px; background: #fafafa; }
.lms-head { display: flex; justify-content: space-between; align-items: center; font-size: 12px; font-weight: 600; color: #374151; margin-bottom: 8px; }
.lms-empty { font-size: 12px; color: #9ca3af; padding: 4px 0; }
.lms-row { display: flex; align-items: center; gap: 8px; padding: 4px 0; }
.lms-name { flex: 1; font-size: 12px; cursor: pointer; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.lms-name:hover { text-decoration: underline; }
.lms-row.sel .lms-name { font-weight: 700; color: #2563eb; }
.badge { font-size: 10px; padding: 1px 6px; border-radius: 999px; }
.badge.loaded { background: #d1fae5; color: #065f46; }
.badge.not-loaded { background: #f3f4f6; color: #6b7280; }
.mini { font-size: 11px; padding: 3px 8px; border: 1px solid #d1d5db; background: #fff; border-radius: 6px; cursor: pointer; }
.mini:disabled { opacity: .5; cursor: default; }
.lms-dl { display: flex; gap: 6px; margin-top: 8px; }
.lms-dl .input { font-size: 12px; padding: 5px 8px; }
.lms-status { font-size: 11px; color: #059669; margin-top: 6px; }
.lms-status.err { color: #dc2626; }
</style>
