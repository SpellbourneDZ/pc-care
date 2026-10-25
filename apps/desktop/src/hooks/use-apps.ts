import { useState } from 'react'
import { useMutation, useQuery } from '@tanstack/react-query'
import type { AppQuery } from '@/generated/AppQuery'
import { CleanupError, cleanupRequest } from '@/lib/cleanup-api'

export function useApps(enabled: boolean) {
  const [query, setQuery] = useState<AppQuery>({
    search: '',
    kind: null,
    source: null,
    sort: 'name',
    offset: 0,
    limit: 100,
  })
  const [selected, setSelected] = useState<string | null>(null)
  const catalog = useQuery({
    queryKey: ['apps-catalog'],
    enabled,
    staleTime: Infinity,
    queryFn: () => cleanupRequest({ type: 'refresh_apps' }, 'apps_catalog'),
  })
  const id = catalog.data?.id
  const page = useQuery({
    queryKey: ['apps-page', id, query],
    enabled: enabled && !!id && !catalog.isFetching,
    gcTime: 0,
    queryFn: () =>
      cleanupRequest(
        { type: 'get_apps_page', data: { snapshot_id: id!, query } },
        'apps_page',
      ),
  })
  const check = useMutation({
    mutationFn: async (app_id: string) => {
      const result = await cleanupRequest(
        { type: 'check_uninstall', data: { snapshot_id: id!, app_id } },
        'uninstall_check',
      )
      if (result.snapshot_id !== id || result.app_id !== app_id)
        throw new CleanupError('target_changed')
      return result
    },
  })
  return {
    catalog,
    page,
    query,
    selected,
    setSelected: (value: string | null) => {
      setSelected(value)
      check.reset()
    },
    check: (appId: string) => check.mutate(appId),
    checking: check.isPending,
    checkError: check.error,
    checkResult:
      check.data?.app_id === selected && check.data.snapshot_id === id
        ? check.data
        : undefined,
    setQuery: (value: AppQuery) => {
      setSelected(null)
      check.reset()
      setQuery(value)
    },
    refresh: () => {
      setSelected(null)
      check.reset()
      setQuery({ ...query, offset: 0 })
      void catalog.refetch()
    },
    error: catalog.error ?? page.error,
  }
}
export type AppsController = ReturnType<typeof useApps>
