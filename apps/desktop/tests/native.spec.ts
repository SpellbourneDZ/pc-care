import { resolve } from 'node:path'
import { createServer } from 'node:http'
import { expect, test } from '@playwright/test'
import { ru } from '../src/i18n/ru'
import type { ApiReply } from '../src/generated/ApiReply'
import { checkPrototypeFlow } from './ui-flow'
import { startNative, waitForPageTarget } from './native-app'

test('native startup waits for a debuggable page after the CDP port is ready', async () => {
  let requests = 0
  const responses = [
    null,
    {},
    [],
    [{ type: 'worker' }],
    [{ type: 'page' }],
    [{ type: 'page', webSocketDebuggerUrl: 'ws://127.0.0.1/fixture' }],
  ]
  const server = createServer((request, response) => {
    expect(request.url).toBe('/json/list')
    response.setHeader('Content-Type', 'application/json')
    response.end(JSON.stringify(responses[requests++]))
  })
  await new Promise<void>((accept) => server.listen(0, '127.0.0.1', accept))
  try {
    const address = server.address()
    if (!address || typeof address === 'string')
      throw new Error('No fixture port')
    let checked = 0
    await waitForPageTarget(`http://127.0.0.1:${address.port}`, () => checked++)
    expect(requests).toBe(responses.length)
    expect(checked).toBe(requests)
  } finally {
    await new Promise<void>((accept, reject) =>
      server.close((error) => (error ? reject(error) : accept())),
    )
  }
})

test('release executable renders offline in Windows WebView2', async ({
  playwright,
}, testInfo) => {
  test.skip(process.platform !== 'win32', 'Windows x64 only')
  const binary = resolve(
    import.meta.dirname,
    '../../../target/release/pc-care.exe',
  )
  const { page, context, errors, close } = await startNative(
    playwright,
    binary,
    testInfo.outputPath('webview-profile'),
  )
  try {
    await context.setOffline(true)
    await page.reload()
    await expect(
      page.getByRole('heading', { name: ru.cleanupUx.overviewTitle }),
    ).toBeVisible()
    await expect(page.getByText(ru.exampleData)).not.toBeVisible()
    await page.screenshot({ path: testInfo.outputPath('overview.png') })
    await page.getByRole('link', { name: ru.startup, exact: true }).click()
    await expect(
      page.getByText(ru.startupUx.collected, { exact: false }),
    ).toBeVisible()
    await expect(
      page.getByText(ru.cleanupUx.fixture, { exact: false }),
    ).not.toBeVisible()
    await expect(page.getByRole('switch')).not.toBeVisible()
    await page.getByRole('link', { name: ru.apps, exact: true }).click()
    await expect(
      page.getByText(ru.appsUx.collected, { exact: false }),
    ).toBeVisible({ timeout: 30_000 })
    await expect(
      page.getByText(ru.cleanupUx.fixture, { exact: false }),
    ).not.toBeVisible()
    await page.getByRole('link', { name: ru.overview, exact: true }).click()
    await page.getByRole('button', { name: ru.openCleanup }).click()
    await expect(page.getByText(ru.cleanupUx.previewOnly)).toBeVisible()
    await page.screenshot({ path: testInfo.outputPath('read-only-start.png') })
    await page
      .getByRole('button', { name: ru.cleanupUx.scan, exact: true })
      .click()
    await expect(
      page.getByRole('heading', { name: ru.cleanupUx.ready, exact: true }).or(
        page.getByRole('heading', {
          name: ru.cleanupUx.noFindings,
          exact: true,
        }),
      ),
    ).toBeVisible({ timeout: 30_000 })
    await expect(
      page.getByText(ru.cleanupUx.fixture, { exact: false }),
    ).not.toBeVisible()
    await page.screenshot({
      path: testInfo.outputPath('read-only-results.png'),
    })
    await page.getByRole('button', { name: ru.settings, exact: true }).click()
    await expect(page.getByRole('dialog')).toBeVisible()
    await page.screenshot({
      path: testInfo.outputPath('read-only-settings.png'),
    })
    await page.keyboard.press('Escape')
    await page.keyboard.press('Control+k')
    await expect(
      page.getByRole('dialog', { name: ru.quickSearch }),
    ).toBeVisible()
    await page.screenshot({ path: testInfo.outputPath('quick-search.png') })
    await page.keyboard.press('Escape')
    const about = page.getByRole('button', { name: ru.about })
    await about.focus()
    await page.keyboard.press('Enter')
    await expect(page.getByText(ru.aboutDescription)).toBeVisible()
    await page.keyboard.press('Escape')
    await expect(about).toBeFocused()
    await about.click()
    await page.getByRole('button', { name: ru.openPrototype }).click()
    await expect(page.getByText(ru.exampleData)).toBeVisible()
    await checkPrototypeFlow(page)
    expect(errors).toEqual([])
    const apiResults = await page.evaluate(async () => {
      const native = (
        window as unknown as {
          __TAURI_INTERNALS__: {
            invoke<T>(
              command: string,
              args: Record<string, unknown>,
            ): Promise<T>
            transformCallback(
              callback: (data: unknown) => void,
              once: boolean,
            ): number
          }
        }
      ).__TAURI_INTERNALS__
      const call = (request: unknown) =>
        native.invoke<ApiReply>('operation_api', {
          request,
          progress: `__CHANNEL__:${native.transformCallback(() => {}, true)}`,
        })
      const missing = await call({
        type: 'get_task',
        data: { task_id: '11111111-1111-4111-8111-111111111111' },
      })
      const unavailable = await call({
        type: 'start_scan',
        data: { kind: 'mock', scenario: 'complete', timeout_ms: 1000 },
      })
      const denied: boolean[] = []
      for (const request of [
        { type: 'delete_file', data: { path: 'C:/Windows' } },
        {
          type: 'delete_registry_key',
          data: { path: 'HKLM/SOFTWARE/OtherApp' },
        },
        {
          type: 'execute_plan',
          data: {
            plan_id: '11111111-1111-4111-8111-111111111111',
            explicit_confirmation: true,
            path: 'C:/Windows',
          },
        },
      ]) {
        try {
          await call(request)
          denied.push(false)
        } catch {
          denied.push(true)
        }
      }
      return { missing, unavailable, denied }
    })
    expect(apiResults.missing).toEqual({ status: 'error', error: 'not_found' })
    expect(apiResults.unavailable).toEqual({
      status: 'error',
      error: 'unsupported',
    })
    expect(apiResults.denied).toEqual([true, true, true])
    await page.screenshot({ path: testInfo.outputPath('webview2.png') })
  } finally {
    await close()
  }
})
