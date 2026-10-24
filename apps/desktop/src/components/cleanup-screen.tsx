import { useRef, useState } from 'react'
import { CircleAlert, FolderSearch, ShieldCheck } from 'lucide-react'
import type { CleanupRule } from '@/generated/CleanupRule'
import type { OperationPlan } from '@/generated/OperationPlan'
import type { CleanupController } from '@/hooks/use-cleanup'
import { ru } from '@/i18n/ru'
import {
  byteCount,
  CleanupError,
  cleanupErrorMessage,
  formatCount,
} from '@/lib/cleanup-api'
import { Button } from '@/ui/button'
import { Checkbox } from '@/ui/checkbox'
import { Input } from '@/ui/input'
import { Progress } from '@/ui/progress'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/ui/select'
import { Skeleton } from '@/ui/skeleton'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/ui/table'
import { CleanupFiles } from './cleanup-files'
import { CleanupPreview } from './cleanup-preview'
import { RiskLabel } from './risk-label'

export function CleanupScreen({
  controller: c,
}: {
  controller: CleanupController
}) {
  const [query, setQuery] = useState('')
  const [risk, setRisk] = useState('all')
  const [drawer, setDrawer] = useState<CleanupRule | 'issues' | null>(null)
  const [plan, setPlan] = useState<OperationPlan | null>(null)
  const returnTo = useRef<HTMLElement | null>(null)
  const returnFocus = () =>
    (returnTo.current?.isConnected
      ? returnTo.current
      : document.getElementById('page-title')
    )?.focus()
  const snapshot = c.snapshot.data
  const issues = snapshot?.issues ?? []
  const partial = !!issues.length || snapshot?.truncated
  const max = c.catalog.data?.max_plan_items ?? 0
  const defaultLimited =
    (snapshot?.candidates.filter((item) => item.safety === 'safe').length ??
      0) > max
  const groups = (c.catalog.data?.rules ?? []).filter(
    (rule) =>
      (risk === 'all' || risk === rule.safety) &&
      (rule.name
        .toLocaleLowerCase('ru')
        .includes(query.toLocaleLowerCase('ru')) ||
        snapshot?.candidates.some(
          (item) =>
            item.rule_id === rule.id &&
            item.display_path
              .toLocaleLowerCase('ru')
              .includes(query.toLocaleLowerCase('ru')),
        )),
  )
  const prepare = async () => {
    try {
      setPlan(await c.prepare())
    } catch {
      /* The controller renders the typed failure. */
    }
  }
  const error =
    c.error ?? (c.task.data?.error ? new CleanupError(c.task.data.error) : null)
  return (
    <section className="cleanup-screen" aria-label={ru.cleanup}>
      {c.catalog.data?.fixture_mode && (
        <p className="notice">
          <CircleAlert aria-hidden="true" />
          {ru.cleanupUx.fixture} · {ru.cleanupUx.fixtureDescription}
        </p>
      )}
      {error && (
        <div className="notice" role="alert">
          <CircleAlert aria-hidden="true" />
          <p>{cleanupErrorMessage(error)}</p>
          <Button variant="outline" size="sm" onClick={c.refresh}>
            {ru.cleanupUx.reload}
          </Button>
        </div>
      )}
      {c.running ? (
        <div className="cleanup-progress" role="status">
          <h2>{c.starting ? ru.cleanupUx.preparing : ru.cleanupUx.scanning}</h2>
          <p>
            {ru.cleanupUx.viewed}:{' '}
            <strong className="numeric">
              {c.task.data?.processed.toLocaleString('ru-RU') ?? 0}
            </strong>
          </p>
          <Progress aria-label={ru.cleanupUx.scanning} />
          <p className="supporting-text">{ru.cleanupUx.unknownTotal}</p>
          <Button
            variant="outline"
            disabled={c.cancelling || !c.task.data}
            onClick={c.cancel}
          >
            {ru.cleanupUx.cancel}
          </Button>
        </div>
      ) : c.initializing || c.snapshot.isFetching ? (
        <div
          className="cleanup-progress"
          role="status"
          aria-label={ru.cleanupUx.collecting}
        >
          <p>{ru.cleanupUx.collecting}</p>
          <Skeleton className="h-12 w-full" />
          <Skeleton className="h-12 w-full" />
          <Skeleton className="h-12 w-full" />
        </div>
      ) : c.outcome ? (
        <div className="empty-state" role="status">
          <ShieldCheck aria-hidden="true" />
          <h2>{ru.cleanupUx.previewDone}</h2>
          <p>{ru.cleanupUx.previewDoneDescription}</p>
          <Button onClick={c.start} disabled={!c.canStart}>
            {ru.cleanupUx.rescan}
          </Button>
        </div>
      ) : !snapshot ? (
        <div className="empty-state">
          <FolderSearch aria-hidden="true" />
          <h2>
            {c.task.data?.status === 'cancelled'
              ? ru.cleanupUx.cancelled
              : ru.cleanupUx.intro}
          </h2>
          <p>
            {c.task.data?.status === 'cancelled'
              ? ru.cleanupUx.cancelledDescription
              : ru.cleanupUx.introDescription}
          </p>
          <Button disabled={!c.canStart} onClick={c.start}>
            {ru.cleanupUx.scan}
          </Button>
          <p className="supporting-text">{ru.cleanupUx.previewOnly}</p>
        </div>
      ) : (
        <>
          <div className="cleanup-toolbar">
            <div>
              <h2>
                {snapshot.total ? ru.cleanupUx.ready : ru.cleanupUx.noFindings}
              </h2>
              <p className="supporting-text">
                {snapshot.total
                  ? ru.cleanupUx.sizeDescription
                  : ru.cleanupUx.noFindingsDescription}
              </p>
            </div>
            <Button variant="outline" onClick={c.start} disabled={!c.canStart}>
              {ru.cleanupUx.rescan}
            </Button>
          </div>
          {partial && (
            <div className="notice">
              <CircleAlert aria-hidden="true" />
              <div>
                <strong>
                  {snapshot.truncated
                    ? ru.cleanupUx.truncated
                    : ru.cleanupUx.partial}
                </strong>
                <p>
                  {snapshot.truncated
                    ? ru.cleanupUx.truncatedDescription
                    : ru.cleanupUx.skippedDescription}
                </p>
              </div>
            </div>
          )}
          {c.staleSettings && (
            <p className="notice" role="alert">
              {ru.cleanupUx.stale}
            </p>
          )}
          <div className="cleanup-toolbar cleanup-filters">
            <Input
              aria-label={ru.cleanupUx.filter}
              placeholder={ru.cleanupUx.filterPlaceholder}
              value={query}
              onChange={(event) => setQuery(event.target.value)}
            />
            <Select value={risk} onValueChange={setRisk}>
              <SelectTrigger aria-label={ru.cleanupUx.riskFilter}>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">{ru.cleanupUx.allRisks}</SelectItem>
                <SelectItem value="safe">{ru.risks.safe}</SelectItem>
                <SelectItem value="review">{ru.risks.review}</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div className="table-panel cleanup-categories">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>
                    <span className="sr-only">{ru.selected}</span>
                  </TableHead>
                  <TableHead>{ru.category}</TableHead>
                  <TableHead>{ru.cleanupUx.items}</TableHead>
                  <TableHead>{ru.cleanupUx.allocated}</TableHead>
                  <TableHead>
                    <span className="sr-only">{ru.cleanupUx.details}</span>
                  </TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {groups.map((rule) => {
                  const rows = snapshot.candidates.filter(
                    (item) => item.rule_id === rule.id,
                  )
                  const selectable = rows.filter(
                    (item) => item.safety !== 'protected',
                  )
                  const count = selectable.filter((item) =>
                    c.selected.has(item.id),
                  ).length
                  const bytes = rows.reduce(
                    (sum, item) => sum + byteCount(item.bytes),
                    0n,
                  )
                  return (
                    <TableRow key={rule.id}>
                      <TableCell>
                        <Checkbox
                          aria-label={`${ru.selected}: ${rule.name}`}
                          disabled={
                            !selectable.length || c.busy || c.staleSettings
                          }
                          checked={
                            count === selectable.length && !!count
                              ? true
                              : count
                                ? 'indeterminate'
                                : false
                          }
                          onCheckedChange={(checked) =>
                            c.select(
                              selectable.map((item) => item.id),
                              checked === true,
                            )
                          }
                        />
                      </TableCell>
                      <TableCell>
                        <strong className="row-title">
                          {rule.name}
                          <RiskLabel risk={rule.safety} />
                        </strong>
                        <span className="row-detail">{rule.description}</span>
                      </TableCell>
                      <TableCell className="numeric">
                        {rows.length.toLocaleString('ru-RU')}
                      </TableCell>
                      <TableCell className="numeric">
                        {formatCount(bytes)}
                      </TableCell>
                      <TableCell>
                        <Button
                          variant="ghost"
                          size="sm"
                          aria-label={`${ru.cleanupUx.details}: ${rule.name}`}
                          disabled={!rows.length}
                          onClick={(event) => {
                            returnTo.current = event.currentTarget
                            setDrawer(rule)
                          }}
                        >
                          {ru.open}
                        </Button>
                      </TableCell>
                    </TableRow>
                  )
                })}
                {!groups.length && (
                  <TableRow>
                    <TableCell colSpan={5}>{ru.cleanupUx.noMatches}</TableCell>
                  </TableRow>
                )}
                <TableRow>
                  <TableCell>
                    <Checkbox
                      disabled
                      checked={false}
                      aria-label={ru.cleanupUx.protected}
                    />
                  </TableCell>
                  <TableCell>
                    <strong className="row-title">
                      {ru.cleanupUx.protected}
                      <RiskLabel risk="protected" />
                    </strong>
                    <span className="row-detail">
                      {ru.cleanupUx.protectedDescription}
                    </span>
                  </TableCell>
                  <TableCell colSpan={2}>{ru.cleanupUx.notCounted}</TableCell>
                  <TableCell>
                    <Button
                      variant="ghost"
                      size="sm"
                      disabled={!issues.length}
                      onClick={(event) => {
                        returnTo.current = event.currentTarget
                        setDrawer('issues')
                      }}
                    >
                      {ru.cleanupUx.skipped}
                    </Button>
                  </TableCell>
                </TableRow>
              </TableBody>
            </Table>
          </div>
          {(c.selectionLimited || defaultLimited) && (
            <p className="supporting-text mt-4" role="status">
              {ru.cleanupUx.limit}: {max}.{' '}
              {c.selectionLimited && ru.cleanupUx.limitReached}
            </p>
          )}
          <div className="action-bar">
            <div>
              <strong>
                {ru.cleanupUx.selected}:{' '}
                <span className="numeric" data-testid="cleanup-selection">
                  {c.selected.size}
                </span>{' '}
                · <span className="numeric">{formatCount(c.total)}</span>
              </strong>
              <p className="supporting-text">{ru.cleanupUx.previewShort}</p>
            </div>
            <div className="flex flex-wrap items-center gap-2">
              <Button
                variant="ghost"
                disabled={!c.selected.size || c.busy}
                onClick={() => c.select([...c.selected], false)}
              >
                {ru.cleanupUx.clear}
              </Button>
              <Button
                disabled={!c.selected.size || c.busy || c.staleSettings}
                onClick={(event) => {
                  returnTo.current = event.currentTarget
                  void prepare()
                }}
              >
                {ru.cleanupUx.preview}
              </Button>
            </div>
          </div>
        </>
      )}
      {drawer && (
        <CleanupFiles
          rule={drawer === 'issues' ? undefined : drawer}
          issues={issues}
          controller={c}
          onClose={() => setDrawer(null)}
          onReturnFocus={returnFocus}
        />
      )}
      {plan && (
        <CleanupPreview
          key={plan.id}
          plan={plan}
          deadline={c.planDeadline}
          fixture={!!c.catalog.data?.fixture_mode}
          busy={c.confirming || c.preparing}
          stale={c.staleSettings}
          onClose={() => setPlan(null)}
          onReturnFocus={returnFocus}
          onRefresh={() => void prepare()}
          onConfirm={() => {
            void c
              .confirm(plan)
              .then(() => setPlan(null))
              .catch(() => setPlan(null))
          }}
        />
      )}
    </section>
  )
}
