let messages: Record<string, string> = {}
export function initializeUI(values: Record<string, string>) {
  messages = values || {}
}
export function tr(source: string, values?: Record<string, string | number>): string {
  const text = messages[source] || source
  return values
    ? text.replace(/\{(\w+)\}/g, (token, name) =>
        values[name] === undefined ? token : String(values[name]),
      )
    : text
}
