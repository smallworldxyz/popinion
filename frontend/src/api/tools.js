import service from './index'

/**
 * Perform a live search using the backend tools
 * @param {Object} params - { query, limit }
 */
export const searchRealWorld = (params) => {
    return service.post('/api/tools/live-search', params)
}
