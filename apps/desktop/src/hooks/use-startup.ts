import { useState } from 'react'
import { useMutation, useQuery } from '@tanstack/react-query'
import type { StartupQuery } from '@/generated/StartupQuery'
import type { StartupSource } from '@/generated/StartupSource'
import type { SettingsRevision } from '@/generated/SettingsRevision'
import { cleanupRequest, CleanupError } from '@/lib/cleanup-api'

export const startupSources: StartupSource[] = [
  'hkcu_run64',
  'hkcu_run32',
  'hklm_run64',
  'hklm_run32',
  'user_folder',
  'common_folder',
]
export function useStartup(
  enabled: boolean,
  revision: SettingsRevision | null | undefined,
  settingsError: unknown = null,
) {
  const [query, setQuery] = useState<StartupQuery>({
    search: '',
    source: null,
    scope: null,
    offset: 0,
    limit: 100,
  })
  const [selected, setSelected] = useState<number | null>(null)
  const catalog = useQuery({
    queryKey: ['startup-catalog'],
    enabled: enabled && revision !== undefined,
    staleTime: Infinity,
    queryFn: async () => ({
      ...(await cleanupRequest({ type: 'refresh_startup' }, 'startup_catalog')),
      settingsRevision: revision,
    }),
  })
  const id = catalog.data?.id
  const page = useQuery({
    queryKey: ['startup-page', id, query],
    enabled: enabled && !!id && !catalog.isFetching,
    gcTime: 0,
    queryFn: () =>
      cleanupRequest(
        { type: 'get_startup_page', data: { snapshot_id: id!, query } },
        'startup_page',
      ),
  })
  const reveal = useMutation({
    mutationFn: (entry_id: number) =>
      cleanupRequest(
        { type: 'open_startup_location', data: { snapshot_id: id!, entry_id } },
        'storage_location_opened',
      ),
  })
  const copy = useMutation({
    mutationFn: async () => {
      if (!catalog.data) throw new CleanupError('not_ready')
      const entries = []
      let offset = 0,
        total = 1
      while (offset < total) {
        const result = await cleanupRequest(
          {
            type: 'get_startup_page',
            data: {
              snapshot_id: catalog.data.id,
              query: { ...query, offset, limit: 500 },
            },
          },
          'startup_page',
        )
        if (
          result.snapshot_id !== catalog.data.id ||
          result.offset !== offset ||
          result.total > catalog.data.total ||
          (!result.entries.length && offset < result.total)
        )
          throw new CleanupError('internal')
        entries.push(
          ...result.entries.map(
            ({ name, source, scope, publisher, enabled }) => ({
              name,
              source,
              scope,
              publisher,
              enabled,
            }),
          ),
        )
        offset += result.entries.length
        total = result.total
      }
      await navigator.clipboard.writeText(
        JSON.stringify(
          {
            schema_version: 1,
            kind: 'pc-care-startup-inventory',
            collected_ms: catalog.data.collected_ms,
            fixture_mode: catalog.data.fixture_mode,
            truncated: catalog.data.truncated,
            issues: catalog.data.issues,
            entries,
          },
          null,
          2,
        ),
      )
    },
  })
  return {
    catalog,
    stale: !!catalog.data && catalog.data.settingsRevision !== revision,
    page,
    query,
    setQuery,
    selected,
    setSelected: (id: number | null) => {
      setSelected(id)
      reveal.reset()
    },
    refresh: () => {
      setSelected(null)
      setQuery({ ...query, offset: 0 })
      reveal.reset()
      copy.reset()
      void catalog.refetch()
    },
    reveal: (id: number) => reveal.mutate(id),
    revealing: reveal.isPending,
    revealError: reveal.error,
    copy: () => copy.mutate(),
    copying: copy.isPending,
    copied: copy.isSuccess,
    copyError: copy.error,
    error: settingsError ?? catalog.error ?? page.error ?? reveal.error,
  }
}
export type StartupController = ReturnType<typeof useStartup>
