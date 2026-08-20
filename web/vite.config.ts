import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  // GitHub Actions sets CI=true automatically; only the Pages-deploy build
  // needs the /cross-tetris/ subpath (project site, not domain root) — local
  // dev and preview stay at / unaffected.
  base: process.env.CI ? '/cross-tetris/' : '/',
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
