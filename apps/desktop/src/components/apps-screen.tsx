import { useRef } from 'react'
import { Package, CircleAlert } from 'lucide-react'
import type { AppsController } from '@/hooks/use-apps'
import type { AppKind } from '@/generated/AppKind'
import type { AppSource } from '@/generated/AppSource'
import type { AppSort } from '@/generated/AppSort'
import {
  cleanupErrorMessage,
  displayPath,
  formatCount,
} from '@/lib/cleanup-api'
import { ru } from '@/i18n/ru'
import { Button } from '@/ui/button'
import { Input } from '@/ui/input'
import { Badge } from '@/ui/badge'
import {
  Table,
  TableHeader,
  TableBody,
  TableRow,
  TableHead,
  TableCell,
} from '@/ui/table'
import {
  Select,
  SelectTrigger,
  SelectContent,
  SelectValue,
  SelectItem,
} from '@/ui/select'
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetDescription,
} from '@/ui/sheet'
const kinds: AppKind[] = ['win32', 'msi', 'msix']
const sources: AppSource[] = ['hkcu64', 'hkcu32', 'hklm64', 'hklm32', 'msix']
const sorts: AppSort[] = ['name', 'publisher', 'size']
const u = ru.appsUx

export function AppsScreen({ controller: c }: { controller: AppsController }) {
  const returnTo = useRef<HTMLElement | null>(null)
  const catalog = c.catalog.data,
    page = c.page.data
  const selected = page?.entries.find((e) => e.id === c.selected)
  const loading = c.catalog.isFetching || c.page.isFetching
  return (
    <section className="apps-screen" aria-label={ru.apps}>
      {catalog?.fixture_mode && (
        <p className="notice">{ru.cleanupUx.fixture}</p>
      )}
      <div className="startup-toolbar">
        <p>{u.readonly}</p>
        <Button variant="outline" disabled={loading} onClick={c.refresh}>
          {u.refresh}
        </Button>
      </div>
      {c.error && (
        <p role="alert" className="notice">
          <CircleAlert aria-hidden="true" />
          {cleanupErrorMessage(c.error)}
        </p>
      )}
      {catalog && (
        <p className="supporting-text">
          {u.count}: {catalog.total.toLocaleString('ru')} · {u.collected}:{' '}
          {new Date(catalog.collected_ms).toLocaleString('ru-RU')}
        </p>
      )}
      {!!catalog?.issues.length && (
        <div className="notice">
          <CircleAlert aria-hidden="true" />
          <div>
            <strong>{catalog.truncated ? u.limit : u.partial}</strong>
            <ul>
              {catalog.issues.map((i, index) => (
                <li key={index}>
                  {u.sources[i.source]}: {u.issues[i.reason]} ({i.count})
                </li>
              ))}
            </ul>
          </div>
        </div>
      )}
      <div className="apps-filters">
        <Input
          aria-label={u.search}
          placeholder={u.search}
          maxLength={256}
          value={c.query.search}
          onChange={(e) =>
            c.setQuery({ ...c.query, search: e.target.value, offset: 0 })
          }
        />
        <Select
          value={c.query.kind ?? 'all'}
          onValueChange={(v) =>
            c.setQuery({
              ...c.query,
              kind: kinds.find((k) => k === v) ?? null,
              offset: 0,
            })
          }
        >
          <SelectTrigger aria-label={u.kind}>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">{u.allKinds}</SelectItem>
            {kinds.map((k) => (
              <SelectItem key={k} value={k}>
                {u.kinds[k]}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <Select
          value={c.query.source ?? 'all'}
          onValueChange={(v) =>
            c.setQuery({
              ...c.query,
              source: sources.find((s) => s === v) ?? null,
              offset: 0,
            })
          }
        >
          <SelectTrigger aria-label={u.source}>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">{u.allSources}</SelectItem>
            {sources.map((s) => (
              <SelectItem key={s} value={s}>
                {u.sources[s]}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <Select
          value={c.query.sort}
          onValueChange={(v) =>
            c.setQuery({
              ...c.query,
              sort: sorts.find((s) => s === v) ?? 'name',
              offset: 0,
            })
          }
        >
          <SelectTrigger aria-label={u.sort}>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {sorts.map((s) => (
              <SelectItem key={s} value={s}>
                {u.sorts[s]}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
      {loading || (!c.error && c.catalog.isPending) ? (
        <p role="status">{u.loading}</p>
      ) : page?.entries.length ? (
        <>
          <div className="startup-table">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>{u.program}</TableHead>
                  <TableHead>{u.publisher}</TableHead>
                  <TableHead>{u.version}</TableHead>
                  <TableHead>{u.kind}</TableHead>
                  <TableHead>{u.size}</TableHead>
                  <TableHead>
                    <span className="sr-only">{u.details}</span>
                  </TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {page.entries.map((e) => (
                  <TableRow key={e.id}>
                    <TableCell>
                      <span className="startup-name">
                        <Package
                          className="size-4 shrink-0"
                          aria-hidden="true"
                        />
                        <span title={displayPath(e.name)}>
                          {displayPath(e.name)}
                        </span>
                      </span>
                      {e.system_component === true && (
                        <Badge variant="secondary">{u.system}</Badge>
                      )}
                    </TableCell>
                    <TableCell>
                      <span className="apps-cell">
                        {e.publisher ? displayPath(e.publisher) : u.unknown}
                      </span>
                    </TableCell>
                    <TableCell>
                      {e.version ? displayPath(e.version) : u.unknown}
                    </TableCell>
                    <TableCell>{u.kinds[e.kind]}</TableCell>
                    <TableCell className="numeric">
                      {e.estimated_bytes === null
                        ? u.unknown
                        : formatCount(e.estimated_bytes)}
                    </TableCell>
                    <TableCell>
                      <Button
                        variant="ghost"
                        size="sm"
                        aria-label={`${u.details}: ${e.name}`}
                        onClick={(event) => {
                          returnTo.current = event.currentTarget
                          c.setSelected(e.id)
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
              aria-label={ru.cleanupUx.previousPage}
              disabled={!c.query.offset}
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
              aria-label={ru.cleanupUx.nextPage}
              disabled={c.query.offset + page.entries.length >= page.total}
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
          <Package aria-hidden="true" />
          <h2>{catalog.total ? u.noMatches : u.empty}</h2>
        </div>
      ) : null}
      <p className="supporting-text">{u.sizeHelp}</p>
      {selected && (
        <Sheet
          open
          onOpenChange={(open) => {
            if (!open) c.setSelected(null)
          }}
        >
          <SheetContent
            className="cleanup-drawer"
            onCloseAutoFocus={(e) => {
              e.preventDefault()
              ;(returnTo.current?.isConnected
                ? returnTo.current
                : document.getElementById('page-title')
              )?.focus()
            }}
          >
            <SheetHeader>
              <SheetTitle>{displayPath(selected.name)}</SheetTitle>
              <SheetDescription>
                {u.kinds[selected.kind]} · {u.sources[selected.source]}
              </SheetDescription>
            </SheetHeader>
            {catalog?.fixture_mode && (
              <p className="notice">{ru.cleanupUx.fixture}</p>
            )}
            {selected.kind === 'msi' &&
              selected.system_component === false &&
              selected.uninstall_block !== 'protected' && (
                <div className="apps-check">
                  <Button
                    variant="outline"
                    disabled={c.checking}
                    onClick={() => c.check(selected.id)}
                  >
                    {c.checking ? u.checking : u.check}
                  </Button>
                  <p className="supporting-text">{u.checkHint}</p>
                  {c.checkError && (
                    <p className="notice" role="alert">
                      {cleanupErrorMessage(c.checkError)}
                    </p>
                  )}
                  {c.checkResult && (
                    <div className="notice" role="status">
                      <div>
                        <p>{u.checkStatuses[c.checkResult.status]}</p>
                        {c.checkResult.binary && (
                          <p className="supporting-text">
                            {u.binaryStatuses[c.checkResult.binary]}
                          </p>
                        )}
                        {c.checkResult.preview && (
                          <p className="supporting-text">
                            {u.contexts[c.checkResult.preview.context]}
                          </p>
                        )}
                      </div>
                    </div>
                  )}
                </div>
              )}
            <dl className="startup-details">
              <dt>{u.publisher}</dt>
              <dd>
                {selected.publisher
                  ? displayPath(selected.publisher)
                  : u.unknown}
              </dd>
              <dt>{u.version}</dt>
              <dd>
                {selected.version ? displayPath(selected.version) : u.unknown}
              </dd>
              <dt>{u.size}</dt>
              <dd>
                {selected.estimated_bytes === null
                  ? u.unknown
                  : formatCount(selected.estimated_bytes)}
              </dd>
              <dt>{u.date}</dt>
              <dd>
                {selected.registered_date
                  ? displayPath(selected.registered_date)
                  : u.unknown}
              </dd>
              <dt>{u.location}</dt>
              <dd dir="ltr">
                <bdi>
                  {selected.install_location
                    ? displayPath(selected.install_location)
                    : u.unknown}
                </bdi>
              </dd>
              <dt>{u.registration}</dt>
              <dd dir="ltr">
                <bdi>{displayPath(selected.registration)}</bdi>
              </dd>
              <dt>{u.identity}</dt>
              <dd dir="ltr">
                <bdi>{displayPath(selected.id)}</bdi>
              </dd>
              <dt>{u.systemLabel}</dt>
              <dd>
                {selected.system_component === null
                  ? u.unknown
                  : selected.system_component
                    ? u.system
                    : u.notMarked}
              </dd>
            </dl>
            <div>
              <Button disabled>{u.uninstall}</Button>
              <p className="supporting-text mt-2">
                {u.blocked[selected.uninstall_block]}
              </p>
            </div>
            <div>
              <Button variant="outline" disabled>
                {u.leftovers}
              </Button>
              <p className="supporting-text mt-2">{u.leftoversHelp}</p>
            </div>
          </SheetContent>
        </Sheet>
      )}
    </section>
  )
}
