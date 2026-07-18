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

const ROUNDS = 16
const EVENT_AT = 6 // a "subsidy offset" is announced, softening some opposition

// A stance trajectory in [-1,1] over rounds: start at the faction lean, then let
// the mid-run event pull a share of opponents toward neutral so a front visibly
// sweeps the field when you scrub or play.
function trajectory(base, sway) {
  const out = []
  let s = base
  for (let r = 0; r < ROUNDS; r++) {
    if (r >= EVENT_AT) s += (0 - base) * 0.14 * sway // decay toward neutral
    s += (rnd() - 0.5) * 0.05 // small per-round noise
    out.push(Math.max(-1, Math.min(1, s)))
  }
  return out
}

// ?n=1200 scales the synthetic population for profiling the WebGL layer at scale.
const N = Math.max(1, Math.min(5000, Number(new URLSearchParams(location.search).get('n')) || 130))

const personas = ref(
  Array.from({ length: N }, (_, i) => {
    const t = pick(TYPES)
    const forced = lean[t]
    const faction = forced !== undefined ? forced : rnd() < 0.45 ? 'con' : rnd() < 0.6 ? 'pro' : null
    const synthetic = t === 'Citizen' && rnd() < 0.5
    const base = faction === 'con' ? -0.75 : faction === 'pro' ? 0.75 : 0
    // Opponents who are ordinary citizens are the most persuadable; the ministry
    // and unions hold firm.
    const sway = t === 'Ministry' || t === 'Union' ? 0.1 : rnd()
    return {
      user_id: i + 1,
      name: `${t} ${i + 1}`,
      user_name: `${t.toLowerCase()}_${i + 1}`,
      faction,
      source_entity_type: t,
      synthetic,
      evidence: synthetic ? [] : FACTS[t] || [],
      trajectory: trajectory(base, faction === 'con' ? sway : 0.15),
    }
  })
)
</script>

<style scoped>
.demo { display: flex; flex-direction: column; height: 100vh; background: var(--navy); font-family: var(--font-sans); }
.demo-head { display: flex; align-items: center; justify-content: space-between; padding: 14px 20px; color: var(--on-dark); }
.tag { font: 700 11px/1 var(--font-sans); letter-spacing: .14em; text-transform: uppercase; color: var(--emerald-bright); }
.note { margin-left: 14px; font-size: 13px; color: var(--on-dark-muted); }
.back { color: var(--emerald-bright); text-decoration: none; font-size: 13px; }
.back:hover { text-decoration: underline; }
.demo :deep(.field) { flex: 1; }
</style>
