<template>
  <div class="sentiment-pulse-container">
    <div class="pulse-header">
        <span class="pulse-title">NARRATIVE PULSE</span>
        <span class="pulse-value">{{ currentTension.toFixed(1) }}%</span>
    </div>
    <div ref="chartContainer" class="pulse-chart"></div>
  </div>
</template>

<script setup>
import { ref, onMounted, onUnmounted, watch, computed } from 'vue'
import * as d3 from 'd3'

const props = defineProps({
  simulationId: String,
  rounds: {
      type: Array,
      default: () => []
  },
  height: {
      type: Number,
      default: 100
  }
})

const chartContainer = ref(null)
const currentTension = computed(() => {
    if (!props.rounds || props.rounds.length === 0) return 0;
    // Simple mock tension: Actions Count normalized? 
    // Or just use the action count of the last round as a proxy for "Kinetic Energy"
    const last = props.rounds[props.rounds.length - 1];
    return Math.min(100, (last.actions_count || 0) * 2); // Mock scale
})

// We need a stable chronological list of data points
// { round: 1, value: 10 }
const chartData = computed(() => {
    return props.rounds.map(r => ({
        round: r.round_num,
        value: (r.actions_count || 0)
    })).sort((a,b) => a.round - b.round);
})

const renderChart = () => {
    if (!chartContainer.value || chartData.value.length === 0) return;

    const data = chartData.value;
    const width = chartContainer.value.clientWidth;
    const height = props.height;
    const margin = { top: 10, right: 10, bottom: 20, left: 30 };
    
    // Clear
    d3.select(chartContainer.value).selectAll("*").remove();
    
    const svg = d3.select(chartContainer.value)
        .append("svg")
        .attr("width", width)
        .attr("height", height)
        .attr("viewBox", [0, 0, width, height]);
        
    const x = d3.scaleLinear()
        .domain(d3.extent(data, d => d.round))
        .range([margin.left, width - margin.right]);
        
    const y = d3.scaleLinear()
        .domain([0, d3.max(data, d => d.value) * 1.2 || 10]) // Add headroom
        .range([height - margin.bottom, margin.top]);
        
    const line = d3.line()
        .x(d => x(d.round))
        .y(d => y(d.value))
        .curve(d3.curveMonotoneX); // Smooth curve
        
    const area = d3.area()
        .x(d => x(d.round))
        .y0(height - margin.bottom)
        .y1(d => y(d.value))
        .curve(d3.curveMonotoneX);
        
    // Gradient
    const defs = svg.append("defs");
    const gradient = defs.append("linearGradient")
        .attr("id", "pulse-gradient")
        .attr("x1", "0%")
        .attr("y1", "0%")
        .attr("x2", "0%")
        .attr("y2", "100%");
        
    gradient.append("stop").attr("offset", "0%").attr("stop-color", "#ff4500").attr("stop-opacity", 0.6);
    gradient.append("stop").attr("offset", "100%").attr("stop-color", "#ff4500").attr("stop-opacity", 0);
    
    // Draw Area
    svg.append("path")
        .datum(data)
        .attr("fill", "url(#pulse-gradient)")
        .attr("d", area);
        
    // Draw Line
    svg.append("path")
        .datum(data)
        .attr("fill", "none")
        .attr("stroke", "#ff4500")
        .attr("stroke-width", 2)
        .attr("d", line);
        
    // Axes (Minimal)
    svg.append("g")
        .attr("transform", `translate(0,${height - margin.bottom})`)
        .call(d3.axisBottom(x).ticks(5).tickSize(3))
        .select(".domain").remove();
        
    svg.append("g")
        .attr("transform", `translate(${margin.left},0)`)
        .call(d3.axisLeft(y).ticks(3).tickSize(3))
        .select(".domain").remove();
}

watch(chartData, () => {
    renderChart();
}, { deep: true });

onMounted(() => {
    renderChart();
    window.addEventListener('resize', renderChart);
});
onUnmounted(() => {
    window.removeEventListener('resize', renderChart);
})
</script>

<style scoped>
.sentiment-pulse-container {
    background: rgba(0,0,0,0.4);
    border: 1px solid rgba(255, 69, 0, 0.3);
    border-radius: 4px;
    padding: 10px;
    height: 100%;
}

.pulse-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 5px;
}

.pulse-title {
    font-size: 10px;
    font-weight: 700;
    color: #ff4500;
    letter-spacing: 1px;
}

.pulse-value {
    font-family: 'Space Mono', monospace;
    color: #fff;
    font-size: 14px;
    font-weight: bold;
}

.pulse-chart {
    width: 100%;
    height: 100px; /* Default fallthrough */
}

/* D3 Axis Styles */
:deep(text) {
    fill: #666;
    font-size: 9px;
    font-family: 'Inter', sans-serif;
}

:deep(line) {
    stroke: #333;
}
</style>
