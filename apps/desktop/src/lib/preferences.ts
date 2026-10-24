import { ru } from '@/i18n/ru'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { operationApi } from './operation-api'
import type { ApiError } from '@/generated/ApiError'
import type { ApiRequest } from '@/generated/ApiRequest'
import type { Preferences } from '@/generated/Preferences'
import type { SettingsSnapshot } from '@/generated/SettingsSnapshot'

export class PreferencesError extends Error {
  constructor(readonly code: ApiError) {
    super(code)
  }
}
async function requestPreferences(
  request: ApiRequest,
): Promise<SettingsSnapshot> {
  const reply = await operationApi(request)
  if (reply.status === 'error') throw new PreferencesError(reply.error)
  if (reply.value.type !== 'preferences') throw new PreferencesError('internal')
  return reply.value.data
}
export function usePreferences(enabled: boolean) {
  const client = useQueryClient()
  const query = useQuery({
    queryKey: ['preferences'],
    enabled,
    queryFn: () => requestPreferences({ type: 'get_preferences' }),
  })
  const mutation = useMutation({
    mutationFn: (preferences: Preferences) =>
      requestPreferences({
        type: 'save_preferences',
        data: { expected_revision: query.data?.revision ?? null, preferences },
      }),
    onSuccess: (snapshot) => client.setQueryData(['preferences'], snapshot),
  })
  return { query, mutation }
}

export function preferencesErrorMessage(error: unknown) {
  if (!(error instanceof PreferencesError)) return ru.settingsUnavailable
  switch (error.code) {
    case 'settings_invalid':
      return ru.settingsInvalid
    case 'settings_conflict':
      return ru.settingsConflict
    case 'unsupported':
      return ru.settingsFuture
    case 'invalid_request':
      return ru.exclusionInvalid
    case 'transport':
      return ru.settingsDesktopOnly
    case 'timeout':
      return ru.settingsTimeout
    default:
      return ru.settingsUnavailable
  }
}
