import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { expect, test } from 'vitest'
import { App } from './app'
import { ru } from './i18n/ru'

test('starts without fabricated scan results and opens about with the keyboard', async () => {
  const user = userEvent.setup()
  render(<App />)
  expect(screen.getByRole('heading', { name: ru.emptyTitle })).toBeVisible()
  expect(screen.getByText(ru.noChanges)).toBeVisible()
  const about = screen.getByRole('button', { name: ru.about })
  expect(about).toHaveAttribute('aria-expanded', 'false')
  expect(screen.getByText(ru.aboutDescription)).not.toBeVisible()
  await user.tab()
  expect(about).toHaveFocus()
  await user.keyboard('{Enter}')
  expect(about).toHaveAttribute('aria-expanded', 'true')
  expect(screen.getByText(ru.aboutDescription)).toBeVisible()
  await user.keyboard(' ')
  expect(screen.getByText(ru.aboutDescription)).not.toBeVisible()
})
