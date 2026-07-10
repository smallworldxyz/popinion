/**
 * Temporarily store files and requirements pending upload
 * Used for immediate redirect after clicking start engine on homepage, then API call in Process page
 */
import { reactive } from 'vue'

const state = reactive({
  files: [],
  simulationRequirement: '',
  telegramChannel: '',
  telegramMaxPosts: 50,
  isPending: false
})

export function setPendingUpload(files, requirement, telegram = null) {
  state.files = files
  state.simulationRequirement = requirement
  state.telegramChannel = telegram?.channel || ''
  state.telegramMaxPosts = telegram?.maxPosts || 50
  state.isPending = true
}

export function getPendingUpload() {
  return {
    files: state.files,
    simulationRequirement: state.simulationRequirement,
    telegramChannel: state.telegramChannel,
    telegramMaxPosts: state.telegramMaxPosts,
    isPending: state.isPending
  }
}

export function clearPendingUpload() {
  state.files = []
  state.simulationRequirement = ''
  state.telegramChannel = ''
  state.telegramMaxPosts = 50
  state.isPending = false
}

export default state
