<template>
  <router-view />
  <GlobalLoadingIndicator />
</template>

<script setup>
import GlobalLoadingIndicator from '@/components/GlobalLoadingIndicator.vue';
// Use Vue Router for page management
</script>

<style>
/* Import Fonts */
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&family=JetBrains+Mono:wght@400;500;700&display=swap');

/* Global Design Tokens */
:root {
  /* Brand Colors */
  --primary: #FF4500;       /* Orange - Action */
  --primary-hover: #E03E00;
  --accent: #000000;        /* Black - Strong Elements */
  
  /* Neutral scale */
  --bg-app: #FAFAFA;        /* App Background - Off-white */
  --bg-surface: #FFFFFF;    /* Cards/Panels */
  --bg-subtle: #F5F5F7;      /* Secondary Backgrounds */
  
  --text-main: #1A1A1A;     /* Primary Text */
  --text-muted: #666666;    /* Secondary Text */
  --text-faint: #A1A1A1;    /* Tertiary/Placeholder */
  --text-inverse: #FFFFFF;

  --border-light: #EAEAEA;
  --border-subtle: #F0F0F0;

  /* Semantic - Status */
  --status-success: #10B981;
  --status-warning: #F59E0B;
  --status-error: #EF4444;
  --status-info: #3B82F6;

  /* Typography */
  --font-sans: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
  --font-mono: 'JetBrains Mono', monospace;

  /* Spacing & Radius */
  --radius-sm: 4px;
  --radius-md: 8px;
  --radius-lg: 12px;
  --radius-full: 9999px;
  
  --shadow-sm: 0 1px 2px rgba(0,0,0,0.05);
  --shadow-md: 0 4px 6px -1px rgba(0,0,0,0.1), 0 2px 4px -1px rgba(0,0,0,0.06);
  --shadow-lg: 0 10px 15px -3px rgba(0,0,0,0.1), 0 4px 6px -2px rgba(0,0,0,0.05);
  
  --header-height: 64px;
}

/* Global Reset & Base */
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: var(--font-sans);
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  color: var(--text-main);
  background-color: var(--bg-app);
  font-size: 14px;
  line-height: 1.5;
}

button {
  font-family: inherit;
  border: none;
  outline: none;
  cursor: pointer;
  background: transparent;
}

input, textarea {
  font-family: inherit;
  outline: none;
}

/* Utility Utilities */
.u-container {
  max-width: 1200px;
  margin: 0 auto;
  padding: 0 24px;
}

/* Custom Scrollbar */
::-webkit-scrollbar {
  width: 6px;
  height: 6px;
}

::-webkit-scrollbar-track {
  background: transparent;
}

::-webkit-scrollbar-thumb {
  background: #D4D4D4;
  border-radius: 3px;
}

::-webkit-scrollbar-thumb:hover {
  background: #A3A3A3;
}
</style>
