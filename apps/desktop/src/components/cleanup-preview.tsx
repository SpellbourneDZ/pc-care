import { useEffect, useState } from 'react'
import { CircleAlert, ShieldCheck } from 'lucide-react'
import type { OperationPlan } from '@/generated/OperationPlan'
import { ru } from '@/i18n/ru'
import { displayPath, formatCount } from '@/lib/cleanup-api'
import {
  AlertDialog,
  AlertDialogContent,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogCancel,
} from '@/ui/alert-dialog'
import { Button } from '@/ui/button'
import { RiskLabel } from './risk-label'

export function CleanupPreview({
  plan,
  deadline,
  fixture,
  busy,
  stale,
  onClose,
  onConfirm,
  onRefresh,
  onReturnFocus,
}: {
  plan: OperationPlan
  deadline: number
  fixture: boolean
  busy: boolean
  stale: boolean
  onClose: () => void
  onConfirm: () => void
  onRefresh: () => void
  onReturnFocus: () => void
}) {
  const [now, setNow] = useState(Date.now)
  const [page, setPage] = useState(0)
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 1000)
    return () => clearInterval(timer)
  }, [])
  const expired = now >= deadline
  const targets = plan.targets.slice(page * 100, page * 100 + 100)
  return (
    <AlertDialog
      open
      onOpenChange={(open) => {
        if (!open && !busy) onClose()
      }}
    >
      <AlertDialogContent
        className="cleanup-confirm"
        onCloseAutoFocus={(event) => {
          event.preventDefault()
          onReturnFocus()
        }}
      >
        <div className="cleanup-preview-body">
          <AlertDialogHeader>
            <ShieldCheck className="size-6" aria-hidden="true" />
            <AlertDialogTitle>{ru.cleanupUx.previewTitle}</AlertDialogTitle>
            <AlertDialogDescription>
              {ru.cleanupUx.previewDescription}
            </AlertDialogDescription>
          </AlertDialogHeader>
          {fixture && <p className="notice">{ru.cleanupUx.fixture}</p>}
          <div className="cleanup-preview-total">
            <strong>
              {ru.cleanupUx.selected}: {plan.targets.length}
            </strong>
            <span className="numeric">{formatCount(plan.planned_bytes)}</span>
          </div>
          <ul className="cleanup-plan-files">
            {targets.map((item) => (
              <li key={item.id}>
                <div className="min-w-0">
                  <bdi dir="ltr">{displayPath(item.display_path)}</bdi>
                  <RiskLabel risk={item.safety} />
                </div>
                <span className="numeric">{formatCount(item.bytes)}</span>
              </li>
            ))}
          </ul>
          {plan.targets.length > 100 && (
            <div className="cleanup-pagination">
              <Button
                variant="outline"
                size="sm"
                disabled={!page}
                onClick={() => setPage(page - 1)}
                aria-label={ru.cleanupUx.previousPage}
              >
                ←
              </Button>
              <span>
                {page * 100 + 1}–
                {Math.min((page + 1) * 100, plan.targets.length)}{' '}
                {ru.cleanupUx.of} {plan.targets.length}
              </span>
              <Button
                variant="outline"
                size="sm"
                disabled={(page + 1) * 100 >= plan.targets.length}
                onClick={() => setPage(page + 1)}
                aria-label={ru.cleanupUx.nextPage}
              >
                →
              </Button>
            </div>
          )}
          <p className="supporting-text">{ru.cleanupUx.undoLimits}</p>
          {plan.simulated && (
            <p className="notice">
              <CircleAlert aria-hidden="true" />
              {ru.cleanupUx.previewOnly}
            </p>
          )}
          {(expired || stale) && (
            <p className="notice" role="alert">
              {stale ? ru.cleanupUx.stale : ru.cleanupUx.expired}
            </p>
          )}
        </div>
        <AlertDialogFooter>
          <AlertDialogCancel disabled={busy}>{ru.cancel}</AlertDialogCancel>
          {expired && !stale ? (
            <Button disabled={busy} onClick={onRefresh}>
              {ru.cleanupUx.preview}
            </Button>
          ) : (
            <Button disabled={busy || stale || expired} onClick={onConfirm}>
              {ru.cleanupUx.finishPreview}
            </Button>
          )}
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  )
}
