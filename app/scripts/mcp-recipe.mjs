// Client protocolaire de recette ; ne peut ni accepter ni écrire un fichier.
import { spawn } from 'node:child_process'
import { createInterface } from 'node:readline'
export async function proposeThroughMcp(binary, invitation) {
  const child = spawn(binary, ['--mcp', '--invitation', invitation], {
    stdio: ['pipe', 'pipe', 'pipe'],
  })
  const pending = new Map()
  let id = 0,
    log = ''
  child.stderr.on('data', (data) => {
    log += data
  })
  const lines = createInterface({ input: child.stdout })
  lines.on('line', (line) => {
    const value = JSON.parse(line)
    if (value.id !== undefined) {
      const task = pending.get(value.id)
      pending.delete(value.id)
      if (value.error) task?.reject(new Error(JSON.stringify(value.error)))
      else task?.resolve(value.result)
    }
  })
  const request = (method, params = {}) =>
    new Promise((resolve, reject) => {
      const next = ++id
      pending.set(next, { resolve, reject })
      child.stdin.write(JSON.stringify({ jsonrpc: '2.0', id: next, method, params }) + '\n')
    })
  const call = async (name, args = {}) => {
    const result = await request('tools/call', { name, arguments: args })
    if (result.isError) throw new Error(name + ': ' + JSON.stringify(result))
    return result.structuredContent || JSON.parse(result.content[0].text)
  }
  const timeout = setTimeout(() => child.kill(), 30000)
  try {
    await request('initialize', {
      protocolVersion: '2025-03-26',
      capabilities: {},
      clientInfo: { name: 'migration-recipe', version: '1' },
    })
    child.stdin.write(
      JSON.stringify({ jsonrpc: '2.0', method: 'notifications/initialized' }) + '\n',
    )
    const list = await request('tools/list')
    if (list.tools.some((t) => /accept|save|export/.test(t.name)))
      throw new Error('Permissions excessives')
    const contracts = await call('commands.list')
    if (!contracts.outcome.entries.some((c) => c.id === 'insert.table'))
      throw new Error('Catalogue incomplet')
    const inspected = await call('document.inspect')
    const snapshot = await call('document.structure', { document: inspected.document })
    const zone = snapshot.outcome.zones.filter((z) => z.zone.kind === 'body').at(-1)
    if (!zone) throw new Error('Zone du corps introuvable')
    const proposal = await call('proposals.submit', {
      document: snapshot.document,
      request_id: 'native-mcp',
      summary: 'Lot de recette MCP',
      steps: [
        {
          id: 'text.insert',
          params: {
            target: { zone: zone.zone, from: zone.length, to: zone.length },
            text: ' NATIVE-MCP',
            marks: [{ type: 'bold' }],
          },
        },
      ],
    })
    if (proposal.outcome.status !== 'pending')
      throw new Error('Publication sans validation humaine')
    return {
      tools: list.tools.map((t) => t.name),
      proposal: proposal.outcome.id,
      status: proposal.outcome.status,
    }
  } catch (error) {
    throw new Error(String(error) + ' ' + log)
  } finally {
    clearTimeout(timeout)
    lines.close()
    child.kill()
  }
}
