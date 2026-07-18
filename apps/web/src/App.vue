<template>
  <router-link v-if="!['Settings', 'Home', 'Run'].includes($route.name)" to="/settings" class="settings-fab" title="Model settings">⚙ Models</router-link>
  <router-view />
</template>

<script setup>
// Use Vue Router for page management
</script>

<style>
/* Global style reset */
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

#app {
  /* Riverbase: Inter for body/UI, Fraunces for display (loaded in index.html). */
  font-family: var(--font-sans);
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  color: var(--text-strong);
  background-color: var(--subtle);
  font-size: 15px;
  line-height: 1.6;
}

/* Scrollbar styles */
::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}

::-webkit-scrollbar-track {
  background: var(--subtle);
}

::-webkit-scrollbar-thumb {
  background: var(--border);
  border-radius: var(--r-pill);
}

::-webkit-scrollbar-thumb:hover {
  background: var(--text-muted);
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
  font-size: 13px;
  font-weight: 500;
  font-family: inherit;
  color: var(--text-strong);
  background: var(--white);
  border: 1px solid var(--border);
  border-radius: var(--r-sm);
  text-decoration: none;
  box-shadow: var(--elev-1);
}
.settings-fab:hover {
  background: var(--navy);
  color: var(--on-dark);
  border-color: var(--navy);
}
</style>
