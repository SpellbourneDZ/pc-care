import { spawn, execFileSync } from 'node:child_process'
import { access } from 'node:fs/promises'
import { createServer } from 'node:net'
import { resolve } from 'node:path'
import { expect, test } from '@playwright/test'
import { ru } from '../src/i18n/ru'
import type { ApiReply } from '../src/generated/ApiReply'

test('release executable renders offline in Windows WebView2', async ({
  playwright,
}, testInfo) => {
  test.skip(process.platform !== 'win32', 'Windows x64 only')
  const binary = resolve(
    import.meta.dirname,
    '../../../target/release/pc-care.exe',
  )
  await access(binary)
  const server = createServer()
  await new Promise<void>((accept, reject) => {
    server.once('error', reject)
    server.listen(0, '127.0.0.1', accept)
  })
  const address = server.address()
  if (!address || typeof address === 'string')
    throw new Error('No CDP port allocated')
  const port = address.port
  await new Promise<void>((accept, reject) =>
    server.close((error) => (error ? reject(error) : accept())),
  )
  const app = spawn(binary, [], {
    windowsHide: true,
    env: {
      ...process.env,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port} --remote-debugging-address=127.0.0.1`,
      WEBVIEW2_USER_DATA_FOLDER: testInfo.outputPath('webview-profile'),
    },
  })
  let launchError: Error | undefined
  app.on('error', (error) => {
    launchError = error
  })
  let browser:
    Awaited<ReturnType<typeof playwright.chromium.connectOverCDP>> | undefined
  try {
    const endpoint = `http://127.0.0.1:${port}`
    await expect
      .poll(
        async () => {
          if (launchError) throw launchError
          if (app.exitCode !== null)
            throw new Error(`PC Care exited with ${app.exitCode}`)
          return fetch(`${endpoint}/json/version`)
            .then((response) => response.ok)
            .catch(() => false)
        },
        { timeout: 30_000 },
      )
      .toBe(true)
    browser = await playwright.chromium.connectOverCDP(endpoint)
    const context = browser.contexts()[0]
    if (!context) throw new Error('WebView2 context is missing')
    await expect.poll(() => context.pages().length).toBeGreaterThan(0)
    const page = context.pages()[0]
    if (!page) throw new Error('WebView2 page is missing')
    const errors: string[] = []
    page.on('pageerror', (error) => errors.push(error.message))
    // CDP can attach before Tauri leaves its initial blank page.
    await expect(
      page.getByRole('heading', { name: ru.emptyTitle }),
    ).toBeVisible()
    await page.waitForLoadState('load')
    await context.setOffline(true)
    await page.reload()
    await expect(
      page.getByRole('heading', { name: ru.emptyTitle }),
    ).toBeVisible()
    const about = page.getByRole('button', { name: ru.about })
    await page.keyboard.press('Tab')
    await expect(about).toBeFocused()
    await page.keyboard.press('Enter')
    await expect(page.getByText(ru.aboutDescription)).toBeVisible()
    expect(
      await about.evaluate((element) => getComputedStyle(element).display),
    ).toBe('inline-flex')
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
    await browser?.close()
    if (app.pid && app.exitCode === null) {
      execFileSync('taskkill', ['/PID', String(app.pid), '/T', '/F'], {
        windowsHide: true,
      })
    }
  }
})
