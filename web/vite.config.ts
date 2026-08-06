import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  server: {
    port: process.env.PORT ? Number(process.env.PORT) : 5173,
    strictPort: true,
    // The wasm-pack build output lives in ../wasm/pkg, outside this
    // project's root; allow the dev server to serve it directly.
    fs: {
      allow: ['..'],
    },
  },
})
