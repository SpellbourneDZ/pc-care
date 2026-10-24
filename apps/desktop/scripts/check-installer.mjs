import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { readFile, readdir, writeFile } from 'node:fs/promises'
import { resolve } from 'node:path'

export function verifyPe(data, machine) {
  assert(
    data.length >= 64 && data.toString('ascii', 0, 2) === 'MZ',
    'Missing DOS header',
  )
  const offset = data.readUInt32LE(0x3c)
  assert(offset >= 64 && offset + 24 <= data.length, 'Truncated PE header')
  assert(data.readUInt32LE(offset) === 0x4550, 'Missing PE signature')
  if (machine)
    assert.equal(
      data.readUInt16LE(offset + 4),
      machine,
      'Wrong app architecture',
    )
}

if (import.meta.main) {
  const root = resolve(import.meta.dirname, '../../../target/release')
  const bundle = resolve(root, 'bundle/nsis')
  const names = (await readdir(bundle)).filter((name) =>
    name.endsWith('_x64-setup.exe'),
  )
  assert.equal(names.length, 1, 'Expected one draft x64 NSIS installer')
  const files = []
  for (const [path, name, machine] of [
    [resolve(root, 'pc-care.exe'), 'pc-care.exe', 0x8664],
    [resolve(bundle, names[0]), names[0], undefined],
  ]) {
    const data = await readFile(path)
    assert(data.length > 64 * 1024, 'Unexpectedly small artifact')
    // NSIS uses an x86 setup stub even when it packages an x64 application.
    verifyPe(data, machine)
    files.push({
      name,
      bytes: data.length,
      sha256: createHash('sha256').update(data).digest('hex'),
    })
  }
  const manifest = {
    draft: true,
    signing: 'not configured',
    requires_webview2: true,
    files,
  }
  await writeFile(
    resolve(bundle, 'draft-manifest.json'),
    JSON.stringify(manifest, null, 2) + '\n',
  )
  console.log(JSON.stringify(manifest))
}
