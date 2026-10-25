import { useState } from 'react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { ru } from '@/i18n/ru'
import { SidebarInset, SidebarProvider, SidebarTrigger } from '@/ui/sidebar'
import { TooltipProvider } from '@/ui/tooltip'
import { Button } from '@/ui/button'
import { AppNavigation, ThemeSelect } from '@/components/app-navigation'
import { QuickSearch } from '@/components/quick-search'
import { LiveWorkspace } from '@/components/live-workspace'
import { PreferencesPanel } from '@/components/preferences-panel'
import { useTheme, type Theme } from '@/hooks/use-theme'
import { useCleanup } from '@/hooks/use-cleanup'
import { useStorage } from '@/hooks/use-storage'
import { useStartup } from '@/hooks/use-startup'
import { useApps } from '@/hooks/use-apps'
import { routeHash, useRoute } from '@/lib/routes'
import { usePreferences, preferencesErrorMessage } from '@/lib/preferences'
import type { PageId } from '@/preview/data'
import { PreviewWorkspace } from '@/preview/workspace'

function Shell() {
  const route = useRoute()
  const preferences = usePreferences(!route.preview)
  const [previewTheme, setPreviewTheme] = useState<Theme>('system')
  const [settingsOpen, setSettingsOpen] = useState(false)
  const snapshot = preferences.query.isError
    ? undefined
    : preferences.query.data
  const cleanup = useCleanup(
    !route.preview,
    snapshot?.revision,
    preferences.query.error,
  )
  const storage = useStorage(!route.preview, snapshot?.revision)
  const apps = useApps(!route.preview && route.page === 'apps')
  const startup = useStartup(
    !route.preview && route.page === 'startup',
    snapshot?.revision,
    preferences.query.error,
  )
  const theme = route.preview
    ? previewTheme
    : (snapshot?.preferences.theme ?? 'system')
  useTheme(theme)
  const navigate = (page: PageId) => {
    window.location.hash = routeHash(page, route.preview)
    requestAnimationFrame(() => document.getElementById('page-title')?.focus())
  }
  const saveTheme = (value: Theme) => {
    if (route.preview) setPreviewTheme(value)
    else if (snapshot)
      preferences.mutation.mutate({ ...snapshot.preferences, theme: value })
  }
  const error = preferences.mutation.error ?? preferences.query.error
  const reload = () => {
    preferences.mutation.reset()
    void preferences.query.refetch()
  }
  return (
    <TooltipProvider>
      <a
        className="skip-link"
        href="#page-title"
        onClick={(event) => {
          event.preventDefault()
          document.getElementById('page-title')?.focus()
        }}
      >
        {ru.skip}
      </a>
      <SidebarProvider className="app-shell">
        <AppNavigation
          page={route.page}
          preview={route.preview}
          onNavigate={navigate}
          theme={theme}
          onThemeChange={saveTheme}
          settingsOpen={settingsOpen}
          onSettingsOpen={setSettingsOpen}
          onPreview={() => {
            setPreviewTheme(theme)
            window.location.hash = routeHash('overview', true)
          }}
          settingsContent={
            route.preview ? undefined : (
              <PreferencesPanel
                preferences={snapshot?.preferences}
                pending={preferences.query.isPending}
                saving={preferences.mutation.isPending}
                saved={preferences.mutation.isSuccess}
                error={error}
                onReload={reload}
                onSave={async (value) => {
                  await preferences.mutation.mutateAsync(value)
                }}
              />
            )
          }
        />
        <SidebarInset className="min-w-0">
          <header className="app-topbar">
            <div className="flex items-center gap-3">
              <SidebarTrigger aria-label={ru.navigation} />
              <QuickSearch
                onNavigate={navigate}
                onSettings={() => setSettingsOpen(true)}
              />
            </div>
            <ThemeSelect
              theme={theme}
              onThemeChange={saveTheme}
              disabled={
                !route.preview && (!snapshot || preferences.mutation.isPending)
              }
            />
          </header>
          {!route.preview && preferences.mutation.isError && (
            <div className="notice shell-error" role="alert">
              <p>{preferencesErrorMessage(preferences.mutation.error)}</p>
              <Button
                size="sm"
                variant="outline"
                onClick={() => setSettingsOpen(true)}
              >
                {ru.settings}
              </Button>
            </div>
          )}
          {route.preview ? (
            <PreviewWorkspace
              key={route.hash}
              page={route.page}
              onNavigate={navigate}
              onExit={() => {
                window.location.hash = routeHash('overview', false)
              }}
            />
          ) : (
            <LiveWorkspace
              page={route.page}
              onNavigate={navigate}
              cleanup={cleanup}
              storage={storage}
              startup={startup}
              apps={apps}
            />
          )}
          <footer className="app-status">
            <span>{ru.noChanges}</span>
            <span>
              {!route.preview &&
                (preferences.mutation.isPending
                  ? ru.settingsSaving
                  : preferences.query.isPending
                    ? ru.settingsLoading
                    : error
                      ? ru.settingsUnavailable
                      : ru.settingsLocal)}
            </span>
            <span>v{__APP_VERSION__}</span>
          </footer>
        </SidebarInset>
      </SidebarProvider>
    </TooltipProvider>
  )
}
export function App() {
  const [client] = useState(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: {
            retry: false,
            networkMode: 'always',
            refetchOnWindowFocus: false,
          },
          mutations: { retry: false, networkMode: 'always' },
        },
      }),
  )
  return (
    <QueryClientProvider client={client}>
      <Shell />
    </QueryClientProvider>
  )
}
