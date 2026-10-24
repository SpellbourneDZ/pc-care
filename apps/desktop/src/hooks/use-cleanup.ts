import { useRef, useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import type { OperationPlan } from '@/generated/OperationPlan'
import type { SettingsRevision } from '@/generated/SettingsRevision'
import type { TaskProgress } from '@/generated/TaskProgress'
import {
  byteCount,
  CleanupError,
  cleanupRequest,
  mergeProgress,
  readCleanupSnapshot,
} from '@/lib/cleanup-api'

const taskKey = (id: string | null) => ['cleanup-task', id] as const

export function useCleanup(
  enabled: boolean,
  settingsRevision: SettingsRevision | null | undefined,
  settingsError: unknown = null,
) {
  const client = useQueryClient()
  const generation = useRef(0)
  const [taskId, setTaskId] = useState<string | null>(null)
  const [revision, setRevision] = useState<SettingsRevision | null | undefined>(
    undefined,
  )
  const [selection, setSelection] = useState<{
    scanId: string
    ids: Set<string>
  } | null>(null)
  const [selectionLimited, setSelectionLimited] = useState(false)
  const [outcome, setOutcome] = useState<OperationPlan | null>(null)
  const catalog = useQuery({
    queryKey: ['cleanup-rules'],
    enabled,
    queryFn: () =>
      cleanupRequest({ type: 'get_cleanup_rules' }, 'cleanup_rules'),
  })
  const startMutation = useMutation({
    mutationFn: async (token: number) => {
      const started = await cleanupRequest(
        {
          type: 'start_scan',
          data: { kind: 'cleanup', scenario: 'complete', timeout_ms: 300_000 },
        },
        'scan_started',
        (progress) => {
          if (token !== generation.current) return
          setTaskId(progress.task_id)
          client.setQueryData<TaskProgress>(
            taskKey(progress.task_id),
            (current) => mergeProgress(current, progress),
          )
        },
      )
      if (token === generation.current) setTaskId(started.task_id)
      return started
    },
  })
  const task = useQuery({
    queryKey: taskKey(taskId),
    enabled: !!taskId,
    gcTime: 0,
    queryFn: async () =>
      mergeProgress(
        client.getQueryData<TaskProgress>(taskKey(taskId)),
        await cleanupRequest(
          { type: 'get_task', data: { task_id: taskId! } },
          'task',
        ),
      ),
    refetchInterval: (query) =>
      !query.state.data || query.state.data.status === 'running' ? 500 : false,
  })
  const scanId = task.data?.scan_id
  const snapshot = useQuery({
    queryKey: ['cleanup-snapshot', scanId],
    enabled: !!scanId && !!catalog.data,
    gcTime: 0,
    queryFn: async ({ signal }) => {
      const data = await readCleanupSnapshot(scanId!, signal)
      if (
        data.candidates.some(
          (item) =>
            !catalog.data?.rules.some((rule) => rule.id === item.rule_id),
        )
      )
        throw new CleanupError('internal')
      return data
    },
  })
  const cancelMutation = useMutation({
    mutationFn: async () =>
      cleanupRequest(
        { type: 'cancel_task', data: { task_id: taskId! } },
        'task',
      ),
    onSuccess: (progress) =>
      client.setQueryData<TaskProgress>(taskKey(progress.task_id), (current) =>
        mergeProgress(current, progress),
      ),
  })
  const staleSettings = revision !== settingsRevision
  const selected = staleSettings
    ? new Set<string>()
    : selection && selection.scanId === scanId
      ? selection.ids
      : new Set(
          snapshot.data?.truncated
            ? []
            : snapshot.data?.candidates
                .filter(
                  (item) =>
                    item.safety === 'safe' &&
                    catalog.data?.rules.some(
                      (rule) =>
                        rule.id === item.rule_id && rule.safety === 'safe',
                    ),
                )
                .slice(0, catalog.data?.max_plan_items ?? 0)
                .map((item) => item.id),
        )
  const running = startMutation.isPending || task.data?.status === 'running'
  const planMutation = useMutation({
    mutationFn: async () => {
      if (!scanId || !selected.size || staleSettings)
        throw new CleanupError('not_ready')
      const ids = [...selected]
      const plan = await cleanupRequest(
        {
          type: 'build_operation_plan',
          data: { scan_id: scanId, selected_candidate_ids: ids },
        },
        'operation_plan',
      )
      if (!plan.simulated) throw new CleanupError('unsupported')
      if (
        plan.scan_id !== scanId ||
        plan.targets.length !== ids.length ||
        plan.targets.some(
          (item) => !selected.has(item.id) || item.safety === 'protected',
        ) ||
        new Set(plan.targets.map((item) => item.id)).size !== ids.length
      )
        throw new CleanupError('internal')
      return plan
    },
  })
  const confirmMutation = useMutation({
    mutationFn: async (plan: OperationPlan) => {
      if (staleSettings) throw new CleanupError('target_changed')
      if (!plan.simulated) throw new CleanupError('unsupported')
      if (
        plan.id !== planMutation.data?.id ||
        Date.now() >= planMutation.submittedAt + plan.expires_in_ms
      )
        throw new CleanupError('plan_expired')
      return plan
    },
    onSuccess: setOutcome,
  })
  const busy =
    running ||
    snapshot.isFetching ||
    planMutation.isPending ||
    confirmMutation.isPending
  const canStart = !!catalog.data && settingsRevision !== undefined && !busy
  const start = () => {
    if (!canStart) return
    const token = ++generation.current
    setTaskId(null)
    setRevision(settingsRevision)
    setSelection(null)
    setSelectionLimited(false)
    setOutcome(null)
    cancelMutation.reset()
    planMutation.reset()
    confirmMutation.reset()
    startMutation.mutate(token)
  }
  const select = (ids: string[], checked: boolean) => {
    if (!scanId || staleSettings || busy) return
    const next = new Set(selected)
    const allowed = new Set(
      snapshot.data?.candidates
        .filter((item) => item.safety !== 'protected')
        .map((item) => item.id),
    )
    let limited = false
    for (const id of ids) {
      if (!checked) next.delete(id)
      else if (allowed.has(id) && !next.has(id)) {
        if (next.size < (catalog.data?.max_plan_items ?? 0)) next.add(id)
        else limited = true
      }
    }
    setSelection({ scanId, ids: next })
    setSelectionLimited(limited)
    planMutation.reset()
    confirmMutation.reset()
    setOutcome(null)
  }
  const selectedRows =
    snapshot.data?.candidates.filter((item) => selected.has(item.id)) ?? []
  const total = selectedRows.reduce(
    (sum, item) => sum + byteCount(item.bytes),
    0n,
  )
  const error =
    settingsError ??
    confirmMutation.error ??
    planMutation.error ??
    cancelMutation.error ??
    snapshot.error ??
    task.error ??
    (!taskId ? startMutation.error : null) ??
    catalog.error
  return {
    catalog,
    task,
    snapshot,
    selected,
    selectedRows,
    selectionLimited,
    staleSettings,
    total,
    outcome,
    error,
    busy,
    running,
    canStart,
    initializing:
      catalog.isPending || (settingsRevision === undefined && !settingsError),
    starting: startMutation.isPending,
    cancelling: cancelMutation.isPending,
    preparing: planMutation.isPending,
    confirming: confirmMutation.isPending,
    start,
    select,
    cancel: () => {
      if (taskId) cancelMutation.mutate()
    },
    prepare: () => planMutation.mutateAsync(),
    confirm: (plan: OperationPlan) => confirmMutation.mutateAsync(plan),
    planDeadline:
      planMutation.submittedAt + (planMutation.data?.expires_in_ms ?? 0),
    refresh: () => {
      void catalog.refetch()
      void client.invalidateQueries({ queryKey: ['preferences'] })
      if (taskId) void task.refetch()
      if (scanId) void snapshot.refetch()
    },
  }
}
export type CleanupController = ReturnType<typeof useCleanup>
