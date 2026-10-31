// Foundation closure recipe: actual WebView/IPC, CLI subprocesses and MCP STDIO on the same files.
import assert from 'node:assert/strict'
import { spawn, execFileSync } from 'node:child_process'
import { mkdir, readFile, writeFile, copyFile, readdir, access } from 'node:fs/promises'
import { fileURLToPath } from 'node:url'
import path from 'node:path'
import { attachedRecipe, autonomousRecipe } from './closure-mcp.mjs'
import {
  cliClient,
  bodyTarget,
  lastBody,
  catalogueShape,
  compareExports,
  comparePages,
  hash,
} from './closure-tools.mjs'
const repo = fileURLToPath(new URL('../../', import.meta.url))
const root = path.resolve(
  process.argv[2] || path.join(repo, 'output/foundation-closure', `run-${Date.now()}`),
)
await mkdir(root, { recursive: true })
assert.equal((await readdir(root)).length, 0, 'Use a new, empty recipe directory')
const extension = process.platform === 'win32' ? '.exe' : ''
const binary =
  process.env.TOG_SPIKE_BINARY ||
  path.join(repo, 'target/recipes/debug', 'typstographe' + extension)
const cliBinary =
  process.env.TOG_CLI_BINARY ||
  path.join(repo, 'target/recipes/debug', 'typstographe-cli' + extension)
const cli = cliClient(cliBinary, repo)
const catalogue = cli(['commands', 'export'])
const report = {
  success: false,
  git: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: repo, encoding: 'utf8' }).trim(),
  platform: process.platform,
  recipe: 'foundation-closure-v2',
  cases: [],
}
const inspect = (project) => cli(['inspect', project]).document.state
const query = (id, project, params = {}) =>
  cli(['query', id, project, '--params', JSON.stringify(params)])
