import { readFileSync } from 'node:fs'
import { expect, test } from 'vitest'

const css = readFileSync('src/index.css', 'utf8')

function luminance(hex: string) {
  const rgb = [1, 3, 5]
    .map((index) => parseInt(hex.slice(index, index + 2), 16) / 255)
    .map((value) =>
      value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4,
    )
  return rgb[0]! * 0.2126 + rgb[1]! * 0.7152 + rgb[2]! * 0.0722
}
function contrast(a: string, b: string) {
  const values = [luminance(a), luminance(b)].sort((x, y) => y - x)
  return (values[0]! + 0.05) / (values[1]! + 0.05)
}
test('all literal colors are monochrome and both themes keep text/control contrast', () => {
  for (const [color] of css.matchAll(/#[\da-f]{6}\b/gi))
    expect(color).toMatch(/^#([\da-f]{2})\1\1$/i)
  const themes = [...css.matchAll(/:root(?:\.dark)?\s*\{([^}]+)\}/g)]
  expect(themes).toHaveLength(2)
  for (const theme of themes) {
    const colors = Object.fromEntries(
      [...theme[1]!.matchAll(/(--[\w-]+):\s*(#[\da-f]{6})/gi)].map((match) => [
        match[1]!,
        match[2]!,
      ]),
    )
    for (const [fg, bg, minimum] of [
      ['--foreground', '--card', 4.5],
      ['--muted-foreground', '--background', 4.5],
      ['--muted-foreground', '--muted', 4.5],
      ['--control-border', '--card', 3],
      ['--control-border', '--background', 3],
      ['--ring', '--card', 3],
    ] as const)
      expect(contrast(colors[fg]!, colors[bg]!)).toBeGreaterThanOrEqual(minimum)
  }
})
