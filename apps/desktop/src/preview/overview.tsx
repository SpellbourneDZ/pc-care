import {
  AppWindow,
  ArrowRight,
  ArrowUpFromLine,
  BrushCleaning,
  HardDrive,
  Search,
} from 'lucide-react'
import { ru } from '@/i18n/ru'
import type { PageId } from './data'
import { Button } from '@/ui/button'
import { Card, CardContent } from '@/ui/card'
export function OverviewScreen({
  onNavigate,
  onStart,
}: {
  onNavigate: (page: PageId) => void
  onStart: () => void
}) {
  return (
    <>
      <Card className="overview-start">
        <CardContent>
          <Search className="mb-5 size-8" aria-hidden="true" />
          <h2>{ru.emptyTitle}</h2>
          <p>{ru.overviewIntro}</p>
          <Button size="lg" className="mt-6" onClick={onStart}>
            <BrushCleaning aria-hidden="true" />
            {ru.findJunk}
          </Button>
        </CardContent>
      </Card>
      <div className="module-shortcuts">
        {(
          [
            { id: 'disks', icon: HardDrive },
            { id: 'startup', icon: ArrowUpFromLine },
            { id: 'apps', icon: AppWindow },
          ] as const
        ).map(({ id, icon: Icon }) => (
          <button key={id} onClick={() => onNavigate(id)}>
            <Icon aria-hidden="true" />
            <span>
              <strong>{ru[id]}</strong>
              <span>{ru.moduleHints[id]}</span>
            </span>
            <ArrowRight aria-hidden="true" />
          </button>
        ))}
      </div>
    </>
  )
}
