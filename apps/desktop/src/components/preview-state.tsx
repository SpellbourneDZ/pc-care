import type { ReactNode } from 'react'
import {
  CircleAlert,
  HardDrive,
  LoaderCircle,
  Shield,
  WifiOff,
} from 'lucide-react'
import { ru } from '@/i18n/ru'
import { Button } from '@/ui/button'
import { Progress } from '@/ui/progress'
import { Skeleton } from '@/ui/skeleton'
import type { PreviewState } from '@/preview/data'

export function PreviewContent({
  state,
  onScan,
  onCancel,
  children,
}: {
  state: PreviewState
  onScan: () => void
  onCancel: () => void
  children: ReactNode
}) {
  if (state === 'ready' || state === 'partial' || state === 'offline')
    return (
      <>
        {state !== 'ready' && (
          <div className="notice" role="status">
            {state === 'partial' ? (
              <CircleAlert aria-hidden="true" />
            ) : (
              <WifiOff aria-hidden="true" />
            )}
            <div>
              {state === 'partial' ? (
                <>
                  <strong>{ru.partialTitle}</strong>
                  <p>{ru.partialDescription}</p>
                </>
              ) : (
                ru.offlineDescription
              )}
            </div>
          </div>
        )}
        {children}
      </>
    )
  const loading = state === 'loading'
  const error = state === 'error'
  const unsupported = state === 'unsupported'
  const Icon = loading
    ? LoaderCircle
    : error
      ? CircleAlert
      : unsupported
        ? Shield
        : HardDrive
  const title = loading
    ? ru.loading
    : error
      ? ru.errorTitle
      : unsupported
        ? ru.unsupportedTitle
        : ru.emptyModule
  return (
    <section className="empty-state" aria-busy={loading} aria-live="polite">
      <Icon
        className={loading ? 'animate-spin motion-reduce:animate-none' : ''}
        aria-hidden="true"
      />
      <h2>{title}</h2>
      <p>
        {loading
          ? ru.loadingDescription
          : error
            ? ru.errorDescription
            : unsupported
              ? ru.unsupportedDescription
              : ru.emptyModuleDescription}
      </p>
      {loading && (
        <div className="loading-sample">
          <Progress aria-label={ru.loading} value={null} />
          <Skeleton className="mt-4 h-3 w-4/5" />
          <Skeleton className="mt-3 h-3 w-3/5" />
        </div>
      )}
      {!unsupported && (
        <Button
          variant={loading ? 'outline' : 'default'}
          onClick={loading ? onCancel : onScan}
        >
          {loading ? ru.cancel : error ? ru.retry : ru.scan}
        </Button>
      )}
    </section>
  )
}
