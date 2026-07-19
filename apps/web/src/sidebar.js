import { ref } from 'vue'

// Shared collapse-state for the left app rail. Kept in a module (not per-view
// local state) so the sidebar stays collapsed/expanded as the user moves
// between the five app-shell views — mirrors settingsDrawer.js.
export const collapsed = ref(false)
export const toggleSidebar = () => { collapsed.value = !collapsed.value }
