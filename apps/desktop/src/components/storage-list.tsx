import { useId, useRef, useState } from 'react'
import { getNonce } from 'get-nonce'
import { File, Folder, ChevronRight } from 'lucide-react'
import type { StorageNode } from '@/generated/StorageNode'
import { ru } from '@/i18n/ru'
import { displayPath, formatCount } from '@/lib/cleanup-api'
import { Button } from '@/ui/button'

export function StorageList({
  rows,
  offset,
  total,
  selected,
  onSelect,
  onEnter,
}: {
  rows: StorageNode[]
  offset: number
  total: number
  selected: number | null
  onSelect: (id: number) => void
  onEnter: (id: number) => void
}) {
  const id = useId(),
    viewport = useRef<HTMLDivElement>(null)
  const [top, setTop] = useState(0)
  const start = Math.max(0, Math.floor(top / 64) - 2),
    end = Math.min(rows.length, start + 12)
  const focus = (index: number) => {
    if (index < 0 || index >= rows.length) return
    if (viewport.current) viewport.current.scrollTop = index * 64
    setTop(index * 64)
    requestAnimationFrame(() =>
      document.getElementById(`${id}-${index}`)?.focus(),
    )
  }
  return (
    <div
      ref={viewport}
      className="storage-list"
      data-storage-list={id}
      onScroll={(e) => setTop(e.currentTarget.scrollTop)}
      role="list"
      aria-label={ru.storage.list}
    >
      <style nonce={getNonce()}>
        {`[data-storage-list="${id}"] .storage-list-space{height:${rows.length * 64}px}`}
        {rows
          .map(
            (_, i) =>
              `[data-storage-list="${id}"] [data-row="${i}"]{top:${i * 64}px}`,
          )
          .join('')}
      </style>
      {!rows.length && <p className="p-6">{ru.storage.empty}</p>}
      <div className="storage-list-space">
        {rows.slice(start, end).map((row, i) => {
          const index = start + i
          return (
            <div
              key={row.id}
              role="listitem"
              aria-posinset={offset + index + 1}
              aria-setsize={total}
              className="storage-row"
              data-row={index}
            >
              <Button
                id={`${id}-${index}`}
                variant="ghost"
                className="storage-row-select"
                aria-pressed={selected === row.id}
                onClick={() => onSelect(row.id)}
                onKeyDown={(event) => {
                  const next =
                    event.key === 'ArrowDown'
                      ? index + 1
                      : event.key === 'ArrowUp'
                        ? index - 1
                        : event.key === 'Home'
                          ? 0
                          : event.key === 'End'
                            ? rows.length - 1
                            : null
                  if (next !== null) {
                    event.preventDefault()
                    focus(next)
                  }
                }}
              >
                {row.directory ? (
                  <Folder aria-hidden="true" />
                ) : (
                  <File aria-hidden="true" />
                )}
                <span className="min-w-0">
                  <span
                    className="block truncate"
                    title={displayPath(row.name)}
                  >
                    {displayPath(row.name)}
                  </span>
                  {row.skipped && (
                    <span className="row-detail truncate">
                      {ru.cleanupReasons[row.skipped]}
                    </span>
                  )}
                </span>
                <span className="numeric ms-auto">
                  {formatCount(row.bytes)}
                </span>
              </Button>
              {row.directory && (
                <Button
                  size="icon"
                  variant="ghost"
                  aria-label={`${ru.storage.enter}: ${displayPath(row.name)}`}
                  onClick={() => onEnter(row.id)}
                  disabled={!!row.skipped}
                >
                  <ChevronRight aria-hidden="true" />
                </Button>
              )}
            </div>
          )
        })}
      </div>
    </div>
  )
}
