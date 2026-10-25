export type Tile = {
  id: number
  bytes: bigint
  x: number
  y: number
  width: number
  height: number
}

export function formatShare(bytes: bigint, total: bigint): string {
  if (!total) return '—'
  const tenths = (bytes * 1000n + total / 2n) / total
  return `${tenths / 10n},${tenths % 10n} %`
}
export function treemap(items: { id: number; bytes: bigint }[]): Tile[] {
  const result: Tile[] = []
  function split(
    rows: typeof items,
    x: number,
    y: number,
    width: number,
    height: number,
  ) {
    if (!rows.length) return
    if (rows.length === 1) {
      result.push({ ...rows[0]!, x, y, width, height })
      return
    }
    const total = rows.reduce((sum, n) => sum + n.bytes, 0n)
    let sum = 0n,
      index = 0
    while (index < rows.length - 1 && sum * 2n < total) {
      sum += rows[index]!.bytes
      index++
    }
    const ratio = Number((sum * 1_000_000n) / total) / 1_000_000
    if (width >= height) {
      split(rows.slice(0, index), x, y, width * ratio, height)
      split(
        rows.slice(index),
        x + width * ratio,
        y,
        width * (1 - ratio),
        height,
      )
    } else {
      split(rows.slice(0, index), x, y, width, height * ratio)
      split(
        rows.slice(index),
        x,
        y + height * ratio,
        width,
        height * (1 - ratio),
      )
    }
  }
  split(
    items.filter((n) => n.bytes > 0n),
    0,
    0,
    100,
    100,
  )
  return result
}
