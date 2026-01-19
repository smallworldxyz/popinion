import { ref, computed } from 'vue'
import { SimulationPhase } from '../types/envSetup'

export function useEnvSetupState() {
    const phase = ref<SimulationPhase>(SimulationPhase.Initializing)
    const taskId = ref<string | null>(null)
    const prepareProgress = ref(0)
    const currentStage = ref('')
    const progressMessage = ref('')

    const updateProgress = (stage: string, progress: number, message: string) => {
        currentStage.value = stage
        prepareProgress.value = progress
        progressMessage.value = message

        // Auto-advance phase based on stage
        if (stage === 'GeneratingAgentProfile' || stage === 'generating_profiles') {
            phase.value = SimulationPhase.GeneratingProfiles
        } else if (stage === 'GeneratingSimulationconfiguration' || stage === 'generating_config') {
            phase.value = SimulationPhase.GeneratingConfig
        }
    }

    return {
        phase,
        taskId,
        prepareProgress,
        currentStage,
        progressMessage,
        updateProgress
    }
}
