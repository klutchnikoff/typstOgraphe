const assets = import.meta.glob('../../assets/ui/icons/lucide/*.svg', {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>
export function availableIcon(name: string): string | undefined {
  if (name === 'footprints') return '<span class="icon-footnote" aria-hidden="true">1</span>'
  if (name === 'book-open')
    return '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linejoin="round" aria-hidden="true"><path d="M12 5C9 3 5 3 2 4v16c3-1 7-1 10 1 3-2 7-2 10-1V4c-3-1-7-1-10 1Zm0 0v16"/></svg>'
  if (name === 'plus')
    return '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" aria-hidden="true"><path d="M12 5v14M5 12h14"/></svg>'
  if (name === 'table-2')
    return '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" aria-hidden="true"><rect x="3" y="3" width="18" height="18" rx="2"/><path d="M3 9h18M3 15h18M10 3v18"/></svg>'
  const mapped: Record<string, string> = {
    image: 'image-plus',
    'message-square': 'pen-line',
    footprints: 'pilcrow',
    'table-2': 'list-collapse',
    settings: 'settings-2',
  }
  return Object.entries(assets).find(([path]) => path.endsWith(`/${mapped[name] || name}.svg`))?.[1]
}
export function icon(name: string) {
  return availableIcon(name) || availableIcon('pilcrow')!
}
