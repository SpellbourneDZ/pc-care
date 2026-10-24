import { spawn, execFileSync } from 'node:child_process'
import { access } from 'node:fs/promises'
import { createServer } from 'node:net'
import { expect } from '@playwright/test'
import { ru } from '../src/i18n/ru'

export async function startNative(
  playwright: Pick<typeof import('@playwright/test'), 'chromium'>,
  binary: string,
  profile: string,
  fixtureRoot?: string,
) {
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
      PC_CARE_FIXTURE_ROOT: fixtureRoot,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port} --remote-debugging-address=127.0.0.1`,
      WEBVIEW2_USER_DATA_FOLDER: profile,
    },
  })
  let browser:
    Awaited<ReturnType<typeof playwright.chromium.connectOverCDP>> | undefined
  let launchError: Error | undefined
  app.on('error', (error) => {
    launchError = error
  })
  const close = async () => {
    await browser?.close()
    if (app.pid && app.exitCode === null)
      execFileSync('taskkill', ['/PID', String(app.pid), '/T', '/F'], {
        windowsHide: true,
      })
  }
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
    page.on('console', (message) => {
      if (
        message.type() === 'error' &&
        /Content Security Policy/i.test(message.text())
      )
        errors.push(message.text())
    })
    // CDP can attach before Tauri leaves its initial blank page.
    await expect(
      page.getByRole('heading', { name: ru.moduleNotReady }),
    ).toBeVisible()
    await page.waitForLoadState('load')
    return { page, context, errors, close }
  } catch (error) {
    await close()
    throw error
  }
}
