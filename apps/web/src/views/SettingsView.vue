<template>
  <Transition name="drawer">
    <div v-if="settingsOpen" class="drawer-backdrop" @click.self="closeSettings">
      <aside class="settings" role="dialog" aria-modal="true" aria-label="Model settings">
    <header class="head">
      <button class="close" aria-label="Close settings" @click="closeSettings">✕</button>
      <h1>Model Settings</h1>
      <p class="sub">
        Pick the model Popinion runs on. One model does everything — local providers need no key,
        and signing in with ChatGPT needs no key either.
      </p>
      <div class="head-row">
        <div v-if="ready" class="ready-line" :class="ready.ready ? 'ready-ok' : 'ready-warn'">
          <template v-if="ready.ready">Model ready ✓ — {{ ready.model }}</template>
          <template v-else>Not ready — {{ ready.reason }}</template>
        </div>
      </div>
    </header>

    <div class="cards" :class="{ solo: !useBoost }">
      <div v-for="c in cards" :key="c.key" class="card">
        <h3>{{ c.title }}</h3>
        <p class="card-hint">{{ c.hint }}</p>

        <div class="tabs" role="tablist">
          <button
            v-for="t in ['remote', 'local']" :key="t"
            class="tab" :class="{ on: tabOf(c.key) === t }"
            role="tab" :aria-selected="tabOf(c.key) === t"
            @click="tabs[c.key] = t"
          >
            {{ t === 'remote' ? 'Remote' : 'Local' }}
          </button>
        </div>
        <p class="tab-hint">
          {{ tabOf(c.key) === 'remote'
            ? 'Hosted providers — a subscription or an API key. Fast enough to finish a run.'
            : 'Runs on this machine — free and private, but needs a GPU to be practical.' }}
        </p>

        <div class="prov-grid">
          <button
            v-for="p in providersFor(tabOf(c.key))" :key="p.id"
            class="prov" :class="{ on: sameUrl(c.state.base_url, p.base_url) }"
            @click="pickProvider(c.state, p)"
          >
            <span class="prov-label">{{ p.label }}</span>
            <span v-if="p.kind === 'local'" class="dot" :class="p.running ? 'up' : 'down'"
                  :title="p.running ? 'running' : 'not running'"></span>
          </button>
        </div>
        <p v-if="inTab(c)" class="prov-hint">{{ pickedFor(c.state).hint }}</p>
        <p v-else-if="pickedFor(c.state)" class="prov-hint other">
          This slot currently uses <b>{{ pickedFor(c.state).label }}</b>
          ({{ pickedFor(c.state).kind }}) — pick one above to switch it.
        </p>

        <label>Base URL</label>
        <input class="input" v-model="c.state.base_url" placeholder="https://…/v1" />

        <label>Model</label>
        <input class="input" v-model="c.state.model" placeholder="model name" />
        <div v-if="modelsFor(c.state).length" class="model-chips">
          <button
            v-for="m in modelsFor(c.state)" :key="m"
            class="chip" :class="{ on: c.state.model === m }"
            @click="c.state.model = m"
          >{{ m }}</button>
        </div>

        <!-- LM Studio: manage local models (list → load/unload → download). -->
        <div v-if="isLmStudio(c.state) && inTab(c)" class="lms">
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

        <!-- Ollama: pull a model and enable it, or pick an already-installed one. -->
        <div v-if="isOllama(c.state) && inTab(c)" class="lms">
          <div class="lms-head">
            <span>Pull a model</span>
            <button class="mini" @click="reloadProviders" title="Rescan installed models">↻</button>
          </div>
          <div v-if="!ollamaModels.length" class="lms-empty">No models installed yet — pull one below.</div>
          <div class="lms-dl">
            <input class="input" v-model="pullModel" placeholder="model to pull, e.g. gemma4:e4b" />
            <button class="mini" :disabled="pullBusy" @click="pullOllama(c.state)">{{ pullBusy ? '…' : 'Pull' }}</button>
          </div>
          <div v-if="pullStatus" class="lms-status" :class="{ err: pullStatus.err }">{{ pullStatus.msg }}</div>
        </div>

        <!-- ChatGPT subscription: OAuth sign-in instead of an API key. -->
        <div v-if="isChatgpt(c.state)" class="lms">
          <div class="lms-head"><span>ChatGPT subscription</span></div>
          <div v-if="chatgpt.logged_in" class="cg-in">
            <span class="badge loaded">signed in</span>
            <span class="cg-who">{{ chatgpt.email || '' }}<template v-if="chatgpt.plan"> · {{ chatgpt.plan }}</template></span>
            <button class="mini" @click="cgLogout">Sign out</button>
          </div>
          <div v-else>
            <button class="mini" :disabled="cgBusy" @click="cgLogin">
              {{ cgBusy ? 'Waiting for browser…' : 'Sign in with ChatGPT' }}
            </button>
            <div v-if="cgMsg" class="lms-status" :class="{ err: cgErr }">{{ cgMsg }}</div>
          </div>
          <p class="cg-note">Uses your ChatGPT plan via OpenAI's Codex backend — no API key, billed to your subscription. Unofficial; OpenAI may change it.</p>
        </div>

        <template v-else>
          <label>{{ c.state.has_key ? 'API key (leave blank to keep current)' : 'API key (optional for local)' }}</label>
          <input class="input" type="password" v-model="c.state.key"
                 :placeholder="c.state.has_key ? '•••••••• saved' : ''" />

          <div class="row">
            <button class="test" @click="testSlot(c.key)">Test</button>
            <span v-if="status[c.key]" :class="status[c.key].ok ? 'ok' : 'err'">{{ status[c.key].msg }}</span>
          </div>
        </template>
      </div>
    </div>

    <label class="split">
      <input type="checkbox" v-model="useBoost" />
      <span>
        <b>Use a second model for reports &amp; personas</b>
        <em>
          Only worth it to pair a cheap bulk model with a pricier, better one for the handful of
          quality-sensitive calls. Off = one model does everything.
        </em>
      </span>
    </label>

    <div class="actions">
      <button class="save" :disabled="saving" @click="save">{{ saving ? 'Saving…' : 'Save settings' }}</button>
      <span v-if="savedMsg" class="saved">{{ savedMsg }}</span>
    </div>
      </aside>
    </div>
  </Transition>
