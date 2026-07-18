<template>
  <div class="run">
    <header class="run-head">
      <div class="crumbs">
        <router-link :to="`/world/${graphId}`" class="crumb">← World</router-link>
        <span class="sep">·</span>
        <span class="run-name">{{ name }}</span>
      </div>
      <a :href="`/simulation/${simId}`" class="classic">Open classic controls</a>
    </header>

    <!-- Trust is a property of the Run, not of a generated report: the
         credibility and honesty panels live in the header so they're present on
         any Run, with no report required. -->
    <div class="trust-header">
      <CredibilityPanel :simulation-id="simId" />
      <TrustChecksPanel :simulation-id="simId" />
    </div>

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

    <FieldMap v-else :personas="personas" />
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import FieldMap from '../components/FieldMap.vue'
import CredibilityPanel from '../components/CredibilityPanel.vue'
import TrustChecksPanel from '../components/TrustChecksPanel.vue'
import { getSimulation, getSimulationProfiles, preparePreview } from '../api/simulation'

const props = defineProps({
  graphId: { type: String, required: true },
  simId: { type: String, required: true },
})

const loading = ref(true)
const personas = ref([])
const name = ref('Run')

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
.run { display: flex; flex-direction: column; height: 100vh; background: oklch(0.19 0.021 265); color: oklch(0.88 0.012 265); }
.run-head { display: flex; align-items: center; justify-content: space-between; padding: 14px 20px; flex: none; }
.trust-header { flex: none; padding: 0 20px; overflow-y: auto; max-height: 42vh; }
.crumbs { display: flex; align-items: center; gap: 10px; min-width: 0; }
.crumb { color: oklch(0.7 0.05 250); text-decoration: none; font-size: 13px; }
.crumb:hover { text-decoration: underline; }
.sep { color: oklch(0.45 0.02 265); }
.run-name { font-weight: 650; font-size: 15px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.classic { color: oklch(0.62 0.03 265); text-decoration: none; font-size: 12.5px; }
.classic:hover { text-decoration: underline; }
.state { flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; text-align: center; gap: 10px; color: oklch(0.6 0.02 265); padding: 40px; }
.state-title { font-size: 17px; font-weight: 600; color: oklch(0.82 0.012 265); }
.state-body { max-width: 480px; line-height: 1.55; font-size: 14px; margin: 0; }
.state-link { color: oklch(0.7 0.15 74); text-decoration: none; font-size: 13px; }
.state-link:hover { text-decoration: underline; }
.run :deep(.field) { flex: 1; }
</style>
