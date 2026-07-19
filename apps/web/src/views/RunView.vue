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
import { getSimulation, getSimulationProfiles } from '../api/simulation'

const props = defineProps({
  graphId: { type: String, required: true },
  simId: { type: String, required: true },
})

const loading = ref(true)
const personas = ref([])
const name = ref('Run')

onMounted(async () => {
  try {
    const [meta, profs] = await Promise.all([
      getSimulation(props.simId).catch(() => null),
      getSimulationProfiles(props.simId).catch(() => null),
    ])
    const n = meta?.data?.name
    name.value = (n && n !== 'Untitled Simulation') ? n : 'Baseline run'
    personas.value = profs?.data?.profiles || []
  } finally {
    loading.value = false
  }
})
</script>

<style scoped>
.run { display: flex; flex-direction: column; height: 100vh; background: oklch(0.19 0.021 265); color: oklch(0.88 0.012 265); }
.run-head { display: flex; align-items: center; justify-content: space-between; padding: 14px 20px; flex: none; }
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
