<template>
  <aside class="app-sidebar" :class="{ collapsed }">
    <!-- Brand + collapse toggle -->
    <div class="sidebar-brand-row">
      <button class="brand" @click="router.push('/')" title="POPINION — Home">
        <span class="brand-mark">P</span>
        <span class="brand-name">POPINION</span>
      </button>
      <button class="collapse-toggle" @click="toggleSidebar" :title="collapsed ? 'Expand' : 'Collapse'">
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2">
          <polyline :points="collapsed ? '9 18 15 12 9 6' : '15 18 9 12 15 6'"></polyline>
        </svg>
      </button>
    </div>

    <!-- Primary navigation -->
    <nav class="sidebar-nav">
      <button class="nav-item" @click="router.push('/')" title="New run">
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2">
          <line x1="12" y1="5" x2="12" y2="19"></line>
          <line x1="5" y1="12" x2="19" y2="12"></line>
        </svg>
        <span class="nav-label">New run</span>
      </button>
      <button class="nav-item" :class="{ active: route.name === 'Worlds' }" @click="router.push('/worlds')" title="Worlds">
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="10"></circle>
          <line x1="2" y1="12" x2="22" y2="12"></line>
          <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"></path>
        </svg>
        <span class="nav-label">Worlds</span>
      </button>
    </nav>

    <!-- Contextual step + status — only on the workflow shell views -->
    <template v-if="stepNum">
      <div class="sidebar-divider"></div>
      <div class="sidebar-context">
        <div class="workflow-step">
          <span class="step-num mono">Step {{ stepNum }}/5</span>
          <span class="step-name">{{ stepName }}</span>
        </div>
        <span class="status-indicator" :class="statusClass">
          <span class="dot"></span>
          <span class="status-text">{{ statusText }}</span>
        </span>
      </div>
    </template>

    <!-- View switcher — only when a view mode is bound (v-model) -->
    <template v-if="modelValue">
      <div class="sidebar-divider"></div>
      <div class="sidebar-views">
        <span class="views-eyebrow">View</span>
        <button
          v-for="mode in ['graph', 'split', 'workbench']"
          :key="mode"
          class="view-btn"
          :class="{ active: modelValue === mode }"
          @click="$emit('update:modelValue', mode)"
          :title="viewLabels[mode]"
        >
          <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
            <template v-if="mode === 'graph'"><circle cx="6" cy="6" r="2"></circle><circle cx="18" cy="8" r="2"></circle><circle cx="10" cy="17" r="2"></circle><line x1="7.5" y1="7" x2="9" y2="15.5"></line><line x1="8" y1="6.5" x2="16" y2="7.5"></line></template>
            <template v-else-if="mode === 'split'"><rect x="3" y="4" width="18" height="16" rx="2"></rect><line x1="12" y1="4" x2="12" y2="20"></line></template>
            <template v-else><rect x="3" y="4" width="18" height="16" rx="2"></rect></template>
          </svg>
          <span class="view-label">{{ viewLabels[mode] }}</span>
        </button>
      </div>
    </template>

    <div class="sidebar-spacer"></div>

    <!-- Bottom: Models + GitHub -->
    <div class="sidebar-bottom">
      <button class="bottom-item" @click="openSettings" title="Model settings">
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="3"></circle>
          <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"></path>
        </svg>
        <span class="bottom-label">Models</span>
      </button>
      <a class="bottom-item" href="https://github.com/rithythul/popinion" target="_blank" rel="noopener" title="GitHub">
        <svg viewBox="0 0 24 24" width="18" height="18" fill="currentColor">
          <path d="M12 .5C5.37.5 0 5.87 0 12.5c0 5.3 3.44 9.8 8.21 11.39.6.11.82-.26.82-.58v-2c-3.34.73-4.04-1.61-4.04-1.61-.55-1.39-1.34-1.76-1.34-1.76-1.09-.75.08-.73.08-.73 1.2.09 1.84 1.24 1.84 1.24 1.07 1.83 2.81 1.3 3.5.99.11-.78.42-1.31.76-1.61-2.67-.3-5.47-1.33-5.47-5.93 0-1.31.47-2.38 1.24-3.22-.13-.3-.54-1.52.11-3.17 0 0 1.01-.32 3.3 1.23a11.5 11.5 0 0 1 6 0c2.29-1.55 3.3-1.23 3.3-1.23.65 1.65.24 2.87.12 3.17.77.84 1.23 1.91 1.23 3.22 0 4.61-2.8 5.62-5.48 5.92.43.37.81 1.1.81 2.22v3.29c0 .32.22.7.83.58C20.57 22.29 24 17.79 24 12.5 24 5.87 18.63.5 12 .5z"/>
        </svg>
        <span class="bottom-label">GitHub</span>
        <svg class="ext-icon" viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M7 17L17 7M8 7h9v9"></path>
        </svg>
      </a>
    </div>
  </aside>
</template>

<script setup>
import { useRoute, useRouter } from 'vue-router'
import { collapsed, toggleSidebar } from '../sidebar'
import { openSettings } from '../settingsDrawer'

defineProps({
  stepNum: [Number, String],
  stepName: String,
  statusText: String,
  statusClass: String,
  modelValue: { type: String, default: undefined }
})

defineEmits(['update:modelValue'])

