import { expect, test, type Page } from '@playwright/test'
import { ru } from '../src/i18n/ru'
import type { ApiRequest } from '../src/generated/ApiRequest'
import type { StorageNode } from '../src/generated/StorageNode'

async function fixture(
  page: Page,
  mode: 'large' | 'empty' | 'error' | 'loading' = 'large',
) {
  await page.addInitScript(
    ({ mode }) => {
      let failed = false,
        cancelled = false
      const make = (
        id: number,
        parent_id: number | null,
        directory: boolean,
        name: string,
        bytes: string,
      ): StorageNode => ({
        id,
        parent_id,
        directory,
        name,
        path: `C:\\fixture\\${name}`,
        bytes,
        logical_bytes: bytes,
        modified_ms: 1600000000000,
        skipped: null,
      })
      const root = make(0, null, true, 'Тестовая папка', '409588712')
      const folder = make(1, 0, true, 'Вложенная папка', '1000')
      const rows = [
        root,
        folder,
        make(2, 1, false, 'внутри.txt', '1000'),
        ...Array.from({ length: 99997 }, (_, i) =>
          make(i + 3, 0, false, `файл-${i + 3}.bin`, '4096'),
        ),
      ]
      if (mode === 'empty') root.bytes = root.logical_bytes = '0'
      const task = () => ({
        task_id: 'storage-task',
        status: cancelled
          ? 'cancelled'
          : mode === 'loading'
            ? 'running'
            : 'completed',
        processed: 99999,
        total: null,
        scan_id: cancelled || mode === 'loading' ? null : 'storage-scan',
        error: null,
      })
      Object.assign(window, {
        __TAURI_INTERNALS__: {
          transformCallback: () => 1,
          unregisterCallback: () => {},
          invoke: async (_: string, args: { request: ApiRequest }) => {
            const request = args.request
            switch (request.type) {
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
                      fixture_mode: true,
                      rules: [],
                      max_plan_items: 500,
                    },
                  },
                }
              case 'list_volumes':
                return {
                  status: 'ok',
                  value: {
                    type: 'volumes',
                    data: [
                      {
                        path: 'C:\\',
                        filesystem: 'NTFS',
                        total_bytes: '107374182400',
                        free_bytes: '53687091200',
                        available_bytes: '53687091200',
                        error: null,
                      },
                    ],
                  },
                }
              case 'start_storage_scan':
                if (mode === 'error' && !failed) {
                  failed = true
                  return { status: 'error', error: 'adapter_error' }
                }
                return {
                  status: 'ok',
                  value: {
                    type: 'scan_started',
                    data: { task_id: 'storage-task' },
                  },
                }
              case 'get_task':
                return { status: 'ok', value: { type: 'task', data: task() } }
              case 'cancel_task':
                cancelled = true
                return { status: 'ok', value: { type: 'task', data: task() } }
              case 'get_storage_page': {
                const q = request.data.query,
                  current = q.parent_id === 1 ? folder : root
                const matching =
                  mode === 'empty'
                    ? []
                    : rows.filter(
                        (n) =>
                          n.id !== 0 &&
                          (q.view === 'files'
                            ? !n.directory &&
                              (q.parent_id === 0 || n.parent_id === 1)
                            : n.parent_id === q.parent_id) &&
                          n.name.includes(q.search) &&
                          (!q.extension ||
                            n.name.endsWith(`.${q.extension}`)) &&
                          BigInt(n.bytes) >= BigInt(q.min_bytes),
                      )
                matching.sort((a, b) =>
                  q.sort === 'name'
                    ? a.name.localeCompare(b.name)
                    : Number(BigInt(b.bytes) - BigInt(a.bytes)),
                )
                return {
                  status: 'ok',
                  value: {
                    type: 'storage_page',
                    data: {
                      scan_id: 'storage-scan',
                      root,
                      current,
                      ancestors: q.parent_id === 1 ? [root, folder] : [root],
                      rows: matching.slice(q.offset, q.offset + q.limit),
                      total: matching.length,
                      node_count: mode === 'empty' ? 1 : 100000,
                      issues: [
                        {
                          rule_id: 'storage',
                          reason: 'hard_link',
                          count: 2,
                          sample_path: null,
                        },
                      ],
                      truncated: false,
                      completed_ms: 1600000000000,
                    },
                  },
                }
              }
              default:
                throw new Error(`Unexpected operation ${request.type}`)
            }
          },
        },
      })
    },
    { mode },
  )
  await page.goto('/#/disks')
  await page.getByRole('button', { name: ru.scan, exact: true }).click()
}

