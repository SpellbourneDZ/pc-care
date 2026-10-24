import { spawnSync } from 'node:child_process'
import { resolve } from 'node:path'

if (!process.env.npm_execpath) throw new Error('Run through pnpm build:qa')
const result = spawnSync(
  process.execPath,
  [
    process.env.npm_execpath,
    'exec',
    'tauri',
    'build',
    '--no-bundle',
    '--features',
    'qa-fixtures',
  ],
  {
    stdio: 'inherit',
    env: {
      ...process.env,
      CARGO_TARGET_DIR: resolve('../../target/native-qa'),
    },
  },
)
if (result.error) throw result.error
process.exit(result.status ?? 1)
