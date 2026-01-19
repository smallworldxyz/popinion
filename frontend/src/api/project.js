import service from './index'

export const getProjects = async () => {
    try {
        const response = await service.get('/projects/')
        return { success: true, data: response.data.data }
    } catch (error) {
        return { success: false, error: error.response?.data?.error || error.message }
    }
}

export const deleteProject = async (projectId) => {
    try {
        const response = await service.delete(`/projects/${projectId}`)
        return { success: true, message: response.data.message }
    } catch (error) {
        return { success: false, error: error.response?.data?.error || error.message }
    }
}
