import { afterEach, expect, test, vi } from 'vitest'
import { Channel } from '@tauri-apps/api/core'
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks'
import type { ApiReply } from '@/generated/ApiReply'
import type { TaskProgress } from '@/generated/TaskProgress'
import { operationApi } from './operation-api'

afterEach(() => {
  clearMocks()
  vi.useRealTimers()
})

const taskId = '11111111-1111-4111-8111-111111111111'

test('sends typed requests and delivers ordered progress on its private channel', async () => {
  const events: TaskProgress[] = []
  mockIPC((command, payload) => {
    expect(command).toBe('operation_api')
    expect(payload).toMatchObject({
      request: { type: 'start_scan', data: { kind: 'mock' } },
    })
    if (
      !payload ||
      typeof payload !== 'object' ||
      !('progress' in payload) ||
      !(payload.progress instanceof Channel)
    ) {
      throw new Error('Missing private channel')
    }
    const internals = (
      window as unknown as {
        __TAURI_INTERNALS__: { runCallback(id: number, data: unknown): void }
      }
    ).__TAURI_INTERNALS__
    const progress: TaskProgress = {
      task_id: taskId,
      status: 'completed',
      processed: 3,
      total: 3,
      scan_id: null,
      error: null,
    }
    internals.runCallback(payload.progress.id, { index: 0, message: progress })
    internals.runCallback(payload.progress.id, { index: 1, end: true })
    return {
      status: 'ok',
      value: { type: 'scan_started', data: { task_id: taskId } },
    } satisfies ApiReply
  })
  const result = await operationApi(
    {
      type: 'start_scan',
      data: { kind: 'mock', scenario: 'complete', timeout_ms: 1_000 },
    },
    (event) => events.push(event),
  )
  expect(result.status).toBe('ok')
  expect(events).toHaveLength(1)
  expect(events[0]?.status).toBe('completed')
})

test('preserves domain errors and hides untrusted transport rejection text', async () => {
  mockIPC(() => ({ status: 'error', error: 'not_found' }) satisfies ApiReply)
  expect(
    await operationApi({ type: 'get_task', data: { task_id: taskId } }),
  ).toEqual({ status: 'error', error: 'not_found' })
  mockIPC(() => {
    throw new Error('private path or native details')
  })
  expect(
    await operationApi({ type: 'get_task', data: { task_id: taskId } }),
  ).toEqual({ status: 'error', error: 'transport' })
})

test('bounds an unresponsive IPC call without automatic retries', async () => {
  vi.useFakeTimers()
  const handler = vi.fn(() => new Promise<never>(() => {}))
  mockIPC(handler)
  const result = operationApi({
    type: 'cancel_task',
    data: { task_id: taskId },
  })
  await vi.advanceTimersByTimeAsync(5_000)
  expect(await result).toEqual({ status: 'error', error: 'timeout' })
  expect(handler).toHaveBeenCalledTimes(1)
  expect(vi.getTimerCount()).toBe(0)
})

test('returns a transport error when opened outside Tauri', async () => {
  clearMocks()
  expect(
    await operationApi({ type: 'get_task', data: { task_id: taskId } }),
  ).toEqual({ status: 'error', error: 'transport' })
})
