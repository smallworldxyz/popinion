import { ref, computed } from 'vue'
import type { SimulationProfile } from '../types/envSetup'

export function useProfileManagement() {
    const profiles = ref<SimulationProfile[]>([])
    const showExplorer = ref(false)
    const selectedProfile = ref<SimulationProfile | null>(null)
    const showProfilesDetail = ref(true)

    const displayProfiles = computed(() => {
        if (showProfilesDetail.value) {
            return profiles.value
        }
        return profiles.value.slice(0, 6)
    })

    const getAgentUsername = (agentId: number) => {
        if (profiles.value && profiles.value.length > agentId && agentId >= 0) {
            const profile = profiles.value[agentId]
            return profile?.username || `agent_${agentId}`
        }
        return `agent_${agentId}`
    }

    const totalTopicsCount = computed(() => {
        return profiles.value.reduce((sum, p) => {
            return sum + (p.interested_topics?.length || 0)
        }, 0)
    })

    return {
        profiles,
        showExplorer,
        selectedProfile,
        showProfilesDetail,
        displayProfiles,
        totalTopicsCount,
        getAgentUsername
    }
}
