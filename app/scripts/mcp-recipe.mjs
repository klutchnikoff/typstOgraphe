// Client protocolaire de recette ; ne peut ni accepter ni écrire un fichier.
import { withMcp } from './mcp-client.mjs'
export async function proposeThroughMcp(binary, invitation) {
  return withMcp(binary, ['--mcp', '--invitation', invitation], async ({ request, call }) => {
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
  })
}
