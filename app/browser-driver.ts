import { spawn } from 'node:child_process'
import { mkdtempSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { createInterface } from 'node:readline'
import type { Plugin } from 'vite'
import type { ServerResponse } from 'node:http'

/** Recette seulement : rendu Rust réel, relais SSE à la place des événements Tauri. */
export function browserDriver(root: string): Plugin {
  return {
    name: 'local-rust-diagnostic',
    configureServer(server) {
      const binary =
        process.env.TOG_SPIKE_BINARY ||
        resolve(root, '../target/debug/typstographe' + (process.platform === 'win32' ? '.exe' : ''))
      const session = process.env.TOG_SPIKE_SESSION || mkdtempSync(join(tmpdir(), 'tog-web-spike-'))
      const child = spawn(binary, ['--stdio', '--session-dir', session], {
        stdio: ['pipe', 'pipe', 'inherit'],
      })
      let next = 0
      const requests = new Map<
        number,
        { resolve: (value: unknown) => void; reject: (error: unknown) => void }
      >()
      const streams = new Set<ServerResponse>()
      const lines = createInterface({ input: child.stdout })
      lines.on('line', (line) => {
        const response = JSON.parse(line)
        const request = requests.get(response.id)
        if (!request) return
        requests.delete(response.id)
        if (response.error) request.reject(response.error)
        else request.resolve(response.result)
      })
      const fail = (error: unknown) => {
        for (const request of requests.values()) request.reject(error)
        requests.clear()
      }
      child.on('error', fail)
      child.on('exit', (code) => fail(`Le moteur de diagnostic est arrêté (${code}).`))
      const broadcast = (name: string, payload: unknown) => {
        for (const stream of streams) stream.write(`data: ${JSON.stringify({ name, payload })}\n\n`)
      }
      server.middlewares.use('/__spike/events', (request, response) => {
        response.writeHead(200, {
          'Content-Type': 'text/event-stream',
          'Cache-Control': 'no-cache',
          Connection: 'keep-alive',
        })
        response.write(': connected\n\n')
        streams.add(response)
        request.on('close', () => streams.delete(response))
      })
      server.middlewares.use('/__spike/invoke', async (request, response) => {
        if (request.method !== 'POST') {
          response.writeHead(405).end()
          return
        }
        try {
          const chunks: Buffer[] = []
          let size = 0
          for await (const chunk of request) {
            size += chunk.length
            if (size > 24 * 1024 * 1024) throw new Error('Requête trop grande')
            chunks.push(chunk)
          }
          const input = JSON.parse(Buffer.concat(chunks).toString('utf8'))
          const id = ++next
          const result = await new Promise((resolve, reject) => {
            requests.set(id, { resolve, reject })
            child.stdin.write(JSON.stringify({ ...input, id }) + '\n')
          })
          if (input.command === 'commit_structured_commands') {
            const publication = result as {
              changed: boolean
              guard: { revision: number }
              update: number[]
            }
            if (publication.changed)
              broadcast('web-editor:change', {
                from: input.peer,
                revision: publication.guard.revision,
                update: publication.update,
              })
          }
          if (input.command === 'relay')
            broadcast('web-editor:change', {
              from: input.peer,
              revision: result,
              update: input.args.update,
            })
          if (input.command === 'awareness')
            broadcast('web-editor:awareness', { from: input.peer, update: input.args.update })
          if (input.command === 'import_image') broadcast('web-editor:asset', result)
          if (input.command === 'preview_frame') {
            response.setHeader('Content-Type', 'application/octet-stream')
            response.end(Buffer.from((result as { binary: string }).binary, 'base64'))
          } else {
            response.setHeader('Content-Type', 'application/json')
            response.end(JSON.stringify({ result }))
          }
        } catch (error) {
          response.setHeader('Content-Type', 'application/json')
          response.end(JSON.stringify({ error: String(error) }))
        }
      })
      server.httpServer?.on('close', () => {
        for (const stream of streams) stream.end()
        child.kill()
        lines.close()
      })
    },
  }
}
