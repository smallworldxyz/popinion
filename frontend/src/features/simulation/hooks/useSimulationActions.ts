import { ref } from 'vue'
import {
    prepareSimulation,
    preparePreview,
    getPrepareStatus,
    getSimulationProfilesRealtime,
    getSimulationConfigRealtime,
} from '../../../api/simulation'
import { SimulationPhase } from '../types/envSetup'

export function useSimulationActions(
    simulationId: any, // passed as ref or value
    state: any,
    profilesState: any,
    configState: any,
    entityState: any,
    emit: any
) {
    let pollTimer: any = null
    let profilesTimer: any = null
    let configTimer: any = null

    // Helper to add log
    const addLog = (msg: string) => {
        emit('add-log', msg)
    }

    // Polling control
    const stopPolling = () => {
        if (pollTimer) {
            clearInterval(pollTimer)
            pollTimer = null
        }
    }

    const stopProfilesPolling = () => {
        if (profilesTimer) {
            clearInterval(profilesTimer)
            profilesTimer = null
        }
    }

    const stopConfigPolling = () => {
        if (configTimer) {
            clearInterval(configTimer)
            configTimer = null
        }
    }

    const stopAllPolling = () => {
        stopPolling()
        stopProfilesPolling()
        stopConfigPolling()
    }

    // Load Entity Preview
    const loadEntityPreview = async () => {
        const simId = typeof simulationId === 'function' ? simulationId() : simulationId.value
        if (!simId) return

        addLog('Loading entities for selection...')
        try {
            const res = await preparePreview({ simulation_id: simId })
            if (res.success && res.data) {
                entityState.previewEntities.value = res.data.entities
                entityState.previewByType.value = res.data.by_type
                addLog(`Found ${res.data.total_count} entities in ${Object.keys(res.data.by_type).length} categories`)
                entityState.showEntityModal.value = true
            } else {
                addLog(`Failed to load entities: ${res.error || 'Unknown error'}`)
            }
        } catch (err: any) {
            addLog(`Error loading entities: ${err.message}`)
        }
    }

    // Fetch Profiles Realtime
    let lastLoggedProfileCount = 0
    const fetchProfilesRealtime = async () => {
        const simId = typeof simulationId === 'function' ? simulationId() : simulationId.value
        if (!simId) return

        try {
            const res = await getSimulationProfilesRealtime(simId, 'reddit') // Default platform?
            if (res.success && res.data) {
                profilesState.profiles.value = res.data.profiles || []
                entityState.expectedTotal.value = res.data.total_expected

                // Log logic
                const currentCount = profilesState.profiles.value.length
                if (currentCount > 0 && currentCount !== lastLoggedProfileCount) {
                    lastLoggedProfileCount = currentCount
                    const total = entityState.expectedTotal.value || '?'
                    const latestProfile = profilesState.profiles.value[currentCount - 1]
                    const profileName = latestProfile?.name || latestProfile?.username
                    addLog(`→ AgentProfile ${currentCount}/${total}: ${profileName}`)
                }
            }
        } catch (err) {
            console.warn('get Profiles Failed', err)
        }
    }

    // Poll Prepare Status
    let lastLoggedMessage = ''
    const pollPrepareStatus = async () => {
        const simId = typeof simulationId === 'function' ? simulationId() : simulationId.value
        if (!state.taskId.value && !simId) return

        try {
            const res = await getPrepareStatus({
                task_id: state.taskId.value,
                simulation_id: simId
            })

            if (res.success && res.data) {
                const data = res.data
                state.updateProgress(
                    data.progress_detail?.current_stage_name || '',
                    data.progress || 0,
                    data.message || ''
                )

                // Log message if new
                if (data.message && data.message !== lastLoggedMessage) {
                    lastLoggedMessage = data.message
                    addLog(data.message)
                }

                if (data.status === 'completed' || data.status === 'ready' || data.already_prepared) {
                    addLog('✓ Preparation Work Completed')
                    stopPolling()
                    stopProfilesPolling()
                    await loadPreparedData()
                } else if (data.status === 'failed') {
                    addLog(`✗ Preparation Failed: ${data.error}`)
                    stopPolling()
                    stopProfilesPolling()
                }
            }
        } catch (err) {
            console.warn('Polling status Failed', err)
        }
    }

    // Poll Config
    let lastLoggedConfigStage = ''
    const fetchConfigRealtime = async () => {
        const simId = typeof simulationId === 'function' ? simulationId() : simulationId.value
        if (!simId) return

        try {
            const res = await getSimulationConfigRealtime(simId)
            if (res.success && res.data) {
                const data = res.data
                if (data.config_generated && data.config) {
                    configState.simulationConfig.value = data.config
                    addLog('✓ Simulation configuration Generation Completed')
                    stopConfigPolling()
                    state.phase.value = SimulationPhase.Completed
                    emit('update-status', 'completed')
                } else if (data.generation_stage && data.generation_stage !== lastLoggedConfigStage) {
                    lastLoggedConfigStage = data.generation_stage
                    addLog(`Config Generation: ${data.generation_stage}`)
                }
            }
        } catch (err) {
            console.warn('Fetch config failed', err)
        }
    }

    // Load Prepared Data
    const loadPreparedData = async () => {
        state.phase.value = SimulationPhase.GeneratingConfig
        addLog('Loading existing configuration data...')

        await fetchProfilesRealtime()

        try {
            const simId = typeof simulationId === 'function' ? simulationId() : simulationId.value
            const res = await getSimulationConfigRealtime(simId)
            if (res.success && res.data && res.data.config_generated) {
                configState.simulationConfig.value = res.data.config
                state.phase.value = SimulationPhase.Completed
                addLog('✓ Environment setup complete')
                emit('update-status', 'completed')
            } else {
                addLog('Configuration generating...')
                configTimer = setInterval(fetchConfigRealtime, 2000)
            }
        } catch (err: any) {
            addLog(`Loading failed: ${err.message}`)
        }
    }

    // Start Preparation
    const startPrepareSimulation = async () => {
        const simId = typeof simulationId === 'function' ? simulationId() : simulationId.value
        if (!simId) {
            addLog('Error: Missing simulationId')
            return
        }

        state.phase.value = SimulationPhase.GeneratingProfiles
        addLog(`Starting preparation for ${simId}`)
        emit('update-status', 'processing')

        try {
            const res = await prepareSimulation({
                simulation_id: simId,
                selected_entity_ids: entityState.selectedEntityIds.value,
                use_llm_for_profiles: true,
                parallel_profile_count: 5
            })

            if (res.success && res.data) {
                if (res.data.already_prepared) {
                    await loadPreparedData()
                    return
                }

                state.taskId.value = res.data.task_id
                if (res.data.expected_agents) entityState.expectedTotal.value = res.data.expected_agents

                addLog('Starting to poll preparation progress...')
                pollTimer = setInterval(pollPrepareStatus, 2000)
                profilesTimer = setInterval(fetchProfilesRealtime, 3000)
            } else {
                addLog(`Preparation failed: ${res.error}`)
                emit('update-status', 'error')
            }
        } catch (err: any) {
            addLog(`Exception: ${err.message}`)
            emit('update-status', 'error')
        }
    }

    const handleStartSimulation = () => {
        const params: any = {}
        if (configState.useCustomRounds.value) {
            params.maxRounds = configState.customMaxRounds.value
            addLog(`Custom Rounds: ${params.maxRounds}`)
        } else {
            addLog(`Auto Rounds: ${configState.autoGeneratedRounds.value}`)
        }
        emit('next-step', params)
    }

    return {
        loadEntityPreview,
        startPrepareSimulation,
        stopAllPolling,
        handleStartSimulation,
        loadPreparedData
    }
}
