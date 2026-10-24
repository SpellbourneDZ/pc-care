import { act, render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, expect, test, vi } from 'vitest'
import { App } from './app'
import { ru } from './i18n/ru'
import { operationApi } from './lib/operation-api'
import type { ApiReply } from './generated/ApiReply'
import { parseRoute } from './lib/routes'

vi.mock('./lib/operation-api', () => ({ operationApi: vi.fn() }))
const api = vi.mocked(operationApi)
const snapshot = {
  schema_version: 1,
  revision: null,
  preferences: { theme: 'system', language: 'ru', exclusions: [] },
} as const
const loaded = (): ApiReply => ({
  status: 'ok',
  value: {
    type: 'preferences',
    data: {
      ...snapshot,
      preferences: { ...snapshot.preferences, exclusions: [] },
    },
  },
})
beforeEach(() => {
  window.history.replaceState(null, '', '#/overview')
  api.mockReset()
  api.mockResolvedValue(loaded())
})
test('closed routes fall back safely and live shell never starts a mock scan', async () => {
  expect(parseRoute('#/anything')).toEqual({ page: 'overview', preview: false })
  expect(parseRoute('#/preview/disks')).toEqual({
    page: 'disks',
    preview: true,
  })
  const user = userEvent.setup()
  render(<App />)
  await waitFor(() => expect(api).toHaveBeenCalledTimes(2))
  expect(screen.queryByText(ru.exampleData)).not.toBeInTheDocument()
  expect(screen.getByRole('button', { name: ru.findJunk })).toBeDisabled()
  await user.click(screen.getByRole('link', { name: ru.apps }))
  expect(
    await screen.findByRole('heading', { name: ru.apps, level: 1 }),
  ).toBeVisible()
  expect(
    api.mock.calls.every(([request]) =>
      ['get_preferences', 'get_cleanup_rules'].includes(request.type),
    ),
  ).toBe(true)
})
test('settings show loading, wait for backend acknowledgement and preserve failed input', async () => {
  let resolveLoad!: (reply: ApiReply) => void
  api.mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        resolveLoad = resolve
      }),
  )
  const user = userEvent.setup()
  render(<App />)
  await user.click(screen.getByRole('button', { name: ru.settings }))
  const dialog = screen.getByRole('dialog', { name: ru.settings })
  expect(within(dialog).getByText(ru.settingsLoading)).toBeVisible()
  await act(async () => resolveLoad(loaded()))
  const input = await within(dialog).findByRole('textbox', {
    name: ru.exclusionPath,
  })
  await user.type(input, 'C:\\example')
  let resolveSave!: (reply: ApiReply) => void
  api.mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        resolveSave = resolve
      }),
  )
  await user.click(within(dialog).getByRole('button', { name: ru.add }))
  expect(input).toBeDisabled()
  expect(
    within(dialog).queryByRole('button', {
      name: `${ru.removeExclusion}: C:\\example`,
    }),
  ).not.toBeInTheDocument()
  await act(async () =>
    resolveSave({ status: 'error', error: 'invalid_request' }),
  )
  expect(await within(dialog).findByRole('alert')).toHaveTextContent(
    ru.exclusionInvalid,
  )
  expect(input).toHaveValue('C:\\example')
  expect(api.mock.calls.at(-1)?.[0]).toEqual({
    type: 'save_preferences',
    data: {
      expected_revision: null,
      preferences: {
        theme: 'system',
        language: 'ru',
        exclusions: ['C:\\example'],
      },
    },
  })
})
test('storage failure and conflict stay visible without automatic retry', async () => {
  api.mockResolvedValueOnce({ status: 'error', error: 'settings_invalid' })
  const user = userEvent.setup()
  render(<App />)
  await user.click(screen.getByRole('button', { name: ru.settings }))
  const dialog = screen.getByRole('dialog', { name: ru.settings })
  expect(await within(dialog).findByRole('alert')).toHaveTextContent(
    ru.settingsInvalid,
  )
  expect(within(dialog).queryByRole('textbox')).not.toBeInTheDocument()
  expect(api).toHaveBeenCalledTimes(2)
  await user.click(
    within(dialog).getByRole('button', { name: ru.reloadSettings }),
  )
  const theme = await within(dialog).findByRole('combobox', { name: ru.theme })
  api.mockResolvedValueOnce({ status: 'error', error: 'settings_conflict' })
  await user.click(theme)
  await user.click(screen.getByRole('option', { name: ru.themes.dark }))
  expect(await within(dialog).findByRole('alert')).toHaveTextContent(
    ru.settingsConflict,
  )
  expect(document.documentElement).not.toHaveClass('dark')
  expect(api).toHaveBeenCalledTimes(4)
})
