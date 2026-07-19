import service from './index'

// LLM provider settings (the model selector). The backend never returns keys —
// only a has_key flag per slot.
export const getLlmSettings = () => service.get('/api/settings/llm')
// Fast readiness probe of the bulk slot: { ready, model, base_url, reason }
export const getLlmStatus = () => service.get('/api/settings/llm/status')
export const updateLlmSettings = (data) => service.put('/api/settings/llm', data)
export const testLlm = (data) => service.post('/api/settings/llm/test', data)
export const getProviders = () => service.get('/api/settings/llm/providers')

// "Sign in with ChatGPT" — use a ChatGPT subscription as the provider.
// login returns { auth_url } to open; status returns { logged_in, email, plan, status }.
export const chatgptLogin = () => service.post('/api/settings/llm/chatgpt/login')
export const chatgptStatus = () => service.get('/api/settings/llm/chatgpt/status')
export const chatgptLogout = () => service.post('/api/settings/llm/chatgpt/logout')