test('storage fixture: 100k records stay paged and virtualized; navigation, filters and inspector agree', async ({
  page,
}, info) => {
  await fixture(page)
  const list = page.getByRole('list', { name: ru.storage.list })
  await expect(list.getByRole('listitem')).toHaveCount(12)
  await list.getByRole('button').first().click()
  const inspector = page.getByRole('complementary', {
    name: ru.storage.inspect,
  })
  await expect(
    inspector.getByRole('button', { name: ru.cleanupUx.confirm }),
  ).toBeDisabled()
  await expect(
    page.getByText(ru.cleanupUx.fixture, { exact: true }),
  ).toBeVisible()
  await page.getByLabel(ru.map, { exact: true }).scrollIntoViewIfNeeded()
  await page.screenshot({
    path: info.outputPath('storage-map.png'),
    fullPage: true,
  })
  await list.getByRole('button').first().focus()
  await page.keyboard.press('End')
  await expect(list.getByRole('listitem').last()).toHaveAttribute(
    'aria-posinset',
    '500',
  )
  expect(await list.getByRole('listitem').count()).toBeLessThan(15)
  await page.getByRole('button', { name: ru.cleanupUx.nextPage }).click()
  await expect(list.getByRole('listitem').first()).toHaveAttribute(
    'aria-posinset',
    '501',
  )
  await page.getByLabel(ru.storage.search, { exact: true }).fill('Вложенная')
  await page.getByRole('button', { name: ru.storage.apply }).click()
  await expect(list.getByRole('listitem')).toHaveCount(1)
  await list
    .getByRole('button', {
      name: `${ru.storage.enter}: Вложенная папка`,
      exact: true,
    })
    .click()
  await expect(page.getByRole('navigation', { name: ru.path })).toContainText(
    'Вложенная папка',
  )
  await expect(list.getByRole('listitem')).toHaveCount(1)
  await list.getByRole('button').first().click()
  await expect(inspector).toContainText('внутри.txt')
  await inspector.scrollIntoViewIfNeeded()
  await page.screenshot({
    path: info.outputPath('storage-inspector.png'),
    fullPage: true,
  })
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true)
})

test('storage fixture: empty, error with retry and cancellation remain honest', async ({
  page,
}, info) => {
  for (const mode of ['empty', 'error', 'loading'] as const) {
    const next = await page.context().newPage()
    try {
      await fixture(next, mode)
      if (mode === 'error') {
        await expect(next.getByRole('alert')).toContainText(
          ru.cleanupUx.unavailable,
        )
        await next.getByRole('button', { name: ru.scan, exact: true }).click()
        await expect(
          next.getByRole('list', { name: ru.storage.list }),
        ).toBeVisible()
      } else if (mode === 'empty')
        await expect(next.getByText(ru.storage.empty)).toBeVisible()
      else {
        await expect(next.getByRole('progressbar')).not.toHaveAttribute(
          'aria-valuenow',
        )
        await next.getByRole('button', { name: ru.cleanupUx.cancel }).click()
        await expect(next.getByText(ru.storage.cancelled)).toBeVisible()
        await expect(
          next.getByRole('list', { name: ru.storage.list }),
        ).not.toBeVisible()
      }
      await next.screenshot({
        path: info.outputPath(`storage-${mode}.png`),
        fullPage: true,
      })
    } finally {
      await next.close()
    }
  }
})
