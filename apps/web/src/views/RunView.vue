<template>
  <div class="canvas-shell">
    <!-- Far-left icon rail -->
    <nav class="rail">
      <router-link to="/" class="glyph serif" title="Popinion">P</router-link>
      <router-link to="/worlds" class="nav" title="Atlas / Worlds">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7"><circle cx="12" cy="12" r="9"/><path d="M3 12h18M12 3a15 15 0 010 18M12 3a15 15 0 000 18"/></svg>
      </router-link>
      <router-link :to="`/world/${graphId}`" class="nav active" title="Current World">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7"><path d="M4 15c3-1 5-4 8-4s5 3 8 2M4 9c3 1 5-2 8-2s5 3 8 2"/><rect x="3.5" y="4.5" width="17" height="15" rx="2"/></svg>
      </router-link>
      <span class="spacer"></span>
      <router-link to="/settings" class="nav" title="Settings">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7"><circle cx="12" cy="12" r="3"/><path d="M12 2v3M12 19v3M4.9 4.9l2.1 2.1M17 17l2.1 2.1M2 12h3M19 12h3M4.9 19.1l2.1-2.1M17 7l2.1-2.1"/></svg>
      </router-link>
    </nav>

    <!-- Left conversation / control column -->
    <section class="convo">
      <div class="convo-head">
        <div class="convo-title-row">
          <div class="convo-title serif">{{ name }}</div>
          <router-link :to="`/world/${graphId}`" class="icon-btn" title="Back to World">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7"><path d="M15 18l-6-6 6-6"/></svg>
          </router-link>
        </div>
        <router-link :to="`/world/${graphId}`" class="lineage-pill">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><path d="M6 3v6a3 3 0 003 3h6"/><circle cx="18" cy="12" r="2.4"/><circle cx="6" cy="18" r="2.4"/><path d="M6 6v9"/></svg>
          Part of this World
        </router-link>
      </div>

      <div class="summary-card">
        <div class="sc-head">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><path d="M4 19V5a2 2 0 012-2h9l5 5v11a2 2 0 01-2 2H6a2 2 0 01-2-2z"/><path d="M14 3v6h6"/></svg>
          <span class="sc-title">Run summary</span>
        </div>
        <div class="sc-body">
          <b class="mono">{{ shownCount }}</b> grounded personas
          <template v-if="refusedCount">· <b class="mono">{{ refusedCount }}</b> below the evidence bar (refused)</template>
          <template v-else>on the field</template>.
        </div>
      </div>

      <div class="convo-body">
        <div class="eyebrow">Trust &amp; honesty</div>
        <!-- Trust is a property of the Run, not a generated report: the credibility
             and honesty panels stay in the control column so they're present on any
             Run, with no report required. -->
        <div class="trust-header">
          <CredibilityPanel :simulation-id="simId" />
          <TrustChecksPanel :simulation-id="simId" />
        </div>
      </div>

      <div class="composer">
        <div class="composer-box">
          <div class="composer-input">Extend or fork this World into a new scenario…</div>
          <div class="composer-row">
            <router-link :to="`/simulation/${simId}`" class="model-sel" title="Classic controls">
              Classic controls
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M6 9l6 6 6-6"/></svg>
            </router-link>
            <span class="grow"></span>
            <router-link :to="`/simulation/${simId}`" class="send">
              Open
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9"><path d="M5 12h14M13 6l6 6-6 6"/></svg>
            </router-link>
          </div>
        </div>
      </div>
    </section>

    <!-- Main canvas -->
    <main class="canvas-col">
      <div class="toolbar">
        <div class="tb-left">
          <router-link :to="`/world/${graphId}`" class="tb-icon" title="Back to World">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7"><path d="M15 18l-6-6 6-6"/></svg>
          </router-link>
          <span class="tb-run"><span class="serif">Field</span> · {{ name }}</span>
        </div>
        <div class="tb-spacer"></div>
        <div class="tb-right">
          <span class="tb-item" v-if="personas.length">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7"><circle cx="11" cy="11" r="7"/><path d="M21 21l-4-4"/></svg>
            {{ shownCount + refusedCount }} marks
          </span>
          <a :href="`/simulation/${simId}`" class="tb-item">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7"><path d="M12 20h9"/><path d="M16.5 3.5a2.1 2.1 0 013 3L7 19l-4 1 1-4z"/></svg>
            Classic controls
          </a>
          <button class="tb-share">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><circle cx="18" cy="5" r="2.4"/><circle cx="6" cy="12" r="2.4"/><circle cx="18" cy="19" r="2.4"/><path d="M8.1 10.9l7.8-4.8M8.1 13.1l7.8 4.8"/></svg>
            Share
          </button>
          <div class="avatar">RT</div>
        </div>
      </div>

      <div class="stage">
        <div class="field-wrap">
          <div v-if="loading" class="state">Loading the population…</div>

          <div v-else-if="!personas.length" class="state empty">
            <div class="state-title">This run has no prepared Personas yet.</div>
            <p class="state-body">
              Personas are compiled from the World's graph when a run is prepared. If preparation was
              refused, the evidence bar was not met by any entity. Prepare or lower the bar in the classic
              controls, then this map will populate.
            </p>
            <a :href="`/simulation/${simId}`" class="state-link">Open classic controls →</a>
          </div>

          <template v-else>
            <div class="field-legend">
              <div class="fl-eyebrow">{{ name }}</div>
              <div class="fl-title serif">The opinion field</div>
              <div class="axis-legend">
                <div class="row"><span class="sw pro"></span>Pro</div>
                <div class="row"><span class="sw con"></span>Against</div>
                <div class="row"><span class="sw ref"></span>Refused (below bar)</div>
              </div>
            </div>
            <FieldMap :personas="personas" />
          </template>
        </div>
        <a v-if="personas.length" :href="`/simulation/${simId}`" class="edit-pill" title="Extend or fork this World">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><path d="M12 20h9"/><path d="M16.5 3.5a2.1 2.1 0 013 3L7 19l-4 1 1-4z"/></svg>
          Edit
        </a>
      </div>
    </main>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import FieldMap from '../components/FieldMap.vue'
