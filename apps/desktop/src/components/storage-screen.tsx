import { useState } from 'react'
import { CircleAlert, HardDrive, Search } from 'lucide-react'
import type { StorageController } from '@/hooks/use-storage'
import {
  cleanupErrorMessage,
  byteCount,
  CleanupError,
  displayPath,
  formatCount,
} from '@/lib/cleanup-api'
import { ru } from '@/i18n/ru'
import { Button } from '@/ui/button'
import { Input } from '@/ui/input'
import { Card, CardContent } from '@/ui/card'
import { Progress } from '@/ui/progress'
import {
  Select,
  SelectTrigger,
  SelectContent,
  SelectItem,
  SelectValue,
} from '@/ui/select'
import { Breadcrumb, BreadcrumbItem, BreadcrumbList } from '@/ui/breadcrumb'
import { StorageMap } from './storage-map'
import { StorageList } from './storage-list'
import { formatShare } from '@/lib/treemap'

export function Volumes({
  controller: c,
  onChoose,
}: {
  controller: StorageController
  onChoose: (path: string) => void
}) {
  return (
    <section className="storage-volumes" aria-label={ru.storage.volumes}>
      {c.volumes.isPending ? (
        <p role="status">{ru.cleanupUx.collecting}</p>
      ) : c.volumes.error ? (
        <p role="alert">{cleanupErrorMessage(c.volumes.error)}</p>
      ) : !c.volumes.data?.length ? (
        <p>{ru.storage.unavailable}</p>
      ) : (
        c.volumes.data.map((volume) => (
          <Card key={volume.path}>
            <CardContent className="pt-5">
              <h2 className="flex items-center gap-2 font-medium">
                <HardDrive className="size-4" aria-hidden="true" />
                {volume.path}{' '}
                <span className="supporting-text">{volume.filesystem}</span>
              </h2>
              <dl className="storage-volume-sizes">
                <div>
                  <dt>{ru.storage.used}</dt>
                  <dd className="numeric">
                    {volume.total_bytes !== null &&
                    volume.available_bytes !== null &&
                    byteCount(volume.total_bytes) >=
                      byteCount(volume.available_bytes)
                      ? formatCount(
                          byteCount(volume.total_bytes) -
                            byteCount(volume.available_bytes),
                        )
                      : '—'}
                  </dd>
                </div>
                <div>
                  <dt>{ru.storage.available}</dt>
                  <dd className="numeric">
                    {volume.available_bytes === null
                      ? '—'
                      : formatCount(volume.available_bytes)}
                  </dd>
                </div>
                <div>
                  <dt>{ru.storage.capacity}</dt>
                  <dd className="numeric">
                    {volume.total_bytes === null
                      ? '—'
                      : formatCount(volume.total_bytes)}
                  </dd>
                </div>
              </dl>
              {volume.error && (
                <p className="supporting-text">
                  {volume.error === 'unsupported'
                    ? ru.storage.unsupported
                    : ru.storage.unavailable}
                </p>
              )}
              {volume.last_scan_ms != null && (
                <p className="supporting-text mt-3">
                  {ru.storage.updated}:{' '}
                  {new Date(volume.last_scan_ms).toLocaleString('ru-RU')}
                  <br />
                  <bdi dir="ltr">
                    {displayPath(volume.last_scan_path ?? '')}
                  </bdi>
                </p>
              )}
              <Button
                className="mt-3"
                variant="outline"
                disabled={!!volume.error || c.running}
                onClick={() => onChoose(volume.path)}
              >
                {ru.open}
              </Button>
            </CardContent>
          </Card>
        ))
      )}
      <p className="supporting-text storage-volume-note">{ru.storage.quota}</p>
      <Button
        className="storage-volume-note justify-self-start"
        size="sm"
        variant="ghost"
        disabled={c.volumes.isFetching}
        onClick={() => void c.volumes.refetch()}
      >
        {ru.cleanupUx.reload}
      </Button>
    </section>
  )
}

