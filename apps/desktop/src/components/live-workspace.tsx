import { HardDrive, History, ShieldCheck } from 'lucide-react'
import { ru } from '@/i18n/ru'
import type { PageId } from '@/preview/data'
import { Button } from '@/ui/button'
import { Card, CardContent, CardHeader } from '@/ui/card'
import type { CleanupController } from '@/hooks/use-cleanup'
import { CleanupScreen } from './cleanup-screen'
export function LiveWorkspace({
  page,
  onNavigate,
  cleanup,
}: {
  page: PageId
  onNavigate: (page: PageId) => void
  cleanup: CleanupController
}) {
  return (
    <div className="workspace">
      <div className="page-heading">
        <h1 id="page-title" tabIndex={-1}>
          {ru[page]}
        </h1>
        <p>{ru.descriptions[page]}</p>
      </div>
      {page === 'overview' ? (
        <>
          <Card className="overview-start">
            <CardContent>
              <ShieldCheck className="mb-5 size-8" aria-hidden="true" />
              <h2>{ru.cleanupUx.overviewTitle}</h2>
              <p>{ru.cleanupUx.overviewDescription}</p>
              <div className="mt-6 flex flex-wrap gap-3">
                <Button
                  disabled={!cleanup.canStart}
                  onClick={() => {
                    onNavigate('cleanup')
                    cleanup.start()
                  }}
                >
                  {ru.findJunk}
                </Button>
                <Button variant="outline" onClick={() => onNavigate('cleanup')}>
                  {ru.openCleanup}
                </Button>
              </div>
            </CardContent>
          </Card>
          <Card className="mt-6">
            <CardHeader>
              <h2 className="flex items-center gap-2 text-base font-medium">
                <HardDrive className="size-5" aria-hidden="true" />
                {ru.disks}
              </h2>
            </CardHeader>
            <CardContent>
              <p className="font-medium">{ru.volumeNotReady}</p>
              <p className="supporting-text mt-2">
                {ru.volumeNotReadyDescription}
              </p>
              <Button
                variant="outline"
                className="mt-4"
                onClick={() => onNavigate('disks')}
              >
                {ru.openDisks}
              </Button>
            </CardContent>
          </Card>
        </>
      ) : page === 'cleanup' ? (
        <CleanupScreen controller={cleanup} />
      ) : page === 'history' ? (
        <section className="empty-state">
          <History aria-hidden="true" />
          <h2>{ru.historyEmpty}</h2>
          <p>{ru.historyDescription}</p>
        </section>
      ) : (
        <section className="empty-state">
          <ShieldCheck aria-hidden="true" />
          <h2>{ru.moduleNotReady}</h2>
          <p>{ru.moduleNotReadyDescription}</p>
          <Button disabled>{ru.scan}</Button>
        </section>
      )}
    </div>
  )
}
