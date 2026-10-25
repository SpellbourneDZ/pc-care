import { act, render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, expect, test, vi } from 'vitest'
import { App } from './app'
import { ru } from './i18n/ru'
import { operationApi } from './lib/operation-api'
import {
  candidate,
  catalog,
  ok,
  operationPlan,
  resultPage,
} from '../tests/cleanup-fixtures'
import type { CandidatePage } from './generated/CandidatePage'
import type { TaskProgress } from './generated/TaskProgress'
import type { ApiReply } from './generated/ApiReply'
import { CleanupPreview } from './components/cleanup-preview'

vi.mock('./lib/operation-api', () => ({ operationApi: vi.fn() }))
const api = vi.mocked(operationApi)
let page: CandidatePage
let task: TaskProgress
let revision: string | null
beforeEach(() => {
  window.history.replaceState(null, '', '#/cleanup')
  api.mockReset()
  revision = null
  page = resultPage()
  task = {
    task_id: 'task',
    status: 'completed',
    processed: 2,
    total: null,
    scan_id: 'scan',
    error: null,
  }
  api.mockImplementation(async (request, progress) => {
    switch (request.type) {
      case 'get_preferences':
        return ok({
          type: 'preferences',
          data: {
            schema_version: 1,
            revision,
            preferences: { theme: 'system', language: 'ru', exclusions: [] },
          },
        })
      case 'save_preferences':
        return ok({
          type: 'preferences',
          data: {
            schema_version: 1,
            revision,
            preferences: request.data.preferences,
          },
        })
      case 'get_cleanup_rules':
        return ok({ type: 'cleanup_rules', data: catalog })
      case 'start_scan':
        progress?.(task)
        return ok({ type: 'scan_started', data: { task_id: 'task' } })
      case 'get_task':
        return ok({ type: 'task', data: task })
      case 'get_scan_result':
        return ok({
          type: 'scan_result',
          data: {
            ...page,
            offset: request.data.offset,
            candidates: page.candidates.slice(
              request.data.offset,
              request.data.offset + request.data.limit,
            ),
          },
        })
      case 'build_operation_plan':
        return ok({
          type: 'operation_plan',
          data: operationPlan(
            page.candidates.filter((item) =>
              request.data.selected_candidate_ids.includes(item.id),
            ),
          ),
        })
      case 'cancel_task':
        task = { ...task, status: 'cancelled', scan_id: null }
        return ok({ type: 'task', data: task })
      default:
        throw new Error(`Unexpected mutation: ${request.type}`)
    }
  })
})
async function scan() {
  const user = userEvent.setup()
  render(<App />)
  const start = await screen.findByRole('button', { name: ru.cleanupUx.scan })
  await waitFor(() => expect(start).toBeEnabled())
  await user.click(start)
  return user
}
test('real flow selects Safe only, keeps review manual, previews exact IDs without execute and survives navigation', async () => {
  const user = await scan()
  expect(await screen.findByTestId('cleanup-selection')).toHaveTextContent('1')
  const review = screen.getByRole('checkbox', {
    name: `${ru.selected}: ${catalog.rules[1]!.name}`,
  })
  expect(review).not.toBeChecked()
  expect(
    screen.getByRole('checkbox', { name: ru.cleanupUx.protected }),
  ).toBeDisabled()
  await user.click(review)
  expect(screen.getByTestId('cleanup-selection')).toHaveTextContent('2')
  await user.click(screen.getByRole('link', { name: ru.apps }))
  await user.click(screen.getByRole('link', { name: ru.cleanup }))
  expect(await screen.findByTestId('cleanup-selection')).toHaveTextContent('2')
  const preview = screen.getByRole('button', { name: ru.cleanupUx.preview })
  await user.click(preview)
  const dialog = await screen.findByRole('alertdialog')
  expect(dialog).toHaveTextContent('8,0 КиБ')
  expect(dialog).toHaveTextContent(ru.cleanupUx.previewOnly)
  expect(
    within(dialog).getByRole('button', { name: ru.cleanupUx.confirm }),
  ).toBeDisabled()
  await user.click(within(dialog).getByRole('button', { name: ru.cancel }))
  expect(preview).toHaveFocus()
  await user.click(preview)
  await user.click(
    await screen.findByRole('button', { name: ru.cleanupUx.finishPreview }),
  )
  expect(
    await screen.findByRole('heading', { name: ru.cleanupUx.previewDone }),
  ).toBeVisible()
  expect(screen.getByText(ru.cleanupUx.previewDoneDescription)).toBeVisible()
  expect(
    api.mock.calls.some(([request]) => request.type === 'execute_plan'),
  ).toBe(false)
})
test('partial and truncated results disable automatic choice; changes to settings require a new scan', async () => {
  page = {
    ...page,
    truncated: true,
    issues: [
      {
        rule_id: 'temp',
        reason: 'locked',
        count: 2,
        sample_path: 'C:\\fixture\\locked.tmp',
      },
    ],
  }
  const user = await scan()
  expect(await screen.findByTestId('cleanup-selection')).toHaveTextContent('0')
  expect(screen.getByText(ru.cleanupUx.truncated)).toBeVisible()
  await user.click(
    screen.getByRole('checkbox', {
      name: `${ru.selected}: ${catalog.rules[0]!.name}`,
    }),
  )
  expect(screen.getByTestId('cleanup-selection')).toHaveTextContent('1')
  revision = 'changed'
  await user.click(screen.getByRole('button', { name: ru.settings }))
  await user.type(
    screen.getByRole('textbox', { name: ru.exclusionPath }),
    'C:\\fixture\\excluded',
  )
  await user.click(screen.getByRole('button', { name: ru.add }))
  await screen.findByText(ru.settingsSaved)
  await user.keyboard('{Escape}')
  expect(await screen.findByText(ru.cleanupUx.stale)).toBeVisible()
  expect(
    screen.getByRole('button', { name: ru.cleanupUx.preview }),
  ).toBeDisabled()
  expect(screen.getByTestId('cleanup-selection')).toHaveTextContent('0')
})
test('selection cap, drawer filtering and category filters preserve full totals', async () => {
  page = resultPage(
    Array.from({ length: 501 }, (_, index) => candidate(`safe-${index}`)),
  )
  const user = await scan()
  expect(await screen.findByTestId('cleanup-selection')).toHaveTextContent(
    '500',
  )
  expect(screen.getByText(`${ru.cleanupUx.limit}: 500.`)).toBeVisible()
  await user.click(
    screen.getByRole('button', {
      name: `${ru.cleanupUx.details}: ${catalog.rules[0]!.name}`,
    }),
  )
  const dialog = await screen.findByRole('dialog')
  await user.type(within(dialog).getByRole('textbox'), 'safe-500.tmp')
  const last = within(dialog).getByRole('checkbox')
  expect(last).not.toBeChecked()
  await user.click(last)
  expect(last).not.toBeChecked()
  await user.keyboard('{Escape}')
  await user.type(
    screen.getByRole('textbox', { name: ru.cleanupUx.filter }),
    'no-matching-category',
  )
  expect(screen.getByText(ru.cleanupUx.noMatches)).toBeVisible()
  expect(screen.getByTestId('cleanup-selection')).toHaveTextContent('500')
})
test('progress has no fake percentage and cancel waits for acknowledgement', async () => {
  task = { ...task, status: 'running', scan_id: null, processed: 12 }
  const user = await scan()
  expect(await screen.findByRole('progressbar')).not.toHaveAttribute(
    'aria-valuenow',
  )
  const original = api.getMockImplementation()!
  let acknowledge!: (value: ApiReply) => void
  api.mockImplementation((request, progress) =>
    request.type === 'cancel_task'
      ? new Promise((resolve) => {
          acknowledge = resolve
        })
      : original(request, progress),
  )
  await user.click(screen.getByRole('button', { name: ru.cleanupUx.cancel }))
  expect(screen.getByRole('progressbar')).toBeVisible()
  task = { ...task, status: 'cancelled' }
  await act(async () => acknowledge(ok({ type: 'task', data: task })))
  expect(
    await screen.findByRole('heading', { name: ru.cleanupUx.cancelled }),
  ).toBeVisible()
  expect(screen.queryByRole('table')).not.toBeInTheDocument()
})
test('empty findings and typed failure remain recoverable without retrying mutations', async () => {
  page = resultPage([])
  const user = await scan()
  expect(
    await screen.findByRole('heading', { name: ru.cleanupUx.noFindings }),
  ).toBeVisible()
  const original = api.getMockImplementation()!
  api.mockImplementation((request, progress) =>
    request.type === 'start_scan'
      ? Promise.resolve({ status: 'error', error: 'adapter_error' })
      : original(request, progress),
  )
  await user.click(screen.getByRole('button', { name: ru.cleanupUx.rescan }))
  expect(await screen.findByRole('alert')).toHaveTextContent(
    ru.cleanupUx.unavailable,
  )
  expect(
    api.mock.calls.filter(([request]) => request.type === 'start_scan'),
  ).toHaveLength(2)
  api.mockImplementation(original)
  await user.click(screen.getByRole('button', { name: ru.cleanupUx.scan }))
  await waitFor(() =>
    expect(screen.queryByRole('alert')).not.toBeInTheDocument(),
  )
  expect(
    await screen.findByRole('heading', { name: ru.cleanupUx.noFindings }),
  ).toBeVisible()
  expect(
    api.mock.calls.filter(([request]) => request.type === 'start_scan'),
  ).toHaveLength(3)
})
test('expired preview requires a fresh plan and never confirms', async () => {
  const refresh = vi.fn(),
    confirm = vi.fn()
  const user = userEvent.setup()
  render(
    <CleanupPreview
      plan={operationPlan([candidate('safe')])}
      deadline={Date.now() - 1}
      fixture
      busy={false}
      stale={false}
      onClose={() => {}}
      onReturnFocus={() => {}}
      onRefresh={refresh}
      onConfirm={confirm}
    />,
  )
  expect(screen.getByRole('alert')).toHaveTextContent(ru.cleanupUx.expired)
  expect(
    screen.queryByRole('button', { name: ru.cleanupUx.finishPreview }),
  ).not.toBeInTheDocument()
  await user.click(screen.getByRole('button', { name: ru.cleanupUx.preview }))
  expect(refresh).toHaveBeenCalledOnce()
  expect(confirm).not.toHaveBeenCalled()
})

test.each(['foreign-id', 'real-effect'] as const)(
  'unexpected plan %s never reaches confirmation',
  async (mode) => {
    const user = await scan()
    await screen.findByTestId('cleanup-selection')
    const original = api.getMockImplementation()!
    api.mockImplementation((request, progress) =>
      request.type === 'build_operation_plan'
        ? Promise.resolve(
            ok({
              type: 'operation_plan',
              data:
                mode === 'real-effect'
                  ? { ...operationPlan([candidate('safe')]), simulated: false }
                  : operationPlan([candidate('foreign')]),
            }),
          )
        : original(request, progress),
    )
    await user.click(screen.getByRole('button', { name: ru.cleanupUx.preview }))
    expect(await screen.findByRole('alert')).toHaveTextContent(
      mode === 'real-effect'
        ? ru.cleanupUx.unsupported
        : ru.cleanupUx.unavailable,
    )
    expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument()
    expect(
      api.mock.calls.some(([request]) => request.type === 'execute_plan'),
    ).toBe(false)
  },
)
