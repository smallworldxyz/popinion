import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

// https://vite.dev/config/
export default defineConfig({
  plugins: [vue()],
  server: {
    port: 3000,
    open: true,
    proxy: {
      '/api': {
        target: 'http://localhost:5001',
        changeOrigin: true,
        secure: false
      }
    }
  },
  preview: {
    port: 4173,
    proxy: {
      '/api': {
        target: 'https://api-vi.tunnel.koompi.cloud',
        changeOrigin: true,
        secure: false
      }
    }
  }
})

// bun run build && bun run preview && jrok--port 4173 --domain vi && jrok--port 5001 --domain api - vi