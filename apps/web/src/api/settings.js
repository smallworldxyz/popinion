import service from './index'

// LLM provider settings (the model selector). The backend never returns keys —
// only a has_key flag per slot.
export const getLlmSettings = () => service.get('/api/settings/llm')
// Fast readiness probe of the bulk slot: { ready, model, base_url, reason }
export const getLlmStatus = () => service.get('/api/settings/llm/status')
export const updateLlmSettings = (data) => service.put('/api/settings/llm', data)
export const testLlm = (data) => service.post('/api/settings/llm/test', data)
export const getProviders = () => service.get('/api/settings/llm/providers')

// LM Studio local model management
export const lmsModels = () => service.get('/api/settings/lmstudio/models')
export const lmsLoad = (model) => service.post('/api/settings/lmstudio/load', { model })
export const lmsUnload = (model) => service.post('/api/settings/lmstudio/unload', { model })
export const lmsDownload = (model) => service.post('/api/settings/lmstudio/download', { model })
export const lmsDownloadStatus = (taskId) => service.post('/api/settings/lmstudio/download/status', { task_id: taskId })

// Ollama local model management (native pull — no CLI needed)
export const ollamaPull = (model, base_url) => service.post('/api/settings/ollama/pull', { model, base_url })
export const ollamaPullStatus = (taskId) => service.post('/api/settings/ollama/pull/status', { task_id: taskId })
