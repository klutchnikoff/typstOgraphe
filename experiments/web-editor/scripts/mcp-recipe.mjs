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
    const contracts = await call('capabilities.list')
    if (!contracts.commands.some((c) => c.id === 'insert.table'))
      throw new Error('Catalogue incomplet')
    const snapshot = await call('document.structure')
    const proposal = await call('proposals.submit', {
      revision: snapshot.revision,
      request_id: 'native-mcp',
      summary: 'Lot de recette MCP',
      steps: [
        { id: 'text.bold', params: { value: 'on' } },
        { id: 'text.insert', params: { text: ' NATIVE-MCP' } },
        { id: 'text.bold', params: { value: 'off' } },
      ],
    })
    if (proposal.status !== 'pending') throw new Error('Publication sans validation humaine')
    return { tools: list.tools.map((t) => t.name), proposal: proposal.id, status: proposal.status }
  } catch (error) {
    throw new Error(String(error) + ' ' + log)
  } finally {
    clearTimeout(timeout)
    lines.close()
    child.kill()
  }
}
