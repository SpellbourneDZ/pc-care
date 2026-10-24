import { defineConfig } from '@playwright/test'

export default defineConfig({
  testDir: './tests',
  testMatch: 'settings-native.spec.ts',
  forbidOnly: !!process.env.CI,
  workers: 1,
  timeout: 90_000,
})
