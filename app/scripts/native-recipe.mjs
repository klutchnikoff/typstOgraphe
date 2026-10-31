// Recette macOS/Windows/Linux avec session propre ; nécessite un affichage natif.
import { spawn, execFileSync } from 'node:child_process'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { fileURLToPath } from 'node:url'
import path from 'node:path'
import { proposeThroughMcp } from './mcp-recipe.mjs'

const repo = fileURLToPath(new URL('../../', import.meta.url))
const root = path.resolve(
  process.argv[2] || path.join(repo, 'output/web-editor', `native-${Date.now()}`),
)
const binary =
  process.env.TOG_SPIKE_BINARY ||
  path.join(
    repo,
    'target/recipes/debug',
    `typstographe${process.platform === 'win32' ? '.exe' : ''}`,
  )
await mkdir(root, { recursive: true })
const results = []
for (const mode of ['full', 'restore']) {
  const started = performance.now()
  const child = spawn(
    binary,
    [mode === 'full' ? '--diagnostic' : '--diagnostic-restore', '--session-dir', root],
    { stdio: ['ignore', 'pipe', 'pipe'] },
  )
  let log = '',
    peakKiB = 0
  child.stdout.on('data', (data) => {
    log += data
  })
  child.stderr.on('data', (data) => {
    log += data
    process.stderr.write(data)
  })
  let mcpProbe,
    mcpDone = false
  const invitation = path.join(root, 'preferences', 'invitation.json')
  const mcpTimer = setInterval(async () => {
    if (mode !== 'full' || mcpProbe || mcpDone) return
    try {
      await readFile(invitation)
    } catch {
      return
    }
    mcpDone = true
    mcpProbe = proposeThroughMcp(binary, invitation)
      .then(async (report) => {
        await writeFile(path.join(root, 'native-mcp-report.json'), JSON.stringify(report, null, 2))
      })
      .catch((error) => {
        log += '\nMCP: ' + String(error)
        child.kill()
      })
  }, 100)
  const timer = setInterval(() => {
    if (process.platform === 'win32' || !child.pid) return
    try {
      peakKiB = Math.max(
        peakKiB,
        Number(
          execFileSync('ps', ['-o', 'rss=', '-p', String(child.pid)], { encoding: 'utf8' }).trim(),
        ) || 0,
      )
    } catch {
      /* Processus déjà terminé. */
    }
  }, 500)
  const sampleTimer = setTimeout(() => {
    if (process.platform === 'darwin' && child.pid) {
      try {
        execFileSync(
          'sample',
          [String(child.pid), '1', '-file', path.join(root, 'native-sample.txt')],
          { stdio: 'ignore' },
        )
      } catch {}
    }
  }, 30000)
  const timeout = setTimeout(() => child.kill(), 120_000)
  const code = await new Promise((resolve, reject) => {
    child.once('exit', resolve)
    child.once('error', reject)
  }).finally(() => {
    clearInterval(timer)
    clearInterval(mcpTimer)
    clearTimeout(timeout)
    clearTimeout(sampleTimer)
  })
  await writeFile(path.join(root, `native-${mode}.log`), log)
  if (code !== 0) throw new Error(`Recette ${mode} interrompue (${code}) : ${log}`)
  const report = JSON.parse(await readFile(path.join(root, `native-${mode}-report.json`), 'utf8'))
  if (!report.success) throw new Error(JSON.stringify(report))
  if (mode === 'full' && !report.steps.some((step) => step.includes('MCP STDIO')))
    throw new Error('Binaire de recette périmé : parcours MCP absent.')
  results.push({
    mode,
    elapsedMs: performance.now() - started,
    rustProcessPeakResidentMiB: peakKiB ? peakKiB / 1024 : null,
    ...report,
  })
}
const output = {
  session: root,
  platform: process.platform,
  memoryScope: 'RSS du seul processus Rust, hors processus WebKit ; échantillonnage 500 ms.',
  results,
}
await writeFile(path.join(root, 'native-metrics.json'), JSON.stringify(output, null, 2))
console.log(JSON.stringify(output, null, 2))