</template>

<script setup>
import { reactive, ref, computed, onMounted, onUnmounted, watch } from 'vue'
import {
  getLlmSettings, getLlmStatus, updateLlmSettings, testLlm, getProviders,
  lmsModels, lmsLoad, lmsUnload, lmsDownload, lmsDownloadStatus,
  ollamaPull, ollamaPullStatus,
  chatgptLogin, chatgptStatus, chatgptLogout,
} from '../api/settings'
import { settingsOpen, closeSettings } from '../settingsDrawer'

const bulk = reactive({ base_url: '', model: '', has_key: false, key: '' })
const boost = reactive({ base_url: '', model: '', has_key: false, key: '' })

// One model by default. An empty boost slot means "same as bulk" server-side, so
// the split is opt-in rather than two identical cards to fill in.
const useBoost = ref(false)
const cards = computed(() => {
  const list = [{
    key: 'bulk',
    title: useBoost.value ? 'Main model' : 'Model',
    hint: useBoost.value
      ? 'Extraction, the simulation loop — everything except reports & personas'
      : 'Runs everything: extraction, the simulation loop, reports, personas',
    state: bulk,
  }]
  if (useBoost.value) {
    list.push({
      key: 'boost',
      title: 'Report & persona model',
      hint: 'Only the low-volume, quality-sensitive calls',
      state: boost,
    })
  }
  return list
})
const providers = ref([])
const status = reactive({ bulk: null, boost: null })
const saving = ref(false)
const savedMsg = ref('')

