<template>
  <button v-if="!['Home', 'Process', 'Simulation', 'SimulationRun'].includes($route.name)" class="settings-fab" title="Model settings" @click="openSettings">⚙ Models</button>
  <router-view />
  <SettingsView />
</template>

<script setup>
import SettingsView from './views/SettingsView.vue'
import { openSettings } from './settingsDrawer'
</script>

<style>
/* Global style reset */
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

#app {
  /* Baray UI type: Inter for body/UI, headings pick up Fraunces (theme.css). */
  font-family: var(--font-body);
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  color: var(--color-text);
  background-color: var(--color-bg);
  font-size: var(--fs-sm);
  line-height: 1.6;
}

/* Scrollbar styles */
::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}

::-webkit-scrollbar-track {
  background: var(--color-bg);
}

::-webkit-scrollbar-thumb {
  background: var(--color-border);
  border-radius: 4px;
}

::-webkit-scrollbar-thumb:hover {
  background: var(--color-text-muted);
}

/* Global button styles */
button {
  font-family: inherit;
}

/* Persistent link to the model settings, visible on every page. */
.settings-fab {
  position: fixed;
  top: 14px;
  right: 16px;
  z-index: 1000;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 7px 12px;
  font-size: var(--fs-xs);
  font-weight: 600;
  font-family: inherit;
  color: var(--color-text);
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius);
  text-decoration: none;
  cursor: pointer;
  box-shadow: 0 1px 4px rgba(14, 35, 64, 0.12);
}
.settings-fab:hover {
  background: var(--color-accent);
  color: var(--color-on-accent);
  border-color: var(--color-accent);
}
</style>
