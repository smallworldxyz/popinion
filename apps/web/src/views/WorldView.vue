<template>
  <div class="world">
    <header class="head">
      <router-link class="back" to="/worlds">← Worlds</router-link>
      <h1>{{ world.name }}</h1>
      <p class="sub">{{ world.metaText }}</p>
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
            {{ r.name || 'Untitled run' }}
            <span v-if="r.parent_id" class="from" :title="'forked from ' + r.parent_id">↳ derived</span>
          </td>
          <td><span class="kind" :class="r.kind">{{ r.kind }}</span></td>
          <td><span class="badge" :class="statusOf(r).cls">{{ statusOf(r).label }}</span></td>
          <td class="num">{{ r.num_agents }}</td>
          <td class="date">{{ fmtDate(r.created_at) }}</td>
          <td class="act">
            <router-link class="bar-link" :to="`/world/${graphId}/prepare/${r.simulation_id}`" @click.stop>
              ⊹ evidence bar
            </router-link>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { listSimulations } from '../api/simulation'
import { listProjects } from '../api/graph'

const props = defineProps({ graphId: { type: String, required: true } })
const router = useRouter()

const loading = ref(true)
const runs = ref([])
const world = ref({ name: 'World', metaText: '' })

// Map the backend's run status onto a small, honest set. "alive"/"completed"
// both mean the run finished; "prepared" is ready-but-never-run.
const statusOf = (r) => {
  const s = r.status || ''
  if (s.startsWith('error')) return { label: 'Failed', cls: 'failed' }
  if (s === 'running' || s === 'initializing') return { label: 'Running', cls: 'running' }
  if (s === 'alive' || s === 'completed') return { label: 'Completed', cls: 'completed' }
  if (s === 'stopped') return { label: 'Stopped', cls: 'stopped' }
  if (s === 'prepared') return { label: 'Ready', cls: 'ready' }
  return { label: 'Draft', cls: 'draft' }
}

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
      world.value = { name: p.name || 'Untitled world', metaText: bits.join(' · ') }
    } else {
      world.value = { name: 'Untitled world', metaText: `${runs.value.length} runs · graph ${props.graphId.slice(0, 8)}` }
    }
  } finally {
    loading.value = false
  }
})
</script>

<style scoped>
.world {
  max-width: 900px; margin: 0 auto; padding: 40px 20px;
  font-family: var(--font-sans); color: var(--text-strong);
  background: var(--subtle); min-height: 100vh;
}
.back {
  display: inline-block; color: var(--emerald); text-decoration: none;
  font-size: 14px; font-weight: 500; margin-bottom: 10px;
}
.back:hover { text-decoration: underline; }
.head h1 {
  margin: 0 0 6px; font-family: var(--font-serif); font-weight: 500;
  font-size: 26px; color: var(--text-strong);
}
.sub { color: var(--text-muted); margin: 0; font-size: 14px; font-variant-numeric: tabular-nums; }
.note { margin-top: 32px; color: var(--text-muted); }
.runs {
  width: 100%; border-collapse: collapse; margin-top: 28px; font-size: 14px;
  background: var(--white); border: 1px solid var(--border);
  border-radius: var(--r-lg); overflow: hidden; box-shadow: var(--elev-1);
}
.runs th {
  text-align: left; font-size: 11px; text-transform: uppercase; letter-spacing: .14em;
  color: var(--text-muted); font-weight: 700; padding: 14px 16px 10px;
  border-bottom: 1px solid var(--border);
}
.runs th.num, td.num { text-align: right; }
.run { cursor: pointer; transition: background .12s; }
.run:hover { background: var(--subtle); }
.run td { padding: 14px 16px; border-bottom: 1px solid var(--border); }
.run:last-child td { border-bottom: none; }
.name { font-family: var(--font-serif); font-weight: 500; color: var(--text-strong); }
.from { margin-left: 8px; font-size: 12px; color: var(--text-muted); font-weight: 400; }
.kind {
  font-size: 11px; padding: 2px 9px; border-radius: var(--r-pill);
  background: rgba(12,133,119,.1); color: var(--emerald); text-transform: capitalize; font-weight: 600;
}
.kind.canonical { background: rgba(12,133,119,.1); color: var(--emerald); }
.kind.alt, .kind.replicate, .kind.ablation { background: var(--subtle); color: var(--text-muted); }
.badge { font-size: 11px; padding: 2px 10px; border-radius: var(--r-pill); font-weight: 600; }
.badge.running { background: rgba(232,106,76,.14); color: var(--coral-on-light); }
.badge.completed { background: rgba(12,133,119,.1); color: var(--emerald); }
.badge.ready { background: rgba(12,133,119,.1); color: var(--emerald); }
.badge.stopped, .badge.draft { background: var(--subtle); color: var(--text-muted); }
.badge.failed { background: rgba(199,78,51,.12); color: var(--coral-on-light); }
.date { color: var(--text-muted); white-space: nowrap; font-variant-numeric: tabular-nums; }
.num { font-variant-numeric: tabular-nums; color: var(--text-muted); }
.act { text-align: right; white-space: nowrap; }
.bar-link { font-size: 12.5px; color: var(--emerald); text-decoration: none; font-weight: 500; }
.bar-link:hover { text-decoration: underline; }
</style>
