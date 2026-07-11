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
      const err = new Error(res.error || res.message || 'Error')
      err.isBusinessError = true // a deterministic server rejection — don't retry
      return Promise.reject(err)
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

// Only idempotent requests may be safely retried. Retrying a POST that already
// reached the server (e.g. after a timeout) duplicates its side effects — a
// second simulation, report, or upload — so non-GET requests and deterministic
// business errors (success:false) are never retried.
const isIdempotent = (method) => ['get', 'head', 'options'].includes((method || '').toLowerCase())

export const requestWithRetry = async (requestFn, maxRetries = 3, delay = 1000) => {
  for (let i = 0; i < maxRetries; i++) {
    try {
      return await requestFn()
    } catch (error) {
      const lastAttempt = i === maxRetries - 1
      if (lastAttempt || error.isBusinessError || !isIdempotent(error.config?.method)) {
        throw error
      }
      console.warn(`Request failed, retrying (${i + 1}/${maxRetries})...`)
      await new Promise(resolve => setTimeout(resolve, delay * Math.pow(2, i)))
    }
  }
}

export default service
