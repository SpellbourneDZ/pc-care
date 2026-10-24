import { randomUUID } from 'node:crypto'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { resolve } from 'node:path'
import { expect, test } from '@playwright/test'
import { ru } from '../src/i18n/ru'
import { startNative } from './native-app'

test('native cleanup: real read-only scan, reviewed cache and exact preview use artificial profile only', async ({
  playwright,
}, info) => {
  test.skip(process.platform !== 'win32', 'Windows x64 only')
  const root = resolve(
    import.meta.dirname,
    '../../../target/native-fixtures',
    randomUUID(),
  )
  const temp = resolve(root, 'Profile/AppData/Local/Temp')
  const cache = resolve(
    root,
    'Profile/AppData/Local/Google/Chrome/User Data/Default/Cache',
  )
  await mkdir(temp, { recursive: true })
  await mkdir(cache, { recursive: true })
  const sentinel = resolve(temp, 'искусственный-файл.tmp')
  const review = resolve(cache, 'artificial-cache')
  const credentials = resolve(temp, 'Login Data.tmp')
  for (const file of [sentinel, review, credentials])
    await writeFile(file, 'PC Care fixture - must remain unchanged')
  const app = await startNative(
    playwright,
    resolve(
      import.meta.dirname,
      '../../../target/native-qa/release/pc-care.exe',
    ),
    info.outputPath('profile'),
    root,
    true,
  )
  try {
    const { page } = app
    await app.context.setOffline(true)
    await page.getByRole('button', { name: ru.findJunk }).click()
    await expect(page.getByTestId('cleanup-selection')).toHaveText('1')
    await expect(
      page.getByText(ru.cleanupUx.fixture, { exact: false }),
    ).toBeVisible()
    await page.screenshot({ path: info.outputPath('native-cleanup-ready.png') })
    const chrome = page.getByRole('checkbox', {
      name: `${ru.selected}: HTTP-кеш Google Chrome`,
    })
    await expect(chrome).not.toBeChecked()
    await chrome.click()
    await expect(page.getByTestId('cleanup-selection')).toHaveText('2')
    await page
      .getByRole('button', {
        name: `${ru.cleanupUx.details}: HTTP-кеш Google Chrome`,
      })
      .click()
    const drawer = page.getByRole('dialog')
    await expect(drawer).toContainText('artificial-cache')
    await expect(drawer.getByRole('checkbox')).toBeChecked()
    await page.screenshot({
      path: info.outputPath('native-cleanup-drawer.png'),
    })
    await page.keyboard.press('Escape')
    await page
      .getByRole('button', { name: ru.cleanupUx.skipped, exact: true })
      .click()
    await expect(page.getByRole('dialog')).toContainText(
      ru.cleanupReasons.sensitive_data,
    )
    await page.keyboard.press('Escape')
    await page
      .getByRole('button', { name: ru.cleanupUx.preview, exact: true })
      .click()
    const dialog = page.getByRole('alertdialog')
    await expect(dialog).toContainText('искусственный-файл.tmp')
    await expect(dialog).toContainText('artificial-cache')
    await expect(dialog).not.toContainText('Login Data.tmp')
    await page.screenshot({
      path: info.outputPath('native-cleanup-preview.png'),
    })
    await dialog
      .getByRole('button', { name: ru.cleanupUx.finishPreview })
      .click()
    await expect(
      page.getByRole('heading', { name: ru.cleanupUx.previewDone }),
    ).toBeVisible()
    for (const file of [sentinel, review, credentials])
      expect(await readFile(file, 'utf8')).toBe(
        'PC Care fixture - must remain unchanged',
      )
    expect(app.errors).toEqual([])
  } finally {
    await app.close()
  }
})
