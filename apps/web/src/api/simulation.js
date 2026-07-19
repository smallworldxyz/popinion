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
 * Poll Agent Profiles while they're still being generated. There is no separate
 * realtime endpoint — /profiles already serves current state, and 404s until the
 * first profile lands — so this is the same call under a name that says why.
 */
export const getSimulationProfilesRealtime = getSimulationProfiles

/**
 * Get simulation config
 * @param {string} simulationId
 */
export const getSimulationConfig = (simulationId) => {
  return service.get(`/api/simulation/${simulationId}/config`)
}

/**
 * Poll the simulation config while it's being written. As with profiles above,
 * /config is already the live view — there is no /config/realtime route.
 */
export const getSimulationConfigRealtime = getSimulationConfig

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
 * Delete a simulation (run): stops it if live and removes it from disk. Irreversible.
 * @param {string} simulationId
 */
export const deleteSimulation = (simulationId) => {
  return service.delete(`/api/simulation/${simulationId}`)
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
 * Inject a post into a LIVE run mid-discussion (disinformation red-teaming,
 * a policy reversal, an opponent's statement). Enters agents' feeds from the
 * next round, authored by the system Event account.
 * @param {string} simulationId
 * @param {Object} data - { content }
 */
export const injectPost = (simulationId, data) => {
  return service.post(`/api/simulation/${simulationId}/inject`, data)
}

/**
 * Spread readout for the seed/injected post(s): reach, engagement, and the
 * exposed-vs-unexposed stance difference.
 * @param {string} simulationId
 * @param {number} postId - optional, scope to one post
 */
export const getSpread = (simulationId, postId = null) => {
  const params = postId != null ? { post_id: postId } : {}
  return service.get(`/api/simulation/${simulationId}/spread`, { params })
}

/**
 * Get credibility snapshot (population provenance + both stance weightings)
 * Cheap read, no LLM calls
 * @param {string} simulationId
 */
export const getCredibility = (simulationId) => {
  return service.get(`/api/simulation/${simulationId}/credibility`)
}

/**
 * Run independent stance classification (real LLM calls, slow)
 * @param {string} simulationId
 * @param {Object} data - { topic? }
 */
export const classifyStance = (simulationId, data = {}) => {
  return service.post(`/api/simulation/${simulationId}/classify-stance`, data)
}

/**
 * Duplicate a prepared simulation with a different event (scenario A/B rehearsal).
 * Same personas, same seed — the event is the only variable.
 * @param {string} simulationId - source simulation
 * @param {Object} data - { event }
 */
export const duplicateSimulation = (simulationId, data) => {
  return service.post(`/api/simulation/${simulationId}/duplicate`, data)
}

/**
 * Compare two finished runs: stance distributions, total-variation distance,
 * noise floor and whether the A→B delta is significant.
 * @param {string} a - baseline simulation id
 * @param {string} b - alternative simulation id
 * @param {string[]} runs - optional rerun ids of A for the noise floor
 */
export const compareSimulations = (a, b, runs = []) => {
  const params = { a, b }
  if (runs.length > 0) params.runs = runs.join(',')
  return service.get('/api/simulation/compare', { params })
}

/**
 * Honesty tests over completed runs: sibling reruns → noise floor, and an
 * optional baseline-vs-permuted pair → persona ablation verdict.
 * @param {Object} data - { simulation_ids?, baseline_id?, permuted_id? }
 */
export const validateSimulation = (data) => {
  return service.post('/api/simulation/validate', data)
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



