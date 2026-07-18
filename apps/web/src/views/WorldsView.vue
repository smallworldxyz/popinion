<template>
  <div class="worlds">
    <header class="head">
      <h1>Worlds</h1>
      <p class="sub">
        A World is a knowledge graph and its Personas. Runs are the questions you fire at it.
        Every run is saved here, grouped under the World it came from.
      </p>
      <router-link class="back" to="/">← New simulation</router-link>
    </header>

    <div v-if="loading" class="note">Loading…</div>
    <div v-else-if="!worlds.length" class="note">
      No worlds yet. <router-link to="/">Start a simulation</router-link> to build one.
    </div>

    <ul v-else class="list">
      <li v-for="w in worlds" :key="w.graphId" class="world">
        <router-link class="world-link" :to="`/world/${w.graphId}`">
          <div class="world-main">
            <span class="world-name">{{ w.name }}</span>
            <span class="world-meta">{{ w.entityText }}</span>
          </div>
          <div class="world-stats">
            <span class="runs">{{ w.runCount }} run{{ w.runCount === 1 ? '' : 's' }}</span>
            <span class="last">{{ w.lastText }}</span>
          </div>
        </router-link>
      </li>
    </ul>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { listSimulations } from '../api/simulation'
import { listProjects } from '../api/graph'

const loading = ref(true)
const worlds = ref([])

const fmtDate = (iso) => {
  if (!iso) return ''
  const d = new Date(iso)
  return isNaN(d) ? '' : d.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })
}

onMounted(async () => {
  try {
    const [simsRes, projRes] = await Promise.all([
      listSimulations().catch(() => ({ data: { simulations: [] } })),
      listProjects().catch(() => ({ data: { projects: [] } })),
    ])
    const sims = simsRes.data?.simulations || []
    const projects = projRes.data?.projects || []
    const byGraph = new Map(projects.filter(p => p.graph_id).map(p => [p.graph_id, p]))

    // Group runs by their World (graph_id). Runs with no graph are skipped:
    // they predate graph-linked sims and have no World to belong to.
    const groups = new Map()
    for (const s of sims) {
      if (!s.graph_id) continue
      if (!groups.has(s.graph_id)) groups.set(s.graph_id, [])
      groups.get(s.graph_id).push(s)
    }

    worlds.value = [...groups.entries()].map(([graphId, runs]) => {
      const p = byGraph.get(graphId)
      const entities = p?.node_count
      const last = runs.map(r => r.created_at).sort().at(-1)
      return {
        graphId,
        name: p?.name || 'Untitled world',
        entityText: entities != null ? `${entities} entities` : 'graph ' + graphId.slice(0, 8),
        runCount: runs.length,
        lastText: last ? `last run ${fmtDate(last)}` : '',
        last,
      }
    }).sort((a, b) => (b.last || '').localeCompare(a.last || ''))
  } finally {
    loading.value = false
  }
})
</script>

<style scoped>
.worlds {
  max-width: 820px; margin: 0 auto; padding: 40px 20px;
  font-family: var(--font-sans); color: var(--text-strong);
  background: var(--subtle); min-height: 100vh;
}
.head h1 {
  margin: 0 0 8px; font-family: var(--font-serif); font-weight: 500;
  font-size: 28px; color: var(--text-strong);
}
.sub { color: var(--text-muted); line-height: 1.55; max-width: 620px; margin: 0 0 4px; }
.back {
  display: inline-block; margin-top: 10px; color: var(--emerald);
  text-decoration: none; font-size: 14px; font-weight: 500;
}
.back:hover { text-decoration: underline; }
.note { margin-top: 32px; color: var(--text-muted); }
.note a { color: var(--emerald); }
.list { list-style: none; padding: 0; margin: 28px 0 0; display: flex; flex-direction: column; gap: 10px; }
.world-link {
  display: flex; align-items: center; justify-content: space-between; gap: 16px;
  padding: 16px 18px; border: 1px solid var(--border); border-radius: var(--r-md);
  background: var(--white); box-shadow: var(--elev-1);
  text-decoration: none; color: inherit;
  transition: border-color .15s, box-shadow .15s, transform .15s;
}
.world-link:hover {
  border-color: var(--emerald); box-shadow: var(--elev-2); transform: translateY(-1px);
}
.world-main { display: flex; flex-direction: column; gap: 3px; min-width: 0; }
.world-name {
  font-family: var(--font-serif); font-weight: 500; font-size: 17px;
  color: var(--text-strong);
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
.world-meta {
  font-size: 12.5px; color: var(--text-muted); font-variant-numeric: tabular-nums;
}
.world-stats { display: flex; flex-direction: column; align-items: flex-end; gap: 3px; flex: none; }
.runs {
  font-weight: 600; font-size: 14px; color: var(--text-muted);
  font-variant-numeric: tabular-nums;
}
.last {
  font-size: 12px; color: var(--text-muted); font-variant-numeric: tabular-nums;
}
</style>
