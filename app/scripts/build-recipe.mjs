// Pair the explicit Vite recipe mode with recipe-only Rust IPC; normal assets stay intact.
import { spawnSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { copyFileSync, mkdirSync, mkdtempSync, rmSync, renameSync, constants } from 'node:fs'
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
const staged = []
const probe = mkdtempSync(path.join(tmpdir(), 'tog-recipe-build-'))
try {
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
    const temporary = path.join(
      directory,
      `.build-${process.pid}-${path.basename(artifact.executable)}`,
    )
    staged.push({ temporary, destination })
    copyFileSync(artifact.executable, temporary, constants.COPYFILE_FICLONE)
    if (name === 'typstographe') application = temporary
  }
  // Check the candidate before replacing a previously working recipe executable.
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
  for (const { temporary, destination } of staged) {
    // Atomic replacement leaves an already running Unix process on its original inode.
    // Windows refuses replacement of an in-use executable instead of truncating it.
    renameSync(temporary, destination)
    console.log(`Recipe executable: ${destination}`)
  }
} finally {
  for (const { temporary } of staged) rmSync(temporary, { force: true })
  rmSync(probe, { recursive: true, force: true })
}
