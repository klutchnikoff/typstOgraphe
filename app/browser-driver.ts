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
      const call = (input: Record<string, unknown>): Promise<unknown> => {
        const id = ++next
        return new Promise((resolve, reject) => {
          requests.set(id, { resolve, reject })
          child.stdin.write(JSON.stringify({ ...input, id }) + '\n')
        })
      }
      const connections = new Map<string, { token: string; generation?: string }>()
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
        const query = new URL(request.url || '/', 'http://localhost').searchParams
        const peer = query.get('peer') || '',
          token = query.get('token') || ''
        if (peer && token) connections.set(peer, { token })
        request.on('close', () => {
          streams.delete(response)
          const lease = connections.get(peer)
          if (lease?.token !== token) return
          connections.delete(peer)
          if (lease.generation)
            void call({ command: 'leave', peer, args: { generation: lease.generation } })
              .then((status) => broadcast('web-editor:synchronization', status))
              .catch(() => {})
        })
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
          const result = await call(input)
          if (input.command === 'comparison_apply') {
            const outcome = result as {
              id: string
              publication?: {
                resources: { path: string; data: string }[]
                guard: { document: string; revision: number }
                update: number[]
              }
            }
            const publication = outcome.publication
            if (publication) {
              for (const resource of publication.resources)
                broadcast('web-editor:asset', {
                  ...resource,
                  generation: publication.guard.document,
                })
              broadcast('web-editor:change', {
                from: 'comparison',
                acceptedBy: input.peer,
                generation: publication.guard.document,
                revision: publication.guard.revision,
                update: publication.update,
              })
            }
            broadcast('web-editor:saved', (result as { saved: unknown }).saved)
            broadcast('comparison:changed', { id: outcome.id })
            broadcast('agent:changed', {})
          }
          if (input.command === 'versions_change')
            broadcast('web-editor:versions', {
              document: input.args.guard.document,
              ...(result as object),
            })
          if (input.command === 'join') {
            const bootstrap = result as { generation: string; synchronization: unknown }
            const lease = connections.get(input.peer)
            if (lease && lease.token === input.token) lease.generation = bootstrap.generation
            broadcast('web-editor:synchronization', bootstrap.synchronization)
          }
          if (input.command === 'diagnostic_request_close')
            broadcast('app:request-close', { window: input.peer })
          if (input.command === 'connection' || input.command === 'set_synchronization')
            broadcast('web-editor:synchronization', result)
          if (
            input.command === 'commit_structured_commands' &&
            (result as { broadcast: boolean }).broadcast
          ) {
            const publication = result as {
              resources: { path: string; data: string }[]
              changed: boolean
              guard: { revision: number }
              update: number[]
            }
            for (const resource of publication.resources)
              broadcast('web-editor:asset', {
                path: resource.path,
                data: resource.data,
                generation: input.args.request.generation,
              })
            if (publication.changed)
              broadcast('web-editor:change', {
                from: input.peer,
                generation: input.args.request.generation,
                revision: publication.guard.revision,
                update: publication.update,
              })
          }
          if (input.command === 'relay' || input.command === 'editing_history') {
            const receipt = result as { revision: number; update: number[]; broadcast: boolean }
            if (receipt.update.length && receipt.broadcast)
              broadcast('web-editor:change', {
                from: input.peer,
                generation: input.args.generation,
                revision: receipt.revision,
                update: receipt.update,
              })
          }
          if (
            ['new_document', 'close_document', 'work_restore', 'work_import', 'open_file'].includes(
              input.command,
            )
          )
            broadcast('app:document', null)
          if (input.command === 'transition_begin') broadcast('recovery:prepare-transition', result)
          if (input.command === 'transition_end')
            broadcast('recovery:end-transition', input.args.id)
          if (input.command === 'awareness' && result === true)
            broadcast('web-editor:awareness', { from: input.peer, ...input.args })
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
