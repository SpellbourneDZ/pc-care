import { defineConfig } from '@playwright/test'

export default defineConfig({
  testDir: './tests',
  outputDir: './test-results/browser',
  testMatch: [
    'browser.spec.ts',
    'cleanup-browser.spec.ts',
    'storage-browser.spec.ts',
  ],
  forbidOnly: !!process.env.CI,
  workers: 2,
  use: {
    channel: 'msedge',
    baseURL: 'http://127.0.0.1:1420',
    trace: 'retain-on-failure',
  },
  projects: [
    {
      name: 'light',
      use: { colorScheme: 'light', viewport: { width: 1280, height: 720 } },
    },
    {
      name: 'dark',
      use: { colorScheme: 'dark', viewport: { width: 1920, height: 1080 } },
    },
    {
      name: 'compact',
      use: { colorScheme: 'light', viewport: { width: 720, height: 520 } },
    },
    {
      name: 'scale-125',
      use: {
        colorScheme: 'light',
        viewport: { width: 1280, height: 720 },
        deviceScaleFactor: 1.25,
      },
    },
    {
      name: 'scale-150',
      use: {
        colorScheme: 'dark',
        viewport: { width: 1280, height: 720 },
        deviceScaleFactor: 1.5,
      },
    },
  ],
  webServer: {
    command: 'pnpm dev:web',
    url: 'http://127.0.0.1:1420',
    reuseExistingServer: false,
  },
})
