import { defineConfig } from 'vitest/config'
export default defineConfig({
  server: { fs: { allow: [new URL('../../', import.meta.url).pathname] } },
  test: { environment: 'jsdom', include: ['tests/**/*.test.ts'], setupFiles: ['tests/setup.ts'] },
})