// Which tab a card shows. Unset → inferred from the slot's saved base_url, so a
// card configured for Ollama opens on Local.
const tabs = reactive({ bulk: null, boost: null })
const isLocalUrl = (u) => /127\.0\.0\.1|localhost/.test(u || '')
const tabOf = (key) => tabs[key] ?? (isLocalUrl((key === 'bulk' ? bulk : boost).base_url) ? 'local' : 'remote')

// localhost and 127.0.0.1 are the same server; a saved slot may use either.
const normUrl = (u) => (u || '').trim().replace(/\/+$/, '').replace('://localhost', '://127.0.0.1')
const sameUrl = (a, b) => normUrl(a) === normUrl(b)

const providersFor = (kind) => providers.value.filter(p => p.kind === kind)
const pickedFor = (state) => providers.value.find(p => sameUrl(p.base_url, state.base_url))
const modelsFor = (state) => pickedFor(state)?.models || []
// Is the slot's provider the one this tab is showing? Keeps local-only controls
// (Ollama pull, LM Studio loader) out of the Remote tab.
const inTab = (c) => pickedFor(c.state)?.kind === tabOf(c.key)

// Readiness of the saved bulk slot (what Start Engine uses); null while probing.
const ready = ref(null)
async function refreshReady() {
  try { ready.value = (await getLlmStatus()).data } catch (e) { ready.value = null }
}

