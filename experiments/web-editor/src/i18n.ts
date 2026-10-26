let messages: Record<string, string> = {}
export function initializeUI(values: Record<string, string>) {
  messages = values || {}
}
export function tr(source: string): string {
  return messages[source] || source
}
