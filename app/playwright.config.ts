import { defineConfig } from '@playwright/test'
// Port distinct de l'application : ne jamais réutiliser sa session de rédaction.
const port = Number(process.env.TOG_BROWSER_PORT || 1421)
const baseURL = `http://127.0.0.1:${port}`
export default defineConfig({
  testDir: 'tests/browser',
  fullyParallel: false,
  workers: 1,
  timeout: 60_000,
  use: {
    baseURL,
    viewport: { width: 1280, height: 900 },
    trace: 'off',
    screenshot: 'off',
    video: 'off',
  },
  projects: [
    { name: 'chromium', use: { browserName: 'chromium' } },
    { name: 'webkit', use: { browserName: 'webkit' } },
  ],
  webServer: {
    command: `npm run dev -- --mode browser-test --port ${port}`,
    url: baseURL,
    reuseExistingServer: false,
    timeout: 30_000,
  },
})
