import service, { requestWithRetry } from './index'

/**
 * Generate ontology (upload files and simulation requirements)
 * @param {Object} data - containing files, simulation_requirement, project_name, etc.
 * @returns {Promise}
 */
export function generateOntology(formData) {
  return requestWithRetry(() =>
    service({
      url: '/api/graph/ontology/generate',
      method: 'post',
      data: formData,
      headers: {
        'Content-Type': 'multipart/form-data'
      }
    })
  )
}

/**
 * Build graph
 * @param {Object} data - containing project_id, graph_name, etc.
 * @returns {Promise}
 */
export function buildGraph(data) {
  return requestWithRetry(() =>
    service({
      url: '/api/graph/build',
      method: 'post',
      data
    })
  )
}

/**
 * Query task status
 * @param {String} taskId - 任务ID
 * @returns {Promise}
 */
export function getTaskStatus(taskId) {
  return service({
    url: `/api/graph/task/${taskId}`,
    method: 'get'
  })
}

/**
 * Get graph data
 * @param {String} graphId - 图谱ID
 * @returns {Promise}
 */
export function getGraphData(graphId) {
  return service({
    url: `/api/graph/data/${graphId}`,
    method: 'get'
  })
}

/**
 * Get project information
 * @param {String} projectId - Project ID
 * @returns {Promise}
 */
export function getProject(projectId) {
  return service({
    url: `/api/graph/project/${projectId}`,
    method: 'get'
  })
}

/**
 * Get list of all projects
 * @returns {Promise}
 */
export function listProjects() {
  return service({
    url: '/api/graph/projects',
    method: 'get'
  })
}

/**
 * Update project ontology
 * @param {String} projectId - Project ID
 * @param {Object} ontology - Ontology object { entity_types, edge_types }
 * @returns {Promise}
 */
export function updateOntology(projectId, ontology) {
  return service({
    url: `/api/graph/project/${projectId}/ontology`,
    method: 'put',
    data: ontology
  })
}

/**
 * Preview graph merge
 * @param {Object} data - { source_graph_id, target_graph_id }
 * @returns {Promise}
 */
export function previewMerge(data) {
  return service({
    url: '/api/graph/merge/preview',
    method: 'post',
    data
  })
}

/**
 * Execute graph merge
 * @param {Object} data - { source_graph_id, target_graph_id, strategy }
 * @returns {Promise}
 */
export function executeMerge(data) {
  return service({
    url: '/api/graph/merge/execute',
    method: 'post',
    data
  })
}
