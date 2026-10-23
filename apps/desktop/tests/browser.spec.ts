import { expect, test } from '@playwright/test'
import { ru } from '../src/i18n/ru'

test('opens offline, uses the shadcn button with keyboard, and fits the window', async ({
  page,
  context,
}, testInfo) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  await page.goto('/')
  await expect(page.getByRole('heading', { name: ru.emptyTitle })).toBeVisible()
  await context.setOffline(true)
  const about = page.getByRole('button', { name: ru.about })
  await page.keyboard.press('Tab')
  await expect(about).toBeFocused()
  await page.keyboard.press('Enter')
  await expect(page.getByText(ru.aboutDescription)).toBeVisible()
  await expect(about).toHaveAttribute('aria-expanded', 'true')
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true)
  expect(errors).toEqual([])
  await page.screenshot({
    path: testInfo.outputPath('shell.png'),
    fullPage: true,
  })
})
