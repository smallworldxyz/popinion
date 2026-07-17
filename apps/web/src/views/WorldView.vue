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
.world { max-width: 900px; margin: 0 auto; padding: 40px 20px; color: #1a1a2e; }
.back { display: inline-block; color: #2563eb; text-decoration: none; font-size: 14px; margin-bottom: 10px; }
.back:hover { text-decoration: underline; }
.head h1 { margin: 0 0 6px; font-size: 26px; }
.sub { color: #6b7280; margin: 0; font-size: 14px; }
.note { margin-top: 32px; color: #6b7280; }
.runs { width: 100%; border-collapse: collapse; margin-top: 28px; font-size: 14px; }
.runs th {
  text-align: left; font-size: 11.5px; text-transform: uppercase; letter-spacing: .04em;
  color: #9ca3af; font-weight: 600; padding: 0 12px 8px; border-bottom: 1px solid #e5e7eb;
}
.runs th.num, td.num { text-align: right; }
.run { cursor: pointer; transition: background .12s; }
.run:hover { background: #fafbff; }
.run td { padding: 12px; border-bottom: 1px solid #f1f2f4; }
.name { font-weight: 550; }
.from { margin-left: 8px; font-size: 12px; color: #9ca3af; font-weight: 400; }
.kind { font-size: 11px; padding: 2px 8px; border-radius: 999px; background: #eef2ff; color: #4338ca; text-transform: capitalize; }
.kind.canonical { background: #eef2ff; color: #4338ca; }
.kind.alt, .kind.replicate, .kind.ablation { background: #f3f4f6; color: #6b7280; }
.badge { font-size: 11px; padding: 2px 9px; border-radius: 999px; font-weight: 600; }
.badge.running { background: #fef3c7; color: #92400e; }
.badge.completed { background: #d1fae5; color: #065f46; }
.badge.ready { background: #dbeafe; color: #1e40af; }
.badge.stopped, .badge.draft { background: #f3f4f6; color: #6b7280; }
.badge.failed { background: #fee2e2; color: #b91c1c; }
.date { color: #9ca3af; white-space: nowrap; }
</style>
