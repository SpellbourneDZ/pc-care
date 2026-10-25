import { randomUUID } from 'node:crypto'
import { mkdir } from 'node:fs/promises'
import { resolve } from 'node:path'
import { expect, test } from '@playwright/test'
import { ru } from '../src/i18n/ru'
import { startNative } from './native-app'
test('apps native: isolated compiled registrations use actual IPC and cannot launch uninstall', async ({
  playwright,
}, info) => {
  test.skip(process.platform !== 'win32', 'Windows only')
  const root = resolve(
    import.meta.dirname,
    '../../../target/native-fixtures',
    randomUUID(),
  )
  await mkdir(resolve(root, 'Profile'), { recursive: true })
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
    await page.getByRole('link', { name: ru.apps, exact: true }).click()
    await expect(page.getByRole('row')).toHaveCount(6)
    await expect(
      page.getByText(ru.cleanupUx.fixture, { exact: true }),
    ).toBeVisible()
    await page.screenshot({ path: info.outputPath('apps-native-table.png') })
    await page
      .getByRole('button', {
        name: `${ru.appsUx.details}: Искусственная программа`,
      })
      .first()
      .click()
    const dialog = page.getByRole('dialog')
    await expect(
      dialog.getByRole('button', { name: ru.appsUx.uninstall, exact: true }),
    ).toBeDisabled()
    await expect(
      dialog.getByRole('button', { name: ru.appsUx.leftovers, exact: true }),
    ).toBeDisabled()
    await page.screenshot({ path: info.outputPath('apps-native-details.png') })
    await page.keyboard.press('Escape')
    await page
      .getByRole('row')
      .filter({ has: page.getByRole('cell', { name: 'MSI', exact: true }) })
      .getByRole('button')
      .click()
    const msi = page.getByRole('dialog')
    await msi
      .getByRole('button', { name: ru.appsUx.check, exact: true })
      .click()
    await expect(msi.getByRole('status')).toContainText(
      ru.appsUx.checkStatuses.msi_matched,
    )
    await expect(msi.getByRole('status')).toContainText(
      ru.appsUx.binaryStatuses.fixture,
    )
    await expect(
      msi.getByRole('button', { name: ru.appsUx.uninstall, exact: true }),
    ).toBeDisabled()
    await page.screenshot({
      path: info.outputPath('apps-native-msi-check.png'),
    })
    expect(app.errors).toEqual([])
  } finally {
    await app.close()
  }
})
