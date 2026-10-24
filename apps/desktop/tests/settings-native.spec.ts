import { randomUUID } from 'node:crypto'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { resolve } from 'node:path'
import { expect, test } from '@playwright/test'
import { ru } from '../src/i18n/ru'
import { startNative } from './native-app'
import type { ApiReply } from '../src/generated/ApiReply'
import type { ApiRequest } from '../src/generated/ApiRequest'
import type { TaskProgress } from '../src/generated/TaskProgress'

test('settings persist and cleanup scan stays read-only using only isolated fixtures', async ({
  playwright,
}, testInfo) => {
  test.skip(process.platform !== 'win32', 'Windows x64 only')
  const root = resolve(
    import.meta.dirname,
    '../../../target/native-fixtures',
    randomUUID(),
  )
  await mkdir(root, { recursive: true })
  const temp = resolve(root, 'Profile/AppData/Local/Temp')
  await mkdir(temp, { recursive: true })
  const sentinel = resolve(temp, 'artificial-recent-file.txt')
  await writeFile(sentinel, 'PC Care read-only fixture')
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
    const analysis = await first.page.evaluate(async () => {
      const native = (
        window as unknown as {
          __TAURI_INTERNALS__: {
            invoke<T>(
              command: string,
              args: Record<string, unknown>,
            ): Promise<T>
            transformCallback(
              callback: (data: { message?: TaskProgress }) => void,
              once: boolean,
            ): number
            unregisterCallback(id: number): void
          }
        }
      ).__TAURI_INTERNALS__
      const events: TaskProgress[] = []
      const callback = native.transformCallback((packet) => {
        if (packet.message) events.push(packet.message)
      }, false)
      const call = async (request: ApiRequest) => {
        const reply = await native.invoke<ApiReply>('operation_api', {
          request,
          progress: `__CHANNEL__:${callback}`,
        })
        if (reply.status === 'error') throw new Error(reply.error)
        return reply.value
      }
      try {
        const rules = await call({ type: 'get_cleanup_rules' })
        const started = await call({
          type: 'start_scan',
          data: { kind: 'cleanup', scenario: 'complete', timeout_ms: 5_000 },
        })
        if (started.type !== 'scan_started') throw new Error('Missing scan ID')
        for (let attempt = 0; attempt < 250; attempt++) {
          const task = await call({
            type: 'get_task',
            data: { task_id: started.data.task_id },
          })
          if (task.type !== 'task') throw new Error('Missing task')
          if (task.data.status === 'failed')
            throw new Error(task.data.error ?? 'Scan failed')
          if (
            task.data.scan_id &&
            events.some((event) => event.status === 'completed')
          ) {
            const result = await call({
              type: 'get_scan_result',
              data: { scan_id: task.data.scan_id, offset: 0, limit: 500 },
            })
            return { rules, result, events }
          }
          await new Promise((resolve) => setTimeout(resolve, 20))
        }
        throw new Error('No terminal progress received')
      } finally {
        native.unregisterCallback(callback)
      }
    })
    if (
      analysis.rules.type !== 'cleanup_rules' ||
      analysis.result.type !== 'scan_result'
    )
      throw new Error('Unexpected cleanup response')
    expect(analysis.rules.data.rules).toHaveLength(3)
    expect(analysis.rules.data.fixture_mode).toBe(true)
    expect(analysis.result.data.candidates).toEqual([])
    expect(analysis.result.data.issues).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ reason: 'too_recent', count: 1 }),
      ]),
    )
    expect(analysis.result.data.truncated).toBe(false)
    expect(analysis.events.every((event) => event.total === null)).toBe(true)
    expect(await readFile(sentinel, 'utf8')).toBe('PC Care read-only fixture')
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
