// Pair the explicit Vite recipe mode with recipe-only Rust IPC; normal assets stay intact.
import { spawnSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { copyFileSync, mkdirSync, mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import assert from 'node:assert/strict'
const repo = fileURLToPath(new URL('../../', import.meta.url))
const run = (command, args, options = {}) => {
  const result = spawnSync(command, args, { cwd: repo, stdio: 'inherit', ...options })
  if (result.error) throw result.error
  if (result.status !== 0) {
    if (result.stdout) process.stderr.write(result.stdout)
    process.exit(result.status || 1)
  }
  return result
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
const compilation = run(
  'cargo',
  [
    'build',
    '--message-format=json-render-diagnostics',
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
    stdio: ['ignore', 'pipe', 'inherit'],
    encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024,
    env: {
      ...process.env,
      TAURI_CONFIG: JSON.stringify({ build: { frontendDist: '../dist-recipe' } }),
    },
  },
)

const artifacts = compilation.stdout
  .split('\n')
  .filter(Boolean)
  .map((line) => JSON.parse(line))
for (const artifact of artifacts)
  if (artifact.reason === 'compiler-message')
    process.stderr.write(artifact.message.rendered || artifact.message.message)
let application
for (const name of ['typstographe', 'typstographe-cli']) {
  const artifact = artifacts.find(
    (value) =>
      value.reason === 'compiler-artifact' && value.target.name === name && value.executable,
  )
  assert.ok(artifact, `No executable artifact for ${name}`)
  if (name === 'typstographe') assert.ok(artifact.features.includes('recipe-driver'))
  const profile = path.dirname(artifact.executable)
  const directory = path.resolve(profile, '../recipes', path.basename(profile))
  mkdirSync(directory, { recursive: true })
  const destination = path.join(directory, path.basename(artifact.executable))
  copyFileSync(artifact.executable, destination)
  if (name === 'typstographe') application = destination
  console.log(`Recipe executable: ${destination}`)
}
// Fail immediately if a concurrent development build replaced Cargo's conventional output.
const probe = mkdtempSync(path.join(tmpdir(), 'tog-recipe-build-'))
try {
  const checked = spawnSync(application, ['--stdio', '--session-dir', probe], {
    input: '',
    encoding: 'utf8',
    timeout: 15000,
  })
  assert.equal(
    checked.status,
    0,
    `Recipe capability check failed: ${checked.stderr}. Rebuild the recipe.`,
  )
} finally {
  rmSync(probe, { recursive: true, force: true })
}
