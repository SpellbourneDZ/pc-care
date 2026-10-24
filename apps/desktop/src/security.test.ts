import { expect, test } from 'vitest'
import config from '../src-tauri/tauri.conf.json'
import capability from '../src-tauri/capabilities/main.json'
import draft from '../src-tauri/tauri.ci.conf.json'

test('only typed operation IPC is granted and production content stays local', () => {
  expect(config.app.security.capabilities).toEqual(['main'])
  expect(capability.windows).toEqual(['main'])
  expect(capability.permissions).toEqual(['allow-operation-api'])
  expect(config.app.security.csp).toContain("default-src 'self'")
  expect(config.app.security.csp).toContain("object-src 'none'")
  expect(config.app.security.csp).not.toMatch(
    /unsafe-eval|unsafe-inline|https:|\*/,
  )
  expect(config.bundle.windows.nsis.installMode).toBe('currentUser')
  expect(config.bundle.active).toBe(false)
})

test('draft bundling preserves app security and uses only per-user NSIS without runtime installation', () => {
  expect(draft).not.toHaveProperty('app')
  expect(draft).not.toHaveProperty('build')
  expect(draft.bundle.targets).toEqual(['nsis'])
  expect(draft.bundle.windows.nsis.installMode).toBe('currentUser')
  expect(draft.bundle.windows.webviewInstallMode.type).toBe('skip')
})
