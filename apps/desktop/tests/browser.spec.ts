import { expect, test } from '@playwright/test'
import { ru } from '../src/i18n/ru'
import { checkPrototypeFlow, exampleState, navigate } from './ui-flow'

test('four prototype modules are interactive offline and fit the window', async ({
  page,
  context,
}, testInfo) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  await page.goto('/')
  await expect(page.getByRole('heading', { name: ru.emptyTitle })).toBeVisible()
  await expect(page.getByText(ru.exampleData)).toBeVisible()
  await context.setOffline(true)
  await checkPrototypeFlow(page)
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true)
  expect(errors).toEqual([])
  await page.screenshot({ path: testInfo.outputPath('apps.png') })
})
test('empty, loading, error, partial, unsupported and offline states recover', async ({
  page,
}) => {
  await page.goto('/')
  for (const module of [ru.cleanup, ru.disks, ru.startup, ru.apps]) {
    await navigate(page, module)
    await expect(
      page.getByRole('heading', { name: ru.emptyModule }),
    ).toBeVisible()
    await exampleState(page, ru.states.loading)
    await expect(
      page.getByRole('progressbar', { name: ru.loading }),
    ).toBeVisible()
    await page.getByRole('button', { name: ru.cancel, exact: true }).click()
    await expect(
      page.getByRole('heading', { name: ru.emptyModule }),
    ).toBeVisible()
    await exampleState(page, ru.states.error)
    await expect(
      page.getByRole('heading', { name: ru.errorTitle }),
    ).toBeVisible()
    await exampleState(page, ru.states.unsupported)
    await expect(
      page.getByRole('heading', { name: ru.unsupportedTitle }),
    ).toBeVisible()
    await exampleState(page, ru.states.partial)
    await expect(page.getByRole('status')).toContainText(ru.partialDescription)
    await exampleState(page, ru.states.offline)
    await expect(page.getByRole('status')).toContainText(ru.offlineDescription)
  }
})
test('theme follows system, manual choices override it, dialogs restore focus', async ({
  page,
}) => {
  await page.goto('/')
  await page.emulateMedia({ colorScheme: 'dark', reducedMotion: 'reduce' })
  await expect(page.locator('html')).toHaveClass('dark')
  const theme = page.getByRole('combobox', { name: ru.theme })
  await theme.focus()
  await page.keyboard.press('Enter')
  await page.getByRole('option', { name: ru.themes.light, exact: true }).click()
  await expect(page.locator('html')).not.toHaveClass('dark')
  await theme.click()
  await page.getByRole('option', { name: ru.themes.dark, exact: true }).click()
  await page.emulateMedia({ colorScheme: 'light' })
  await expect(page.locator('html')).toHaveClass('dark')
  await theme.click()
  await page
    .getByRole('option', { name: ru.themes.system, exact: true })
    .click()
  await expect(page.locator('html')).not.toHaveClass('dark')
  const colors = await page.evaluate(() =>
    [
      '--background',
      '--foreground',
      '--card',
      '--primary',
      '--muted',
      '--muted-foreground',
      '--destructive',
      '--border',
      '--ring',
      '--sidebar',
    ].map((name) =>
      getComputedStyle(document.documentElement).getPropertyValue(name).trim(),
    ),
  )
  for (const color of colors) expect(color).toMatch(/^#([a-f\d]{2})\1\1$/i)
  const about = page.getByRole('button', { name: ru.about, exact: true })
  if (!(await about.isVisible()))
    await page.getByRole('button', { name: ru.navigation, exact: true }).click()
  await about.focus()
  await page.keyboard.press('Enter')
  await expect(page.getByRole('dialog', { name: ru.aboutTitle })).toBeVisible()
  await page.keyboard.press('Escape')
  await expect(about).toBeFocused()
})
