import { randomUUID } from 'node:crypto'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { resolve } from 'node:path'
import { expect, test } from '@playwright/test'
import { ru } from '../src/i18n/ru'
import { startNative } from './native-app'

test('settings persist across native process restart using only isolated fixtures', async ({
  playwright,
}, testInfo) => {
  test.skip(process.platform !== 'win32', 'Windows x64 only')
  const root = resolve(
    import.meta.dirname,
    '../../../target/native-fixtures',
    randomUUID(),
  )
  await mkdir(root, { recursive: true })
  const binary = resolve(
    import.meta.dirname,
    '../../../target/native-qa/release/pc-care.exe',
  )
  const settingsFile = resolve(root, 'config/settings.json')
  const exclusion = 'C:\\PC Care fixture\\пример'
  const first = await startNative(
    playwright,
    binary,
    testInfo.outputPath('profile-1'),
    root,
  )
  try {
    await first.context.setOffline(true)
    await first.page
      .getByRole('button', { name: ru.settings, exact: true })
      .click()
    const dialog = first.page.getByRole('dialog', { name: ru.settings })
    await dialog.getByRole('combobox', { name: ru.theme }).click()
    await first.page
      .getByRole('option', { name: ru.themes.dark, exact: true })
      .click()
    await expect(first.page.locator('html')).toHaveClass('dark')
    await dialog
      .getByRole('textbox', { name: ru.exclusionPath })
      .fill(exclusion)
    await dialog.getByRole('button', { name: ru.add, exact: true }).click()
    await expect(
      dialog.getByRole('button', {
        name: `${ru.removeExclusion}: ${exclusion}`,
      }),
    ).toBeVisible()
    await expect(dialog.getByRole('status')).toContainText(ru.settingsSaved)
    await first.page.screenshot({
      path: testInfo.outputPath('settings-dark.png'),
    })
    expect(first.errors).toEqual([])
  } finally {
    await first.close()
  }
  const saved = JSON.parse(await readFile(settingsFile, 'utf8'))
  expect(saved.preferences).toEqual({
    theme: 'dark',
    language: 'ru',
    exclusions: [exclusion],
  })

  const second = await startNative(
    playwright,
    binary,
    testInfo.outputPath('profile-2'),
    root,
  )
  try {
    await expect(second.page.locator('html')).toHaveClass('dark')
    await second.page
      .getByRole('button', { name: ru.settings, exact: true })
      .click()
    const dialog = second.page.getByRole('dialog', { name: ru.settings })
    const remove = dialog.getByRole('button', {
      name: `${ru.removeExclusion}: ${exclusion}`,
    })
    await expect(remove).toBeVisible()
    await remove.click()
    await second.page
      .getByRole('button', { name: ru.cancel, exact: true })
      .click()
    await expect(remove).toBeVisible()
    await remove.click()
    await second.page
      .getByRole('alertdialog')
      .getByRole('button', { name: ru.removeExclusion, exact: true })
      .click()
    await expect(dialog.getByText(ru.noExclusions)).toBeVisible()
    await expect(
      dialog.getByRole('textbox', { name: ru.exclusionPath }),
    ).toBeFocused()
    expect(second.errors).toEqual([])
  } finally {
    await second.close()
  }
  expect(
    JSON.parse(await readFile(settingsFile, 'utf8')).preferences.exclusions,
  ).toEqual([])

  await writeFile(settingsFile, '{invalid fixture')
  const third = await startNative(
    playwright,
    binary,
    testInfo.outputPath('profile-3'),
    root,
  )
  try {
    await third.page
      .getByRole('button', { name: ru.settings, exact: true })
      .click()
    const dialog = third.page.getByRole('dialog', { name: ru.settings })
    await expect(dialog.getByRole('alert')).toContainText(ru.settingsInvalid)
    await expect(dialog.getByRole('textbox')).not.toBeVisible()
    expect(third.errors).toEqual([])
  } finally {
    await third.close()
  }
  expect(await readFile(settingsFile, 'utf8')).toBe('{invalid fixture')
})
