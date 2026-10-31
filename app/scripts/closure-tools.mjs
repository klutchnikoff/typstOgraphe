import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { readFile, readdir } from 'node:fs/promises'
import { createHash } from 'node:crypto'
import path from 'node:path'
export const hash = (bytes) => createHash('sha256').update(bytes).digest('hex')
export function cliClient(binary, cwd) {
  return (args, success = true) => {
    const result = spawnSync(binary, [...args, '--lang', 'en', '--json'], {
      cwd,
      encoding: 'utf8',
      maxBuffer: 16 * 1024 * 1024,
      timeout: 60000,
    })
    if (result.error) throw result.error
    const value = JSON.parse(result.stdout)
    assert.equal(value.ok, success, `${args.join(' ')}: ${result.stdout} ${result.stderr}`)
    assert.equal(result.status, success ? 0 : 1, result.stderr)
    return value.outcome || value.error
  }
}
export const bodyTarget = (zone, offset) => ({
  zone: zone.zone,
  from: offset ?? zone.length,
  to: offset ?? zone.length,
})
export function lastBody(structure) {
  const zone = structure.zones
    .filter((z) => z.zone.kind === 'body' && z.zone.path.length === 1)
    .at(-1)
  assert.ok(zone, 'No body insertion zone')
  return zone
}
export function catalogueShape(catalogue) {
  return catalogue.entries
    .map(({ id, params_schema }) => ({ id, params_schema }))
    .sort((a, b) => a.id.localeCompare(b.id))
}
export async function files(directory, prefix = '') {
  const result = []
  for (const entry of await readdir(path.join(directory, prefix), { withFileTypes: true })) {
    const name = path.posix.join(prefix, entry.name)
    if (entry.isDirectory()) result.push(...(await files(directory, name)))
    else if (entry.isFile()) result.push(name)
    else throw new Error(`Unexpected export entry ${name}`)
  }
  return result.sort()
}
export async function compareExports(a, b) {
  // PDFs may contain creation timestamps. Compare their magic, then all source,
  // metadata, dependencies and resource bytes; never normalize their semantics.
  const left = await files(a),
    right = await files(b)
  assert.deepEqual(left, right, 'Export file manifest differs')
  for (const name of left) {
    const one = await readFile(path.join(a, name)),
      two = await readFile(path.join(b, name))
    if (name.endsWith('.pdf')) {
      assert.ok(one.subarray(0, 4).equals(Buffer.from('%PDF')))
      assert.ok(two.subarray(0, 4).equals(Buffer.from('%PDF')))
    } else assert.ok(one.equals(two), `Export differs: ${name}`)
  }
  return left.length
}
export function comparePages(a, b) {
  assert.equal(a.length, b.length)
  for (let i = 0; i < a.length; i++)
    for (let axis = 0; axis < 2; axis++)
      assert.ok(Math.abs(a[i][axis] - b[i][axis]) < 0.01, 'Page geometry differs')
}
export async function until(check, label) {
  const deadline = Date.now() + 30000
  let value
  while (!(value = await check())) {
    if (Date.now() > deadline) throw new Error(`Timeout: ${label}`)
    await new Promise((resolve) => setTimeout(resolve, 50))
  }
  return value
}
