import {
  render,
  renderHook,
  screen,
  within,
  waitFor,
  act,
} from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import type { ReactNode } from 'react'
import { useStartup } from './hooks/use-startup'
import userEvent from '@testing-library/user-event'
import { beforeEach, expect, test, vi } from 'vitest'
import { App } from './app'
import { operationApi } from './lib/operation-api'
import { ru } from './i18n/ru'
import type { StartupEntry } from './generated/StartupEntry'
vi.mock('./lib/operation-api', () => ({ operationApi: vi.fn() }))
const api = vi.mocked(operationApi)
const records: StartupEntry[] = [
  {
    id: 0,
    name: 'Искусственный Run',
    source: 'hkcu_run64',
    scope: 'user',
    path: 'HKEY_CURRENT_USER\\private-source',
    command: '"C:\\private-path\\test.exe" --secret argument',
    publisher: null,
    enabled: null,
    can_toggle: false,
    can_reveal: false,
  },
  {
    id: 1,
    name: 'Искусственный общий ярлык',
    source: 'common_folder',
    scope: 'machine',
    path: 'C:\\private-path\\artificial.lnk',
    command: null,
    publisher: null,
    enabled: null,
    can_toggle: false,
    can_reveal: true,
  },
]
let writes: ReturnType<typeof vi.fn>
beforeEach(() => {
  window.history.replaceState(null, '', '#/startup')
  api.mockReset()
  writes = vi.fn(async () => {})
  Object.defineProperty(navigator, 'clipboard', {
    configurable: true,
    value: { writeText: writes },
  })
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
            data: { rules: [], fixture_mode: true, max_plan_items: 500 },
          },
        }
      case 'list_volumes':
        return { status: 'ok', value: { type: 'volumes', data: [] } }
      case 'refresh_startup':
        return {
          status: 'ok',
          value: {
            type: 'startup_catalog',
            data: {
              id: 'inventory',
              total: records.length,
              issues: [],
              truncated: false,
              fixture_mode: true,
              collected_ms: 1600000000000,
            },
          },
        }
      case 'get_startup_page': {
        const q = request.data.query
        const all = records.filter(
          (e) =>
            (q.source === null || q.source === e.source) &&
            (q.scope === null || q.scope === e.scope) &&
            e.name.includes(q.search),
        )
        return {
          status: 'ok',
          value: {
            type: 'startup_page',
            data: {
              snapshot_id: 'inventory',
              total: all.length,
              offset: q.offset,
              entries: all.slice(q.offset, q.offset + q.limit),
            },
          },
        }
      }
      case 'open_startup_location':
        return { status: 'error', error: 'target_changed' }
      default:
        throw new Error(`Unexpected command ${request.type}`)
    }
  })
})
test('startup inventory shows honest unknown state, opens literal details and exports no commands or paths', async () => {
  const user = userEvent.setup()
  writes = vi.spyOn(navigator.clipboard, 'writeText').mockResolvedValue()
  render(<App />)
  const details = await screen.findByRole('button', {
    name: `${ru.startupUx.details}: ${records[0]!.name}`,
  })
  expect(screen.queryByRole('switch')).not.toBeInTheDocument()
  await user.click(details)
  const dialog = screen.getByRole('dialog')
  expect(dialog).toHaveTextContent(records[0]!.command!)
  expect(
    within(dialog).getByRole('button', { name: ru.storage.reveal }),
  ).toBeDisabled()
  await user.keyboard('{Escape}')
  expect(details).toHaveFocus()
  await user.click(screen.getByRole('button', { name: ru.startupUx.copy }))
  await waitFor(() => expect(writes).toHaveBeenCalledOnce())
  const copied = JSON.parse(writes.mock.calls[0]![0] as string)
  expect(copied.entries).toHaveLength(2)
  expect(copied.entries[0]).not.toHaveProperty('path')
  expect(copied.entries[0]).not.toHaveProperty('command')
  expect(copied.fixture_mode).toBe(true)
  expect(copied.entries[0].enabled).toBeNull()
  expect(
    api.mock.calls.every(([request]) =>
      [
        'get_preferences',
        'get_cleanup_rules',
        'list_volumes',
        'refresh_startup',
        'get_startup_page',
      ].includes(request.type),
    ),
  ).toBe(true)
})
test('location failures remain visible inside the sheet and clipboard rejection has a local error', async () => {
  const user = userEvent.setup()
  writes = vi.spyOn(navigator.clipboard, 'writeText').mockResolvedValue()
  render(<App />)
  await user.click(
    await screen.findByRole('button', {
      name: `${ru.startupUx.details}: ${records[1]!.name}`,
    }),
  )
  const dialog = screen.getByRole('dialog')
  await user.click(
    within(dialog).getByRole('button', { name: ru.storage.reveal }),
  )
  expect(await within(dialog).findByRole('alert')).toHaveTextContent(
    ru.cleanupUx.outdated,
  )
  await user.keyboard('{Escape}')
  await user.click(screen.getByRole('button', { name: ru.startupUx.refresh }))
  await screen.findByRole('button', {
    name: `${ru.startupUx.details}: ${records[0]!.name}`,
  })
  writes.mockRejectedValueOnce(new Error('clipboard denied'))
  await user.click(screen.getByRole('button', { name: ru.startupUx.copy }))
  expect(await screen.findByText(ru.startupUx.copyFailed)).toBeVisible()
})

test('a settings change marks the inventory stale until an explicit refresh', async () => {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  })
  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={client}>{children}</QueryClientProvider>
  )
  const { result, rerender } = renderHook(
    ({ revision }: { revision: string | null | undefined }) =>
      useStartup(true, revision),
    {
      wrapper,
      initialProps: { revision: undefined as string | null | undefined },
    },
  )
  expect(api).not.toHaveBeenCalled()
  rerender({ revision: null })
  await waitFor(() => expect(result.current.page.data?.total).toBe(2))
  expect(result.current.stale).toBe(false)
  rerender({ revision: 'new-settings' })
  expect(result.current.stale).toBe(true)
  expect(
    api.mock.calls.filter(([r]) => r.type === 'refresh_startup'),
  ).toHaveLength(1)
  act(() => result.current.refresh())
  await waitFor(() => expect(result.current.stale).toBe(false))
  expect(
    api.mock.calls.filter(([r]) => r.type === 'refresh_startup'),
  ).toHaveLength(2)
})
