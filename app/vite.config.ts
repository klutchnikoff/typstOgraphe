import { defineConfig } from 'vite'
import { fileURLToPath } from 'node:url'
import { resolve } from 'node:path'
import { browserDriver } from './browser-driver.ts'
const root = fileURLToPath(new URL('.', import.meta.url))
export default defineConfig(({ mode }) => ({
  plugins: mode === 'browser-test' ? [browserDriver(root)] : [],
  server: { fs: { allow: [resolve(root, '..')] } },
  build: { target: 'es2022' },
}))
