import type { ApiReply } from '../src/generated/ApiReply'
import type { ApiResponse } from '../src/generated/ApiResponse'
import type { Candidate } from '../src/generated/Candidate'
import type { CleanupCatalog } from '../src/generated/CleanupCatalog'
import type { CandidatePage } from '../src/generated/CandidatePage'
import type { OperationPlan } from '../src/generated/OperationPlan'

export const catalog: CleanupCatalog = {
  fixture_mode: true,
  max_plan_items: 500,
  rules: [
    {
      id: 'temp',
      name: 'Временные файлы пользователя',
      description: 'Искусственные .tmp старше семи дней.',
      safety: 'safe',
      min_age_hours: 168,
    },
    {
      id: 'chrome',
      name: 'HTTP-кеш Google Chrome',
      description: 'Проверьте необходимость офлайн-копий.',
      safety: 'review',
      min_age_hours: 24,
    },
    {
      id: 'edge',
      name: 'HTTP-кеш Microsoft Edge',
      description: 'Проверьте необходимость офлайн-копий.',
      safety: 'review',
      min_age_hours: 24,
    },
  ],
}
export const candidate = (id: string, rule = 'temp'): Candidate => ({
  id,
  source: 'cleanup',
  source_key: rule,
  rule_id: rule,
  identity: 'opaque-fixture',
  display_path: `C:\\PC Care fixture\\${id}.tmp`,
  bytes: '4096',
  logical_bytes: '42',
  safety: rule === 'temp' ? 'safe' : 'review',
})
export const resultPage = (
  candidates = [candidate('safe'), candidate('review', 'chrome')],
): CandidatePage => ({
  scan_id: 'scan',
  offset: 0,
  total: candidates.length,
  candidates,
  truncated: false,
  issues: [],
})
export const operationPlan = (targets: Candidate[]): OperationPlan => ({
  id: 'plan',
  scan_id: 'scan',
  targets,
  simulated: true,
  expires_in_ms: 30_000,
  planned_bytes: targets
    .reduce((sum, item) => sum + BigInt(item.bytes), 0n)
    .toString(),
  warnings: ['simulation_only'],
})
export const ok = (value: ApiResponse): ApiReply => ({ status: 'ok', value })
