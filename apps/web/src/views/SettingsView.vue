<template>
  <Transition name="drawer">
    <div v-if="settingsOpen" class="drawer-backdrop" @click.self="closeSettings">
      <aside class="settings" role="dialog" aria-modal="true" aria-label="Model settings">
    <header class="head">
      <button class="close" aria-label="Close settings" @click="closeSettings">✕</button>
      <h1>Model Settings</h1>
      <p class="sub">
        Pick the model Popinion runs on. One model does everything — pick a hosted provider and add
        its API key, or sign in with ChatGPT and skip the key entirely.
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
        <div class="card-body">
          <div class="card-left">
            <h3>{{ c.title }}</h3>
            <p class="card-hint">{{ c.hint }}</p>

            <div class="prov-grid">
              <button
                v-for="p in providers" :key="p.id"
                class="prov" :class="{ on: sameUrl(c.state.base_url, p.base_url) }"
                @click="pickProvider(c.state, p)"
              >
                <span class="prov-label">{{ p.label }}</span>
              </button>
            </div>
            <p v-if="pickedFor(c.state)" class="prov-hint">{{ pickedFor(c.state).hint }}</p>
          </div>

          <div class="card-right">
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
              <p class="cg-note">Uses your ChatGPT plan via OpenAI's Codex backend - no API key, billed to your subscription. Unofficial; OpenAI may change it.</p>
            </div>

            <template v-else>
              <label>{{ c.state.has_key ? 'API key (leave blank to keep current)' : 'API key' }}</label>
              <input class="input" type="password" v-model="c.state.key"
                     :placeholder="c.state.has_key ? '•••••••• saved' : ''" />

              <div class="row">
                <button class="test" @click="testSlot(c.key)">Test</button>
                <span v-if="status[c.key]" :class="status[c.key].ok ? 'ok' : 'err'">{{ status[c.key].msg }}</span>
              </div>
            </template>
          </div>
        </div>
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
  chatgptLogin, chatgptStatus, chatgptLogout,
} from '../api/settings'
import { settingsOpen, closeSettings } from '../settingsDrawer'

// In Tauri, window.open won't reach the system browser; use the opener plugin.
const openExternal = async (url) => {
  if (typeof window !== 'undefined' && ('__TAURI_INTERNALS__' in window || '__TAURI__' in window)) {
    const { openUrl } = await import('@tauri-apps/plugin-opener')
    await openUrl(url)
  } else {
    window.open(url, '_blank')
  }
}

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

// localhost and 127.0.0.1 are the same server; a saved slot may use either.
const normUrl = (u) => (u || '').trim().replace(/\/+$/, '').replace('://localhost', '://127.0.0.1')
const sameUrl = (a, b) => normUrl(a) === normUrl(b)

const pickedFor = (state) => providers.value.find(p => sameUrl(p.base_url, state.base_url))
const modelsFor = (state) => pickedFor(state)?.models || []

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
  stopCgPoll(); cgFailures = 0
  cgBusy.value = true; cgErr.value = false; cgMsg.value = 'Opening browser…'
  try {
    const { auth_url } = (await chatgptLogin()).data
    await openExternal(auth_url)
    cgMsg.value = 'Complete sign-in in the opened tab…'
    pollChatgpt()
  } catch (e) {
    cgBusy.value = false; cgErr.value = true
    cgMsg.value = e?.response?.data?.error || 'could not start sign-in'
  }
}
// The sign-in poll outlives the drawer unless it is cancelled: SettingsView is
// mounted for the app's lifetime, so nothing else would ever stop it.
let cgTimer = null
let cgFailures = 0
const CG_MAX_FAILURES = 10
function stopCgPoll() {
  clearTimeout(cgTimer)
  cgTimer = null
}
async function pollChatgpt() {
  cgTimer = null
  const s = (await chatgptStatus().catch(() => null))?.data
  if (!s) {
    if (++cgFailures > CG_MAX_FAILURES) {
      cgBusy.value = false; cgErr.value = true; cgMsg.value = 'lost contact with the backend'
      return
    }
    cgTimer = setTimeout(pollChatgpt, 1500)
    return
  }
  cgFailures = 0
  Object.assign(chatgpt, s)
  if (s.logged_in) {
    cgBusy.value = false; cgMsg.value = ''; refreshReady()
  } else if ((s.status || '').startsWith('failed')) {
    cgBusy.value = false; cgErr.value = true; cgMsg.value = s.status
  } else {
    cgTimer = setTimeout(pollChatgpt, 1500)
  }
}
async function cgLogout() {
  await chatgptLogout().catch(() => {})
  Object.assign(chatgpt, { logged_in: false, email: '', plan: '' })
  refreshReady()
}

// Remote/hosted providers only — filter out any local providers the backend
// still advertises so nothing local is selectable.
async function reloadProviders() {
  try {
    providers.value = ((await getProviders()).data.providers || []).filter(p => p.kind !== 'local')
  } catch (e) { /* best-effort */ }
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
  refreshChatgpt() // best-effort; only shown when a slot uses ChatGPT
  refreshReady()
}
watch(settingsOpen, (open) => {
  if (open) return load()
  // Closing abandons any sign-in in flight; reopening starts a fresh one.
  stopCgPoll()
  cgBusy.value = false; cgMsg.value = ''
})

