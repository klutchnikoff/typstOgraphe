/** Read-only queries use the same schema and implementation as CLI/MCP. */
import type { Provider, Transport } from './provider'
import { settingsFor } from './state'
export function bibliographyQuery<T>(
  provider: Provider,
  transport: Transport,
  operation: string,
  params: Record<string, unknown> = {},
  document?: unknown,
) {
  return transport.invoke<T>('bibliography_query', {
    metadata: settingsFor(provider).metadata,
    operation,
    params,
    document,
  })
}
