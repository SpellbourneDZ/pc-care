import type { ApiError } from '@/generated/ApiError'
import type { ApiRequest } from '@/generated/ApiRequest'
import type { ApiResponse } from '@/generated/ApiResponse'
import type { CandidatePage } from '@/generated/CandidatePage'
import type { TaskProgress } from '@/generated/TaskProgress'
import { operationApi } from './operation-api'
import { ru } from '@/i18n/ru'

export class CleanupError extends Error {
  constructor(readonly code: ApiError) {
    super(code)
  }
}

export function cleanupErrorMessage(error: unknown): string {
  const code = error instanceof Error && 'code' in error ? error.code : null
  switch (code) {
    case 'transport':
      return ru.cleanupUx.desktopOnly
    case 'capacity':
      return ru.cleanupUx.busy
    case 'timeout':
      return ru.cleanupUx.timeout
    case 'not_found':
    case 'not_ready':
    case 'plan_expired':
    case 'target_changed':
      return ru.cleanupUx.outdated
    case 'unsupported':
      return ru.cleanupUx.unsupported
    case 'settings_invalid':
      return ru.settingsInvalid
    case 'settings_unavailable':
      return ru.settingsUnavailable
    case 'settings_conflict':
      return ru.settingsConflict
    default:
      return ru.cleanupUx.unavailable
  }
}

export function displayPath(path: string): string {
  return path.replace(
    /[\u202a-\u202e\u2066-\u2069]/g,
    (value) => `\\u${value.charCodeAt(0).toString(16).padStart(4, '0')}`,
  )
}

type ResponseData = {
  [K in ApiResponse['type']]: Extract<ApiResponse, { type: K }>['data']
}
export async function cleanupRequest<T extends keyof ResponseData>(
  request: ApiRequest,
  type: T,
  onProgress?: (value: TaskProgress) => void,
): Promise<ResponseData[T]> {
  const reply = await operationApi(request, onProgress)
  if (reply.status === 'error') throw new CleanupError(reply.error)
  if (reply.value.type !== type) throw new CleanupError('internal')
  return reply.value.data as ResponseData[T]
}

export function byteCount(value: string): bigint {
  if (!/^(0|[1-9]\d{0,19})$/.test(value)) throw new CleanupError('internal')
  const bytes = BigInt(value)
  if (bytes > 18_446_744_073_709_551_615n) throw new CleanupError('internal')
  return bytes
}

export function formatCount(value: string | bigint): string {
  const bytes = typeof value === 'string' ? byteCount(value) : value
  const units = ['Б', 'КиБ', 'МиБ', 'ГиБ', 'ТиБ', 'ПиБ', 'ЭиБ']
  let unit = 1n,
    index = 0
  while (bytes >= unit * 1024n && index < units.length - 1) {
    unit *= 1024n
    index++
  }
  if (!index) return `${bytes.toLocaleString('ru-RU')} ${units[index]}`
  const tenths = (bytes * 10n + unit / 2n) / unit
  return `${(tenths / 10n).toLocaleString('ru-RU')},${tenths % 10n} ${units[index]}`
}

export function mergeProgress(
  current: TaskProgress | undefined,
  incoming: TaskProgress,
): TaskProgress {
  if (
    current?.task_id === incoming.task_id &&
    (current.status !== 'running' ||
      (incoming.status === 'running' && current.processed > incoming.processed))
  )
    return current
  return incoming
}

export async function readCleanupSnapshot(
  scanId: string,
  signal: AbortSignal,
): Promise<CandidatePage> {
  const first = await cleanupRequest(
    {
      type: 'get_scan_result',
      data: { scan_id: scanId, offset: 0, limit: 500 },
    },
    'scan_result',
  )
  const candidates = [...first.candidates]
  if (first.scan_id !== scanId || first.offset !== 0 || first.total > 10_000)
    throw new CleanupError('internal')
  while (candidates.length < first.total) {
    signal.throwIfAborted()
    const page = await cleanupRequest(
      {
        type: 'get_scan_result',
        data: { scan_id: scanId, offset: candidates.length, limit: 500 },
      },
      'scan_result',
    )
    if (
      page.scan_id !== scanId ||
      page.total !== first.total ||
      page.offset !== candidates.length ||
      !page.candidates.length
    )
      throw new CleanupError('internal')
    candidates.push(...page.candidates)
  }
  signal.throwIfAborted()
  if (
    candidates.length !== first.total ||
    new Set(candidates.map((item) => item.id)).size !== candidates.length
  )
    throw new CleanupError('internal')
  for (const candidate of candidates) {
    if (candidate.source !== 'cleanup') throw new CleanupError('internal')
    byteCount(candidate.bytes)
    byteCount(candidate.logical_bytes)
  }
  return { ...first, candidates }
}
