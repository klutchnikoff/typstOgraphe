import assert from 'node:assert/strict'
import { spawn, execFileSync } from 'node:child_process'
import { mkdir, readdir, readFile, writeFile } from 'node:fs/promises'
import { fileURLToPath } from 'node:url'
import path from 'node:path'

const repo = fileURLToPath(new URL('../../', import.meta.url))
const root = path.resolve(
  process.argv[2] || path.join(repo, 'output/a3-conservation', `run-${Date.now()}`),
)
const executable =
  process.env.TOG_SPIKE_BINARY ||
  path.join(
    repo,
    'target/recipes/debug',
    `typstographe${process.platform === 'win32' ? '.exe' : ''}`,
  )
await mkdir(root, { recursive: true })
assert.equal((await readdir(root)).length, 0, 'Use a new empty recipe directory')
const run = async (mode) => {
  const verify = mode !== 'conservation'
  const child = spawn(executable, [`--diagnostic-draft-${mode}`, '--session-dir', root], {
    stdio: ['ignore', 'pipe', 'pipe'],
  })
  let log = '',
    interrupted = false
  const collect = (bytes) => {
    log += bytes
    if (!verify && log.includes('draftConservationReady')) {
      interrupted = true
      child.kill('SIGKILL')
    }
  }
  child.stdout.on('data', collect)
  child.stderr.on('data', collect)
  const timeout = setTimeout(() => child.kill('SIGKILL'), 90000)
  const code = await new Promise((resolve, reject) => {
    child.once('exit', resolve)
    child.once('error', reject)
  }).finally(() => clearTimeout(timeout))
  await writeFile(path.join(root, `${mode}.log`), log)
  if (!verify) {
    assert.ok(interrupted, log)
    return
  }
  const report = JSON.parse(
    await readFile(path.join(root, `native-draft-${mode}-report.json`), 'utf8'),
  )
  assert.equal(code, 0, JSON.stringify(report))
  assert.equal(report.success, true, JSON.stringify(report))
  return report
}
try {
  await run('conservation')
  const existing = await run('verify')
  const report = await run('recover')
  report.confirmedReopen = existing.checks
  const exported = JSON.parse(await readFile(path.join(root, 'exported-draft.json'), 'utf8'))
  assert.equal(exported.format, 'typstographe-private-recovery-v1')
  assert.equal(exported.input.recovered.length, 1)
  assert.ok(JSON.stringify(exported.input.recovered).includes('Native unconfirmed note'))
  report.checks.push('native draft export preserves private input and confirmed base')
  report.git = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: repo, encoding: 'utf8' }).trim()
  report.platform = process.platform
  report.interruption = 'SIGKILL after automatic checkpoint and explicit confirmed save'
  await writeFile(path.join(root, 'report.json'), JSON.stringify(report, null, 2) + '\n')
  console.log(JSON.stringify({ root, ...report }, null, 2))
} catch (error) {
  console.error(String(error))
  process.exitCode = 1
}
