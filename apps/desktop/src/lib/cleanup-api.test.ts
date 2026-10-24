import { beforeEach, expect, test, vi } from 'vitest'
import {
  byteCount,
  displayPath,
  formatCount,
  mergeProgress,
  readCleanupSnapshot,
} from './cleanup-api'
import { operationApi } from './operation-api'
import { candidate, ok, resultPage } from '../../tests/cleanup-fixtures'
import type { TaskProgress } from '@/generated/TaskProgress'

vi.mock('./operation-api', () => ({ operationApi: vi.fn() }))
const api = vi.mocked(operationApi)
beforeEach(() => api.mockReset())
test('byte counts preserve u64 precision and reject malformed wire values', () => {
  expect(byteCount('18446744073709551615')).toBe(18_446_744_073_709_551_615n)
  for (const invalid of [
    '18446744073709551616',
    '-1',
    '01',
    '1.2',
    ' 1',
    '1e3',
  ])
    expect(() => byteCount(invalid)).toThrow()
  expect(formatCount('4096')).toBe('4,0 КиБ')
  expect(formatCount('18446744073709551615')).toBe('16,0 ЭиБ')
  expect(displayPath('a\u202etxt')).toBe('a\\u202etxt')
})
test('terminal progress is sticky and old polling cannot move progress backwards', () => {
  const running: TaskProgress = {
    task_id: 'task',
    status: 'running',
    processed: 10,
    total: null,
    scan_id: null,
    error: null,
  }
  expect(mergeProgress(running, { ...running, processed: 2 })).toBe(running)
  const done = { ...running, status: 'completed', scan_id: 'scan' } as const
  expect(mergeProgress(done, running)).toBe(done)
  expect(mergeProgress(running, done)).toBe(done)
})
test('snapshot paging checks provenance, stable offsets, duplicates and cancellation', async () => {
  const first = { ...resultPage([candidate('one')]), total: 2 }
  api.mockResolvedValueOnce(ok({ type: 'scan_result', data: first }))
  api.mockResolvedValueOnce(
    ok({
      type: 'scan_result',
      data: { ...resultPage([candidate('two')]), offset: 1, total: 2 },
    }),
  )
  expect(
    (await readCleanupSnapshot('scan', new AbortController().signal))
      .candidates,
  ).toHaveLength(2)
  for (const bad of [
    {
      ...first,
      total: 1,
      candidates: [{ ...candidate('one'), source: 'mock' as const }],
    },
    { ...first, offset: 1 },
    { ...first, scan_id: 'other' },
    { ...first, total: 10_001 },
    resultPage([candidate('same'), candidate('same')]),
  ]) {
    api.mockReset()
    api.mockResolvedValue(ok({ type: 'scan_result', data: bad }))
    await expect(
      readCleanupSnapshot('scan', new AbortController().signal),
    ).rejects.toThrow()
  }
  api.mockReset()
  api.mockResolvedValue(ok({ type: 'scan_result', data: first }))
  const abort = new AbortController()
  abort.abort()
  await expect(readCleanupSnapshot('scan', abort.signal)).rejects.toThrow()
})