import CredibilityPanel from '../components/CredibilityPanel.vue'
import TrustChecksPanel from '../components/TrustChecksPanel.vue'
import { getSimulation, getSimulationProfiles, preparePreview } from '../api/simulation'
import { isRefused } from '../components/field/marks'

const props = defineProps({
  graphId: { type: String, required: true },
  simId: { type: String, required: true },
})

const loading = ref(true)
const personas = ref([])
const name = ref('Run')

const shownCount = computed(() => personas.value.filter((m) => !isRefused(m)).length)
const refusedCount = computed(() => personas.value.filter(isRefused).length)

// Every entity the graph observed, with its evidence_score and whether it
// cleared the bar. Both live on /prepare/preview — zero new backend.
function flattenPreview(preview) {
  const out = []
  for (const g of preview?.data?.groups || []) {
    for (const e of g.entities || []) out.push({ ...e, source_entity_type: g.entity_type })
  }
  return out
}

// Join the prepared Personas (real positions, faction, evidence) to the preview
// entities (evidence_score, eligibility), then add the below-bar entities as
// hollow REFUSED marks so the map is drawn at true population scale.
function buildMarks(profiles, entities) {
  const byUuid = new Map(entities.map((e) => [e.uuid, e]))
  const matched = new Set()
  const marks = profiles.map((p) => {
    const e = p.source_entity_uuid ? byUuid.get(p.source_entity_uuid) : null
    if (e) matched.add(e.uuid)
    return { ...p, eligible: true, evidence_score: e?.evidence_score ?? p.evidence?.length ?? 0 }
  })
  let refusedId = 1_000_000
  for (const e of entities) {
    if (e.eligible || matched.has(e.uuid)) continue
    marks.push({
      user_id: refusedId++,
      name: e.name,
      source_entity_type: e.source_entity_type,
      faction: null,
      synthetic: false,
      evidence: [],
      eligible: false,
      evidence_score: e.evidence_score,
    })
  }
  return marks
}

onMounted(async () => {
  try {
    const [meta, profs, preview] = await Promise.all([
      getSimulation(props.simId).catch(() => null),
      getSimulationProfiles(props.simId).catch(() => null),
      preparePreview({ simulation_id: props.simId }).catch(() => null),
    ])
    name.value = meta?.data?.name || 'Run'
    const profiles = profs?.data?.profiles || []
    const entities = flattenPreview(preview)
    personas.value = profiles.length || entities.length ? buildMarks(profiles, entities) : []
  } finally {
    loading.value = false
  }
})
</script>

<style scoped>
.canvas-shell { display: flex; height: 100vh; overflow: hidden; background: var(--subtle); }

