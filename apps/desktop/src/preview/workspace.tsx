import { useEffect, useRef, useState } from 'react'
import { FlaskConical, History } from 'lucide-react'
import type { PageId, PreviewState } from './data'
import { ru } from '@/i18n/ru'
import { Button } from '@/ui/button'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/ui/select'
import { PreviewContent } from '@/components/preview-state'
import { OverviewScreen } from './overview'
import { CleanupScreen } from './cleanup'
import { DisksScreen } from './disks'
import { StartupScreen } from './startup'
import { AppsScreen } from './apps'

export function PreviewWorkspace({
  page,
  onNavigate,
  onExit,
}: {
  page: PageId
  onNavigate: (page: PageId) => void
  onExit: () => void
}) {
  const [state, setState] = useState<PreviewState>('empty')
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null)
  const clear = () => {
    if (timer.current !== null) clearTimeout(timer.current)
    timer.current = null
  }
  useEffect(
    () => () => {
      if (timer.current !== null) clearTimeout(timer.current)
    },
    [],
  )
  const change = (value: PreviewState) => {
    clear()
    setState(value)
  }
  const scan = () => {
    clear()
    setState('loading')
    timer.current = setTimeout(() => setState('ready'), 900)
  }
  const module = page !== 'overview' && page !== 'history'
  return (
    <>
      <div className="preview-bar">
        <div>
          <FlaskConical aria-hidden="true" />
          <strong>{ru.prototype}</strong>
          <span>{ru.exampleData}</span>
        </div>
        <div className="flex gap-2">
          {module && (
            <Select
              value={state}
              onValueChange={(value) => {
                if (value in ru.states) change(value as PreviewState)
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
          <Button variant="ghost" size="sm" onClick={onExit}>
            {ru.exitPrototype}
          </Button>
        </div>
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
            onNavigate={onNavigate}
            onStart={() => onNavigate('cleanup')}
          />
        )}
        {module && (
          <PreviewContent
            state={state}
            onScan={scan}
            onCancel={() => change('empty')}
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
    </>
  )
}
