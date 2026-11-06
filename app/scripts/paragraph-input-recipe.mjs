// Actual Tauri WebView/IPC on an isolated archive copy. No screenshots or personal-session writes.
import { spawn } from 'node:child_process'
import { mkdir, copyFile, readFile, writeFile } from 'node:fs/promises'
import { createHash } from 'node:crypto'
import { fileURLToPath } from 'node:url'
import path from 'node:path'
import assert from 'node:assert/strict'
const repo = fileURLToPath(new URL('../../', import.meta.url))
const manual = process.argv.includes('--manual')
const input = path.resolve(
  process.argv.slice(2).find((arg) => !arg.startsWith('--')) ||
    path.join(repo, 'fixtures/projects/article-working.typsto'),
)
const root = path.join(repo, 'output/paragraph-input-native', `run-${Date.now()}`)
await mkdir(root, { recursive: true })
const original = await readFile(input)
await copyFile(input, path.join(root, 'input.typsto'))
const binary =
  process.env.TOG_SPIKE_BINARY ||
  path.join(
    repo,
    'target/recipes/debug',
    `typstographe${process.platform === 'win32' ? '.exe' : ''}`,
  )
const mode = manual ? 'paragraph-input-manual' : 'paragraph-input'
const child = spawn(binary, [`--diagnostic-${mode}`, '--session-dir', root], {
  stdio: ['ignore', 'pipe', 'pipe'],
})
let log = ''
for (const stream of [child.stdout, child.stderr])
  stream.on('data', (data) => {
    log += data
    if (manual) process.stderr.write(data)
  })
const timeout = setTimeout(() => child.kill(), manual ? 190000 : 45000)
const code = await new Promise((resolve, reject) => {
  child.once('exit', resolve)
  child.once('error', reject)
}).finally(() => clearTimeout(timeout))
await writeFile(path.join(root, 'native.log'), log)
const report = JSON.parse(await readFile(path.join(root, `native-${mode}-report.json`), 'utf8'))
const digest = (b) => createHash('sha256').update(b).digest('hex')
assert.equal(digest(await readFile(input)), digest(original), 'The original archive changed')
assert.equal(
  digest(await readFile(path.join(root, 'input.typsto'))),
  digest(original),
  'The archive copy was unexpectedly saved',
)
assert.equal(code, 0, report.error || log)
assert.equal(report.success, true, report.error)
assert.deepEqual(
  report.cases.map((c) => [c.test, c.changed]),
  [
    ['enter-at-end', true],
    ['backspace-at-start', true],
    ['backspace-at-separator', true],
  ],
)
console.log(`Native paragraph input passed: ${root}`)
