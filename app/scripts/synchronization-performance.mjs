// Native IPC, embedded production frontend. Build before running; no Vite server.
import { spawn, execFileSync } from 'node:child_process'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import path from 'node:path'
import os from 'node:os'
const root = path.resolve(process.argv[2] || 'output/synchronization-performance')
const binary = process.env.TOG_SPIKE_BINARY || path.resolve('target/debug/typstographe')
await mkdir(root, { recursive: true })
const mode = process.env.TOG_SYNC_ACTIVE === '1' ? 'synchronization-active' : 'synchronization'
const child = spawn(binary, [`--diagnostic-${mode}`, '--session-dir', root], {
  stdio: ['ignore', 'pipe', 'pipe'],
})
let log = ''
for (const stream of [child.stdout, child.stderr])
  stream.on('data', (data) => {
    log += data
    process.stderr.write(data)
  })
const timeout = setTimeout(() => child.kill(), 120_000)
const code = await new Promise((resolve, reject) => {
  child.on('error', reject)
  child.on('exit', resolve)
})
clearTimeout(timeout)
await writeFile(path.join(root, 'native.log'), log)
const report = JSON.parse(await readFile(path.join(root, `native-${mode}-report.json`), 'utf8'))
report.environment = {
  platform: process.platform,
  release: os.release(),
  architecture: os.arch(),
  cpu: os.cpus()[0]?.model,
  binary,
  git: execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim(),
  profile: process.env.TOG_BENCH_PROFILE || 'debug; embedded production frontend',
}
if (report.latencyMs) {
  const sorted = [...report.latencyMs].sort((a, b) => a - b)
  report.summary = {
    n: sorted.length,
    confirmationP50Ms: sorted[Math.ceil(sorted.length * 0.5) - 1],
    confirmationP95Ms: sorted[Math.ceil(sorted.length * 0.95) - 1],
  }
}
await writeFile(
  path.join(root, `native-${mode}-report.json`),
  JSON.stringify(report, null, 2) + '\n',
)
console.log(JSON.stringify({ code, ...report }, null, 2))
if (code !== 0 || !report.success) process.exitCode = 1
