<template>
  <div class="world">
    <header class="head">
      <router-link class="back" to="/worlds">← Worlds</router-link>
      <h1>{{ world.name }}</h1>
      <p class="sub">{{ world.metaText }}</p>
      <p class="hint">A <b>World</b> is a population built from real evidence. Each <b>run</b> below tests a scenario against that same population - the baseline, plus any alternatives you rehearse.</p>
    </header>

    <div v-if="loading" class="note">Loading…</div>
    <div v-else-if="!runs.length" class="note">
      No runs in this world yet.
    </div>

    <table v-else class="runs">
      <thead>
        <tr>
          <th>Run</th>
          <th>Kind</th>
          <th>Status</th>
          <th class="num">Agents</th>
          <th>Created</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="r in runs" :key="r.simulation_id" class="run" @click="open(r)">
          <td class="name">
            <!-- A real link, so the row is keyboard-reachable and openable in a
                 new tab; the row @click stays as the wider hit target. -->
            <router-link class="run-link" :to="`/world/${graphId}/run/${r.simulation_id}`" @click.stop>
              {{ (r.name && r.name !== 'Untitled Simulation') ? r.name : 'Baseline run' }}
            </router-link>
            <span v-if="r.parent_id" class="from" :title="'forked from ' + r.parent_id">↳ derived</span>
          </td>
          <td><span class="kind" :class="r.kind">{{ r.kind }}</span></td>
          <td><span class="badge" :class="statusOf(r).cls">{{ statusOf(r).label }}</span></td>
          <td class="num">{{ r.num_agents }}</td>
          <td class="date">{{ fmtDate(r.created_at) }}</td>
          <td class="act" @click.stop>
            <template v-if="pendingDelete === r.simulation_id">
              <button class="del-confirm" :disabled="deleting" @click.stop="confirmDel(r)">Delete</button>
              <button class="del-cancel" @click.stop="pendingDelete = null">Cancel</button>
            </template>
            <button v-else class="del-btn" title="Delete run" @click.stop="pendingDelete = r.simulation_id">✕</button>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { listSimulations, deleteSimulation } from '../api/simulation'
import { listProjects } from '../api/graph'
import { runLifecycleStatus } from '../runStatus'
import { worldName } from '../worldName'

const props = defineProps({ graphId: { type: String, required: true } })
const router = useRouter()

const loading = ref(true)
const runs = ref([])
const world = ref({ name: 'World', metaText: '' })
const pendingDelete = ref(null)
const deleting = ref(false)

const confirmDel = async (r) => {
  if (deleting.value) return
  deleting.value = true
  try {
    await deleteSimulation(r.simulation_id)
    runs.value = runs.value.filter((x) => x.simulation_id !== r.simulation_id)
  } catch { /* leave the row on failure */ } finally {
    pendingDelete.value = null
    deleting.value = false
  }
}

const statusOf = (r) => runLifecycleStatus(r.status)

