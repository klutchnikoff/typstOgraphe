// Inspect emitted assets, not just source conditionals: diagnostics must be absent.
import assert from 'node:assert/strict'
import { readdir, readFile } from 'node:fs/promises'
import { fileURLToPath } from 'node:url'
import path from 'node:path'
const directory = fileURLToPath(new URL('../dist', import.meta.url))
const names = await readdir(path.join(directory, 'assets'))
assert.ok(
  names.some((name) => name.endsWith('.js')),
  'Build the normal frontend first',
)
for (const name of names.filter((name) => name.endsWith('.js'))) {
  const text = await readFile(path.join(directory, 'assets', name), 'utf8')
  assert.doesNotMatch(
    name,
    /diagnostic|synchronization-performance|preview-performance|browser-transport/,
  )
  for (const marker of [
    'diagnostic_mode',
    'diagnostic_finish',
    '__SPIKE_TRANSPORT__',
    '__spike',
    'prepare_rust_edit',
  ])
    assert.ok(!text.includes(marker), `Recipe marker ${marker} in distributed asset ${name}`)
}
console.log('Distributed frontend contains no recipe entry points, globals or diagnostic chunks.')
