import { ref } from 'vue'
import type { KnowledgeHighlight, AgentMapping, SimulationProfile } from '../types/envSetup'

export function useKnowledgePreload() {
    const preloadedKnowledge = ref<KnowledgeHighlight[]>([])
    const showMatchingModal = ref(false)
    const pendingHighlights = ref<KnowledgeHighlight[]>([])
    const agentMappings = ref<AgentMapping[]>([])

    const clearPreloadedKnowledge = () => {
        preloadedKnowledge.value = []
        sessionStorage.removeItem('preloadedKnowledge')
    }

    const parseKnowledgeFile = async (file: File, profiles: SimulationProfile[]) => {
        if (!file.name.endsWith('.json')) {
            throw new Error('Only JSON files can be imported.')
        }

        const text = await file.text()
        const data = JSON.parse(text)

        if (!data.highlights || !Array.isArray(data.highlights)) {
            throw new Error('Invalid Knowledge Pad file. Missing highlights array.')
        }

        const highlights = data.highlights
        pendingHighlights.value = highlights

        // Extract unique source agents and count highlights per agent
        const agentCounts: Record<string, number> = {}
        highlights.forEach((h: any) => {
            const agent = h.source?.agent || 'Unknown'
            agentCounts[agent] = (agentCounts[agent] || 0) + 1
        })

        // Perform exact matching against current profiles
        const mappings = Object.keys(agentCounts).map(originalAgent => {
            const exactMatchIdx = profiles.findIndex(p =>
                p.username?.toLowerCase() === originalAgent.toLowerCase()
            )

            return {
                originalAgent,
                matchedProfileIdx: exactMatchIdx >= 0 ? exactMatchIdx : null,
                confidence: exactMatchIdx >= 0 ? 'exact' : 'none',
                highlightCount: agentCounts[originalAgent]
            } as AgentMapping
        })

        agentMappings.value = mappings
        showMatchingModal.value = true

        return { highlights, mappings }
    }

    return {
        preloadedKnowledge,
        showMatchingModal,
        pendingHighlights,
        agentMappings,
        clearPreloadedKnowledge,
        parseKnowledgeFile
    }
}