async function script(directory, file, steps, success = true) {
  const job = path.join(directory, `job-${Date.now()}.json`)
  await writeFile(
    job,
    JSON.stringify(
      { version: 1, document: { kind: 'open', path: file }, steps, save: { kind: 'current' } },
      null,
      2,
    ),
  )
  return cli(['script', job], success)
}
async function native(directory, input, verify = false) {
  await mkdir(directory, { recursive: true })
  const project = path.join(directory, 'project.typsto')
  if (input !== project) await copyFile(input, project)
  const mode = verify ? 'closure-verify' : 'closure'
  console.log(`Native ${mode}: ${directory}`)
  const child = spawn(binary, [`--diagnostic-${mode}`, '--session-dir', directory], {
    stdio: ['ignore', 'pipe', 'pipe'],
  })
  let log = '',
    attached,
    attachedError
  for (const stream of [child.stdout, child.stderr])
    stream.on('data', (data) => {
      log += data
    })
  const timer = setInterval(() => {
    if (verify || attached) return
    const invitation = path.join(directory, 'preferences/invitation.json')
    attached = access(invitation)
      .then(() => attachedRecipe(binary, invitation, cli, project, catalogue))
      .catch((error) => {
        if (error.code === 'ENOENT') {
          attached = undefined
          return
        }
        attachedError = error
        child.kill()
      })
  }, 100)
  const timeout = setTimeout(() => child.kill(), 120000)
  const code = await new Promise((resolve, reject) => {
    child.once('exit', resolve)
    child.once('error', reject)
  }).finally(() => {
    clearInterval(timer)
    clearTimeout(timeout)
  })
  await writeFile(path.join(directory, 'native.log'), log)
  const attachedResult = attached ? await attached : undefined
  if (attachedError) throw attachedError
  const value = JSON.parse(
    await readFile(path.join(directory, `native-${mode}-report.json`), 'utf8'),
  )
  assert.equal(code, 0, JSON.stringify({ error: value.error, steps: value.steps }))
  assert.equal(value.success, true, JSON.stringify({ error: value.error, steps: value.steps }))
  assert.deepEqual(
    value.final,
    inspect(project),
    'Native projection and persisted CLI state differ',
  )
  return { ...value, attached: attachedResult, project }
}
try {
  for (const kind of ['courrier', 'article']) {
    const directory = path.join(root, kind),
      first = path.join(directory, 'native')
    await mkdir(first, { recursive: true })
    const project = path.join(first, 'project.typsto')
    if (kind === 'article')
      await copyFile(path.join(repo, 'examples/article-working.typsto'), project)
    else {
      const example = JSON.parse(
        await readFile(path.join(repo, 'examples/cli/courrier.json'), 'utf8'),
      )
      example.save.path = project
      const job = path.join(directory, 'create.json')
      await writeFile(job, JSON.stringify(example, null, 2))
      cli(['script', job])
    }
    const original = inspect(project)
    const image = await readFile(path.join(repo, 'fixtures/parc.svg'))
    const signature = await readFile(path.join(repo, 'fixtures/signature.svg'))
    const resource = (name, bytes) => ({
      id: 'resource.add',
      params: { resource: { path: `images/${name}.svg`, data: bytes.toString('base64') } },
    })
    const imageNode = query('capabilities.describe', project, { id: 'insert.image' }).examples[0]
      .params.node
    imageNode.attrs.src = 'images/closure-image.svg'
    imageNode.attrs.alt = 'Allée arborée du parc, image de recette A.2.'
    imageNode.content = [{ type: 'text', text: 'Illustration conservée entre les adaptateurs.' }]
    const steps = [
      resource('closure-image', image),
      resource('unused', signature),
      {
        id: 'block.insert',
        params: {
          parent: { root: { kind: 'body' } },
          index: original.document.content.length - 1,
          node: imageNode,
        },
      },
    ]
    if (kind === 'courrier')
      steps.push(resource('closure-signature', signature), {
        id: 'document.set_signature',
        params: { path: 'images/closure-signature.svg' },
      })
    await script(directory, project, steps)
    const admitted = query('resources.list', project)
      .resources.map((r) => r.path)
      .sort()
    assert.ok(!admitted.includes('images/unused.svg'))
    const tauri = await native(first, project)
    assert.deepEqual(tauri.resources, admitted)
    const cliFromTauri = path.join(directory, 'cli-native-export')
    cli(['export', project, '--format', 'both', '--output', cliFromTauri])
    const nativeExportFiles = await compareExports(tauri.exported, cliFromTauri)
    comparePages(tauri.pageSizes, query('render.compile', project).pages)
    const zone = lastBody(query('document.structure', project))
    const cliSteps = [{ id: 'text.insert', params: { target: bodyTarget(zone), text: '[CLI]' } }]
    if (kind === 'courrier')
      cliSteps.push({
        id: 'metadata.set',
        params: { target: { kind: 'metadata', field: 'letter.reference' }, value: 'A2-CLI' },
      })
    else
      cliSteps.push({
        id: 'text.insert',
        params: {
          target: {
            zone: {
              kind: 'record',
              collection: 'localizations',
              id: 'fr',
              field: 'abstract',
              path: [0],
            },
            from: 0,
            to: 0,
          },
          text: '[CLI-RÉSUMÉ] ',
          marks: [{ type: 'italic' }],
        },
      })
    if (kind === 'article') {
      const tableIndex = inspect(project).document.content.findIndex(
        (n) => n.type === 'tableFigure',
      )
      assert.ok(tableIndex >= 0)
      cliSteps.push({
        id: 'table.row.insert',
        params: {
          target: { root: { kind: 'body' }, path: [tableIndex] },
          index: 1,
        },
      })
    }
    const beforeFailure = await readFile(project)
    assert.equal(
      (
        await script(
          directory,
          project,
          [
            ...cliSteps,
            {
              id: 'text.delete',
              params: { target: { zone: { kind: 'body', path: [999] }, from: 0, to: 1 } },
            },
          ],
          false,
        )
      ).code,
      'batch_failed',
    )
    assert.ok((await readFile(project)).equals(beforeFailure))
    const changed = await script(directory, project, cliSteps)
    assert.equal(changed.document.context.undo_steps, 1)
    const afterCli = inspect(project),
      saved = await readFile(project)
    cli([
      'run',
      'text.insert',
      project,
      '--params',
      JSON.stringify({
        target: bodyTarget(lastBody(query('document.structure', project))),
        text: '[CLI-PREVIEW]',
      }),
    ])
    assert.ok((await readFile(project)).equals(saved), 'CLI preview wrote the archive')
    const mcp = await autonomousRecipe(cliBinary, directory, project, afterCli, catalogue, cli)
    assert.deepEqual(inspect(project), mcp.state)
    const finalCli = path.join(directory, 'cli-final-export')
    cli(['export', project, '--format', 'both', '--output', finalCli])
    const mcpExportFiles = await compareExports(mcp.directory, finalCli)
    const png = path.join(directory, 'cli-page.png')
    cli(['render', project, '--page', '1', '--png', png])
    assert.ok(
      (await readFile(png)).subarray(0, 8).equals(Buffer.from([137, 80, 78, 71, 13, 10, 26, 10])),
    )
    const reopened = await native(path.join(directory, 'reopened'), project, true)
    assert.deepEqual(
      reopened.initialAnchor,
      tauri.initialAnchor,
      'Untouched first block identity was rebuilt',
    )
    assert.deepEqual(reopened.initial, mcp.state)
    assert.deepEqual(reopened.final, mcp.state)
    comparePages(reopened.pageSizes, mcp.pages)
    const finalExportFiles = await compareExports(reopened.exported, finalCli)
    cli(['check', reopened.project, '--for-export'])
    report.cases.push({
      kind,
      success: true,
      tauriSteps: tauri.steps,
      reopenSteps: reopened.steps,
      nativeExportFiles,
      mcpExportFiles,
      finalExportFiles,
      pages: mcp.pages.length,
      resources: admitted,
      archiveSha256: hash(await readFile(reopened.project)),
      cli: { atomicRefusal: true, explicitPreview: true, singleBatchHistory: true, png: true },
      mcp: mcp.checks,
      attached: tauri.attached,
      catalogueEntries: catalogueShape(catalogue).length,
    })
  }
  report.success = true
} catch (error) {
  report.error = error.stack || String(error)
  process.exitCode = 1
} finally {
  await writeFile(path.join(root, 'report.json'), JSON.stringify(report, null, 2) + '\n')
  console.log(JSON.stringify(report, null, 2))
}
