// Shared recipe client: real STDIO JSON-RPC, no documentary implementation.
import { spawn } from 'node:child_process'
import { createInterface } from 'node:readline'
export async function withMcp(binary, args, run, cwd) {
  const child = spawn(binary, args, { cwd, stdio: ['pipe', 'pipe', 'pipe'] })
  const pending = new Map()
  let next = 0,
    log = ''
  child.stderr.on('data', (data) => {
    log += data
  })
  const fail = (error) => {
    for (const task of pending.values()) task.reject(error)
    pending.clear()
  }
  const exited = new Promise((resolve) =>
    child.once('exit', (code) => {
      fail(new Error(`MCP exited (${code}): ${log}`))
      resolve(code)
    }),
  )
  child.on('error', fail)
  child.stdin.on('error', fail)
  const lines = createInterface({ input: child.stdout })
  lines.on('line', (line) => {
    try {
      const result = JSON.parse(line),
        task = pending.get(result.id)
      if (!task) return
      pending.delete(result.id)
      if (result.error) task.reject(new Error(JSON.stringify(result.error)))
      else task.resolve(result.result)
    } catch (error) {
      fail(error)
    }
  })
  const request = (method, params = {}) =>
    new Promise((resolve, reject) => {
      const id = ++next
      const timeout = setTimeout(() => {
        pending.delete(id)
        reject(new Error(`MCP timeout: ${method}`))
      }, 30000)
      pending.set(id, {
        resolve: (value) => {
          clearTimeout(timeout)
          resolve(value)
        },
        reject: (error) => {
          clearTimeout(timeout)
          reject(error)
        },
      })
      child.stdin.write(JSON.stringify({ jsonrpc: '2.0', id, method, params }) + '\n')
    })
  const tool = async (name, args = {}, success = true) => {
    const result = await request('tools/call', { name, arguments: args })
    if (!!result.isError === success) throw new Error(`${name}: ${JSON.stringify(result)}`)
    return result.structuredContent || JSON.parse(result.content[0].text)
  }
  try {
    await request('initialize', {
      protocolVersion: '2025-03-26',
      capabilities: {},
      clientInfo: { name: 'typstographe-recipe', version: '1' },
    })
    child.stdin.write(
      JSON.stringify({ jsonrpc: '2.0', method: 'notifications/initialized' }) + '\n',
    )
    return await run({ request, call: tool })
  } finally {
    child.stdin.end()
    let timeout
    await Promise.race([
      exited,
      new Promise((resolve) => {
        timeout = setTimeout(() => {
          child.kill()
          resolve()
        }, 2000)
      }),
    ])
    clearTimeout(timeout)
    lines.close()
  }
}
