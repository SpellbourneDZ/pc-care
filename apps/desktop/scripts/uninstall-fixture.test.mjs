import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { execFileSync } from 'node:child_process'
import { readFileSync, writeFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { test } from 'node:test'

const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i
const tables = [
  'Property',
  'Directory',
  'Component',
  'Feature',
  'FeatureComponents',
  'File',
  'Media',
  'LaunchCondition',
  'InstallExecuteSequence',
  'InstallUISequence',
]
function build() {
  const result = execFileSync(
    'powershell.exe',
    [
      '-NoProfile',
      '-NonInteractive',
      '-File',
      fileURLToPath(new URL('./build-uninstall-fixture.ps1', import.meta.url)),
    ],
    { encoding: 'utf8', timeout: 30_000, windowsHide: true },
  )
  const data = JSON.parse(result.trim().replace(/^\uFEFF/, ''))
  assert.match(data.instance, uuid)
  for (const field of [
    'product_code',
    'component_code',
    'upgrade_code',
    'package_code',
  ]) {
    assert.match(data[field].slice(1, -1), uuid)
    assert.equal(data[field][0], '{')
    assert.equal(data[field].at(-1), '}')
  }
  assert.equal(data.scope, 'current_user')
  assert.equal(data.vm_only, true)
  assert.equal(data.installed_on_host, false)
  assert.equal(data.package, `VM-ONLY-${data.instance}.msi`)
  assert.equal(data.payload, 'fixture.txt')
  assert.equal(
    data.required_install_property,
    `PC_CARE_VM_FIXTURE=${data.instance}`,
  )
  assert.equal(data.destination_suffix, `PC Care VM Fixtures\\${data.instance}`)
  assert.deepEqual(data.tables, tables)
  const root = resolve(
    import.meta.dirname,
    '../../../target/uninstall-fixtures',
    data.instance,
  )
  const packageBytes = readFileSync(resolve(root, data.package)),
    payload = readFileSync(resolve(root, data.payload))
  assert.equal(packageBytes.subarray(0, 8).toString('hex'), 'd0cf11e0a1b11ae1')
  assert.equal(
    createHash('sha256').update(packageBytes).digest('hex'),
    data.msi_sha256,
  )
  assert.equal(
    createHash('sha256').update(payload).digest('hex'),
    data.payload_sha256,
  )
  assert.ok(payload.includes(data.instance))
  assert.ok(payload.includes(data.product_code))
  assert.deepEqual(
    JSON.parse(
      readFileSync(resolve(root, 'manifest.json'), 'utf8').replace(
        /^\uFEFF/,
        '',
      ),
    ),
    data,
  )
  return data
}
test(
  'MSI fixtures are built/read without installation; same-name products retain separate owned identities',
  { skip: process.platform !== 'win32' },
  () => {
    const first = build(),
      second = build()
    assert.equal(first.name, second.name)
    assert.equal(first.publisher, second.publisher)
    for (const key of [
      'instance',
      'product_code',
      'component_code',
      'upgrade_code',
      'package_code',
      'destination_suffix',
    ])
      assert.notEqual(first[key], second[key])
    writeFileSync(
      resolve(
        import.meta.dirname,
        '../../../target/uninstall-fixtures/last-tested.json',
      ),
      JSON.stringify({ schema_version: 1, fixtures: [first, second] }, null, 2),
    )
  },
)