const onKey = (e) => { if (e.key === 'Escape' && settingsOpen.value) closeSettings() }
onMounted(() => {
  if (settingsOpen.value) load()
  window.addEventListener('keydown', onKey)
})
onUnmounted(() => {
  window.removeEventListener('keydown', onKey)
  stopCgPoll()
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
  background: rgba(17, 17, 17, 0.45);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
}
/* Centered 16:9 modal (not a side drawer). Content fits without a scrollbar via
   the two-column card body; a narrow fallback below keeps small screens usable. */
.settings {
  width: min(1040px, 94vw);
  aspect-ratio: 16 / 9;
  max-height: 92vh;
  overflow: hidden;
  padding: 22px 26px;
  background: #fff;
  border-radius: 14px;
  box-shadow: 0 24px 60px rgba(14, 35, 64, 0.28);
  display: flex;
  flex-direction: column;
}
.head { flex-shrink: 0; }
.cards { flex: 1 1 auto; min-height: 0; overflow-y: auto; }
.split, .actions { flex-shrink: 0; }
.card-body { display: flex; gap: 24px; align-items: flex-start; }
.card-left { flex: 0 0 44%; min-width: 0; }
.card-right { flex: 1 1 auto; min-width: 0; }
/* Boost mode (two cards) or narrow: stack the body into one column. */
.cards:not(.solo) .card-body { flex-direction: column; gap: 12px; }
@media (max-width: 760px) {
  .settings { aspect-ratio: auto; width: min(560px, 100%); max-height: 88vh; overflow-y: auto; }
  .card-body { flex-direction: column; gap: 12px; }
}
/* Fade the backdrop; scale the modal up from the center. */
.drawer-enter-active, .drawer-leave-active { transition: opacity 0.2s ease; }
.drawer-enter-active .settings, .drawer-leave-active .settings { transition: transform 0.22s cubic-bezier(0.34, 1.15, 0.64, 1), opacity 0.22s ease; }
.drawer-enter-from, .drawer-leave-to { opacity: 0; }
.drawer-enter-from .settings, .drawer-leave-to .settings { transform: translateY(10px) scale(0.97); opacity: 0; }
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
.head h1 { margin: 0 0 5px; font-size: 22px; }
.sub { color: #555; line-height: 1.45; max-width: 760px; margin: 0; font-size: 13px; }
/* Back + readiness sit on one row: as inline-blocks they collided with no gap. */
.head-row { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; margin-top: 8px; }
.back { color: #2563eb; text-decoration: none; font-size: 14px; }
.back:hover { text-decoration: underline; }
.ready-line { font-size: 13px; padding: 6px 10px; border-radius: 8px; }
.ready-ok { color: #065f46; background: #d1fae5; }
.ready-warn { color: #92400e; background: #fef3c7; }
/* One column: the drawer is too narrow for a side-by-side split. */
.cards { display: grid; grid-template-columns: 1fr; gap: 16px; margin: 12px 0 10px; }
.split { display: flex; gap: 10px; align-items: flex-start; margin: 0 0 6px; cursor: pointer; max-width: 620px; }
.split input { margin-top: 3px; }
.split b { display: block; font-size: 13px; color: #374151; font-weight: 600; }
.split em { display: block; font-size: 11px; color: #9ca3af; font-style: normal; line-height: 1.5; margin-top: 2px; }
.card { border: 1px solid #e5e7eb; border-radius: 12px; padding: 14px 16px; background: #fff; }
.card h3 { margin: 0 0 4px; }
.card-hint { color: #6b7280; font-size: 12px; margin: 0 0 10px; }
.card label { display: block; font-size: 12px; color: #374151; margin: 12px 0 4px; font-weight: 600; }

/* Hosted provider picker */
.prov-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 6px; margin-top: 4px; }
.prov { display: flex; align-items: center; justify-content: center; gap: 6px; padding: 6px 9px; border: 1px solid #e5e7eb; background: #fff; border-radius: 8px; cursor: pointer; font-size: 12px; font-family: inherit; color: #374151; }
.prov:hover { border-color: #d1d5db; background: #fafafa; }
.prov.on { border-color: #2563eb; background: #eff6ff; color: #1d4ed8; font-weight: 600; }
.prov-label { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.prov-hint { font-size: 11px; color: #6b7280; margin: 8px 0 0; line-height: 1.4; }
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

/* ChatGPT subscription panel */
.lms { margin-top: 8px; border: 1px dashed #d1d5db; border-radius: 8px; padding: 8px 10px; background: #fafafa; }
.lms-head { display: flex; justify-content: space-between; align-items: center; font-size: 12px; font-weight: 600; color: #374151; margin-bottom: 8px; }
.badge { font-size: 10px; padding: 1px 6px; border-radius: 999px; }
.badge.loaded { background: #d1fae5; color: #065f46; }
.mini { font-size: 11px; padding: 3px 8px; border: 1px solid #d1d5db; background: #fff; border-radius: 6px; cursor: pointer; }
.mini:disabled { opacity: .5; cursor: default; }
.lms-status { font-size: 11px; color: #059669; margin-top: 6px; }
.lms-status.err { color: #dc2626; }
.cg-in { display: flex; align-items: center; gap: 8px; }
.cg-who { flex: 1; font-size: 12px; color: #374151; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.cg-note { font-size: 11px; color: #9ca3af; margin: 6px 0 0; line-height: 1.35; }
</style>
