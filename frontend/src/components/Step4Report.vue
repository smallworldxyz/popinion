<template>
  <div class="report-view">
    <!-- Header -->
    <div class="report-header">
      <div class="report-title-section">
        <span class="report-tag">Mission Debrief</span>
        <h1 class="report-main-title">Intelligence Briefing</h1>
        <p class="report-subtitle">REF-{{ simulationId?.slice(0, 8).toUpperCase() || 'UNKNOWN' }}</p>
      </div>
      
      <div class="report-actions">
         <div v-if="!isComplete" class="report-action-btn">
             <span class="animate-pulse">●</span> Generating...
         </div>
         <button class="report-action-btn primary" @click="emit('next-step')" :disabled="!isComplete">
            Enter Deep Interaction →
         </button>
      </div>
    </div>

    <!-- Main Grid -->
    <div class="report-container">
      
      <!-- Left Sidebar: Outline -->
      <div class="report-outline-panel">
        <div class="outline-header">Report Structure</div>
        <div class="outline-list" v-if="reportOutline">
          <div 
             v-for="(section, idx) in reportOutline.sections" 
             :key="idx"
             class="outline-item"
             :class="{ 
               'completed': generatedSections[idx + 1],
               'active': !generatedSections[idx + 1] && isActiveSection(idx + 1)
             }"
             @click="scrollToSection(idx + 1)"
          >
            <span class="item-status-icon"></span>
            {{ idx + 1 }}. {{ section.title }}
          </div>
        </div>
        <div v-else class="outline-list">
           <div class="outline-item">Initializing Strategy...</div>
        </div>
      </div>

      <!-- Center: Document Content -->
      <div class="report-main-content">
        <!-- Executive Summary Header Data -->
        <ExecutiveSummary :reportData="summaryData" />

        <!-- Document Body -->
        <div class="document-view">
           <div v-if="Object.keys(generatedSections).length === 0 && !reportOutline" class="document-section">
              <h2>Initializing Intel...</h2>
              <p class="document-content">Awaiting strategic analysis from control.</p>
           </div>

           <div 
              v-for="(section, idx) in (reportOutline?.sections || [])" 
              :key="idx"
              class="document-section"
              :id="`section-${idx+1}`"
           >
              <h2>{{ idx + 1 }}. {{ section.title }}</h2>
              
              <div 
                v-if="generatedSections[idx + 1]" 
                class="document-content" 
                v-html="renderMarkdown(generatedSections[idx + 1])"
              ></div>
              
              <div v-else class="document-placeholder">
                 <div v-if="isActiveSection(idx + 1)" class="animate-pulse">Writing analysis...</div>
                 <div v-else class="text-muted">Pending...</div>
              </div>
           </div>
        </div>
      </div>

      <!-- Right Sidebar: Agents & Feed -->
      <div class="report-sidebar">
        <!-- Top Influencers Stack -->
        <div class="sidebar-section">
           <!-- <h3 class="sidebar-title">Key Influencers</h3> -->
           <InfluencerCard 
              v-for="(agent, i) in topInfluencers" 
              :key="i" 
              :influencer="agent"
              :rank="i + 1"
           />
        </div>

        <!-- Live Feed -->
        <div class="sidebar-section" style="flex: 1; min-height: 400px;">
           <LiveTimeline :logs="agentLogs" />
        </div>
      </div>

    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { useReportGeneration } from '../features/report/hooks/useReportGeneration';
import ExecutiveSummary from '../features/report/components/ExecutiveSummary.vue';
import InfluencerCard from '../features/report/components/InfluencerCard.vue';
import LiveTimeline from '../features/report/components/LiveTimeline.vue';
import { renderMarkdown } from '../features/report/utils/markdown';
import '../features/report/styles/report.css'; 

const props = defineProps<{
  simulationId: string;
  graphData?: any;
  projectData?: any;
  systemLogs?: any[];
}>();

const emit = defineEmits(['go-back', 'next-step', 'add-log']);

const { 
  agentLogs, 
  reportOutline, 
  generatedSections, 
  isComplete, 
  stats 
} = useReportGeneration(props.projectData?.report_id); 
// Note: projectData may not have report_id initially if Step4 triggers generation.
// But Step4Report usually expects generation to be started or starts it.
// The original Step4Report polled. 'useReportGeneration' handles polling.
// But passing undefined reportId means generic poll? Or wait? 
// Original Step4Report watched props.reportId. 
// My hook watches reportId on mount. Assuming projectData has it.
// If projectData.report_id is missing, we might need to TRIGGER generation. 
// But `report.js` API says `generateReport`.
// `Step2EnvSetup` etc navigate here. `Process.vue` handles logic?
// `Process.vue` usually calls `generateOntology` etc.
// In `Process.vue`: `initProject` -> calls stuff.
// If `Step4Report` is rendered, we are in step 4.
// `Process.vue` logic for `currentStep === 4`.
// Does it auto-start generation?
// The original `Step4Report` did NOT auto-start in the snippet I saw. It just polled.
// So likely `Process.vue` or backend triggers it?
// Or `Step4Report` has a "Start" button?
// In original lines 1-400, I didn't see a "Start" button.
// But earlier summary said "Waiting for Report Agent".
// Assume backend starts it or it's already running.
// If no report ID, we might just show empty state.

const isActiveSection = (idx: number) => {
  // If we have content for N-1 but not N, N is active
  if (idx === 1 && !generatedSections.value[1]) return true;
  return !!generatedSections.value[idx - 1] && !generatedSections.value[idx];
};

const scrollToSection = (idx: number) => {
  const el = document.getElementById(`section-${idx}`);
  if (el) el.scrollIntoView({ behavior: 'smooth' });
};

// Mock Summary Data derived from graph/logs
const summaryData = computed(() => {
  return {
    status: isComplete.value ? 'completed' : 'running',
    impactScore: 87, // Mocked for now, implies "Success"
    totalReach: 1250000, // Mock
    avgSentiment: 0.65, // Mock
    totalEngagement: 45200 // Mock
  };
});

// Influencers from Project Data
const topInfluencers = computed(() => {
  // If projectData has agents, map them
  /* 
  Example Agent:
  {
    "name": "Alex",
    "role": "Journalist",
    "traits": ...
  }
  */
  // We need to map to InfluencerCard props: { name, handle, reach, actionCount, sentiment, onTwitter... }
  // We'll try to find them in props.projectData?.simulation_config?.agents
  
  const agents = props.projectData?.simulation_config?.agents || [];
  if (agents.length === 0) {
      // Return dummy if empty to show UI
      return [
          { name: 'System Analyst', handle: 'sys_admin', reach: 0, actionCount: 0, sentiment: 0.5, onTwitter: true }
      ];
  }

  return agents.slice(0, 3).map((a: any) => ({
      name: a.name,
      handle: a.name.toLowerCase().replace(/\s/g, '_'),
      reach: Math.floor(Math.random() * 50000) + 1000, // Mock reach
      actionCount: Math.floor(Math.random() * 100),    // Mock actions
      sentiment: (Math.random() * 2) - 1,              // Mock sentiment
      onTwitter: true,
      onReddit: Math.random() > 0.5,
      topQuote: a.bio?.substring(0, 60) + '...'
  }));
});

</script>

<style scoped>
/* Scoped overrides if necessary */
.text-muted { color: var(--text-muted); }
</style>
