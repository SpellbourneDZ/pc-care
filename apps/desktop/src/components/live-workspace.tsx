import { HardDrive, History, ShieldCheck } from 'lucide-react'
import { ru } from '@/i18n/ru'
import type { PageId } from '@/preview/data'
import { Button } from '@/ui/button'
import { Card, CardContent, CardHeader } from '@/ui/card'
import type { CleanupController } from '@/hooks/use-cleanup'
import { CleanupScreen } from './cleanup-screen'
import type { StorageController } from '@/hooks/use-storage'
import { StorageScreen, Volumes } from './storage-screen'
import { StartupScreen } from './startup-screen'
import type { StartupController } from '@/hooks/use-startup'
import type { AppsController } from '@/hooks/use-apps'
import { AppsScreen } from './apps-screen'
export function LiveWorkspace({
  page,
  onNavigate,
  cleanup,
  storage,
  startup,
  apps,
}: {
  page: PageId
  onNavigate: (page: PageId) => void
  cleanup: CleanupController
  storage: StorageController
  startup: StartupController
  apps: AppsController
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
              <Volumes
                controller={storage}
                onChoose={(path) => {
                  storage.setPath(path)
                  onNavigate('disks')
                }}
              />
            </CardContent>
          </Card>
        </>
      ) : page === 'cleanup' ? (
        <CleanupScreen controller={cleanup} />
      ) : page === 'disks' ? (
        <StorageScreen
          controller={storage}
          fixture={!!cleanup.catalog.data?.fixture_mode}
        />
      ) : page === 'startup' ? (
        <StartupScreen controller={startup} />
      ) : page === 'apps' ? (
        <AppsScreen controller={apps} />
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
