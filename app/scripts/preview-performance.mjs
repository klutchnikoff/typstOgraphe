// Build first with npm run build:recipe -- --release.
// Runs the real native app with embedded production frontend assets, never Vite/HTTP.
import { spawn, execFileSync } from 'node:child_process'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { fileURLToPath } from 'node:url'
import path from 'node:path'
import os from 'node:os'
const repo = fileURLToPath(new URL('../../', import.meta.url))
const root = path.resolve(
  process.argv[2] || path.join(repo, 'output/web-editor', `preview-native-${Date.now()}`),
)
const binary =
  process.env.TOG_SPIKE_BINARY ||
  path.join(repo, 'target/release', `typstographe${process.platform === 'win32' ? '.exe' : ''}`)
await mkdir(root, { recursive: true })
const child = spawn(binary, ['--diagnostic-performance', '--session-dir', root], {
  stdio: ['ignore', 'pipe', 'pipe'],
})
let log = ''
for (const stream of [child.stdout, child.stderr])
  stream.on('data', (data) => {
    log += data
    process.stderr.write(data)
  })
const timeout = setTimeout(() => child.kill(), 180_000)
const code = await new Promise((resolve, reject) => {
  child.on('error', reject)
  child.on('exit', resolve)
})
clearTimeout(timeout)
await writeFile(path.join(root, 'native.log'), log)
const report = JSON.parse(await readFile(path.join(root, 'native-performance-report.json'), 'utf8'))
report.environment = {
  platform: process.platform,
  release: os.release(),
  architecture: os.arch(),
  cpu: os.cpus()[0]?.model,
  totalMemoryBytes: os.totalmem(),
  binary,
  git: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: repo, encoding: 'utf8' }).trim(),
  profile: process.env.TOG_BENCH_PROFILE || 'release; embedded production frontend',
}
const percentile = (values, p) =>
  [...values].sort((a, b) => a - b)[Math.ceil(values.length * p) - 1]
report.summary = []
for (const workload of [...new Set(report.samples.map((s) => s.workload))]) {
  for (const encoding of ['dataurl', 'png', 'rgba', 'svg']) {
    const samples = report.samples.filter(
      (s) => s.workload === workload && s.encoding === encoding && s.round > 0,
    )
    if (!samples.length) continue
    report.summary.push({
      workload,
      encoding,
      pages: samples[0].pages,
      n: samples.length,
      frameP50: percentile(
        samples.map((s) => s.frameMs),
        0.5,
      ),
      frameP95: percentile(
        samples.map((s) => s.frameMs),
        0.95,
      ),
      requestP50: percentile(
        samples.map((s) => s.requestMs),
        0.5,
      ),
      backendP50: percentile(
        samples.map((s) => s.backendMs),
        0.5,
      ),
      bytes: percentile(
        samples.map((s) => s.bytes),
        0.5,
      ),
    })
  }
}
await writeFile(
  path.join(root, 'native-performance-report.json'),
  JSON.stringify(report, null, 2) + '\n',
)
console.log(
  JSON.stringify(
    {
      ok: report.ok,
      code,
      root,
      summary: report.summary,
      typing: report.typing,
      error: report.error,
    },
    null,
    2,
  ),
)
if (code !== 0 || !report.ok) process.exitCode = 1
