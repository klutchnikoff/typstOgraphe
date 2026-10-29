import { defineConfig } from 'vitest/config'
import { fileURLToPath } from 'node:url'
export default defineConfig({
  server: { fs: { allow: [fileURLToPath(new URL('../', import.meta.url))] } },
  test: {
    environment: 'jsdom',
    include: ['tests/**/*.test.ts'],
    setupFiles: ['tests/setup.ts'],
    // Unit tests only: graphical (Playwright) and contract recipes are not measured.
    coverage: {
      provider: 'v8',
      include: ['src/**/*.ts'],
      reporter: ['text-summary', 'html', 'json-summary'],
      reportsDirectory: '../output/coverage-web',
    },
  },
})
