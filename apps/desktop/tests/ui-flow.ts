import { expect, type Page } from '@playwright/test'
import { ru } from '../src/i18n/ru'

export async function navigate(page: Page, name: string) {
  const target = page
    .getByRole('navigation', { name: ru.navigation })
    .getByRole('button', { name, exact: true })
  if (!(await target.isVisible()))
    await page.getByRole('button', { name: ru.navigation, exact: true }).click()
  await target.click()
  await expect(
    page.getByRole('heading', { name, exact: true, level: 1 }),
  ).toBeVisible()
}
export async function exampleState(page: Page, name: string) {
  await page.getByRole('combobox', { name: ru.stateLabel }).click()
  await page.getByRole('option', { name, exact: true }).click()
}
export async function checkPrototypeFlow(page: Page) {
  await navigate(page, ru.cleanup)
  await exampleState(page, ru.states.ready)
  await expect(
    page.getByRole('checkbox', {
      name: `${ru.selected}: ${ru.cleanupNames.recycle}`,
    }),
  ).not.toBeChecked()
  await expect(
    page.getByRole('checkbox', {
      name: `${ru.selected}: ${ru.cleanupNames.documents}`,
    }),
  ).toBeDisabled()
  const action = page.getByRole('button', { name: ru.cleanupAction })
  await action.click()
  const confirm = page.getByRole('alertdialog', { name: ru.cleanupConfirm })
  await expect(confirm).toBeVisible()
  const box = await confirm.boundingBox()
  expect(box).not.toBeNull()
  expect(box!.y).toBeGreaterThanOrEqual(0)
  expect(box!.y + box!.height).toBeLessThanOrEqual(
    await page.evaluate(() => innerHeight),
  )
  await page.keyboard.press('Escape')
  await expect(confirm).not.toBeVisible()
  await expect(action).toBeFocused()
  await action.click()
  await page.getByRole('button', { name: ru.confirmExample }).click()
  await expect(page.getByRole('status')).toContainText(ru.exampleNoChanges)

  await navigate(page, ru.disks)
  await exampleState(page, ru.states.ready)
  await page.getByRole('button', { name: /^Windows / }).click()
  await expect(page.getByRole('button', { name: ru.toRecycle })).toBeDisabled()
  await page.getByRole('tab', { name: ru.tree }).click()
  await expect(page.getByRole('table', { name: ru.tree })).toBeVisible()

  await navigate(page, ru.startup)
  await exampleState(page, ru.states.ready)
  const startup = page.getByRole('switch', {
    name: ru.startupNames.drive,
    includeHidden: true,
  })
  await startup.click()
  await expect(startup).toBeChecked()
  expect(
    await startup.evaluate(
      (element) => getComputedStyle(element).backgroundColor,
    ),
  ).not.toBe('rgba(0, 0, 0, 0)')
  await page.getByRole('button', { name: ru.cancel, exact: true }).click()
  await expect(startup).toBeChecked()
  await startup.click()
  await page.getByRole('button', { name: ru.confirmExample }).click()
  await expect(startup).not.toBeChecked()

  await navigate(page, ru.apps)
  await exampleState(page, ru.states.ready)
  await page
    .getByRole('textbox', { name: ru.searchApps })
    .fill('несуществующая программа')
  await expect(page.getByText(ru.noSearchResults)).toBeVisible()
  await page.getByRole('textbox', { name: ru.searchApps }).clear()
  await page
    .getByRole('button', { name: `${ru.details}: ${ru.appNames.editor}` })
    .click()
  await expect(
    page.getByRole('dialog', { name: ru.appNames.editor }),
  ).toBeVisible()
  await page.getByRole('button', { name: ru.uninstall, exact: true }).click()
  await expect(page.getByRole('alertdialog')).toContainText(ru.previewWarning)
  await page.getByRole('button', { name: ru.cancel, exact: true }).click()
  await expect(
    page.getByRole('dialog', { name: ru.appNames.editor }),
  ).toBeVisible()
  await page.keyboard.press('Escape')
}
