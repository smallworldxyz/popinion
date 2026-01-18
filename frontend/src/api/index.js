import axios from 'axios'

// Create axios instance
const service = axios.create({
  baseURL: import.meta.env.VITE_API_BASE_URL || '',
  timeout: 300000, // 5 minutes timeout (ontology generation might take long)
  headers: {
    'Content-Type': 'application/json'
  }
})

// Request interceptor
service.interceptors.request.use(
  config => {
    // Desktop Mode (BYOK): Inject Keys from localStorage
    const activeProvider = localStorage.getItem('po_active_provider') || 'openai'
    let apiKey = ''
    let model = ''

    // Get appropriate key and model
    if (activeProvider === 'openai') {
      apiKey = localStorage.getItem('po_openai_key')
      model = localStorage.getItem('po_openai_model') || 'gpt-4o-mini'
    } else if (activeProvider === 'google') {
      apiKey = localStorage.getItem('po_google_key')
      model = localStorage.getItem('po_google_model') || 'gemini/gemini-1.5-pro'
    } else if (activeProvider === 'anthropic') {
      apiKey = localStorage.getItem('po_anthropic_key')
      model = localStorage.getItem('po_anthropic_model') || 'anthropic/claude-3-5-sonnet-20240620'
    }

    if (apiKey) {
      config.headers['X-LLM-Key'] = apiKey
    }
    if (model) {
      config.headers['X-LLM-Model'] = model
    }

    return config
  },
  error => {
    console.error('Request error:', error)
    return Promise.reject(error)
  }
)

// Response interceptor (fault tolerance retry mechanism)
service.interceptors.response.use(
  response => {
    const res = response.data

    // If returned status code is not success, throw error
    if (!res.success && res.success !== undefined) {
      console.error('API Error:', res.error || res.message || 'Unknown error')
      return Promise.reject(new Error(res.error || res.message || 'Error'))
    }

    return res
  },
  error => {
    console.error('Response error:', error)

    // Handle timeout
    if (error.code === 'ECONNABORTED' && error.message.includes('timeout')) {
      console.error('Request timeout')
    }

    // Handle network error
    if (error.message === 'Network Error') {
      console.error('Network error - please check your connection')
    }

    return Promise.reject(error)
  }
)

// Request function with retry
export const requestWithRetry = async (requestFn, maxRetries = 3, delay = 1000) => {
  for (let i = 0; i < maxRetries; i++) {
    try {
      return await requestFn()
    } catch (error) {
      if (i === maxRetries - 1) throw error

      console.warn(`Request failed, retrying (${i + 1}/${maxRetries})...`)
      await new Promise(resolve => setTimeout(resolve, delay * Math.pow(2, i)))
    }
  }
}

export default service