// Preselect the provider's first model so a working setup needs no typing.
function pickProvider(state, p) {
  state.base_url = p.base_url
  state.model = p.models?.[0] || ''
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

// ---- ChatGPT subscription sign-in (OAuth) ----
const chatgpt = reactive({ logged_in: false, email: '', plan: '' })
const cgBusy = ref(false)
const cgMsg = ref('')
const cgErr = ref(false)

const isChatgpt = (state) => (state.base_url || '').includes('backend-api/codex')

async function refreshChatgpt() {
  try { Object.assign(chatgpt, (await chatgptStatus()).data) } catch (e) { /* best-effort */ }
}
async function cgLogin() {
  cgBusy.value = true; cgErr.value = false; cgMsg.value = 'Opening browser…'
  try {
    const { auth_url } = (await chatgptLogin()).data
    window.open(auth_url, '_blank')
    cgMsg.value = 'Complete sign-in in the opened tab…'
    pollChatgpt()
  } catch (e) {
    cgBusy.value = false; cgErr.value = true
    cgMsg.value = e?.response?.data?.error || 'could not start sign-in'
  }
}
async function pollChatgpt() {
  const s = (await chatgptStatus().catch(() => null))?.data
  if (!s) { setTimeout(pollChatgpt, 1500); return }
  Object.assign(chatgpt, s)
  if (s.logged_in) {
    cgBusy.value = false; cgMsg.value = ''; refreshReady()
  } else if ((s.status || '').startsWith('failed')) {
    cgBusy.value = false; cgErr.value = true; cgMsg.value = s.status
  } else {
    setTimeout(pollChatgpt, 1500)
  }
}
async function cgLogout() {
  await chatgptLogout().catch(() => {})
  Object.assign(chatgpt, { logged_in: false, email: '', plan: '' })
  refreshReady()
}

// ---- Ollama model management (native pull; enables the model on success) ----
const ollamaModels = ref([])
const pullModel = ref('gemma4:e4b')
const pullBusy = ref(false)
const pullStatus = ref(null)

const isOllama = (state) => (state.base_url || '').includes(':11434')

function refreshOllama() {
  const o = providers.value.find(d => d.id === 'ollama')
  ollamaModels.value = o ? (o.models || []) : []
}
async function reloadProviders() {
  try {
    providers.value = (await getProviders()).data.providers || []
  } catch (e) { /* best-effort */ }
  refreshOllama()
}
async function pullOllama(state) {
  const m = pullModel.value.trim()
  if (!m) return
  pullBusy.value = true
  pullStatus.value = { err: false, msg: 'starting…' }
  try {
    const r = await ollamaPull(m, state.base_url)
    pollPull(r.data.task_id, state)
  } catch (e) {
    pullBusy.value = false
    pullStatus.value = { err: true, msg: 'failed to start — is Ollama running?' }
  }
}
async function pollPull(taskId, state) {
  try {
    const t = (await ollamaPullStatus(taskId)).data
    if (t.status === 'completed') {
      state.model = pullModel.value.trim()   // enable the pulled model in this slot
      await save()                           // persist + activate immediately
      pullStatus.value = { err: false, msg: 'downloaded & enabled ✓' }
      pullBusy.value = false
      await reloadProviders()
    } else if (t.status === 'failed') {
      pullStatus.value = { err: true, msg: t.error || 'pull failed' }
      pullBusy.value = false
    } else {
      pullStatus.value = { err: false, msg: t.message || 'downloading…' }
      setTimeout(() => pollPull(taskId, state), 1500)
    }
  } catch (e) {
    pullStatus.value = { err: true, msg: 'status check failed' }
    pullBusy.value = false
  }
}

// Every success response is `{ success: true, data: <payload> }` — read `.data`.
// Re-run each time the drawer opens so it reflects the live provider state.
async function load() {
  await reloadProviders() // detection is best-effort
  const cur = (await getLlmSettings()).data
  Object.assign(bulk, cur.bulk, { key: '' })
  Object.assign(boost, cur.boost, { key: '' })
  // Reveal the split only if boost is genuinely a *different* model. Older saves
  // duplicated bulk into boost, which isn't a split — it just looked like one.
  useBoost.value = !!(boost.base_url || '').trim() &&
    !(sameUrl(boost.base_url, bulk.base_url) && boost.model === bulk.model)
  refreshLms() // best-effort; only shown when a slot uses LM Studio
  refreshChatgpt() // best-effort; only shown when a slot uses ChatGPT
  refreshReady()
}
watch(settingsOpen, (open) => { if (open) load() })

const onKey = (e) => { if (e.key === 'Escape' && settingsOpen.value) closeSettings() }
onMounted(() => {
  if (settingsOpen.value) load()
  window.addEventListener('keydown', onKey)
})
onUnmounted(() => window.removeEventListener('keydown', onKey))

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
    // Boost off → clear the slot, which is how the server says "reuse bulk".
    const boostPayload = useBoost.value ? payload(boost) : { base_url: '', model: '', api_key: '' }
    const r = (await updateLlmSettings({ bulk: payload(bulk), boost: boostPayload })).data
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
/* Right-side drawer: a dimmed backdrop with a panel sliding in from the edge. */
.drawer-backdrop {
  position: fixed;
  inset: 0;
  z-index: 2000;
  background: rgba(17, 17, 17, 0.35);
  display: flex;
  justify-content: flex-start;
}
.settings {
  width: min(520px, 100vw);
  height: 100%;
  overflow-y: auto;
  padding: 28px 24px 40px;
  background: #fff;
  box-shadow: 8px 0 30px rgba(0, 0, 0, 0.18);
}
/* Slide + fade the whole overlay in from the left. */
.drawer-enter-active, .drawer-leave-active { transition: opacity 0.2s ease; }
.drawer-enter-active .settings, .drawer-leave-active .settings { transition: transform 0.25s cubic-bezier(0.4, 0, 0.2, 1); }
.drawer-enter-from, .drawer-leave-to { opacity: 0; }
.drawer-enter-from .settings, .drawer-leave-to .settings { transform: translateX(-100%); }
.close {
  position: absolute;
  top: 18px;
  right: 20px;
  width: 30px;
  height: 30px;
  border: 1px solid #e5e7eb;
  background: #fff;
  border-radius: 8px;
  cursor: pointer;
  font-size: 15px;
  color: #6b7280;
  line-height: 1;
}
.close:hover { background: #f3f4f6; color: #111; }
.head { position: relative; }
.head h1 { margin: 0 0 8px; }
.sub { color: #555; line-height: 1.5; max-width: 720px; margin: 0; }
/* Back + readiness sit on one row: as inline-blocks they collided with no gap. */
.head-row { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; margin-top: 14px; }
.back { color: #2563eb; text-decoration: none; font-size: 14px; }
.back:hover { text-decoration: underline; }
.ready-line { font-size: 13px; padding: 6px 10px; border-radius: 8px; }
.ready-ok { color: #065f46; background: #d1fae5; }
.ready-warn { color: #92400e; background: #fef3c7; }
/* One column: the drawer is too narrow for a side-by-side split. */
.cards { display: grid; grid-template-columns: 1fr; gap: 20px; margin: 24px 0; }
.split { display: flex; gap: 10px; align-items: flex-start; margin: 0 0 20px; cursor: pointer; max-width: 620px; }
.split input { margin-top: 3px; }
.split b { display: block; font-size: 13px; color: #374151; font-weight: 600; }
.split em { display: block; font-size: 11px; color: #9ca3af; font-style: normal; line-height: 1.5; margin-top: 2px; }
.card { border: 1px solid #e5e7eb; border-radius: 12px; padding: 18px; background: #fff; }
.card h3 { margin: 0 0 4px; }
.card-hint { color: #6b7280; font-size: 13px; margin: 0 0 14px; }
.card label { display: block; font-size: 12px; color: #374151; margin: 12px 0 4px; font-weight: 600; }

/* Remote vs local tabs */
.tabs { display: flex; gap: 4px; background: #f3f4f6; padding: 3px; border-radius: 8px; }
.tab { flex: 1; padding: 6px 10px; border: none; background: transparent; border-radius: 6px; cursor: pointer; font-size: 13px; font-weight: 600; color: #6b7280; font-family: inherit; }
.tab.on { background: #fff; color: #111827; box-shadow: 0 1px 2px rgba(0,0,0,.08); }
.tab-hint { font-size: 11px; color: #9ca3af; margin: 8px 0 10px; line-height: 1.4; }
.prov-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 6px; }
.prov { display: flex; align-items: center; justify-content: center; gap: 6px; padding: 8px 10px; border: 1px solid #e5e7eb; background: #fff; border-radius: 8px; cursor: pointer; font-size: 12px; font-family: inherit; color: #374151; }
.prov:hover { border-color: #d1d5db; background: #fafafa; }
.prov.on { border-color: #2563eb; background: #eff6ff; color: #1d4ed8; font-weight: 600; }
.prov-label { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.dot { width: 6px; height: 6px; border-radius: 999px; flex: none; }
.dot.up { background: #10b981; }
.dot.down { background: #d1d5db; }
.prov-hint { font-size: 11px; color: #6b7280; margin: 8px 0 0; line-height: 1.4; }
.prov-hint.other { color: #92400e; background: #fef3c7; padding: 6px 8px; border-radius: 6px; }
.model-chips { display: flex; flex-wrap: wrap; gap: 4px; margin-top: 6px; }
.chip { padding: 3px 8px; border: 1px solid #e5e7eb; background: #fff; border-radius: 999px; cursor: pointer; font-size: 11px; font-family: inherit; color: #6b7280; }
.chip.on { border-color: #2563eb; background: #eff6ff; color: #1d4ed8; font-weight: 600; }
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
.cg-in { display: flex; align-items: center; gap: 8px; }
.cg-who { flex: 1; font-size: 12px; color: #374151; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.cg-note { font-size: 11px; color: #9ca3af; margin: 8px 0 0; line-height: 1.4; }
</style>
