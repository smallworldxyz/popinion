import { ref } from 'vue'

// Shared open-state for the Model settings drawer. A drawer, not a route, so
// settings are a quick overlay from anywhere without losing the current page.
export const settingsOpen = ref(false)
export const openSettings = () => { settingsOpen.value = true }
export const closeSettings = () => { settingsOpen.value = false }