/* ── Icon rail ──────────────────────────────────────────────── */
.rail { width: 52px; flex: none; background: var(--subtle); border-right: 1px solid var(--border);
  display: flex; flex-direction: column; align-items: center; padding: 14px 0; gap: 6px; }
.rail .glyph { width: 30px; height: 30px; border-radius: 8px; background: var(--emerald); color: #fff;
  display: flex; align-items: center; justify-content: center; font-family: var(--font-serif); font-weight: 600;
  font-size: 18px; box-shadow: var(--elev-1); margin-bottom: 10px; text-decoration: none; }
.rail .nav { width: 34px; height: 34px; border-radius: 9px; display: flex; align-items: center; justify-content: center; color: var(--text-muted); }
.rail .nav svg { width: 19px; height: 19px; }
.rail .nav:hover { background: #e9eef4; color: var(--text-strong); }
.rail .nav.active { background: #fff; color: var(--emerald); box-shadow: var(--elev-1); }
.rail .spacer { flex: 1; }

/* ── Conversation column ────────────────────────────────────── */
.convo { width: 330px; flex: none; background: var(--white); border-right: 1px solid var(--border);
  display: flex; flex-direction: column; height: 100%; }
.convo-head { padding: 14px 16px 12px; border-bottom: 1px solid var(--border); }
.convo-title-row { display: flex; align-items: center; gap: 8px; }
.convo-title { font-family: var(--font-serif); font-weight: 500; font-size: 16px; line-height: 1.25;
  color: var(--text-strong); flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.icon-btn { width: 28px; height: 28px; border-radius: var(--r-sm); flex: none; display: flex; align-items: center;
  justify-content: center; color: var(--text-muted); text-decoration: none; }
.icon-btn:hover { background: var(--subtle); color: var(--text-strong); }
.icon-btn svg { width: 16px; height: 16px; }
.lineage-pill { display: inline-flex; align-items: center; gap: 6px; margin-top: 11px; padding: 4px 10px;
  border-radius: var(--r-pill); background: rgba(12, 133, 119, .08); color: var(--emerald); font-size: 11.5px;
  font-weight: 500; text-decoration: none; }
.lineage-pill svg { width: 12px; height: 12px; }
.summary-card { margin: 14px 16px 4px; padding: 12px 13px; border: 1px solid var(--border); border-radius: var(--r-md); background: var(--subtle); }
.summary-card .sc-head { display: flex; align-items: center; gap: 7px; margin-bottom: 8px; }
.summary-card .sc-head svg { width: 14px; height: 14px; color: var(--emerald); }
.summary-card .sc-title { font-size: 12px; font-weight: 600; color: var(--text-strong); }
.summary-card .sc-body { font-size: 12.5px; line-height: 1.55; color: var(--text-muted); }
.summary-card .sc-body b { color: var(--text-strong); font-weight: 600; }
.summary-card .mono { font-variant-numeric: tabular-nums; }
.convo-body { flex: 1; overflow-y: auto; padding: 16px 16px 8px; }
.convo-body .eyebrow { margin-bottom: 12px; }
.trust-header :deep(.credibility-panel), .trust-header :deep(.trust-panel) { margin-bottom: 16px; }

/* composer */
.composer { border-top: 1px solid var(--border); padding: 12px 14px 14px; background: var(--white); }
.composer-box { border: 1px solid var(--border); border-radius: var(--r-md); background: var(--subtle); padding: 10px 10px 8px; box-shadow: var(--elev-1); }
.composer-input { font-size: 13.5px; color: var(--text-muted); line-height: 1.4; min-height: 24px; padding: 2px 2px 8px; }
.composer-row { display: flex; align-items: center; gap: 8px; }
.model-sel { display: flex; align-items: center; gap: 5px; padding: 5px 9px; border-radius: var(--r-sm);
  border: 1px solid var(--border); background: #fff; font-size: 12px; color: var(--text-strong); font-weight: 500; text-decoration: none; }
.model-sel svg { width: 13px; height: 13px; color: var(--text-muted); }
.composer-row .grow { flex: 1; }
.send { display: flex; align-items: center; gap: 6px; padding: 7px 15px; border-radius: var(--r-sm);
  background: var(--coral); color: #fff; font-size: 13px; font-weight: 600; box-shadow: var(--elev-1); text-decoration: none; }
.send svg { width: 14px; height: 14px; }
.send:hover { background: var(--coral-on-light); }

/* ── Main canvas ────────────────────────────────────────────── */
.canvas-col { flex: 1; display: flex; flex-direction: column; min-width: 0; background: var(--white); }
.toolbar { height: 52px; flex: none; border-bottom: 1px solid var(--border); display: flex; align-items: center; padding: 0 16px; gap: 10px; background: #fff; }
.tb-left { display: flex; align-items: center; gap: 8px; }
.tb-icon { width: 30px; height: 30px; border-radius: var(--r-sm); display: flex; align-items: center; justify-content: center; color: var(--text-muted); text-decoration: none; }
.tb-icon:hover { background: var(--subtle); color: var(--text-strong); }
.tb-icon svg { width: 16px; height: 16px; }
.tb-run { display: flex; align-items: center; gap: 6px; padding: 6px 4px; font-size: 13px; font-weight: 500; color: var(--text-strong); }
.tb-run .serif { font-family: var(--font-serif); font-weight: 500; }
.tb-spacer { flex: 1; }
.tb-right { display: flex; align-items: center; gap: 2px; }
.tb-item { display: flex; align-items: center; gap: 6px; padding: 6px 10px; border-radius: var(--r-sm); font-size: 13px;
  font-weight: 500; color: var(--text-muted); text-decoration: none; background: none; border: none; cursor: pointer; }
.tb-item:hover { background: var(--subtle); color: var(--text-strong); }
.tb-item svg { width: 15px; height: 15px; }
.tb-share { display: flex; align-items: center; gap: 6px; padding: 7px 15px; margin-left: 6px; border-radius: var(--r-sm);
  background: var(--navy); color: #fff; font-size: 13px; font-weight: 600; box-shadow: var(--elev-1); border: none; cursor: pointer; }
.tb-share:hover { background: var(--navy-raised); }
.tb-share svg { width: 15px; height: 15px; }
.avatar { width: 30px; height: 30px; border-radius: 50%; margin-left: 8px; flex: none;
  background: linear-gradient(135deg, var(--emerald), var(--emerald-bright)); color: #fff; font-size: 12px; font-weight: 600;
  display: flex; align-items: center; justify-content: center; }

.stage { flex: 1; padding: 16px; position: relative; overflow: hidden; background: var(--subtle); }
.field-wrap { position: absolute; inset: 16px; border-radius: var(--r-lg); background: var(--navy); overflow: hidden; box-shadow: var(--elev-2); display: flex; }
.field-wrap :deep(.field) { flex: 1; min-height: 0; }
.field-legend { position: absolute; top: 16px; left: 18px; color: var(--on-dark); z-index: 3; pointer-events: none; }
.field-legend .fl-eyebrow { font-family: var(--font-sans); font-weight: 700; font-size: 10px; letter-spacing: .14em; text-transform: uppercase; color: var(--on-dark-muted); }
.field-legend .fl-title { font-family: var(--font-serif); font-weight: 500; font-size: 19px; margin-top: 3px; color: #fff; }
.axis-legend { margin-top: 12px; display: flex; flex-direction: column; gap: 5px; align-items: flex-start; }
.axis-legend .row { display: flex; align-items: center; gap: 7px; font-size: 11px; color: var(--on-dark-muted); }
.axis-legend .sw { width: 11px; height: 11px; border-radius: 3px; }
.axis-legend .sw.pro { background: var(--emerald-bright); }
.axis-legend .sw.con { background: var(--coral-bright); }
.axis-legend .sw.ref { border: 1.5px solid var(--on-dark-muted); background: transparent; border-radius: 50%; }
.edit-pill { position: absolute; bottom: 34px; right: 34px; z-index: 4; display: flex; align-items: center; gap: 7px;
  padding: 9px 16px; border-radius: var(--r-pill); background: rgba(14, 35, 64, .72); backdrop-filter: blur(6px);
  border: 1.5px solid var(--coral-bright); color: var(--coral-bright); font-size: 13px; font-weight: 600; box-shadow: var(--elev-2); text-decoration: none; }
.edit-pill svg { width: 14px; height: 14px; }
.edit-pill:hover { background: rgba(232, 106, 76, .16); }

/* states, centered in the navy field */
.state { flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; text-align: center; gap: 10px; color: var(--on-dark-muted); padding: 40px; }
.state-title { font-family: var(--font-serif); font-size: 19px; font-weight: 500; color: #fff; }
.state-body { max-width: 480px; line-height: 1.55; font-size: 14px; margin: 0; }
.state-link { color: var(--emerald-bright); text-decoration: none; font-size: 13px; }
.state-link:hover { text-decoration: underline; }
</style>
