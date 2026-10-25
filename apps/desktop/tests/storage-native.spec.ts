import { randomUUID } from 'node:crypto'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { resolve } from 'node:path'
import { expect, test } from '@playwright/test'
import { ru } from '../src/i18n/ru'
import { startNative } from './native-app'

test('storage native: real metadata scan, map/list/inspector and filters preserve synthetic files', async ({
  playwright,
}, info) => {
  test.skip(process.platform !== 'win32', 'Windows only')
  const root = resolve(
    import.meta.dirname,
    '../../../target/native-fixtures',
    randomUUID(),
  )
  const folder = resolve(root, 'Profile/Storage/папка')
  await mkdir(folder, { recursive: true })
  const file = resolve(folder, 'искусственный.bin'),
    data = Buffer.alloc(65536, 42)
  await writeFile(file, data)
  for (const [name, size] of [
    ['архив', 196608],
    ['видео', 131072],
    ['документы', 32768],
  ] as const) {
    const dir = resolve(root, 'Profile/Storage', name)
    await mkdir(dir, { recursive: true })
    await writeFile(
      resolve(dir, 'искусственные-данные.bin'),
      Buffer.alloc(size, 42),
    )
  }
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
    await page.getByRole('link', { name: ru.disks, exact: true }).click()
    await page
      .getByLabel(ru.storage.path, { exact: true })
      .fill(resolve(root, 'Profile/Storage'))
    await page.getByRole('button', { name: ru.scan, exact: true }).click()
    const list = page.getByRole('list', { name: ru.storage.list })
    await expect(list.getByRole('listitem')).toHaveCount(4)
    await expect(
      page.getByText(ru.cleanupUx.fixture, { exact: true }),
    ).toBeVisible()
    await page.getByLabel(ru.map, { exact: true }).scrollIntoViewIfNeeded()
    await page.screenshot({
      path: info.outputPath('storage-native-map.png'),
      fullPage: true,
    })
    await list
      .getByRole('button', { name: `${ru.storage.enter}: папка`, exact: true })
      .click()
    await expect(list.getByRole('listitem')).toHaveCount(1)
    await list.getByRole('button').first().click()
    const inspector = page.getByRole('complementary', {
      name: ru.storage.inspect,
    })
    await expect(inspector).toContainText('искусственный.bin')
    await expect(
      inspector.getByRole('button', { name: ru.cleanupUx.confirm }),
    ).toBeDisabled()
    await inspector.scrollIntoViewIfNeeded()
    await page.screenshot({
      path: info.outputPath('storage-native-inspector.png'),
      fullPage: true,
    })
    await page
      .getByLabel(ru.storage.search, { exact: true })
      .fill('нет такого файла')
    await page.getByRole('button', { name: ru.storage.apply }).click()
    await expect(page.getByText(ru.storage.empty)).toBeVisible()
    expect(await readFile(file)).toEqual(data)
    expect(app.errors).toEqual([])
  } finally {
    await app.close()
  }
})
