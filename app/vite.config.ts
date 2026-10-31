import { defineConfig } from 'vite'
import { fileURLToPath } from 'node:url'
import { resolve } from 'node:path'
import { browserDriver } from './browser-driver.ts'
import { watchExternalAssets } from './scripts/watch-external-assets.ts'
const root = fileURLToPath(new URL('.', import.meta.url))
export default defineConfig(({ mode }) => ({
  plugins: [
    watchExternalAssets(resolve(root, '../assets/ui/icons')),
    ...(mode === 'browser-test' ? [browserDriver(root)] : []),
  ],
  server: { fs: { allow: [resolve(root, '..')] } },
  build: { target: 'es2022', outDir: mode === 'recipe' ? 'dist-recipe' : 'dist' },
}))
