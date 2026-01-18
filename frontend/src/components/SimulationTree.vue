<template>
  <div class="simulation-tree-container">
    <div v-if="loading" class="loading-state">Loading Multiverse Map...</div>
    <div v-else-if="error" class="error-state">{{ error }}</div>
    <div v-else class="tree-viz" ref="treeContainer"></div>
  </div>
</template>

<script setup>
import { ref, onMounted, watch, onUnmounted } from 'vue'
import * as d3 from 'd3'
import { getSimulationTree } from '../api/simulation'

const props = defineProps({
  simulationId: {
    type: String,
    required: true
  },
  height: {
      type: Number,
      default: 200
  }
})

const emit = defineEmits(['switch-simulation'])

const loading = ref(false)
const error = ref(null)
const treeContainer = ref(null)
const treeData = ref({ nodes: [] })

const fetchTree = async () => {
    loading.value = true
    try {
        const res = await getSimulationTree(props.simulationId)
        if (res.success) {
            treeData.value = res.data
            renderTree(res.data)
        } else {
            error.value = res.error
        }
    } catch (e) {
        error.value = e.message
    } finally {
        loading.value = false
    }
}

const renderTree = (data) => {
    if (!treeContainer.value) return
    
    // clear previous
    d3.select(treeContainer.value).selectAll("*").remove()
    
    const nodes = data.nodes.map(d => ({...d})) // Copy
    const width = treeContainer.value.clientWidth
    const height = props.height
    
    const svg = d3.select(treeContainer.value)
        .append("svg")
        .attr("width", width)
        .attr("height", height)
        .attr("viewBox", [0, 0, width, height])
        
    // Organize by levels (stratify)
    // D3 Stratify expects specific structure, we have id/parent
    // Root has parent: null
    
    try {
        const root = d3.stratify()
            .id(d => d.id)
            .parentId(d => d.parent)
            (nodes)
            
        // Layout
        // Tree layout?
        const treeLayout = d3.tree().size([width - 100, height - 60])
        treeLayout(root)
        
        const g = svg.append("g")
            .attr("transform", "translate(50, 30)")
            
        // Links
        g.selectAll(".link")
            .data(root.links())
            .join("path")
            .attr("class", "link")
            .attr("data-from", d => d.source.data.id)
            .attr("data-to", d => d.target.data.id)
            .attr("fill", "none")
            .attr("stroke", "#555")
            .attr("stroke-opacity", 0.6)
            .attr("stroke-width", 2)
            .attr("d", d3.linkVertical()
                .x(d => d.x)
                .y(d => d.y))
                
        // Nodes
        const node = g.selectAll(".node")
            .data(root.descendants())
            .join("g")
            .attr("class", d => `node ${d.data.id === props.simulationId ? 'active' : ''}`)
            .attr("transform", d => `translate(${d.x},${d.y})`)
            .on("click", (event, d) => {
                emit('switch-simulation', d.data.id)
            })
            
        node.append("circle")
            .attr("r", 8)
            .attr("fill", d => {
                 if (d.data.id === props.simulationId) return "#00ff9d" // Active
                 if (d.data.status === "running") return "#00aaff"
                 return "#999"
            })
            .attr("stroke", "#333")
            .attr("stroke-width", 2)
            
        node.append("text")
            .attr("dy", "1.5em")
            .attr("x", 0)
            .attr("text-anchor", "middle")
            .text(d => d.data.id === props.simulationId ? 'YOU' : d.data.id.substring(0,6))
            .clone(true).lower()
            .attr("stroke", "black")
            .attr("stroke-width", 3)
            
        node.append("title")
            .text(d => `ID: ${d.data.id}\nStatus: ${d.data.status}\nStart Round: ${d.data.fork_round || 0}`)
            
    } catch (e) {
        console.error("Tree render error", e)
        error.value = "Failed to render tree structure"
    }
}

onMounted(() => {
    fetchTree()
    window.addEventListener('resize', () => renderTree(treeData.value))
})

onUnmounted(() => {
    window.removeEventListener('resize', () => renderTree(treeData.value))
})

watch(() => props.simulationId, () => {
    fetchTree()
})
</script>

<style scoped>
.simulation-tree-container {
    width: 100%;
    background: rgba(0,0,0,0.3);
    border-radius: 8px;
    margin-bottom: 20px;
    border: 1px solid rgba(255,255,255,0.1);
    overflow: hidden;
}

.tree-viz {
    width: 100%;
    height: 100%;
    min-height: 200px;
}

.loading-state, .error-state {
    padding: 20px;
    text-align: center;
    color: #888;
}

.error-state {
    color: #ff6b6b;
}

/* D3 Styles */
:deep(.node) {
    cursor: pointer;
    transition: all 0.3s ease;
}

:deep(.node:hover circle) {
    fill: #fff;
    r: 10;
}

:deep(.node.active circle) {
    stroke: #fff;
    stroke-width: 3px;
    filter: drop-shadow(0 0 5px #00ff9d);
}

:deep(.node text) {
    font-size: 10px;
    fill: #ccc;
    font-family: 'Space Mono', monospace;
    pointer-events: none;
}
</style>
