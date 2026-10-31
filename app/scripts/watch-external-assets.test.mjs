import { test } from 'node:test'
import assert from 'node:assert/strict'
import { mkdtemp, mkdir, writeFile, rm, realpath } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { setTimeout } from 'node:timers/promises'
import { createServer } from 'vite'
import { watchExternalAssets } from './watch-external-assets.ts'

async function until(check, label) {
  const deadline = Date.now() + 8000
  while (Date.now() < deadline) {
    if (await check()) return
    await setTimeout(25)
  }
  assert.fail(label)
}

test(
  'a running Vite server updates external SVG globs after adding and removing icons',
  { timeout: 20000 },
  async () => {
    const directory = await realpath(await mkdtemp(join(tmpdir(), 'typstographe-icon-watch-')))
    const root = join(directory, 'app'),
      icons = join(directory, 'icons')
    let server
    try {
      await mkdir(root)
      await mkdir(icons)
      await writeFile(join(icons, 'existing.svg'), '<svg><path d="M0 0"/></svg>')
      await writeFile(
        join(root, 'icons.js'),
        'export const icons = import.meta.glob("../icons/*.svg", { eager: true, query: "?raw", import: "default" })',
      )
      server = await createServer({
        configFile: false,
        root,
        logLevel: 'silent',
        plugins: [watchExternalAssets(icons)],
        server: { middlewareMode: true, ws: false, fs: { allow: [directory] } },
        optimizeDeps: { noDiscovery: true, include: [] },
      })
      const transformed = async () => (await server.transformRequest('/icons.js')).code
      const initial = await transformed()
      assert.ok(initial.includes('existing.svg'))
      assert.ok(!initial.includes('check.svg'))
      // Wait for the actual directory watch, not an arbitrary delay before writing.
      await until(
        () =>
          Object.values(server.watcher.getWatched()).some((files) =>
            files.includes('existing.svg'),
          ),
        'icon directory was not watched',
      )
      await writeFile(join(icons, 'check.svg'), '<svg><path d="M20 6 9 17l-5-5"/></svg>')
      await writeFile(
        join(icons, 'circle-minus.svg'),
        '<svg><circle r="10"/><path d="M8 12h8"/></svg>',
      )
      await until(async () => {
        const code = await transformed()
        return code.includes('check.svg') && code.includes('circle-minus.svg')
      }, 'added icons never entered the live catalogue')
      await rm(join(icons, 'check.svg'))
      await until(
        async () => !(await transformed()).includes('check.svg'),
        'removed icon remained in the live catalogue',
      )
      assert.ok((await transformed()).includes('circle-minus.svg'))
    } finally {
      await server?.close()
      await rm(directory, { recursive: true, force: true })
    }
  },
)
