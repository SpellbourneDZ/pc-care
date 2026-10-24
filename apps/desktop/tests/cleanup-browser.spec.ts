import { expect, test, type Page } from '@playwright/test'
import { ru } from '../src/i18n/ru'
import { candidate, catalog, resultPage } from './cleanup-fixtures'
import type { ApiRequest } from '../src/generated/ApiRequest'
import type { ApiResponse } from '../src/generated/ApiResponse'

async function fixture(
  page: Page,
  mode: 'ready' | 'empty' | 'partial' | 'error' | 'loading' = 'ready',
) {
  const result = resultPage([
    candidate('safe'),
    {
      ...candidate('review', 'chrome'),
      display_path: `C:\\PC Care fixture\\${'длинное имя '.repeat(15)}.cache`,
    },
  ])
  await page.addInitScript(
    ({ catalog, result, mode }) => {
      let cancelled = false
      const task = () => ({
        task_id: 'task',
        status: cancelled
          ? 'cancelled'
          : mode === 'loading'
            ? 'running'
            : 'completed',
        processed: 12,
        total: null,
        scan_id: cancelled || mode === 'loading' ? null : 'scan',
        error: null,
      })
      Object.assign(window, {
        __TAURI_INTERNALS__: {
          transformCallback: () => 1,
          unregisterCallback: () => {},
          invoke: async (_: string, args: { request: ApiRequest }) => {
            const req = args.request
            const ok = (value: ApiResponse) => ({ status: 'ok', value })
            switch (req.type) {
              case 'get_preferences':
                return ok({
                  type: 'preferences',
                  data: {
                    revision: null,
                    schema_version: 1,
                    preferences: {
                      theme: 'system',
                      language: 'ru',
                      exclusions: [],
                    },
                  },
                })
              case 'get_cleanup_rules':
                return ok({ type: 'cleanup_rules', data: catalog })
              case 'start_scan':
                cancelled = false
                return mode === 'error'
                  ? { status: 'error', error: 'adapter_error' }
                  : ok({ type: 'scan_started', data: { task_id: 'task' } })
              case 'get_task':
                return { status: 'ok', value: { type: 'task', data: task() } }
              case 'cancel_task':
                cancelled = true
                return { status: 'ok', value: { type: 'task', data: task() } }
              case 'get_scan_result':
                return ok({
                  type: 'scan_result',
                  data:
                    mode === 'empty'
                      ? { ...result, candidates: [], total: 0 }
                      : mode === 'partial'
                        ? {
                            ...result,
                            truncated: true,
                            issues: [
                              {
                                rule_id: 'temp',
                                reason: 'locked',
                                count: 2,
                                sample_path: 'C:\\fixture\\locked.tmp',
                              },
                            ],
                          }
                        : result,
                })
              case 'build_operation_plan': {
                const targets = result.candidates.filter((item) =>
                  req.data.selected_candidate_ids.includes(item.id),
                )
                return ok({
                  type: 'operation_plan',
                  data: {
                    id: 'plan',
                    scan_id: 'scan',
                    simulated: true,
                    targets,
                    planned_bytes: targets
                      .reduce((sum, item) => sum + BigInt(item.bytes), 0n)
                      .toString(),
                    expires_in_ms: 30_000,
                    warnings: ['simulation_only'],
                  },
                })
              }
              default:
                throw new Error(`Unexpected effect in UI fixture: ${req.type}`)
            }
          },
        },
      })
    },
    { catalog, result, mode },
  )
  await page.goto('/#/cleanup')
  await expect(
    page.getByText(ru.cleanupUx.fixture, { exact: false }),
  ).toBeVisible()
  await page
    .getByRole('button', { name: ru.cleanupUx.scan, exact: true })
    .click()
}

test('cleanup IPC fixture: selection, long paths, overlays, keyboard and honest preview fit every window', async ({
  page,
  context,
}, info) => {
  await fixture(page)
  await context.setOffline(true)
  await expect(page.getByTestId('cleanup-selection')).toHaveText('1')
  await page.screenshot({ path: info.outputPath('cleanup-ready.png') })
  const review = page.getByRole('checkbox', {
    name: `${ru.selected}: ${catalog.rules[1]!.name}`,
  })
  await expect(review).not.toBeChecked()
  await review.click()
  await expect(page.getByTestId('cleanup-selection')).toHaveText('2')
  await page
    .getByRole('button', {
      name: `${ru.cleanupUx.details}: ${catalog.rules[1]!.name}`,
    })
    .click()
  const drawer = page.getByRole('dialog')
  await expect(
    drawer.getByText(ru.cleanupUx.fixture, { exact: true }),
  ).toBeVisible()
  await expect(drawer.getByRole('checkbox')).toBeChecked()
  await page.keyboard.press('Control+k')
  await expect(
    page.getByRole('dialog', { name: ru.quickSearch }),
  ).not.toBeVisible()
  await page.screenshot({ path: info.outputPath('cleanup-long-path.png') })
  await page.keyboard.press('Escape')
  await expect(
    page.getByRole('button', {
      name: `${ru.cleanupUx.details}: ${catalog.rules[1]!.name}`,
    }),
  ).toBeFocused()
  const preview = page.getByRole('button', {
    name: ru.cleanupUx.preview,
    exact: true,
  })
  await preview.click()
  const dialog = page.getByRole('alertdialog')
  await expect(dialog).toContainText(ru.cleanupUx.previewOnly)
  await expect(
    dialog.getByRole('button', { name: ru.cleanupUx.finishPreview }),
  ).toBeInViewport()
  await page.screenshot({ path: info.outputPath('cleanup-preview.png') })
  await dialog.getByRole('button', { name: ru.cancel, exact: true }).click()
  await expect(preview).toBeFocused()
  await preview.click()
  await page.getByRole('button', { name: ru.cleanupUx.finishPreview }).click()
  await expect(
    page.getByRole('heading', { name: ru.cleanupUx.previewDone }),
  ).toBeVisible()
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true)
})

test('cleanup IPC fixture: empty, partial, failed and cancellable progress recover', async ({
  page,
}, info) => {
  for (const mode of ['empty', 'partial', 'error', 'loading'] as const) {
    const next = await page.context().newPage()
    try {
      await fixture(next, mode)
      if (mode === 'empty')
        await expect(
          next.getByRole('heading', { name: ru.cleanupUx.noFindings }),
        ).toBeVisible()
      if (mode === 'partial') {
        await expect(
          next.getByText(ru.cleanupUx.truncated, { exact: true }),
        ).toBeVisible()
        await expect(next.getByTestId('cleanup-selection')).toHaveText('0')
      }
      if (mode === 'error')
        await expect(next.getByRole('alert')).toContainText(
          ru.cleanupUx.unavailable,
        )
      if (mode === 'loading') {
        await expect(next.getByRole('progressbar')).not.toHaveAttribute(
          'aria-valuenow',
        )
        await next.getByRole('button', { name: ru.cleanupUx.cancel }).click()
        await expect(
          next.getByRole('heading', { name: ru.cleanupUx.cancelled }),
        ).toBeVisible()
      }
      await next.screenshot({ path: info.outputPath(`cleanup-${mode}.png`) })
    } finally {
      await next.close()
    }
  }
})
