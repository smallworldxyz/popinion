<template>
  <div class="demo">
    <header class="demo-head">
      <div>
        <span class="tag">FIELD · preview</span>
        <span class="note">Synthetic population, deterministic layout. Hover a mark.</span>
      </div>
      <router-link to="/" class="back">← Home</router-link>
    </header>
    <FieldMap :personas="personas" />
  </div>
</template>

<script setup>
import { ref } from 'vue'
import FieldMap from '../components/FieldMap.vue'

// A stand-in population so the FIELD look is reviewable without a live run.
// Real runs feed the same component their prepared Personas (with server
// positions and real evidence).
const TYPES = ['Citizen', 'Ministry', 'Union', 'Vendor', 'Economist', 'Student', 'Media']
const FACTS = {
  Citizen: ['Posts about the fare on a neighbourhood channel.', 'Commutes daily on the affected route.'],
  Ministry: ['Issued the proposal defending the adjustment.', 'Cited an unsustainable subsidy.'],
  Union: ['Organized a petition against the increase.', 'Represents affected workers.'],
  Vendor: ['Carries goods on the route each morning.', 'Says the doubled fare is unaffordable.'],
  Economist: ['Called the flat increase regressive.', 'Cited a 20% ridership drop elsewhere.'],
  Student: ['Signed the student-pass petition.', 'Says the fare eats a food budget.'],
  Media: ['Reported both sides of the debate.', 'Ran an explainer on the subsidy.'],
}
// Ministry defends (pro), Union/Vendor/Student oppose (con), Media neutral, rest mixed.
const lean = { Ministry: 'pro', Union: 'con', Vendor: 'con', Student: 'con', Media: null }

let seed = 7
const rnd = () => { seed = (seed * 1103515245 + 12345) & 0x7fffffff; return seed / 0x7fffffff }
const pick = (arr) => arr[Math.floor(rnd() * arr.length)]

const personas = ref(
  Array.from({ length: 130 }, (_, i) => {
    const t = pick(TYPES)
    const forced = lean[t]
    const faction = forced !== undefined ? forced : rnd() < 0.45 ? 'con' : rnd() < 0.6 ? 'pro' : null
    const synthetic = t === 'Citizen' && rnd() < 0.5
    return {
      user_id: i + 1,
      name: `${t} ${i + 1}`,
      user_name: `${t.toLowerCase()}_${i + 1}`,
      faction,
      source_entity_type: t,
      synthetic,
      evidence: synthetic ? [] : FACTS[t] || [],
    }
  })
)
</script>

<style scoped>
.demo { display: flex; flex-direction: column; height: 100vh; background: oklch(0.19 0.021 265); }
.demo-head { display: flex; align-items: center; justify-content: space-between; padding: 14px 20px; color: oklch(0.82 0.012 265); }
.tag { font: 600 12px/1 ui-monospace, monospace; letter-spacing: .08em; color: oklch(0.7 0.15 74); }
.note { margin-left: 14px; font-size: 13px; color: oklch(0.6 0.02 265); }
.back { color: oklch(0.7 0.05 250); text-decoration: none; font-size: 13px; }
.back:hover { text-decoration: underline; }
.demo :deep(.field) { flex: 1; }
</style>
