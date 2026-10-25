import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import type { AppQuery } from '@/generated/AppQuery'
import { cleanupRequest } from '@/lib/cleanup-api'

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
  return {
    catalog,
    page,
    query,
    selected,
    setSelected,
    setQuery: (value: AppQuery) => {
      setSelected(null)
      setQuery(value)
    },
    refresh: () => {
      setSelected(null)
      setQuery({ ...query, offset: 0 })
      void catalog.refetch()
    },
    error: catalog.error ?? page.error,
  }
}
export type AppsController = ReturnType<typeof useApps>