const fmtDate = (iso) => {
  if (!iso) return ''
  const d = new Date(iso)
  return isNaN(d) ? '' : d.toLocaleString(undefined, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' })
}

const open = (r) => router.push(`/world/${props.graphId}/run/${r.simulation_id}`)

onMounted(async () => {
  try {
    const [simsRes, projRes] = await Promise.all([
      listSimulations().catch(() => ({ data: { simulations: [] } })),
      listProjects().catch(() => ({ data: { projects: [] } })),
    ])
    const sims = (simsRes.data?.simulations || []).filter(s => s.graph_id === props.graphId)
    // Canonical runs first, then derived; newest first within each.
    runs.value = sims.sort((a, b) => {
      const ak = a.kind === 'canonical' ? 0 : 1
      const bk = b.kind === 'canonical' ? 0 : 1
      return ak - bk || (b.created_at || '').localeCompare(a.created_at || '')
    })

    const p = (projRes.data?.projects || []).find(p => p.graph_id === props.graphId)
    if (p) {
      const bits = []
      if (p.node_count != null) bits.push(`${p.node_count} entities`)
      if (p.edge_count != null) bits.push(`${p.edge_count} relations`)
      bits.push(`${runs.value.length} run${runs.value.length === 1 ? '' : 's'}`)
      world.value = { name: worldName(p), metaText: bits.join(' · ') }
    } else {
      world.value = { name: 'Untitled world', metaText: `${runs.value.length} runs · graph ${props.graphId.slice(0, 8)}` }
    }
  } finally {
    loading.value = false
  }
})
</script>

<style scoped>
.world { min-height: 100vh; background: var(--color-bg); color: var(--color-text); }
.head, .note, .runs { max-width: 900px; margin-left: auto; margin-right: auto; }
.head { padding: 40px 20px 0; }
.back { display: inline-block; color: var(--color-accent-text); text-decoration: none; font-size: var(--fs-sm); margin-bottom: 10px; }
.back:hover { text-decoration: underline; }
.head h1 { margin: 0 0 6px; font-size: var(--fs-lg); }
.sub { color: var(--color-text-muted); margin: 0; font-size: var(--fs-sm); }
.hint { color: var(--color-text-muted); margin: 10px 0 0; font-size: var(--fs-xs); line-height: 1.55; max-width: 640px; }
.hint b { color: var(--color-text); font-weight: 600; }
.note { padding: 32px 20px; color: var(--color-text-muted); }
.runs { width: calc(100% - 40px); border-collapse: collapse; margin-top: 28px; margin-bottom: 40px; font-size: var(--fs-sm); }
.runs th {
  text-align: left; font-size: var(--fs-2xs); text-transform: uppercase; letter-spacing: .04em;
  color: var(--color-text-muted); font-weight: 600; padding: 0 12px 8px; border-bottom: 1px solid var(--color-border);
}
.runs th.num, td.num { text-align: right; }
.run { cursor: pointer; transition: background .12s; }
.run:hover { background: color-mix(in srgb, var(--color-accent) 7%, transparent); }
.run td { padding: 12px; border-bottom: 1px solid var(--color-border); }
.name { font-weight: 550; }
.run-link { color: inherit; text-decoration: none; }
.run-link:hover { text-decoration: underline; }
.from { margin-left: 8px; font-size: var(--fs-2xs); color: var(--color-text-muted); font-weight: 400; }
/* Kind and status read as outlined pills; only the meaningful states take a
   colour, so a table of runs isn't a fruit salad. */
.kind {
  font-size: var(--fs-2xs); padding: 2px 8px; border-radius: 999px; text-transform: capitalize;
  border: 1px solid var(--color-border); color: var(--color-text-muted);
}
.kind.canonical { border-color: var(--color-accent-2); color: var(--color-accent-2-text); }
.badge {
  font-size: var(--fs-2xs); padding: 2px 9px; border-radius: 999px; font-weight: 600;
  border: 1px solid var(--color-border); color: var(--color-text-muted);
}
/* Chrome only. The stance ramp is the DATA palette (theme.css) - borrowing it
   for status chips would make a run's lifecycle read as a sentiment reading.
   Gold marks the one state wanting attention; failure follows .btn-danger. */
.badge.running { border-color: var(--color-accent); color: var(--color-accent-text); }
.badge.completed { color: var(--color-text); }
.badge.failed { border-color: var(--stance-oppose); color: var(--stance-oppose); }
.date { color: var(--color-text-muted); white-space: nowrap; }
.act { text-align: right; white-space: nowrap; }
.del-btn { border: none; background: none; color: var(--color-text-muted); cursor: pointer; font-size: var(--fs-sm); padding: 4px 7px; border-radius: var(--radius-sm); line-height: 1; }
.del-btn:hover { color: var(--stance-oppose); background: color-mix(in srgb, var(--stance-oppose) 14%, transparent); }
.del-confirm { border: none; background: var(--stance-oppose-strong); color: #fff; border-radius: var(--radius-sm); padding: 4px 10px; font-size: var(--fs-2xs); font-weight: 600; cursor: pointer; }
.del-confirm:disabled { opacity: .6; cursor: default; }
.del-cancel { border: 1px solid var(--color-border); background: transparent; color: var(--color-text-muted); border-radius: var(--radius-sm); padding: 4px 10px; font-size: var(--fs-2xs); cursor: pointer; margin-left: 4px; }
.del-cancel:hover { color: var(--color-text); border-color: var(--color-text-muted); }
</style>
