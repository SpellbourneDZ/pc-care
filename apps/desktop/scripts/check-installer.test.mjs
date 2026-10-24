import assert from 'node:assert/strict'
import { test } from 'node:test'
import { verifyPe } from './check-installer.mjs'

test('artifact smoke rejects truncation, invalid signatures and wrong app architecture', () => {
  const data = Buffer.alloc(128)
  data.write('MZ')
  data.writeUInt32LE(64, 0x3c)
  data.writeUInt32LE(0x4550, 64)
  data.writeUInt16LE(0x8664, 68)
  verifyPe(data, 0x8664)
  assert.throws(() => verifyPe(data, 0x14c))
  assert.throws(() => verifyPe(data.subarray(0, 70)))
  const wrongSignature = Buffer.from(data)
  wrongSignature.writeUInt32LE(0, 64)
  assert.throws(() => verifyPe(wrongSignature))
  data.writeUInt32LE(0xffffffff, 0x3c)
  assert.throws(() => verifyPe(data))
  assert.throws(() => verifyPe(Buffer.alloc(0)))
})
