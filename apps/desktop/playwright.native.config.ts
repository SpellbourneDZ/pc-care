import { defineConfig } from '@playwright/test'

export default defineConfig({
  testDir: './tests',
  outputDir: './test-results/native',
  testMatch: 'native.spec.ts',
  forbidOnly: !!process.env.CI,
  workers: 1,
  timeout: 60_000,
})
