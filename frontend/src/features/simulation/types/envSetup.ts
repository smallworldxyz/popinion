export enum SimulationPhase {
    Initializing = 0,
    GeneratingProfiles = 1,
    GeneratingConfig = 2,
    Completed = 3
}

export interface SimulationProfile {
    username: string
    role_description?: string
    interested_topics?: string[]
    [key: string]: any
}

export interface SimulationConfig {
    time_config?: {
        total_simulation_hours: number
        minutes_per_round: number
    }
    [key: string]: any
}

export interface KnowledgeHighlight {
    source?: {
        agent: string
    }
    content: string
    [key: string]: any
}

export interface AgentMapping {
    originalAgent: string
    matchedProfileIdx: number | null
    confidence: 'exact' | 'none'
    highlightCount: number
}
