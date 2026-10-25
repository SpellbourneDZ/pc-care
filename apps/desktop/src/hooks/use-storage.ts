import { useEffect, useRef, useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import type { StorageQuery } from '@/generated/StorageQuery'
import type { TaskProgress } from '@/generated/TaskProgress'
import type { SettingsRevision } from '@/generated/SettingsRevision'
import { cleanupRequest, mergeProgress } from '@/lib/cleanup-api'

export const storageQuery = (): StorageQuery => ({
  parent_id: 0,
  view: 'children',
  search: '',
  extension: '',
  min_bytes: '0',
  modified_before_ms: null,
  sort: 'size',
  offset: 0,
  limit: 500,
})
export function useStorage(
  enabled: boolean,
  revision: SettingsRevision | null | undefined,
) {
  const client = useQueryClient()
  const generation = useRef(0)
  const [path, setPath] = useState('')
  const [taskId, setTaskId] = useState<string | null>(null)
  const [scanRevision, setScanRevision] = useState(revision)
  const [query, setQuery] = useState(storageQuery)
  const [selected, setSelected] = useState<number | null>(null)
  const volumes = useQuery({
    queryKey: ['volumes'],
    enabled,
    queryFn: () => cleanupRequest({ type: 'list_volumes' }, 'volumes'),
  })
  const target = path || volumes.data?.find((v) => !v.error)?.path || ''
  const key = (id: string | null) => ['storage-task', id]
  const launch = useMutation({
    mutationFn: async (token: number) => {
      const response = await cleanupRequest(
        {
          type: 'start_storage_scan',
          data: { path: target, timeout_ms: 300_000 },
        },
        'scan_started',
        (progress) => {
          if (generation.current !== token) return
          setTaskId(progress.task_id)
          client.setQueryData<TaskProgress>(key(progress.task_id), (current) =>
            mergeProgress(current, progress),
          )
        },
      )
      if (generation.current === token) setTaskId(response.task_id)
    },
  })
  const task = useQuery({
    queryKey: key(taskId),
    enabled: enabled && !!taskId,
    gcTime: 0,
    queryFn: async () =>
      mergeProgress(
        client.getQueryData<TaskProgress>(key(taskId)),
        await cleanupRequest(
          { type: 'get_task', data: { task_id: taskId! } },
          'task',
        ),
      ),
    refetchInterval: (q) =>
      !q.state.data || q.state.data.status === 'running' ? 500 : false,
  })
  const scanId = task.data?.scan_id
  useEffect(() => {
    if (scanId) void client.invalidateQueries({ queryKey: ['volumes'] })
  }, [client, scanId])
  const page = useQuery({
    queryKey: ['storage-page', scanId, query],
    enabled: enabled && !!scanId,
    gcTime: 0,
    queryFn: () =>
      cleanupRequest(
        { type: 'get_storage_page', data: { scan_id: scanId!, query } },
        'storage_page',
      ),
  })
  const map = useQuery({
    queryKey: ['storage-map', scanId, query.parent_id],
    enabled: enabled && !!scanId,
    gcTime: 0,
    queryFn: () =>
      cleanupRequest(
        {
          type: 'get_storage_page',
          data: {
            scan_id: scanId!,
            query: { ...storageQuery(), parent_id: query.parent_id, limit: 60 },
          },
        },
        'storage_page',
      ),
  })
  const cancel = useMutation({
    mutationFn: () =>
      cleanupRequest(
        { type: 'cancel_task', data: { task_id: taskId! } },
        'task',
      ),
    onSuccess: (progress) =>
      client.setQueryData<TaskProgress>(key(progress.task_id), (current) =>
        mergeProgress(current, progress),
      ),
  })
  const reveal = useMutation({
    mutationFn: (id: number) =>
      cleanupRequest(
        {
          type: 'open_storage_location',
          data: { scan_id: scanId!, node_id: id },
        },
        'storage_location_opened',
      ),
  })
  const running = launch.isPending || task.data?.status === 'running'
  const start = () => {
    if (running || revision === undefined) return
    setTaskId(null)
    setQuery(storageQuery())
    setSelected(null)
    setScanRevision(revision)
    cancel.reset()
    reveal.reset()
    launch.mutate(++generation.current)
  }
  return {
    path: target,
    setPath,
    volumes,
    task,
    page,
    map,
    query,
    setQuery,
    selected,
    setSelected,
    running,
    start,
    canStart: !running && revision !== undefined && !!target,
    stale: scanRevision !== revision,
    cancel: () => cancel.mutate(),
    cancelling: cancel.isPending,
    reveal: (id: number) => reveal.mutate(id),
    revealing: reveal.isPending,
    error:
      launch.error ??
      task.error ??
      page.error ??
      map.error ??
      cancel.error ??
      reveal.error,
    navigate: (id: number) => {
      setQuery({ ...storageQuery(), parent_id: id })
      setSelected(null)
    },
  }
}
export type StorageController = ReturnType<typeof useStorage>
