// Pair the explicit Vite recipe mode with recipe-only Rust IPC; normal assets stay intact.
import { spawnSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
const repo = fileURLToPath(new URL('../../', import.meta.url))
const run = (command, args, options = {}) => {
  const result = spawnSync(command, args, { cwd: repo, stdio: 'inherit', ...options })
  if (result.error) throw result.error
  if (result.status !== 0) process.exit(result.status || 1)
}
run(process.execPath, [
  process.env.npm_execpath,
  'run',
  'build',
  '--workspace',
  '@typstographe/app',
  '--',
  '--mode',
  'recipe',
])
run(
  'cargo',
  [
    'build',
    '--locked',
    '-p',
    'typstographe',
    '-p',
    'typstographe-cli',
    '--features',
    'typstographe/recipe-driver',
    ...process.argv.slice(2),
  ],
  {
    env: {
      ...process.env,
      TAURI_CONFIG: JSON.stringify({ build: { frontendDist: '../dist-recipe' } }),
    },
  },
)
