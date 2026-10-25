import { randomUUID } from 'node:crypto'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { resolve } from 'node:path'
import { expect, test } from '@playwright/test'
import { ru } from '../src/i18n/ru'
import { startNative } from './native-app'
test('startup native: fixture inventory displays unknown activation and never changes shortcut bytes', async ({
  playwright,
}, info) => {
  test.skip(process.platform !== 'win32', 'Windows only')
  const root = resolve(
    import.meta.dirname,
    '../../../target/native-fixtures',
    randomUUID(),
  )
  const folder = resolve(root, 'Profile/StartupUser')
  await mkdir(folder, { recursive: true })
  const file = resolve(folder, 'Искусственный ярлык.lnk'),
    text = 'not a real shortcut; never execute or resolve'
  await writeFile(file, text)
  const app = await startNative(
    playwright,
    resolve(
      import.meta.dirname,
      '../../../target/native-qa/release/pc-care.exe',
    ),
    info.outputPath('profile'),
    root,
  )
  try {
    const { page } = app
    await app.context.setOffline(true)
    await page.getByRole('link', { name: ru.startup, exact: true }).click()
    await expect(page.getByRole('row')).toHaveCount(6)
    await expect(page.getByRole('switch')).not.toBeVisible()
    await expect(
      page.getByText(ru.cleanupUx.fixture, { exact: true }),
    ).toBeVisible()
    await page.screenshot({ path: info.outputPath('startup-native-table.png') })
    await page
      .getByRole('button', {
        name: `${ru.startupUx.details}: Искусственный ярлык.lnk`,
        exact: true,
      })
      .click()
    const dialog = page.getByRole('dialog')
    await expect(dialog).toContainText(ru.startupUx.unknown)
    await expect(
      dialog.getByRole('button', { name: ru.storage.reveal }),
    ).toBeEnabled()
    await page.screenshot({
      path: info.outputPath('startup-native-details.png'),
    })
    await page.keyboard.press('Escape')
    expect(await readFile(file, 'utf8')).toBe(text)
    expect(app.errors).toEqual([])
  } finally {
    await app.close()
  }
})
