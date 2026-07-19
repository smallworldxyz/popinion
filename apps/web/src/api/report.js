import service, { requestWithRetry } from './index'

/**
 * Start report generation
 * @param {Object} data - { simulation_id, force_regenerate? }
 */
export const generateReport = (data) => {
  return requestWithRetry(() => service.post('/api/report/generate', data), 3, 1000)
}

/**
 * Does this run already have a report? Cheap read - lets a run link straight to
 * its report instead of routing the user back through the setup screens.
 * @param {string} simulationId
 * @returns {Promise} { has_report, report_id, report_status }
 */
export const checkReport = (simulationId) => {
  return service.get(`/api/report/check/${simulationId}`)
}

/**
 * Get Agent logs (incremental)
 * @param {string} reportId
 * @param {number} fromLine - Start from which line
 */
export const getAgentLog = (reportId, fromLine = 0) => {
  return service.get(`/api/report/${reportId}/agent-log`, { params: { from_line: fromLine } })
}

/**
 * Get console logs (incremental)
 * @param {string} reportId
 * @param {number} fromLine - Start from which line
 */
export const getConsoleLog = (reportId, fromLine = 0) => {
  return service.get(`/api/report/${reportId}/console-log`, { params: { from_line: fromLine } })
}

/**
 * Get report details
 * @param {string} reportId
 */
export const getReport = (reportId) => {
  return service.get(`/api/report/${reportId}`)
}

/**
 * Chat with Report Agent
 * @param {Object} data - { simulation_id, message, chat_history? }
 */
export const chatWithReport = (data) => {
  return requestWithRetry(() => service.post('/api/report/chat', data), 3, 1000)
}
