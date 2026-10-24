import { act, fireEvent, render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, expect, test, vi } from 'vitest'
import { App } from './app'
import { ru } from './i18n/ru'
import { PreviewWorkspace } from './preview/workspace'
import { CleanupScreen } from './preview/cleanup'
import { StartupScreen } from './preview/startup'
import { AppsScreen } from './preview/apps'

beforeEach(() => window.history.replaceState(null, '', '#/preview/overview'))

test('starts with prototype provenance and opens/closes about with keyboard', async () => {
  const user = userEvent.setup()
  render(<App />)
  expect(screen.getByRole('heading', { name: ru.emptyTitle })).toBeVisible()
  expect(screen.getByText(ru.exampleData)).toBeVisible()
  expect(screen.queryByRole('table')).not.toBeInTheDocument()
  const about = screen.getByRole('button', { name: ru.about })
  about.focus()
  await user.keyboard('{Enter}')
  expect(screen.getByRole('dialog', { name: ru.aboutTitle })).toBeVisible()
  await user.keyboard('{Escape}')
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
  expect(about).toHaveFocus()
})
test('bulk selection excludes recycle/protected and cancellation never confirms', async () => {
  const user = userEvent.setup()
  render(<CleanupScreen />)
  const recycle = screen.getByRole('checkbox', {
    name: `${ru.selected}: ${ru.cleanupNames.recycle}`,
  })
  const protectedBox = screen.getByRole('checkbox', {
    name: `${ru.selected}: ${ru.cleanupNames.documents}`,
  })
  const bulk = screen.getByRole('checkbox', { name: ru.allCategories })
  expect(recycle).not.toBeChecked()
  expect(protectedBox).toBeDisabled()
  await user.click(bulk)
  expect(screen.getByRole('button', { name: ru.cleanupAction })).toBeDisabled()
  await user.click(bulk)
  expect(recycle).not.toBeChecked()
  const action = screen.getByRole('button', { name: ru.cleanupAction })
  await user.click(action)
  const dialog = screen.getByRole('alertdialog', { name: ru.cleanupConfirm })
  expect(within(dialog).getByText(ru.previewWarning)).toBeVisible()
  expect(
    within(dialog).queryByText(ru.cleanupNames.recycle),
  ).not.toBeInTheDocument()
  await user.click(within(dialog).getByRole('button', { name: ru.cancel }))
  expect(screen.queryByText(ru.exampleResult)).not.toBeInTheDocument()
  expect(action).toHaveFocus()
  await user.click(action)
  await user.click(screen.getByRole('button', { name: ru.confirmExample }))
  expect(screen.getByRole('status')).toHaveTextContent(ru.exampleNoChanges)
})
test('startup switch changes only after confirmation and protected source stays read-only', async () => {
  const user = userEvent.setup()
  render(<StartupScreen />)
  const control = screen.getByRole('switch', { name: ru.startupNames.drive })
  expect(control).toBeChecked()
  expect(
    screen.getByRole('switch', { name: ru.startupNames.security }),
  ).toBeDisabled()
  await user.click(control)
  expect(control).toBeChecked()
  await user.click(screen.getByRole('button', { name: ru.cancel }))
  expect(control).toBeChecked()
  await user.click(control)
  await user.click(screen.getByRole('button', { name: ru.confirmExample }))
  expect(control).not.toBeChecked()
})
test('app search handles no matches and uninstall preview requires confirmation', async () => {
  const user = userEvent.setup()
  render(<AppsScreen />)
  const search = screen.getByRole('textbox', { name: ru.searchApps })
  await user.type(search, 'нет такой программы')
  expect(screen.getByText(ru.noSearchResults)).toBeVisible()
  await user.clear(search)
  await user.click(
    screen.getByRole('button', {
      name: `${ru.details}: ${ru.appNames.editor}`,
    }),
  )
  await user.click(screen.getByRole('button', { name: ru.uninstall }))
  expect(screen.getByRole('alertdialog')).toHaveTextContent(ru.uninstallWarning)
  await user.click(screen.getByRole('button', { name: ru.confirmExample }))
  expect(screen.getByRole('status')).toHaveTextContent(ru.exampleNoChanges)
  await user.click(screen.getByRole('button', { name: ru.findLeftovers }))
  expect(screen.getByText(ru.leftoversDescription)).toBeVisible()
})
test('cancelling a simulated scan prevents delayed findings', async () => {
  vi.useFakeTimers()
  try {
    render(
      <PreviewWorkspace
        page="cleanup"
        onNavigate={() => {}}
        onExit={() => {}}
      />,
    )
    fireEvent.click(screen.getByRole('button', { name: ru.scan }))
    expect(screen.getByRole('heading', { name: ru.loading })).toBeVisible()
    fireEvent.click(screen.getByRole('button', { name: ru.cancel }))
    await act(() => vi.advanceTimersByTimeAsync(1000))
    expect(screen.queryByRole('table')).not.toBeInTheDocument()
    expect(screen.getByRole('heading', { name: ru.emptyModule })).toBeVisible()
  } finally {
    vi.useRealTimers()
  }
})