const route = useRoute()
const router = useRouter()

const viewLabels = { graph: 'Graph', split: 'Split View', workbench: 'Workbench' }
</script>

<style scoped>
.app-sidebar {
  flex-shrink: 0;
  width: 248px;
  height: 100vh;
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 14px 12px;
  background: var(--color-surface);
  border-right: 1px solid var(--color-border);
  transition: width 0.24s cubic-bezier(0.25, 0.8, 0.25, 1);
  overflow: hidden;
}
.app-sidebar.collapsed {
  width: 60px;
}

/* Brand row */
.sidebar-brand-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 6px;
  min-height: 34px;
}
.brand {
  display: flex;
  align-items: center;
  gap: 10px;
  border: none;
  background: transparent;
  cursor: pointer;
  padding: 4px 6px;
  min-width: 0;
}
.brand-mark {
  flex-shrink: 0;
  width: 26px;
  height: 26px;
  display: grid;
  place-items: center;
  border-radius: var(--radius-sm);
  background: var(--color-accent);
  color: var(--color-on-accent);
  font-family: var(--font-mono);
  font-weight: 600;
  font-size: var(--fs-sm);
}
.brand-name {
  font-family: var(--font-mono);
  font-weight: 600;
  font-size: var(--fs-sm);
  letter-spacing: 1px;
  color: var(--color-text);
  white-space: nowrap;
}
.collapse-toggle {
  flex-shrink: 0;
  display: grid;
  place-items: center;
  width: 30px;
  height: 30px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--color-text-muted);
  cursor: pointer;
  transition: color 0.15s, border-color 0.15s;
}
.collapse-toggle:hover {
  color: var(--color-accent-text);
  border-color: var(--color-accent);
}

/* Nav */
.sidebar-nav {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.nav-item,
.view-btn,
.bottom-item {
  display: flex;
  align-items: center;
  gap: 12px;
  width: 100%;
  padding: 9px 10px;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--color-text);
  font-family: var(--font-body);
  font-size: var(--fs-sm);
  font-weight: 500;
  text-align: left;
  text-decoration: none;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
  white-space: nowrap;
}
.nav-item svg,
.view-btn svg,
.bottom-item svg { flex-shrink: 0; }
.nav-item:hover,
.bottom-item:hover { background: var(--color-bg); }
.nav-item.active {
  color: var(--color-accent-text);
  background: var(--color-bg);
}

.sidebar-divider {
  height: 1px;
  background: var(--color-border);
  margin: 6px 4px;
}

/* Context: step + status */
.sidebar-context {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 2px 10px;
}
.collapsed .sidebar-context { display: none; }
.workflow-step {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.step-num {
  font-size: var(--fs-2xs);
  font-weight: 600;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--color-text-muted);
}
.step-name {
  font-size: var(--fs-sm);
  font-weight: 600;
  color: var(--color-text);
}
.status-indicator {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  font-size: var(--fs-xs);
  color: var(--color-text-muted);
}
.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--color-text-muted);
}
.status-indicator.processing .dot { background: var(--color-accent); animation: pulse 1s infinite; }
.status-indicator.completed .dot,
.status-indicator.ready .dot { background: var(--stance-support); }
.status-indicator.error .dot { background: var(--stance-oppose-strong); }
@keyframes pulse { 50% { opacity: 0.4; } }

/* View switcher */
.sidebar-views {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.views-eyebrow {
  padding: 4px 10px;
  font-family: var(--font-mono);
  font-size: var(--fs-2xs);
  font-weight: 600;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: var(--color-text-muted);
}
.collapsed .views-eyebrow { display: none; }
.view-btn {
  color: var(--color-text-muted);
  font-weight: 500;
}
.view-btn:hover { background: var(--color-bg); color: var(--color-text); }
.view-btn.active {
  background: var(--color-bg);
  color: var(--color-accent-text);
  font-weight: 600;
}

.sidebar-spacer { flex: 1; }

/* Bottom */
.sidebar-bottom {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.bottom-item { color: var(--color-text-muted); }
.bottom-item:hover { color: var(--color-text); }
.ext-icon { margin-left: auto; opacity: 0.6; }

/* Collapsed: hide labels, center icons */
.collapsed .brand { justify-content: center; padding: 4px; }
.collapsed .brand-name,
.collapsed .nav-label,
.collapsed .view-label,
.collapsed .bottom-label,
.collapsed .ext-icon,
.collapsed .status-text { display: none; }
.collapsed .sidebar-brand-row { flex-direction: column; gap: 8px; }
.collapsed .nav-item,
.collapsed .view-btn,
.collapsed .bottom-item { justify-content: center; gap: 0; padding: 9px 0; }

/* Narrow viewports: auto-collapse to the icon rail so content keeps its room. */
@media (max-width: 760px) {
  .app-sidebar { width: 60px; }
  .brand-name,
  .nav-label,
  .view-label,
  .bottom-label,
  .ext-icon,
  .status-text,
  .sidebar-context,
  .views-eyebrow { display: none; }
  .sidebar-brand-row { flex-direction: column; gap: 8px; }
  .brand { justify-content: center; padding: 4px; }
  .nav-item,
  .view-btn,
  .bottom-item { justify-content: center; gap: 0; padding: 9px 0; }
}
</style>
