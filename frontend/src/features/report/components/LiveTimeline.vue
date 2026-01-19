<template>
  <div class="live-timeline">
    <div class="timeline-header">
      <div class="header-icon">⚡</div>
      <h3 class="header-title">Live Intelligence Stream</h3>
    </div>

    <div class="timeline-feed" ref="feedRef">
      <div v-if="logs.length === 0" class="empty-feed">
        Waiting for activity...
      </div>

      <div 
        v-for="(log, i) in displayedLogs" 
        :key="i"
        class="timeline-item"
        :class="getLogClass(log)"
      >
        <!-- Time -->
        <div class="item-time">{{ formatTime(log.timestamp) }}</div>
        
        <!-- Connector -->
        <div class="item-connector">
           <div class="connector-dot"></div>
           <div class="connector-line"></div>
        </div>

        <!-- Content -->
        <div class="item-content">
          <div class="item-header" @click="toggleExpand(i)">
            <span class="action-badge" :class="getActionClass(log.action)">{{ getActionLabel(log.action) }}</span>
            <span class="item-title" v-if="log.action === 'section_start'">{{ log.section_title }}</span>
            <span class="item-title" v-else-if="log.action === 'tool_call'">{{ getToolDisplayName(log.details?.tool_name) }}</span>
            <span class="item-summary" v-else>{{ getLogSummary(log) }}</span>
          </div>

          <!-- Tool Result Visualization -->
          <div v-if="log.action === 'tool_result' && isExpanded(i)" class="tool-result-container">
             <component 
                v-if="getToolComponent(log.details?.tool_name)"
                :is="getToolComponent(log.details?.tool_name)"
                :result="parseToolResult(log.details?.tool_name, log.details?.result)"
                :resultLength="log.details?.result_length"
             />
             <pre v-else class="raw-result">{{ truncate(log.details?.result) }}</pre>
          </div>
          
           <!-- LLM Response -->
           <div v-if="log.action === 'llm_response' && isExpanded(i)" class="llm-response">
              {{ log.details?.response }}
           </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick } from 'vue';
import InsightDisplay from './tools/InsightDisplay.vue';
import PanoramaDisplay from './tools/PanoramaDisplay.vue';
import InterviewDisplay from './tools/InterviewDisplay.vue';
import QuickSearchDisplay from './tools/QuickSearchDisplay.vue';
import { parseInsightForge, parsePanorama, parseInterview, parseQuickSearch } from '../utils/parsers';

const props = defineProps<{
  logs: any[];
}>();

const feedRef = ref<HTMLElement | null>(null);
const expandedSet = ref<Set<number>>(new Set());

// Auto-scroll
watch(() => props.logs.length, () => {
  nextTick(() => {
    if (feedRef.value) {
      feedRef.value.scrollTop = feedRef.value.scrollHeight;
    }
  });
});

const displayedLogs = computed(() => {
  // Filter impactful logs? Or show all?
  // Showing all for now
  return props.logs;
});

const toggleExpand = (index: number) => {
  if (expandedSet.value.has(index)) expandedSet.value.delete(index);
  else expandedSet.value.add(index);
};

const isExpanded = (index: number) => expandedSet.value.has(index);

const formatTime = (ts: number) => {
  if (!ts) return '';
  return new Date(ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
};

const getLogClass = (log: any) => {
  return `log-${log.action}`;
};

const getActionLabel = (action: string) => {
  const map: Record<string, string> = {
    'report_start': 'INIT',
    'planning_start': 'PLAN',
    'planning_complete': 'PLAN',
    'section_start': 'SECTION',
    'section_complete': 'DONE',
    'tool_call': 'TOOL',
    'tool_result': 'DATA',
    'llm_response': 'SYNTHESIS'
  };
  return map[action] || action;
};

const getActionClass = (action: string) => {
  if (action.includes('error')) return 'badge-error';
  if (action === 'tool_call') return 'badge-tool';
  if (action === 'tool_result') return 'badge-data';
  if (action === 'section_start') return 'badge-section';
  return 'badge-info';
};

const getLogSummary = (log: any) => {
  if (log.details?.message) return log.details.message;
  if (log.action === 'tool_call') return log.details?.tool_name;
  return '';
};

const getToolDisplayName = (name: string) => {
  const map: Record<string, string> = {
    'insight_forge': 'Deep Insight',
    'panorama_search': 'Panorama Search',
    'interview_agents': 'Agent Interview',
    'quick_search': 'Quick Search',
    'get_graph_statistics': 'Graph Stats',
    'get_entities_by_type': 'Entity Query'
  };
  return map[name] || name;
};

const getToolComponent = (name: string) => {
  if (name === 'insight_forge') return InsightDisplay;
  if (name === 'panorama_search') return PanoramaDisplay;
  if (name === 'interview_agents') return InterviewDisplay;
  if (name === 'quick_search') return QuickSearchDisplay;
  return null;
};

const parseToolResult = (name: string, result: string) => {
  if (name === 'insight_forge') return parseInsightForge(result);
  if (name === 'panorama_search') return parsePanorama(result);
  if (name === 'interview_agents') return parseInterview(result);
  if (name === 'quick_search') return parseQuickSearch(result);
  return result;
};

const truncate = (text: string) => {
  if (!text) return '';
  return text.length > 200 ? text.substring(0, 200) + '...' : text;
};
</script>

<style scoped>
.live-timeline {
  display: flex;
  flex-direction: column;
  height: 100%;
  border: 1px solid var(--glass-border);
  border-radius: 8px;
  background: var(--bg-surface);
  overflow: hidden;
}

.timeline-header {
  padding: 12px 16px;
  border-bottom: 1px solid var(--glass-border);
  display: flex;
  align-items: center;
  gap: 8px;
  background: rgba(0,0,0,0.2);
}

.header-title {
  font-size: 13px;
  text-transform: uppercase;
  font-weight: 600;
  color: var(--text-secondary);
  margin: 0;
}

.timeline-feed {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
  display: flex;
  flex-direction: column;
}

.empty-feed {
  color: var(--text-muted);
  text-align: center;
  padding: 20px;
  font-size: 13px;
}

.timeline-item {
  display: flex;
  gap: 12px;
  padding-bottom: 16px;
  position: relative;
}

.item-time {
  font-family: 'JetBrains Mono', monospace;
  font-size: 10px;
  color: var(--text-muted);
  width: 50px;
  flex-shrink: 0;
  padding-top: 4px;
}

.item-connector {
  display: flex;
  flex-direction: column;
  align-items: center;
  width: 16px;
}

.connector-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--glass-border);
  margin-top: 5px;
}

.connector-line {
  width: 1px;
  flex: 1;
  background: var(--glass-border);
  margin-top: 4px;
}

.item-content {
  flex: 1;
  min-width: 0;
}

.item-header {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  margin-bottom: 4px;
}

.action-badge {
  font-size: 9px;
  font-weight: 700;
  padding: 2px 6px;
  border-radius: 4px;
  background: var(--bg-subtle);
  color: var(--text-muted);
}

.badge-tool { background: rgba(59, 130, 246, 0.1); color: #60a5fa; }
.badge-data { background: rgba(16, 185, 129, 0.1); color: #34d399; }
.badge-section { background: rgba(255, 87, 34, 0.1); color: var(--primary); }

.item-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-main);
}

.tool-result-container {
  margin-top: 8px;
  border: 1px solid var(--glass-border);
  border-radius: 6px;
  padding: 12px;
  background: rgba(0,0,0,0.2);
}

.raw-result, .llm-response {
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  color: var(--text-secondary);
  white-space: pre-wrap;
  word-break: break-word;
}
</style>
