import { expect, test, type Page } from '@playwright/test'
import { ru } from '../src/i18n/ru'
import type { ApiRequest } from '../src/generated/ApiRequest'
import type { StartupEntry } from '../src/generated/StartupEntry'

async function fixture(
  page: Page,
  mode: 'ready' | 'empty' | 'partial' | 'error' | 'loading' = 'ready',
) {
  await page.addInitScript(
    ({ mode }) => {
      let failed = false
      const entries: StartupEntry[] = Array.from(
        { length: mode === 'empty' ? 0 : 205 },
        (_, id) => ({
          id,
          name: `Искусственная запись ${id.toString().padStart(3, '0')}`,
          source: id % 2 ? 'hklm_run32' : 'hkcu_run64',
          scope: id % 2 ? 'machine' : 'user',
          path: `HKEY_CURRENT_USER\\private fixture source ${id}`,
          command: '"%APPDATA%\\fixture test.exe" --secret "two words"',
          publisher: null,
          enabled: null,
          can_toggle: false,
          can_reveal: false,
        }),
      )
      Object.defineProperty(navigator, 'clipboard', {
        configurable: true,
        value: {
          writeText: async (text: string) =>
            Object.assign(window, { __startupCopy: text }),
        },
      })
      Object.assign(window, {
        __TAURI_INTERNALS__: {
          transformCallback: () => 1,
          unregisterCallback: () => {},
          invoke: async (_: string, args: { request: ApiRequest }) => {
            const req = args.request
            switch (req.type) {
              case 'get_preferences':
                return {
                  status: 'ok',
                  value: {
                    type: 'preferences',
                    data: {
                      schema_version: 1,
                      revision: null,
                      preferences: {
                        theme: 'system',
                        language: 'ru',
                        exclusions: [],
                      },
                    },
                  },
                }
              case 'get_cleanup_rules':
                return {
                  status: 'ok',
                  value: {
                    type: 'cleanup_rules',
                    data: {
                      rules: [],
                      fixture_mode: true,
                      max_plan_items: 500,
                    },
                  },
                }
              case 'list_volumes':
                return { status: 'ok', value: { type: 'volumes', data: [] } }
              case 'refresh_startup':
                if (mode === 'loading') return new Promise(() => {})
                if (mode === 'error' && !failed) {
                  failed = true
                  return { status: 'error', error: 'adapter_error' }
                }
                return {
                  status: 'ok',
                  value: {
                    type: 'startup_catalog',
                    data: {
                      id: 'startup-inventory',
                      total: entries.length,
                      issues:
                        mode === 'partial'
                          ? [
                              {
                                source: 'common_folder',
                                reason: 'access_denied',
                                count: 1,
                              },
                            ]
                          : [],
                      truncated: false,
                      fixture_mode: true,
                      collected_ms: 1600000000000,
                    },
                  },
                }
              case 'get_startup_page': {
                const q = req.data.query,
                  rows = entries.filter(
                    (e) =>
                      (!q.source || q.source === e.source) &&
                      (!q.scope || q.scope === e.scope) &&
                      e.name.toLowerCase().includes(q.search.toLowerCase()),
                  )
                return {
                  status: 'ok',
                  value: {
                    type: 'startup_page',
                    data: {
                      snapshot_id: 'startup-inventory',
                      total: rows.length,
                      offset: q.offset,
                      entries: rows.slice(q.offset, q.offset + q.limit),
                    },
                  },
                }
              }
              default:
                throw new Error(`Unexpected operation: ${req.type}`)
            }
          },
        },
      })
    },
    { mode },
  )
  await page.goto('/#/startup')
}
test('startup fixture: read-only rows, pages, filters, literal details and redacted export work', async ({
  page,
}, info) => {
  await fixture(page)
  await expect(page.getByRole('row')).toHaveCount(101)
  await expect(page.getByRole('switch')).not.toBeVisible()
  await page.screenshot({ path: info.outputPath('startup-table.png') })
  const details = page.getByRole('button', {
    name: `${ru.startupUx.details}: Искусственная запись 000`,
    exact: true,
  })
  await details.click()
  const dialog = page.getByRole('dialog')
  await expect(dialog).toContainText('--secret "two words"')
  await expect(
    dialog.getByRole('button', { name: ru.storage.reveal }),
  ).toBeDisabled()
  await page.keyboard.press('Control+k')
  await expect(
    page.getByRole('dialog', { name: ru.quickSearch }),
  ).not.toBeVisible()
  await page.screenshot({ path: info.outputPath('startup-details.png') })
  await page.keyboard.press('Escape')
  await expect(details).toBeFocused()
  await page.getByRole('button', { name: ru.startupUx.copy }).click()
  await expect(
    page.getByText(ru.startupUx.copied, { exact: true }),
  ).toBeVisible()
  const exported = await page.evaluate(
    () =>
      JSON.parse(
        (window as unknown as { __startupCopy: string }).__startupCopy,
      ) as { entries: Record<string, unknown>[] },
  )
  expect(exported.entries).toHaveLength(205)
  expect(exported.entries[0]).not.toHaveProperty('command')
  expect(exported.entries[0]).not.toHaveProperty('path')
  await page.getByRole('button', { name: ru.cleanupUx.nextPage }).click()
  await expect(page.getByText('101–200 из 205', { exact: true })).toBeVisible()
  await page.getByRole('textbox', { name: ru.startupUx.search }).fill('204')
  await expect(page.getByRole('row')).toHaveCount(2)
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true)
})
test('startup fixture: empty, partial, failed with retry and loading states are explicit', async ({
  page,
}, info) => {
  for (const mode of ['empty', 'partial', 'error', 'loading'] as const) {
    const next = await page.context().newPage()
    try {
      await fixture(next, mode)
      if (mode === 'empty')
        await expect(
          next.getByRole('heading', { name: ru.startupUx.empty }),
        ).toBeVisible()
      if (mode === 'partial')
        await expect(
          next.getByText(ru.startupUx.partial, { exact: true }),
        ).toBeVisible()
      if (mode === 'error') {
        await expect(next.getByRole('alert')).toContainText(
          ru.cleanupUx.unavailable,
        )
        await next.getByRole('button', { name: ru.startupUx.refresh }).click()
        await expect(next.getByRole('row')).toHaveCount(101)
      }
      if (mode === 'loading')
        await expect(
          next.getByText(ru.startupUx.loading, { exact: true }),
        ).toBeVisible()
      await next.screenshot({ path: info.outputPath(`startup-${mode}.png`) })
    } finally {
      await next.close()
    }
  }
})
