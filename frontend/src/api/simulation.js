import service, { requestWithRetry } from './index'

/**
 * Create simulation
 * @param {Object} data - { project_id, graph_id?, enable_twitter?, enable_reddit? }
 */
export const createSimulation = (data) => {
  return requestWithRetry(() => service.post('/api/simulation/create', data), 3, 1000)
}

/**
 * Prepare simulation environment (async task)
 * @param {Object} data - { simulation_id, entity_types?, selected_entity_ids?, use_llm_for_profiles?, parallel_profile_count?, force_regenerate? }
 */
export const prepareSimulation = (data) => {
  return requestWithRetry(() => service.post('/api/simulation/prepare', data), 3, 1000)
}

/**
 * Preview entities before preparation - returns list for user selection
 * @param {Object} data - { simulation_id, entity_types? }
 * @returns {Promise} Entity list grouped by type with counts
 */
export const preparePreview = (data) => {
  return service.post('/api/simulation/prepare/preview', data)
}

/**
 * Query preparation task progress
 * @param {Object} data - { task_id?, simulation_id? }
 */
export const getPrepareStatus = (data) => {
  return service.post('/api/simulation/prepare/status', data)
}

/**
 * Get simulation status
 * @param {string} simulationId
 */
export const getSimulation = (simulationId) => {
  return service.get(`/api/simulation/${simulationId}`)
}

/**
 * Get Agent Profiles of simulation
 * @param {string} simulationId
 * @param {string} platform - 'reddit' | 'twitter'
 */
export const getSimulationProfiles = (simulationId, platform = 'reddit') => {
  return service.get(`/api/simulation/${simulationId}/profiles`, { params: { platform } })
}

/**
 * Real-time get generating Agent Profiles
 * @param {string} simulationId
 * @param {string} platform - 'reddit' | 'twitter'
 */
export const getSimulationProfilesRealtime = (simulationId, platform = 'reddit') => {
  return service.get(`/api/simulation/${simulationId}/profiles/realtime`, { params: { platform } })
}

/**
 * Get simulation config
 * @param {string} simulationId
 */
export const getSimulationConfig = (simulationId) => {
  return service.get(`/api/simulation/${simulationId}/config`)
}

/**
 * Real-time get generating simulation config
 * @param {string} simulationId
 * @returns {Promise} Config info, containing metadata and config content
 */
export const getSimulationConfigRealtime = (simulationId) => {
  return service.get(`/api/simulation/${simulationId}/config/realtime`)
}

/**
 * List all simulations
 * @param {string} projectId - Optional, filter by project ID
 */
export const listSimulations = (projectId) => {
  const params = projectId ? { project_id: projectId } : {}
  return service.get('/api/simulation/list', { params })
}

/**
 * Start simulation
 * @param {Object} data - { simulation_id, platform?, max_rounds?, enable_graph_memory_update? }
 */
export const startSimulation = (data) => {
  return requestWithRetry(() => service.post('/api/simulation/start', data), 3, 1000)
}

/**
 * Stop simulation
 * @param {Object} data - { simulation_id }
 */
export const stopSimulation = (data) => {
  return service.post('/api/simulation/stop', data)
}

/**
 * Get simulation realtime run status
 * @param {string} simulationId
 */
export const getRunStatus = (simulationId) => {
  return service.get(`/api/simulation/${simulationId}/run-status`)
}

/**
 * Get detailed run status (containing recent actions)
 * @param {string} simulationId
 */
export const getRunStatusDetail = (simulationId) => {
  return service.get(`/api/simulation/${simulationId}/run-status/detail`)
}

/**
 * Get posts in simulation
 * @param {string} simulationId
 * @param {string} platform - 'reddit' | 'twitter'
 * @param {number} limit - Return count
 * @param {number} offset - Offset
 */
export const getSimulationPosts = (simulationId, platform = 'reddit', limit = 50, offset = 0) => {
  return service.get(`/api/simulation/${simulationId}/posts`, {
    params: { platform, limit, offset }
  })
}

/**
 * Get simulation timeline (aggregated by round)
 * @param {string} simulationId
 * @param {number} startRound - Start round
 * @param {number} endRound - End round
 */
export const getSimulationTimeline = (simulationId, startRound = 0, endRound = null) => {
  const params = { start_round: startRound }
  if (endRound !== null) {
    params.end_round = endRound
  }
  return service.get(`/api/simulation/${simulationId}/timeline`, { params })
}

/**
 * Get Agent statistics
 * @param {string} simulationId
 */
export const getAgentStats = (simulationId) => {
  return service.get(`/api/simulation/${simulationId}/agent-stats`)
}

