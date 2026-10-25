import { useId } from 'react'
import { getNonce } from 'get-nonce'
import type { StorageNode } from '@/generated/StorageNode'
import { byteCount, displayPath, formatCount } from '@/lib/cleanup-api'
import { treemap } from '@/lib/treemap'
import { ru } from '@/i18n/ru'

export function StorageMap({
  rows,
  total,
  selected,
  onSelect,
}: {
  rows: StorageNode[]
  total: string
  selected: number | null
  onSelect: (id: number) => void
}) {
  const id = useId()
  const sum = rows.reduce((s, n) => s + byteCount(n.bytes), 0n)
  const other = byteCount(total) > sum ? byteCount(total) - sum : 0n
  const tiles = treemap([
    ...rows.map((n) => ({ id: n.id, bytes: byteCount(n.bytes) })),
    { id: -1, bytes: other },
  ])
  return (
    <>
      <p className="supporting-text">{ru.storage.mapDescription}</p>
      <div className="storage-map" data-storage-map={id} aria-label={ru.map}>
        <style nonce={getNonce()}>
          {tiles
            .map(
              (t, i) =>
                `[data-storage-map="${id}"] [data-tile="${i}"]{left:${t.x}%;top:${t.y}%;width:${t.width}%;height:${t.height}%}`,
            )
            .join('')}
        </style>
        {!tiles.length && <p className="p-6">{ru.storage.mapEmpty}</p>}
        {tiles.map((t, i) => {
          const row = rows.find((n) => n.id === t.id)
          const text = `${row ? displayPath(row.name) : ru.storage.other} · ${formatCount(t.bytes)}`
          return row ? (
            <button
              key={t.id}
              type="button"
              data-tile={i}
              className="storage-tile"
              title={text}
              aria-label={text}
              aria-pressed={selected === t.id}
              onClick={() => onSelect(t.id)}
            >
              <span>{displayPath(row.name)}</span>
              <strong className="numeric">{formatCount(t.bytes)}</strong>
            </button>
          ) : (
            <div
              key="other"
              data-tile={i}
              className="storage-tile storage-other"
              title={text}
            >
              <span>{ru.storage.other}</span>
              <strong className="numeric">{formatCount(t.bytes)}</strong>
            </div>
          )
        })}
      </div>
    </>
  )
}
