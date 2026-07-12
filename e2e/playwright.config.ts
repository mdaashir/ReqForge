import { defineConfig } from '@playwright/test'

export default defineConfig({
  testDir: './tests',
  timeout: 60000,
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  workers: 1,
  reporter: process.env.CI ? 'github' : 'list',
  use: {
    // Headless by default; CI will use `xvfb-run` if needed.
    headless: true,
    screenshot: 'only-on-failure',
    trace: 'retain-on-failure',
  },
  projects: [
    {
      name: 'tauri',
      testMatch: '**/*.spec.ts',
      use: {
        // Tauri WebDriver can talk to the app via a WebSocket.
        // The `TAURI_DEV_HOST` env var or a running Tauri app is expected.
        baseURL: process.env.TAURI_DEV_HOST || 'http://localhost:1420',
      },
    },
  ],
})