/**
 * Get simulation action history
 * @param {string} simulationId
 * @param {Object} params - { limit, offset, platform, agent_id, round_num }
 */
export const getSimulationActions = (simulationId, params = {}) => {
  return service.get(`/api/simulation/${simulationId}/actions`, { params })
}

/**
 * Close simulation environment (graceful exit)
 * @param {Object} data - { simulation_id, timeout? }
 */
export const closeSimulationEnv = (data) => {
  return service.post('/api/simulation/close-env', data)
}

/**
 * Get simulation environment status
 * @param {Object} data - { simulation_id }
 */
export const getEnvStatus = (data) => {
  return service.post('/api/simulation/env-status', data)
}

/**
 * Batch interview Agents
 * @param {Object} data - { simulation_id, interviews: [{ agent_id, prompt }] }
 */
export const interviewAgents = (data) => {
  return requestWithRetry(() => service.post('/api/simulation/interview/batch', data), 3, 1000)
}

/**
 * Panel Chat - Ask all agents the same question with aggregated responses
 * @param {Object} data - { simulation_id, prompt, platform?, classify_stance?, generate_summary?, timeout? }
 */
export const panelChat = (data) => {
  return requestWithRetry(() => service.post('/api/simulation/panel-chat', data), 3, 2000)
}

/**
 * Create a survey template
 * @param {Object} data - { title, description?, questions: [{question_text, question_type, options?}] }
 */
export const createSurvey = (data) => {
  return service.post('/api/simulation/survey/create', data)
}

/**
 * Deploy survey to agents
 * @param {Object} data - { simulation_id, survey_id, platform?, timeout? }
 */
export const deploySurvey = (data) => {
  return requestWithRetry(() => service.post('/api/simulation/survey/deploy', data), 3, 2000)
}

/**
 * List all survey templates
 */
export const listSurveys = () => {
  return service.get('/api/simulation/survey/list')
}

/**
 * Get a specific survey template
 * @param {string} surveyId
 */
export const getSurvey = (surveyId) => {
  return service.get(`/api/simulation/survey/${surveyId}`)
}


// ============== Agora Debate API ==============

/**
 * Get available debate templates/goals
 */
export const getAgoraTemplates = () => {
  return service.get('/api/simulation/agora/templates')
}

/**
 * Create a new Agora debate
 * @param {Object} data - { simulation_id, topic, goal_type, agent_ids, agent_names, max_rounds?, debate_mode?, moderator_mode? }
 */
export const createAgoraDebate = (data) => {
  return service.post('/api/simulation/agora/create', data)
}

/**
 * Execute a debate round
 * @param {string} debateId
 * @param {Object} data - { pivot_topic? }
 */
export const executeAgoraRound = (debateId, data = {}) => {
  return requestWithRetry(() => service.post(`/api/simulation/agora/${debateId}/round`, data), 3, 2000)
}

/**
 * List all debates for a simulation
 * @param {string} simulationId
 */
export const listAgoraDebates = (simulationId) => {
  return service.get(`/api/simulation/agora/list/${simulationId}`)
}

/**
 * Execute a timed debate round with real-time SSE streaming.
 * Returns a URL that can be used with EventSource for real-time updates.
 * @param {string} debateId
 * @param {Object} data - { round_duration_seconds?, pivot_topic? }
 * @returns {string} SSE stream URL
 */
export const getStreamAgoraRoundUrl = (debateId) => {
  const baseUrl = import.meta.env.VITE_API_BASE_URL || ''
  return `${baseUrl}/api/simulation/agora/${debateId}/stream-round`
}

/**
 * Get debate status
 * @param {string} debateId
 */
export const getAgoraStatus = (debateId) => {
  return service.get(`/api/simulation/agora/${debateId}/status`)
}

/**
 * Get full debate state by ID
 * @param {string} debateId
 */
export const getAgoraDebate = (debateId) => {
  return service.get(`/api/simulation/agora/${debateId}`)
}

/**
 * Pause a running debate
 * @param {string} debateId
 */
export const pauseAgoraDebate = (debateId) => {
  return service.post(`/api/simulation/agora/${debateId}/pause`)
}

/**
 * Resume a paused debate
 * @param {string} debateId
 */
export const resumeAgoraDebate = (debateId) => {
  return service.post(`/api/simulation/agora/${debateId}/resume`)
}

/**
 * Stop a debate permanently
 * @param {string} debateId
 * @param {Object} data - { generate_summary? }
 */
export const stopAgoraDebate = (debateId, data = {}) => {
  return service.post(`/api/simulation/agora/${debateId}/stop`, data)
}
