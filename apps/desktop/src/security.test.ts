import { expect, test } from 'vitest'
import config from '../src-tauri/tauri.conf.json'
import capability from '../src-tauri/capabilities/main.json'

test('foundation grants no system IPC and confines production content to local assets', () => {
  expect(config.app.security.capabilities).toEqual(['main'])
  expect(capability.windows).toEqual(['main'])
  expect(capability.permissions).toEqual([])
  expect(config.app.security.csp).toContain("default-src 'self'")
  expect(config.app.security.csp).toContain("object-src 'none'")
  expect(config.app.security.csp).not.toMatch(
    /unsafe-eval|unsafe-inline|https:|\*/,
  )
  expect(config.bundle.windows.nsis.installMode).toBe('currentUser')
  expect(config.bundle.active).toBe(false)
})
