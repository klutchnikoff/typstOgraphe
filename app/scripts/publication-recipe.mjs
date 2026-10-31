// A.3 publication: real native WebView/IPC, external mutation by another process and CLI export comparison.
import assert from 'node:assert/strict'
import { spawn, execFileSync } from 'node:child_process'
import { mkdir, readdir, readFile, writeFile, copyFile } from 'node:fs/promises'
import { fileURLToPath } from 'node:url'
import path from 'node:path'
import { cliClient, compareExports, comparePages, hash } from './closure-tools.mjs'

const repo = fileURLToPath(new URL('../../', import.meta.url))
const root = path.resolve(
  process.argv[2] || path.join(repo, 'output/a3-publication', `run-${Date.now()}`),
)
await mkdir(root, { recursive: true })
assert.equal((await readdir(root)).length, 0, 'Use a new empty recipe directory')
const ext = process.platform === 'win32' ? '.exe' : ''
const binary = process.env.TOG_SPIKE_BINARY || path.join(repo, 'target/debug', 'typstographe' + ext)
const cli = cliClient(
  process.env.TOG_CLI_BINARY || path.join(repo, 'target/debug', 'typstographe-cli' + ext),
  repo,
)
const external = Buffer.from('External process content — never overwrite é😀\n')
const report = {
  success: false,
  platform: process.platform,
  git: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: repo, encoding: 'utf8' }).trim(),
  cases: [],
}
async function native(directory, verify) {
  const mode = verify ? 'publication-verify' : 'publication'
  const child = spawn(binary, [`--diagnostic-${mode}`, '--session-dir', directory], {
    stdio: ['ignore', 'pipe', 'pipe'],
  })
  let log = '',
    ready,
    mutation,
    failure
  const collect = (text) => {
    log += text
    if (!verify && !mutation && log.includes('"publicationExternalReady":true')) {
      mutation = (async () => {
        await writeFile(path.join(directory, 'project.typsto'), external)
        await writeFile(path.join(directory, 'external-ready'), 'ready')
      })().catch((error) => {
        failure = error
        child.kill('SIGKILL')
      })
    }
    const line = log
      .split('\n')
      .slice(0, -1)
      .find(
        (line) => line.startsWith('Diagnostic main: ') && line.includes('"publicationReady":true'),
      )
    if (!verify && !ready && line) {
      ready = JSON.parse(line.slice('Diagnostic main: '.length))
      child.kill('SIGKILL')
    }
  }
  for (const stream of [child.stdout, child.stderr]) stream.setEncoding('utf8').on('data', collect)
  const timeout = setTimeout(() => child.kill('SIGKILL'), 90000)
  const code = await new Promise((resolve, reject) => {
    child.once('close', resolve)
    child.once('error', reject)
  }).finally(() => clearTimeout(timeout))
  await mutation
  await writeFile(path.join(directory, `${mode}.log`), log)
  if (failure) throw failure
  if (verify) {
    const result = JSON.parse(
      await readFile(path.join(directory, `native-${mode}-report.json`), 'utf8'),
    )
    assert.equal(code, 0, JSON.stringify(result))
    assert.equal(result.success, true, JSON.stringify(result))
    return result
  }
  if (!ready?.success) {
    const failed = await readFile(path.join(directory, `native-${mode}-report.json`), 'utf8').catch(
      () => log,
    )
    throw new Error(`Native ${mode} failed: ${failed}`)
  }
  return ready
}
try {
  for (const kind of ['courrier', 'article']) {
    const directory = path.join(root, kind)
    await mkdir(directory)
    const project = path.join(directory, 'project.typsto')
    if (kind === 'article')
      await copyFile(path.join(repo, 'examples/article-working.typsto'), project)
    else {
      const job = JSON.parse(await readFile(path.join(repo, 'examples/cli/courrier.json'), 'utf8'))
      job.save.path = project
      const jobPath = path.join(directory, 'create.json')
      await writeFile(jobPath, JSON.stringify(job))
      cli(['script', jobPath])
    }
    const state = cli(['inspect', project]).document.state
    const image = JSON.parse(
      JSON.stringify(
        cli(['query', 'capabilities.describe', project, '--params', '{"id":"insert.image"}'])
          .examples[0].params.node,
      ),
    )
    image.attrs.src = 'images/publication.svg'
    image.attrs.alt = 'Image preserved during publication failures'
    const resource = await readFile(path.join(repo, 'fixtures/parc.svg'))
    const job = {
      version: 1,
      document: { kind: 'open', path: project },
      steps: [
        {
          id: 'resource.add',
          params: { resource: { path: image.attrs.src, data: resource.toString('base64') } },
        },
        {
          id: 'block.insert',
          params: {
            parent: { root: { kind: 'body' } },
            index: state.document.content.length - 1,
            node: image,
          },
        },
      ],
      save: { kind: 'current' },
    }
    const jobPath = path.join(directory, 'resources.json')
    await writeFile(jobPath, JSON.stringify(job))
    cli(['script', jobPath])
    await writeFile(path.join(directory, 'blocked-parent'), 'Keep this file')
    await writeFile(path.join(directory, 'corrupt.typsto'), 'Not a coherent archive')
    const interrupted = await native(directory, false)
    assert.ok((await readFile(project)).equals(external), 'External file was overwritten')
    assert.equal(await readFile(path.join(directory, 'blocked-parent'), 'utf8'), 'Keep this file')
    const copy = path.join(directory, 'independent.typsto')
    const bytes = await readFile(copy)
    assert.deepEqual(cli(['inspect', copy]).document.state, interrupted.final)
    const reopened = await native(directory, true)
    assert.deepEqual(reopened.final, interrupted.final)
    assert.ok((await readFile(copy)).equals(bytes), 'Reopen changed the archive')
    const cliExport = path.join(directory, 'cli-export')
    cli(['export', copy, '--format', 'both', '--output', cliExport])
    const entries = await compareExports(interrupted.exported, cliExport)
    await compareExports(reopened.exported, cliExport)
    const compiled = cli(['query', 'render.compile', copy])
    comparePages(interrupted.pageSizes, compiled.pages)
    comparePages(reopened.pageSizes, compiled.pages)
    assert.ok((await readFile(path.join(cliExport, 'images/publication.svg'))).equals(resource))
    report.cases.push({
      kind,
      checks: [
        ...interrupted.checks,
        ...reopened.checks,
        'native exports match reopened archive and CLI export, including dependencies and resources',
      ],
      pageSizes: compiled.pages,
      archiveSha256: hash(bytes),
      exportedEntries: entries,
    })
  }
  report.success = true
} catch (error) {
  report.error = String(error)
  process.exitCode = 1
} finally {
  await writeFile(path.join(root, 'report.json'), JSON.stringify(report, null, 2) + '\n')
  console.log(JSON.stringify({ root, ...report }, null, 2))
}