export function StorageScreen({
  controller: c,
  fixture,
}: {
  controller: StorageController
  fixture: boolean
}) {
  const [mapView, setMapView] = useState(true)
  const [filters, setFilters] = useState({
    search: '',
    extension: '',
    min: '',
    before: '',
  })
  const [copied, setCopied] = useState<{ id: number; text: string } | null>(
    null,
  )
  const page = c.page.data,
    map = c.map.data
  const selected = [...(page?.rows ?? []), ...(map?.rows ?? [])].find(
    (n) => n.id === c.selected,
  )
  const apply = () => {
    const numeric = Number(filters.min)
    if (!Number.isSafeInteger(numeric) || numeric < 0) return
    const date = filters.before
      ? Date.parse(`${filters.before}T00:00:00`)
      : null
    if (date !== null && (!Number.isFinite(date) || date < 0)) return
    c.setQuery({
      ...c.query,
      search: filters.search,
      extension: filters.extension,
      min_bytes: (BigInt(numeric) * 1048576n).toString(),
      modified_before_ms: date,
      offset: 0,
    })
  }
  const enter = (id: number) => {
    c.navigate(id)
    setFilters({ search: '', extension: '', min: '', before: '' })
    setCopied(null)
  }
  return (
    <section className="storage-screen" aria-label={ru.disks}>
      {fixture && <p className="notice">{ru.cleanupUx.fixture}</p>}
      <Volumes controller={c} onChoose={(path) => c.setPath(path)} />
      <form
        className="storage-target"
        onSubmit={(event) => {
          event.preventDefault()
          c.start()
        }}
      >
        <label className="min-w-0 flex-1">
          {ru.storage.path}
          <Input
            value={c.path}
            onChange={(event) => c.setPath(event.target.value)}
            maxLength={32000}
            disabled={c.running}
            dir="ltr"
          />
        </label>
        <Button disabled={!c.canStart} type="submit">
          <Search aria-hidden="true" />
          {page ? ru.cleanupUx.rescan : ru.scan}
        </Button>
      </form>
      <p className="supporting-text">{ru.storage.readonly}</p>
      {c.error && (
        <p className="notice" role="alert">
          <CircleAlert aria-hidden="true" />
          {cleanupErrorMessage(c.error)}
        </p>
      )}
      {c.task.data?.error && (
        <p className="notice" role="alert">
          {cleanupErrorMessage(new CleanupError(c.task.data.error))}
        </p>
      )}
      {c.running ? (
        <div className="cleanup-progress" role="status">
          <h2>{ru.storage.scanning}</h2>
          <p>
            {ru.cleanupUx.viewed}:{' '}
            {c.task.data?.processed.toLocaleString('ru') ?? 0}
          </p>
          <Progress aria-label={ru.storage.scanning} />
          <Button
            variant="outline"
            disabled={!c.task.data || c.cancelling}
            onClick={c.cancel}
          >
            {ru.cleanupUx.cancel}
          </Button>
        </div>
      ) : c.task.data?.status === 'cancelled' ? (
        <p role="status" className="notice">
          {ru.storage.cancelled}
        </p>
      ) : c.page.isFetching && !page ? (
        <p role="status">{ru.cleanupUx.collecting}</p>
      ) : page ? (
        <>
          {(page.truncated || page.issues.length > 0) && (
            <div className="notice">
              <CircleAlert aria-hidden="true" />
              <div>
                <strong>
                  {page.truncated ? ru.storage.limited : ru.storage.partial}
                </strong>
                <ul>
                  {page.issues.map((issue, i) => (
                    <li key={i}>
                      {ru.cleanupReasons[issue.reason]}:{' '}
                      {issue.count.toLocaleString('ru')}
                    </li>
                  ))}
                </ul>
              </div>
            </div>
          )}
          {c.stale && <p className="notice">{ru.storage.stale}</p>}
          <div className="storage-summary">
            <strong>
              {ru.storage.total}:{' '}
              <span className="numeric">{formatCount(page.root.bytes)}</span>
            </strong>
            <span>
              {ru.storage.logical}: {formatCount(page.root.logical_bytes)}
            </span>
            <span>
              {ru.storage.updated}:{' '}
              {new Date(page.completed_ms).toLocaleString('ru-RU')}
            </span>
          </div>
          <Breadcrumb>
            <BreadcrumbList>
              {page.ancestors.map((n) => (
                <BreadcrumbItem key={n.id}>
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => enter(n.id)}
                    aria-current={
                      n.id === page.current.id ? 'location' : undefined
                    }
                  >
                    {displayPath(n.name)}
                  </Button>
                  <span aria-hidden="true">/</span>
                </BreadcrumbItem>
              ))}
            </BreadcrumbList>
          </Breadcrumb>
          <div className="storage-layout">
            <div className="min-w-0">
              <div className="storage-view-controls">
                <Button
                  variant={mapView ? 'secondary' : 'outline'}
                  aria-pressed={mapView}
                  onClick={() => setMapView(true)}
                >
                  {ru.map}
                </Button>
                <Button
                  variant={!mapView ? 'secondary' : 'outline'}
                  aria-pressed={!mapView}
                  onClick={() => setMapView(false)}
                >
                  {ru.tree}
                </Button>
                <Select
                  value={c.query.view}
                  onValueChange={(value) =>
                    c.setQuery({
                      ...c.query,
                      view: value === 'files' ? 'files' : 'children',
                      offset: 0,
                    })
                  }
                >
                  <SelectTrigger aria-label={ru.disks}>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="children">
                      {ru.storage.children}
                    </SelectItem>
                    <SelectItem value="files">{ru.storage.files}</SelectItem>
                  </SelectContent>
                </Select>
              </div>
              {mapView && map && (
                <StorageMap
                  rows={map.rows}
                  total={map.current.bytes}
                  selected={c.selected}
                  onSelect={c.setSelected}
                />
              )}
              <form
                className="storage-filters"
                onSubmit={(event) => {
                  event.preventDefault()
                  apply()
                }}
              >
                <label>
                  {ru.storage.search}
                  <Input
                    value={filters.search}
                    maxLength={256}
                    onChange={(e) =>
                      setFilters({ ...filters, search: e.target.value })
                    }
                  />
                </label>
                <label>
                  {ru.storage.extension}
                  <Input
                    value={filters.extension}
                    maxLength={32}
                    onChange={(e) =>
                      setFilters({ ...filters, extension: e.target.value })
                    }
                  />
                </label>
                <label>
                  {ru.storage.min}
                  <Input
                    type="number"
                    min="0"
                    max="17592186044415"
                    step="1"
                    value={filters.min}
                    onChange={(e) =>
                      setFilters({ ...filters, min: e.target.value })
                    }
                  />
                </label>
                <label>
                  {ru.storage.before}
                  <Input
                    type="date"
                    min="1970-01-01"
                    value={filters.before}
                    onChange={(e) =>
                      setFilters({ ...filters, before: e.target.value })
                    }
                  />
                </label>
                <Button type="submit" variant="outline">
                  {ru.storage.apply}
                </Button>
              </form>
              <Select
                value={c.query.sort}
                onValueChange={(value) =>
                  c.setQuery({
                    ...c.query,
                    sort:
                      value === 'name'
                        ? 'name'
                        : value === 'modified'
                          ? 'modified'
                          : 'size',
                    offset: 0,
                  })
                }
              >
                <SelectTrigger
                  aria-label={ru.storage.sort}
                  className="mb-3 w-48"
                >
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="size">{ru.storage.size}</SelectItem>
                  <SelectItem value="name">{ru.storage.name}</SelectItem>
                  <SelectItem value="modified">{ru.storage.date}</SelectItem>
                </SelectContent>
              </Select>
              <StorageList
                key={`${page.scan_id}-${page.current.id}-${c.query.offset}`}
                rows={page.rows}
                offset={c.query.offset}
                total={page.total}
                selected={c.selected}
                onSelect={c.setSelected}
                onEnter={enter}
              />
              <div className="cleanup-pagination">
                <Button
                  variant="outline"
                  size="sm"
                  disabled={!c.query.offset || c.page.isFetching}
                  onClick={() =>
                    c.setQuery({
                      ...c.query,
                      offset: Math.max(0, c.query.offset - 500),
                    })
                  }
                  aria-label={ru.cleanupUx.previousPage}
                >
                  ←
                </Button>
                <span>
                  {page.total ? c.query.offset + 1 : 0}–
                  {Math.min(c.query.offset + page.rows.length, page.total)}{' '}
                  {ru.cleanupUx.of} {page.total.toLocaleString('ru')}
                </span>
                <Button
                  variant="outline"
                  size="sm"
                  disabled={
                    c.query.offset + page.rows.length >= page.total ||
                    c.page.isFetching
                  }
                  onClick={() =>
                    c.setQuery({ ...c.query, offset: c.query.offset + 500 })
                  }
                  aria-label={ru.cleanupUx.nextPage}
                >
                  →
                </Button>
              </div>
            </div>
            <aside
              className="storage-inspector"
              aria-label={ru.storage.inspect}
            >
              <h2>{ru.storage.inspect}</h2>
              {selected ? (
                <>
                  <h3>{displayPath(selected.name)}</h3>
                  <p className="storage-path" dir="ltr">
                    <bdi>{displayPath(selected.path)}</bdi>
                  </p>
                  <dl>
                    <dt>{ru.cleanupUx.allocated}</dt>
                    <dd className="numeric">{formatCount(selected.bytes)}</dd>
                    <dt>{ru.storage.share}</dt>
                    <dd className="numeric">
                      {formatShare(
                        byteCount(selected.bytes),
                        byteCount(page.current.bytes),
                      )}
                    </dd>
                    <dt>{ru.storage.logical}</dt>
                    <dd className="numeric">
                      {formatCount(selected.logical_bytes)}
                    </dd>
                    <dt>{ru.storage.modified}</dt>
                    <dd>
                      {selected.modified_ms === null
                        ? '—'
                        : new Date(selected.modified_ms).toLocaleString(
                            'ru-RU',
                          )}
                    </dd>
                  </dl>
                  {selected.skipped && (
                    <p className="notice">
                      {ru.cleanupReasons[selected.skipped]}
                    </p>
                  )}
                  {selected.directory && (
                    <Button
                      variant="outline"
                      disabled={!!selected.skipped}
                      onClick={() => enter(selected.id)}
                    >
                      {ru.storage.enter}
                    </Button>
                  )}
                  <Button
                    variant="outline"
                    onClick={() =>
                      void Promise.resolve()
                        .then(() =>
                          navigator.clipboard.writeText(selected.path),
                        )
                        .then(() =>
                          setCopied({
                            id: selected.id,
                            text: ru.storage.copied,
                          }),
                        )
                        .catch(() =>
                          setCopied({
                            id: selected.id,
                            text: ru.storage.copyFailed,
                          }),
                        )
                    }
                  >
                    {ru.storage.copy}
                  </Button>
                  <Button variant="outline" disabled>
                    {ru.cleanupUx.confirm}
                  </Button>
                  <Button
                    variant="outline"
                    disabled={!!selected.skipped || c.revealing}
                    onClick={() => c.reveal(selected.id)}
                  >
                    {ru.storage.reveal}
                  </Button>
                  <p className="supporting-text">{ru.storage.readonly}</p>
                  <p role="status">
                    {copied?.id === selected.id ? copied.text : ''}
                  </p>
                </>
              ) : (
                <p className="supporting-text">{ru.storage.choose}</p>
              )}
            </aside>
          </div>
          <p className="supporting-text mt-4">{ru.storage.allocations}</p>
        </>
      ) : (
        <div className="empty-state">
          <HardDrive aria-hidden="true" />
          <h2>{ru.storage.intro}</h2>
          <p>{ru.storage.description}</p>
        </div>
      )}
    </section>
  )
}
