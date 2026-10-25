import { expect, test } from 'vitest'
import { formatShare, treemap } from './treemap'
test('treemap keeps large integer proportions, finite geometry and the complete area', () => {
  const tiles = treemap([
    { id: 1, bytes: 2n ** 62n },
    { id: 2, bytes: 2n ** 61n },
    { id: 3, bytes: 0n },
  ])
  expect(tiles).toHaveLength(2)
  expect(tiles.reduce((sum, t) => sum + t.width * t.height, 0)).toBeCloseTo(
    10000,
    4,
  )
  expect((tiles[0]!.width * tiles[0]!.height) / 10000).toBeCloseTo(2 / 3, 4)
  expect(
    tiles.every((t) =>
      [t.x, t.y, t.width, t.height].every(
        (n) => Number.isFinite(n) && n >= 0 && n <= 100,
      ),
    ),
  ).toBe(true)
  expect(treemap([])).toEqual([])
  expect(formatShare(2n ** 62n, 3n * 2n ** 61n)).toBe('66,7 %')
  expect(formatShare(0n, 0n)).toBe('—')
})
