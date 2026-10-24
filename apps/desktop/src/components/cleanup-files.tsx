import { useState } from 'react'
import type { Candidate } from '@/generated/Candidate'
import type { CleanupRule } from '@/generated/CleanupRule'
import type { ScanIssue } from '@/generated/ScanIssue'
import { ru } from '@/i18n/ru'
import { displayPath, formatCount } from '@/lib/cleanup-api'
import type { CleanupController } from '@/hooks/use-cleanup'
import { RiskLabel } from './risk-label'
import { Button } from '@/ui/button'
import { Checkbox } from '@/ui/checkbox'
import { Input } from '@/ui/input'
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from '@/ui/sheet'

export function CleanupFiles({
  rule,
  issues,
  controller,
  onClose,
  onReturnFocus,
}: {
  rule?: CleanupRule
  issues: ScanIssue[]
  controller: CleanupController
  onClose: () => void
  onReturnFocus: () => void
}) {
  const [query, setQuery] = useState('')
  const [page, setPage] = useState(0)
  const all =
    controller.snapshot.data?.candidates.filter(
      (item) => item.rule_id === rule?.id,
    ) ?? []
  const filtered = all.filter((item) =>
    item.display_path
      .toLocaleLowerCase('ru')
      .includes(query.toLocaleLowerCase('ru')),
  )
  const visible = filtered.slice(page * 100, page * 100 + 100)
  return (
    <Sheet
      open
      onOpenChange={(open) => {
        if (!open) onClose()
      }}
    >
      <SheetContent
        className="cleanup-drawer"
        onCloseAutoFocus={(event) => {
          event.preventDefault()
          onReturnFocus()
        }}
      >
        <SheetHeader>
          <SheetTitle>{rule?.name ?? ru.cleanupUx.skipped}</SheetTitle>
          <SheetDescription>
            {rule?.description ?? ru.cleanupUx.skippedDescription}
          </SheetDescription>
        </SheetHeader>
        {controller.catalog.data?.fixture_mode && (
          <p className="notice">{ru.cleanupUx.fixture}</p>
        )}
        {rule ? (
          <>
            <Input
              aria-label={ru.cleanupUx.filter}
              placeholder={ru.cleanupUx.filterPlaceholder}
              value={query}
              onChange={(event) => {
                setQuery(event.target.value)
                setPage(0)
              }}
            />
            <div className="cleanup-files-list">
              {!visible.length && (
                <p className="supporting-text">{ru.cleanupUx.noMatches}</p>
              )}
              {visible.map((item) => (
                <FileRow
                  key={item.id}
                  item={item}
                  selected={controller.selected.has(item.id)}
                  disabled={
                    controller.busy ||
                    controller.staleSettings ||
                    item.safety === 'protected'
                  }
                  onSelect={(checked) => controller.select([item.id], checked)}
                />
              ))}
            </div>
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
                {ru.cleanupUx.showing}: {filtered.length ? page * 100 + 1 : 0}–
                {Math.min((page + 1) * 100, filtered.length)} {ru.cleanupUx.of}{' '}
                {filtered.length}
              </span>
              <Button
                variant="outline"
                size="sm"
                disabled={(page + 1) * 100 >= filtered.length}
                onClick={() => setPage(page + 1)}
                aria-label={ru.cleanupUx.nextPage}
              >
                →
              </Button>
            </div>
          </>
        ) : (
          <div className="cleanup-files-list">
            {issues.map((issue) => (
              <div
                className="cleanup-issue"
                key={`${issue.rule_id}:${issue.reason}`}
              >
                <div className="flex items-center justify-between gap-4">
                  <strong>{ru.cleanupReasons[issue.reason]}</strong>
                  <span className="numeric">{issue.count}</span>
                </div>
                {issue.sample_path && (
                  <bdi dir="ltr" className="cleanup-path">
                    {displayPath(issue.sample_path)}
                  </bdi>
                )}
              </div>
            ))}
          </div>
        )}
        <p className="supporting-text">{ru.cleanupUx.sizeDescription}</p>
      </SheetContent>
    </Sheet>
  )
}

function FileRow({
  item,
  selected,
  disabled,
  onSelect,
}: {
  item: Candidate
  selected: boolean
  disabled: boolean
  onSelect: (value: boolean) => void
}) {
  const path = displayPath(item.display_path)
  return (
    <div className="cleanup-file-row">
      <Checkbox
        aria-label={`${ru.selected}: ${path}`}
        checked={selected}
        disabled={disabled}
        onCheckedChange={(value) => onSelect(value === true)}
      />
      <div className="min-w-0">
        <strong className="row-title">
          <bdi dir="ltr">{path.split('\\').at(-1)}</bdi>
        </strong>
        <bdi dir="ltr" className="cleanup-path">
          {path}
        </bdi>
        <div className="cleanup-file-meta">
          <RiskLabel risk={item.safety} />
          <span>
            {ru.cleanupUx.allocated}: {formatCount(item.bytes)}
          </span>
          <span>
            {ru.cleanupUx.logical}: {formatCount(item.logical_bytes)}
          </span>
        </div>
      </div>
    </div>
  )
}
