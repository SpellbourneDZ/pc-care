import { useRef } from 'react'
import { CircleAlert, FileCog, Info } from 'lucide-react'
import { startupSources, type StartupController } from '@/hooks/use-startup'
import { cleanupErrorMessage, displayPath } from '@/lib/cleanup-api'
import { ru } from '@/i18n/ru'
import { Button } from '@/ui/button'
import { Badge } from '@/ui/badge'
import { Input } from '@/ui/input'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/ui/table'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/ui/select'
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetDescription,
} from '@/ui/sheet'

const stateText = (enabled: boolean | null) =>
  enabled === null ? ru.startupUx.unknown : enabled ? ru.enabled : ru.disabled

export function StartupScreen({
  controller: c,
}: {
  controller: StartupController
}) {
  const returnTo = useRef<HTMLElement | null>(null)
  const catalog = c.catalog.data,
    page = c.page.data
  const selected = page?.entries.find((e) => e.id === c.selected)
  const busy = c.catalog.isFetching || c.copying
  return (
    <section className="startup-screen" aria-label={ru.startup}>
      {catalog?.fixture_mode && (
        <p className="notice">{ru.cleanupUx.fixture}</p>
      )}
      <div className="startup-toolbar">
        <div>
          <p>{ru.startupUx.readonly}</p>
          <p className="supporting-text">{ru.startupUx.statusHelp}</p>
        </div>
        <Button variant="outline" disabled={busy} onClick={c.refresh}>
          {ru.startupUx.refresh}
        </Button>
      </div>
      {c.error && (
        <p className="notice" role="alert">
          <CircleAlert aria-hidden="true" />
          {cleanupErrorMessage(c.error)}
        </p>
      )}
      {c.copyError && (
        <p className="notice" role="alert">
          {ru.startupUx.copyFailed}
        </p>
      )}
      {c.stale && <p className="notice">{ru.startupUx.stale}</p>}
      {catalog && (
        <>
          <p className="supporting-text">
            {ru.startupUx.count}:{' '}
            <span className="numeric">
              {catalog.total.toLocaleString('ru')}
            </span>{' '}
            · {ru.startupUx.collected}:{' '}
            {new Date(catalog.collected_ms).toLocaleString('ru-RU')}
          </p>
          {!!catalog.issues.length && (
            <div className="notice">
              <CircleAlert aria-hidden="true" />
              <div>
                <strong>
                  {catalog.truncated
                    ? ru.startupUx.limit
                    : ru.startupUx.partial}
                </strong>
                <ul>
                  {catalog.issues.map((issue, i) => (
                    <li key={i}>
                      {ru.startupUx.sources[issue.source]}:{' '}
                      {ru.startupUx.issues[issue.reason]} ({issue.count})
                    </li>
                  ))}
                </ul>
              </div>
            </div>
          )}
        </>
      )}
      <div className="startup-filters">
        <Input
          aria-label={ru.startupUx.search}
          placeholder={ru.startupUx.search}
          value={c.query.search}
          maxLength={256}
          onChange={(event) => {
            c.setSelected(null)
            c.setQuery({ ...c.query, search: event.target.value, offset: 0 })
          }}
        />
        <Select
          value={c.query.source ?? 'all'}
          onValueChange={(value) => {
            c.setSelected(null)
            c.setQuery({
              ...c.query,
              source: startupSources.find((source) => source === value) ?? null,
              offset: 0,
            })
          }}
        >
          <SelectTrigger aria-label={ru.startupUx.source}>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">{ru.startupUx.allSources}</SelectItem>
            {startupSources.map((source) => (
              <SelectItem value={source} key={source}>
                {ru.startupUx.sources[source]}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <Select
          value={c.query.scope ?? 'all'}
          onValueChange={(value) => {
            c.setSelected(null)
            c.setQuery({
              ...c.query,
              scope:
                value === 'user'
                  ? 'user'
                  : value === 'machine'
                    ? 'machine'
                    : null,
              offset: 0,
            })
          }}
        >
          <SelectTrigger aria-label={ru.startupUx.scope}>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">{ru.startupUx.allScopes}</SelectItem>
            <SelectItem value="user">{ru.startupUx.scopes.user}</SelectItem>
            <SelectItem value="machine">
              {ru.startupUx.scopes.machine}
            </SelectItem>
          </SelectContent>
        </Select>
      </div>
      {!c.error &&
      (c.catalog.isPending || c.catalog.isFetching || c.page.isFetching) ? (
        <p role="status">{ru.startupUx.loading}</p>
      ) : page?.entries.length ? (
        <>
          <div className="startup-table">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>{ru.startupUx.program}</TableHead>
                  <TableHead>{ru.startupUx.publisher}</TableHead>
                  <TableHead>{ru.startupUx.source}</TableHead>
                  <TableHead>{ru.startupUx.state}</TableHead>
                  <TableHead>{ru.startupUx.scope}</TableHead>
                  <TableHead>
                    <span className="sr-only">{ru.startupUx.details}</span>
                  </TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {page.entries.map((entry) => (
                  <TableRow key={entry.id}>
                    <TableCell>
                      <span className="startup-name">
                        <FileCog
                          aria-hidden="true"
                          className="size-4 shrink-0"
                        />
                        <span title={displayPath(entry.name)}>
                          {displayPath(entry.name) || ru.startupUx.unnamed}
                        </span>
                      </span>
                    </TableCell>
                    <TableCell>
                      {entry.publisher
                        ? displayPath(entry.publisher)
                        : ru.startupUx.unknownPublisher}
                    </TableCell>
                    <TableCell>{ru.startupUx.sources[entry.source]}</TableCell>
                    <TableCell>
                      <Badge variant="outline">
                        <Info aria-hidden="true" />
                        {stateText(entry.enabled)}
                      </Badge>
                    </TableCell>
                    <TableCell>{ru.startupUx.scopes[entry.scope]}</TableCell>
                    <TableCell>
                      <Button
                        size="sm"
                        variant="ghost"
                        aria-label={`${ru.startupUx.details}: ${displayPath(entry.name) || ru.startupUx.unnamed}`}
                        onClick={(event) => {
                          returnTo.current = event.currentTarget
                          c.setSelected(entry.id)
                        }}
                      >
                        {ru.open}
                      </Button>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
          <div className="cleanup-pagination">
            <Button
              size="sm"
              variant="outline"
              disabled={!c.query.offset}
              aria-label={ru.cleanupUx.previousPage}
              onClick={() =>
                c.setQuery({
                  ...c.query,
                  offset: Math.max(0, c.query.offset - 100),
                })
              }
            >
              ←
            </Button>
            <span>
              {c.query.offset + 1}–{c.query.offset + page.entries.length}{' '}
              {ru.cleanupUx.of} {page.total.toLocaleString('ru')}
            </span>
            <Button
              size="sm"
              variant="outline"
              disabled={c.query.offset + page.entries.length >= page.total}
              aria-label={ru.cleanupUx.nextPage}
              onClick={() =>
                c.setQuery({ ...c.query, offset: c.query.offset + 100 })
              }
            >
              →
            </Button>
          </div>
        </>
      ) : catalog && !c.error ? (
        <div className="empty-state">
          <FileCog aria-hidden="true" />
          <h2>{catalog.total ? ru.startupUx.noMatches : ru.startupUx.empty}</h2>
        </div>
      ) : null}
      <div className="startup-toolbar">
        <div>
          <Button
            variant="outline"
            disabled={busy || !catalog || !!c.error}
            onClick={c.copy}
          >
            {ru.startupUx.copy}
          </Button>
          <p className="supporting-text mt-2">{ru.startupUx.copyHint}</p>
        </div>
        <p role="status">{c.copied ? ru.startupUx.copied : ''}</p>
      </div>
      {selected && (
        <Sheet
          open
          onOpenChange={(open) => {
            if (!open) c.setSelected(null)
          }}
        >
          <SheetContent
            className="cleanup-drawer"
            onCloseAutoFocus={(event) => {
              event.preventDefault()
              ;(returnTo.current?.isConnected
                ? returnTo.current
                : document.getElementById('page-title')
              )?.focus()
            }}
          >
            <SheetHeader>
              <SheetTitle>
                {displayPath(selected.name) || ru.startupUx.unnamed}
              </SheetTitle>
              <SheetDescription>
                {ru.startupUx.sources[selected.source]} ·{' '}
                {ru.startupUx.scopes[selected.scope]}
              </SheetDescription>
            </SheetHeader>
            {catalog?.fixture_mode && (
              <p className="notice">{ru.cleanupUx.fixture}</p>
            )}
            <p className="notice">{ru.startupUx.readonly}</p>
            {c.revealError && (
              <p className="notice" role="alert">
                {cleanupErrorMessage(c.revealError)}
              </p>
            )}
            <dl className="startup-details">
              <dt>{ru.startupUx.publisher}</dt>
              <dd>
                {selected.publisher
                  ? displayPath(selected.publisher)
                  : ru.startupUx.unknownPublisher}
              </dd>
              <dt>{ru.startupUx.state}</dt>
              <dd>{stateText(selected.enabled)}</dd>
              <dt>{ru.startupUx.rights}</dt>
              <dd>{ru.startupUx.readonlyCell}</dd>
              <dt>{ru.startupUx.path}</dt>
              <dd dir="ltr">
                <bdi>{displayPath(selected.path)}</bdi>
              </dd>
              {selected.command !== null && (
                <>
                  <dt>{ru.startupUx.command}</dt>
                  <dd className="startup-command" dir="ltr">
                    <bdi>
                      {displayPath(selected.command) ||
                        ru.startupUx.emptyCommand}
                    </bdi>
                  </dd>
                </>
              )}
            </dl>
            <p className="supporting-text">{ru.startupUx.commandHint}</p>
            <Button
              variant="outline"
              disabled={!selected.can_reveal || c.revealing}
              onClick={() => c.reveal(selected.id)}
            >
              {ru.storage.reveal}
            </Button>
            {!selected.can_reveal && (
              <p className="supporting-text">{ru.startupUx.locationHint}</p>
            )}
          </SheetContent>
        </Sheet>
      )}
    </section>
  )
}
