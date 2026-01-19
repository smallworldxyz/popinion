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
</style>
