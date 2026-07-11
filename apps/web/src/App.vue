<template>
  <router-link v-if="!['Settings', 'Home'].includes($route.name)" to="/settings" class="settings-fab" title="Model settings">⚙ Models</router-link>
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
  /* Publication (editorial) typography — a warm, readable system serif stack,
     no external fonts so it works offline and in the desktop shell. */
  font-family: 'Iowan Old Style', 'Palatino Linotype', 'Palatino', 'Book Antiqua',
    Georgia, Cambria, 'Times New Roman', Times, serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  color: #111111;
  background-color: #ffffff;
  font-size: 16px;
  line-height: 1.6;
  letter-spacing: 0.005em;
}

/* Scrollbar styles */
::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}

::-webkit-scrollbar-track {
  background: #f1f1f1;
}

::-webkit-scrollbar-thumb {
  background: #000000;
}

::-webkit-scrollbar-thumb:hover {
  background: #333333;
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
  font-family: inherit;
  color: #000;
  background: #fff;
  border: 1px solid #000;
  border-radius: 8px;
  text-decoration: none;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.12);
}
.settings-fab:hover {
  background: #000;
  color: #fff;
}
</style>
