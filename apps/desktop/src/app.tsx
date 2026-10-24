import { useEffect, useRef, useState } from 'react'
import { FlaskConical, History } from 'lucide-react'
import { ru } from '@/i18n/ru'
import { SidebarInset, SidebarProvider, SidebarTrigger } from '@/ui/sidebar'
import { TooltipProvider } from '@/ui/tooltip'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/ui/select'
import { AppNavigation, ThemeSelect } from '@/components/app-navigation'
import { PreviewContent } from '@/components/preview-state'
import { useTheme } from '@/hooks/use-theme'
import type { PageId, PreviewState } from '@/preview/data'
import { OverviewScreen } from '@/preview/overview'
import { CleanupScreen } from '@/preview/cleanup'
import { DisksScreen } from '@/preview/disks'
import { StartupScreen } from '@/preview/startup'
import { AppsScreen } from '@/preview/apps'
export function App() {
  const [page, setPage] = useState<PageId>('overview')
  const [state, setState] = useState<PreviewState>('empty')
  const [theme, setTheme] = useTheme()
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null)
  const cancelTimer = () => {
    if (timer.current !== null) clearTimeout(timer.current)
    timer.current = null
  }
  useEffect(
    () => () => {
      if (timer.current !== null) clearTimeout(timer.current)
    },
    [],
  )
  const navigate = (target: PageId) => {
    cancelTimer()
    setPage(target)
    setState('empty')
    requestAnimationFrame(() => document.getElementById('page-title')?.focus())
  }
  const scan = () => {
    cancelTimer()
    setState('loading')
    timer.current = setTimeout(() => {
      setState('ready')
      timer.current = null
    }, 900)
  }
  const changeState = (value: PreviewState) => {
    cancelTimer()
    setState(value)
  }
  const module = page !== 'overview' && page !== 'history'
  return (
    <TooltipProvider>
      <a className="skip-link" href="#page-title">
        {ru.skip}
      </a>
      <SidebarProvider className="app-shell">
        <AppNavigation
          page={page}
          onNavigate={navigate}
          theme={theme}
          onThemeChange={setTheme}
        />
        <SidebarInset className="min-w-0">
          <header className="app-topbar">
            <div className="flex items-center gap-3">
              <SidebarTrigger aria-label={ru.navigation} />
              <span className="text-xs text-muted-foreground">
                {ru.preview}
              </span>
            </div>
            <ThemeSelect theme={theme} onThemeChange={setTheme} />
          </header>
          <div className="preview-bar">
            <div>
              <FlaskConical aria-hidden="true" />
              <strong>{ru.prototype}</strong>
              <span>{ru.exampleData}</span>
            </div>
            {module && (
              <Select
                value={state}
                onValueChange={(value) => {
                  if (value in ru.states) changeState(value as PreviewState)
                }}
              >
                <SelectTrigger aria-label={ru.stateLabel}>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {Object.entries(ru.states).map(([value, label]) => (
                    <SelectItem value={value} key={value}>
                      {label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            )}
          </div>
          <div className="workspace">
            <div className="page-heading">
              <h1 id="page-title" tabIndex={-1}>
                {ru[page]}
              </h1>
              <p>{ru.descriptions[page]}</p>
            </div>
            {page === 'overview' && (
              <OverviewScreen
                onNavigate={navigate}
                onStart={() => {
                  navigate('cleanup')
                  scan()
                }}
              />
            )}
            {module && (
              <PreviewContent
                state={state}
                onScan={scan}
                onCancel={() => changeState('empty')}
              >
                {page === 'cleanup' && <CleanupScreen />}
                {page === 'disks' && <DisksScreen />}
                {page === 'startup' && <StartupScreen />}
                {page === 'apps' && <AppsScreen />}
              </PreviewContent>
            )}
            {page === 'history' && (
              <section className="empty-state">
                <History aria-hidden="true" />
                <h2>{ru.historyEmpty}</h2>
                <p>{ru.historyDescription}</p>
              </section>
            )}
          </div>
          <footer className="app-status">
            <span>{ru.noChanges}</span>
            <span>v{__APP_VERSION__}</span>
          </footer>
        </SidebarInset>
      </SidebarProvider>
    </TooltipProvider>
  )
}
