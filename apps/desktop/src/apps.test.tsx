import { render, screen, within, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, expect, test, vi } from 'vitest'
import { App } from './app'
import { operationApi } from './lib/operation-api'
import type { InstalledApp } from './generated/InstalledApp'
import { ru } from './i18n/ru'
vi.mock('./lib/operation-api', () => ({ operationApi: vi.fn() }))
const api = vi.mocked(operationApi),
  u = ru.appsUx
const records: InstalledApp[] = [
  {
    id: 'hkcu64:one',
    name: 'Одинаковое имя',
    publisher: 'Искусственный издатель',
    version: '1.0',
    kind: 'msi',
    source: 'hkcu64',
    system_component: false,
    estimated_bytes: '9007199254740993',
    registered_date: '20261025',
    install_location: 'C:\\fixture\\One',
    registration: 'one',
    uninstall_block: 'not_validated',
  },
  {
    id: 'msix:two',
    name: 'Одинаковое имя',
    publisher: null,
    version: null,
    kind: 'msix',
    source: 'msix',
    system_component: true,
    estimated_bytes: null,
    registered_date: null,
    install_location: null,
    registration: 'two',
    uninstall_block: 'protected',
  },
]
let checkMode: 'match' | 'foreign' | 'error' = 'match'
beforeEach(() => {
  window.history.replaceState(null, '', '#/apps')
  api.mockReset()
  checkMode = 'match'
  api.mockImplementation(async (request) => {
    switch (request.type) {
      case 'get_preferences':
        return {
          status: 'ok',
          value: {
            type: 'preferences',
            data: {
              schema_version: 1,
              revision: null,
              preferences: { theme: 'system', language: 'ru', exclusions: [] },
            },
          },
        }
      case 'get_cleanup_rules':
        return {
          status: 'ok',
          value: {
            type: 'cleanup_rules',
            data: { rules: [], max_plan_items: 500, fixture_mode: true },
          },
        }
      case 'list_volumes':
        return { status: 'ok', value: { type: 'volumes', data: [] } }
      case 'refresh_apps':
        return {
          status: 'ok',
          value: {
            type: 'apps_catalog',
            data: {
              id: 'apps',
              total: 2,
              issues: [],
              truncated: false,
              fixture_mode: true,
              collected_ms: 1600000000000,
            },
          },
        }
      case 'get_apps_page': {
        const q = request.data.query,
          rows = records.filter(
            (e) =>
              (!q.kind || q.kind === e.kind) &&
              (!q.source || q.source === e.source) &&
              e.name.includes(q.search),
          )
        return {
          status: 'ok',
          value: {
            type: 'apps_page',
            data: {
              snapshot_id: 'apps',
              total: rows.length,
              offset: q.offset,
              entries: rows.slice(q.offset, q.offset + q.limit),
            },
          },
        }
      }
      case 'check_uninstall':
        if (checkMode === 'error')
          return { status: 'error', error: 'target_changed' }
        return {
          status: 'ok',
          value: {
            type: 'uninstall_check',
            data: {
              snapshot_id: 'apps',
              app_id:
                checkMode === 'foreign' ? 'other-app' : request.data.app_id,
              status: 'msi_matched',
              execution_enabled: false,
              binary: 'fixture',
              preview: {
                product_code: '{62E06E1F-50AF-4C56-A5C8-2C8E2FBEA775}',
                context: 'user_unmanaged',
                arguments: [
                  '/x',
                  '{62E06E1F-50AF-4C56-A5C8-2C8E2FBEA775}',
                  '/norestart',
                ],
              },
            },
          },
        }
      default:
        throw new Error(`Unexpected command ${request.type}`)
    }
  })
})

test('MSI check shows a read-only match and clears it when changing the selected app', async () => {
  const user = userEvent.setup()
  render(<App />)
  await user.click(
    (
      await screen.findAllByRole('button', {
        name: `${u.details}: Одинаковое имя`,
      })
    )[0]!,
  )
  const dialog = screen.getByRole('dialog')
  await user.click(within(dialog).getByRole('button', { name: u.check }))
  expect(await within(dialog).findByRole('status')).toHaveTextContent(
    u.checkStatuses.msi_matched,
  )
  expect(within(dialog).getByRole('status')).toHaveTextContent(
    u.binaryStatuses.fixture,
  )
  expect(
    within(dialog).getByRole('button', { name: u.uninstall }),
  ).toBeDisabled()
  await user.keyboard('{Escape}')
  await user.click(
    screen.getAllByRole('button', { name: `${u.details}: Одинаковое имя` })[1]!,
  )
  expect(
    within(screen.getByRole('dialog')).queryByText(u.checkStatuses.msi_matched),
  ).not.toBeInTheDocument()
  expect(
    within(screen.getByRole('dialog')).queryByRole('button', { name: u.check }),
  ).not.toBeInTheDocument()
  expect(
    api.mock.calls.filter(([r]) => r.type === 'check_uninstall'),
  ).toHaveLength(1)
})
test('MSI check rejects a response for another app and keeps backend errors inside the sheet', async () => {
  const user = userEvent.setup()
  checkMode = 'foreign'
  render(<App />)
  await user.click(
    (
      await screen.findAllByRole('button', {
        name: `${u.details}: Одинаковое имя`,
      })
    )[0]!,
  )
  const dialog = screen.getByRole('dialog')
  await user.click(within(dialog).getByRole('button', { name: u.check }))
  expect(await within(dialog).findByRole('alert')).toHaveTextContent(
    ru.cleanupUx.outdated,
  )
  expect(
    within(dialog).queryByText(u.checkStatuses.msi_matched),
  ).not.toBeInTheDocument()
  checkMode = 'error'
  await user.click(within(dialog).getByRole('button', { name: u.check }))
  expect(await within(dialog).findByRole('alert')).toHaveTextContent(
    ru.cleanupUx.outdated,
  )
})
test('same-name apps remain distinct; unknown size and disabled actions are explicit; sheet restores focus', async () => {
  const user = userEvent.setup()
  render(<App />)
  await waitFor(() =>
    expect(
      screen.getAllByRole('button', { name: `${u.details}: Одинаковое имя` }),
    ).toHaveLength(2),
  )
  const details = screen.getAllByRole('button', {
    name: `${u.details}: Одинаковое имя`,
  })[1]!
  await user.click(details)
  const dialog = screen.getByRole('dialog')
  expect(dialog).toHaveTextContent('msix:two')
  expect(
    within(dialog).getByRole('button', { name: u.uninstall }),
  ).toBeDisabled()
  expect(
    within(dialog).getByRole('button', { name: u.leftovers }),
  ).toBeDisabled()
  expect(dialog).toHaveTextContent(u.blocked.protected)
  await user.keyboard('{Escape}')
  expect(details).toHaveFocus()
  await user.type(
    screen.getByRole('textbox', { name: u.search }),
    'нет совпадений',
  )
  expect(await screen.findByText(u.noMatches)).toBeVisible()
  expect(
    api.mock.calls.every(([r]) =>
      [
        'get_preferences',
        'get_cleanup_rules',
        'list_volumes',
        'refresh_apps',
        'get_apps_page',
      ].includes(r.type),
    ),
  ).toBe(true)
})
