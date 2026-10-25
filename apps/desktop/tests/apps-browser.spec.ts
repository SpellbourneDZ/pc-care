import { expect, test, type Page } from '@playwright/test'
import { ru } from '../src/i18n/ru'
import type { ApiRequest } from '../src/generated/ApiRequest'
import type { InstalledApp } from '../src/generated/InstalledApp'
const u = ru.appsUx
async function fixture(
  page: Page,
  mode: 'ready' | 'empty' | 'partial' | 'error' | 'loading' = 'ready',
) {
  await page.addInitScript(
    () => {
      const mode =
        new URL(location.href).searchParams.get('apps_fixture') ?? 'ready'
      let failed = false
      const records: InstalledApp[] = Array.from(
        { length: mode === 'empty' ? 0 : 205 },
        (_, i) => ({
          id: `fixture:${i}`,
          name: i < 2 ? 'Одинаковое имя' : `Искусственная программа ${i}`,
          publisher:
            i % 2 ? 'Издатель с очень длинным названием '.repeat(8) : null,
          version: i % 2 ? '1.2.3' : null,
          kind: i % 2 ? 'msi' : 'msix',
          source: i % 2 ? 'hkcu64' : 'msix',
          system_component: !i,
          estimated_bytes: i % 2 ? '9007199254740993' : null,
          registered_date: '20261025',
          install_location: 'C:\\artificial\\' + 'Длинная папка\\'.repeat(25),
          registration: `Fixture-${i}`,
          uninstall_block: i ? 'not_validated' : 'protected',
        }),
      )
      Object.assign(window, {
        __TAURI_INTERNALS__: {
          transformCallback: () => 1,
          unregisterCallback: () => {},
          invoke: async (_: string, args: { request: ApiRequest }) => {
            const r = args.request
            switch (r.type) {
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
                      max_plan_items: 500,
                      fixture_mode: true,
                    },
                  },
                }
              case 'list_volumes':
                return { status: 'ok', value: { type: 'volumes', data: [] } }
              case 'refresh_apps':
                if (mode === 'loading') return new Promise(() => {})
                if (mode === 'error' && !failed) {
                  failed = true
                  return { status: 'error', error: 'adapter_error' }
                }
                return {
                  status: 'ok',
                  value: {
                    type: 'apps_catalog',
                    data: {
                      id: 'apps',
                      total: records.length,
                      fixture_mode: true,
                      collected_ms: 1600000000000,
                      truncated: mode === 'partial',
                      issues:
                        mode === 'partial'
                          ? [
                              {
                                source: 'hklm64',
                                reason: 'access_denied',
                                count: 1,
                              },
                            ]
                          : [],
                    },
                  },
                }
              case 'get_apps_page': {
                const q = r.data.query,
                  rows = records.filter(
                    (e) =>
                      (!q.kind || q.kind === e.kind) &&
                      (!q.source || q.source === e.source) &&
                      e.name.includes(q.search),
                  )
                return {
                  status: 'ok',
                  value: {
                    type: 'apps_page',
                    data: {
                      snapshot_id: 'apps',
                      offset: q.offset,
                      total: rows.length,
                      entries: rows.slice(q.offset, q.offset + q.limit),
                    },
                  },
                }
              }
              default:
                throw new Error(`Unexpected operation ${r.type}`)
            }
          },
        },
      })
    },
    { mode },
  )
  await page.goto(`/?apps_fixture=${mode}#/apps`)
}
test('apps fixture: duplicate names, page filters, unknown size and non-executable details', async ({
  page,
}, info) => {
  await fixture(page)
  await expect(page.getByRole('row')).toHaveCount(101)
  await expect(
    page.getByText(ru.cleanupUx.fixture, { exact: true }),
  ).toBeVisible()
  await page.screenshot({ path: info.outputPath('apps-table.png') })
  await page
    .getByRole('button', { name: `${u.details}: Одинаковое имя`, exact: true })
    .first()
    .click()
  const dialog = page.getByRole('dialog')
  await expect(dialog).toContainText('fixture:0')
  await expect(
    dialog.getByRole('button', { name: u.uninstall, exact: true }),
  ).toBeDisabled()
  await expect(
    dialog.getByRole('button', { name: u.leftovers, exact: true }),
  ).toBeDisabled()
  await page.keyboard.press('Control+k')
  await expect(page.getByRole('dialog')).toHaveCount(1)
  await page.screenshot({ path: info.outputPath('apps-details.png') })
  await page.keyboard.press('Escape')
  await page.getByRole('button', { name: ru.cleanupUx.nextPage }).click()
  await expect(page.getByRole('row')).toHaveCount(101)
  await page
    .getByRole('textbox', { name: u.search })
    .fill('несуществующая программа')
  await expect(page.getByText(u.noMatches)).toBeVisible()
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true)
})
test('apps fixture: empty, partial, retry and loading are explicit', async ({
  page,
}, info) => {
  await fixture(page, 'empty')
  await expect(page.getByText(u.empty, { exact: true })).toBeVisible()
  await page.screenshot({ path: info.outputPath('apps-empty.png') })
  await fixture(page, 'partial')
  await expect(page.getByText(u.limit, { exact: true })).toBeVisible()
  await fixture(page, 'error')
  await expect(page.getByRole('alert')).toBeVisible()
  await page.getByRole('button', { name: u.refresh }).click()
  await expect(page.getByRole('row')).toHaveCount(101)
  await fixture(page, 'loading')
  await expect(page.getByText(u.loading, { exact: true })).toBeVisible()
})
