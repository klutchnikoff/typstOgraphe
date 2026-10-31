import { versionContent } from './version-content'
import { proposalDetails } from './proposal-preview'
import { actionRow, navigationGroup, composing, editingControl } from './keyboard-navigation'
import { icon } from './icons'
import { tr } from './i18n'
import type { Catalogue } from './shell'
import type { ComparisonAPI, ComparisonState, ComparisonGroup } from './comparison-types'
import type { VersionContent } from './version-types'
/** The same immutable comparison and decision controls serve Versions and MCP. */
export function comparisonView(options: {
  state: ComparisonState
  api: ComparisonAPI
  catalogue: Catalogue
  error(reason: unknown): void
  picker?: HTMLElement
}) {
  const element = document.createElement('section')
  element.className = 'comparison'
  let state = options.state,
    alive = true,
    busy = false,
    active = 0,
    serial = 0
  let disposeListener: (() => void) | undefined
  let disposers: (() => void)[] = []
  let previewMode = false,
    previewLoading = false
  let previews: { before?: string[]; after?: string[] } = {}
  let navigate = (delta: number) => {
    void delta
  }
  element.addEventListener('keydown', (event) => {
    if (
      event.defaultPrevented ||
      composing(event) ||
      editingControl(event.target) ||
      event.ctrlKey ||
      event.metaKey ||
      event.altKey ||
      event.shiftKey ||
      !state.report.groups.length
    )
      return
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault()
      event.stopPropagation()
      navigate(event.key === 'ArrowDown' ? 1 : -1)
    }
  })
  const sourceBefore = state.report.source === 'before'
  const button = (label: string, run: () => void) => {
    const b = document.createElement('button')
    b.type = 'button'
    b.dataset.action = label
    b.textContent = tr(label)
    b.onclick = run
    return b
  }
  async function refresh() {
    const request = ++serial
    try {
      const result = await options.api.read(state.id)
      if (!alive || request !== serial) return
      const old = state.report,
        next = result.report
      const changed =
        old.guard.document !== next.guard.document ||
        old.guard.revision !== next.guard.revision ||
        old.stale !== next.stale ||
        old.closed !== next.closed ||
        old.groups.some((g, i) => g.decision !== next.groups[i]?.decision) ||
        old.groups.length !== next.groups.length
      state = { ...state, report: next }
      if (changed) render()
    } catch (e) {
      if (alive && request === serial) options.error(e)
    }
  }
  async function decide(groups: number[], action: 'apply' | 'refuse') {
    if (busy || !alive) return
    busy = true
    render()
    const request = ++serial
    try {
      const result = await options.api.decide(state.id, groups, action)
      if (!alive || request !== serial) return
      state = { ...state, report: result.report }
    } catch (e) {
      if (alive) {
        options.error(e)
        await refresh()
      }
    } finally {
      if (alive) {
        busy = false
        render()
        element.querySelector<HTMLButtonElement>('[data-next]')?.focus({ preventScroll: true })
      }
    }
  }
  function side(content: VersionContent, g: ComparisonGroup, name: 'before' | 'after') {
    const section = document.createElement('div')
    section.className = 'comparison-side'
    section.setAttribute('aria-label', tr(name === 'before' ? 'Avant' : 'Après'))
    const range = g[name]
    const nodes = content.state.document.content!.slice(range.start, range.end)
    if (!nodes.length) {
      section.textContent = tr('Aucun contenu')
      return section
    }
    const rich = versionContent(
      { ...content, state: { ...content.state, document: { type: 'doc', content: nodes } } },
      options.catalogue,
      { kind: name === 'before' ? 'delete' : 'insert', ranges: g[`${name}_highlights`] },
    )
    const info = rich.element.querySelector('details')!
    if (g.section !== 'document') info.remove()
    else info.open = true
    section.append(rich.element)
    disposers.push(rich.destroy)
    return section
  }
  async function togglePreviews() {
    previewMode = !previewMode
    if (!previewMode || previews.before || previewLoading) {
      render()
      return
    }
    previewLoading = true
    render()
    try {
      const [before, after] = await Promise.all([
        options.api.preview(state.id, 'before'),
        options.api.preview(state.id, 'after'),
      ])
      if (alive) previews = { before: before.pages, after: after.pages }
    } catch (reason) {
      if (alive) options.error(reason)
    } finally {
      if (alive) {
        previewLoading = false
        render()
      }
    }
  }
  function render() {
    const focused = element.contains(document.activeElement)
      ? (document.activeElement as HTMLElement)
      : null
    const focusedPicker = focused === options.picker?.querySelector('select')
    const action = focused?.dataset.action,
      group = focused?.closest<HTMLElement>('[data-group]')?.dataset.group
    const scroller = element.closest<HTMLElement>('.version-display'),
      scrollTop = scroller?.scrollTop
    disposers.forEach((fn) => fn())
    disposers = []
    const toolbar = document.createElement('div')
    toolbar.className = 'comparison-toolbar'
    const caption = document.createElement('span')
    caption.textContent = tr('États figés · avant à gauche, après à droite')
    const navigation = actionRow()
    const move = (delta: number) => {
      active = (active + delta + state.report.groups.length) % state.report.groups.length
      render()
      element.querySelector('[data-active="true"]')?.scrollIntoView({ block: 'nearest' })
      element
        .querySelector<HTMLButtonElement>(delta > 0 ? '[data-next]' : '[data-previous]')
        ?.focus({ preventScroll: true })
    }
    navigate = move
    const previous = button('Différence précédente', () => move(-1))
    previous.innerHTML = icon('chevron-up')
    previous.setAttribute('aria-label', tr('Différence précédente'))
    previous.dataset.previous = ''
    const next = button('Différence suivante', () => move(1))
    next.innerHTML = icon('chevron-down')
    next.setAttribute('aria-label', tr('Différence suivante'))
    next.dataset.next = ''
    const count = document.createElement('span')
    count.textContent = state.report.groups.length
      ? `${active + 1} / ${state.report.groups.length}`
      : '0 / 0'
    previous.disabled = next.disabled = !state.report.groups.length
    navigation.append(previous, count, next)
    const toggle = button(
      previewMode ? 'Voir les changements' : 'Aperçus des deux états',
      () => void togglePreviews(),
    )
    toggle.dataset.action = 'preview-toggle'
    toolbar.append(options.picker || caption, toggle, navigation)
    const notice = document.createElement('p')
    notice.className = 'comparison-notice'
    notice.hidden = !state.report.stale
    notice.setAttribute('role', 'alert')
    notice.textContent = tr(
      sourceBefore
        ? 'Le document a changé ailleurs. Fermez puis ouvrez une nouvelle comparaison avant de reprendre les modifications restantes.'
        : 'Le document a changé ailleurs. Demandez une nouvelle proposition à l’agent.',
    )
    const headings = document.createElement('div')
    headings.className = 'comparison-columns comparison-headings'
    for (const [index, label] of [
      sourceBefore ? 'Avant · Version conservée' : 'Avant · État de départ',
      sourceBefore ? 'Après · Document courant figé' : 'Après · Proposition',
    ].entries()) {
      const h = document.createElement('strong')
      const name = index === 0 ? state.report.before_name : state.report.after_name
      h.textContent = name ? `${tr(index === 0 ? 'Avant' : 'Après')} · ${name}` : tr(label)
      headings.append(h)
    }
    const rows = document.createElement('div')
    for (const g of previewMode ? [] : state.report.groups) {
      const row = document.createElement('section')
      row.className = 'comparison-group'
      row.dataset.active = String(g.id === active)
      row.dataset.group = String(g.id)
      const header = document.createElement('header'),
        title = document.createElement('strong'),
        status = document.createElement('span')
      title.textContent = `${g.id + 1}. ${tr(g.section === 'document' ? 'Document et éléments liés' : 'Passage')}`
      status.textContent = tr(
        g.decision === 'pending'
          ? 'À décider'
          : g.decision === 'applied'
            ? sourceBefore
              ? 'Repris dans le document'
              : 'Appliqué au document'
            : sourceBefore
              ? 'Passage actuel conservé'
              : 'Refusé',
      )
      status.hidden =
        g.decision === 'pending' && (!!state.report.read_only || !!state.report.closed)
      header.append(title, status)
      const columns = document.createElement('div')
      columns.className = 'comparison-columns'
      columns.append(side(state.before, g, 'before'), side(state.after, g, 'after'))
      const details = proposalDetails(
        {
          type: 'doc',
          content: state.before.state.document.content!.slice(g.before.start, g.before.end),
        },
        {
          type: 'doc',
          content: state.after.state.document.content!.slice(g.after.start, g.after.end),
        },
      )
      const summary = document.createElement('p')
      summary.className = 'comparison-details'
      summary.textContent = details.join(' · ')
      summary.hidden = !details.length
      const actions = actionRow()
      actions.classList.add('comparison-decisions')
      if (g.linked) {
        const linked = document.createElement('span')
        linked.className = 'comparison-linked'
        linked.textContent = tr('Éléments liés · une décision')
        actions.append(linked)
      }
      if (g.decision === 'pending' && !state.report.read_only && !state.report.closed) {
        const refuse = button(
          sourceBefore ? 'Conserver le passage actuel' : 'Refuser',
          () => void decide([g.id], 'refuse'),
        )
        const apply = button(
          sourceBefore ? 'Reprendre ce passage de la version' : 'Appliquer cette modification',
          () => void decide([g.id], 'apply'),
        )
        apply.className = 'primary'
        apply.disabled = busy || state.report.stale
        refuse.disabled = busy
        actions.append(refuse, apply)
      }
      row.append(header, columns, summary, actions)
      rows.append(row)
    }
    if (previewMode) {
      const pages = document.createElement('div')
      pages.className = 'comparison-columns comparison-previews'
      if (previewLoading) {
        const loading = document.createElement('p')
        loading.textContent = tr('Chargement des aperçus…')
        pages.append(loading)
      } else
        for (const name of ['before', 'after'] as const) {
          const column = document.createElement('div')
          column.setAttribute('aria-label', tr(name === 'before' ? 'Avant' : 'Après'))
          for (const [i, src] of (previews[name] || []).entries()) {
            const image = document.createElement('img')
            image.className = 'version-page'
            image.src = src
            image.alt = `${tr(name === 'before' ? 'Avant' : 'Après')} · ${tr('Page')} ${i + 1}`
            column.append(image)
          }
          pages.append(column)
        }
      rows.append(pages)
    }
    if (!state.report.groups.length && !previewMode) {
      const empty = document.createElement('p')
      empty.textContent = tr('Aucune différence.')
      rows.append(empty)
    }
    const footer = actionRow()
    footer.classList.add('comparison-progress')
    footer.hidden = !!state.report.read_only
    const progress = document.createElement('span')
    progress.setAttribute('role', 'status')
    progress.setAttribute('aria-live', 'polite')
    const done = state.report.groups.filter((g) => g.decision !== 'pending').length
    progress.textContent = `${done} / ${state.report.groups.length} · ${tr('décisions prises · états affichés inchangés')}`
    const all = button(
      sourceBefore ? 'Reprendre le reste' : 'Appliquer le reste',
      () =>
        void decide(
          state.report.groups.filter((g) => g.decision === 'pending').map((g) => g.id),
          'apply',
        ),
    )
    all.disabled = busy || state.report.stale || done === state.report.groups.length
    all.hidden = !!state.report.read_only || !!state.report.closed
    footer.append(progress, all)
    const files = document.createElement('details')
    files.className = 'comparison-files'
    files.hidden = !state.report.files?.length
    const filesTitle = document.createElement('summary')
    filesTitle.textContent = `${tr('Fichiers modifiés')} · ${state.report.files?.length || 0}`
    files.append(filesTitle)
    for (const file of state.report.files || []) {
      const row = document.createElement('div')
      row.className = 'comparison-columns'
      for (const side of ['before', 'after'] as const) {
        const cell = document.createElement('p')
        const v = file[side]
        cell.textContent = `${file.path} · ${v ? `${v.bytes} ${tr('octets')}` : tr('Absent')}`
        row.append(cell)
      }
      files.append(row)
    }
    const readOnly = document.createElement('p')
    readOnly.className = 'comparison-details'
    readOnly.hidden = !state.report.read_only && !state.report.closed
    readOnly.textContent = tr(
      state.report.read_only
        ? 'Consultation seule · deux versions conservées'
        : 'Proposition terminée ou connexion révoquée.',
    )
    element.replaceChildren(toolbar, notice, readOnly, headings, rows, files, footer)
    navigationGroup(navigation, { axis: 'horizontal' })
    if (action) {
      const candidate = [...element.querySelectorAll<HTMLButtonElement>('[data-action]')].find(
        (b) =>
          b.dataset.action === action &&
          b.closest<HTMLElement>('[data-group]')?.dataset.group === group,
      )
      if (candidate && !candidate.disabled) candidate.focus({ preventScroll: true })
    }
    if (focusedPicker) options.picker?.querySelector('select')?.focus({ preventScroll: true })
    if (scroller && scrollTop !== undefined) scroller.scrollTop = scrollTop
  }
  render()
  void options.api
    .listen(() => void refresh())
    .then((dispose) => {
      if (alive) disposeListener = dispose
      else dispose()
    })
    .catch(options.error)
  return {
    element,
    destroy() {
      alive = false
      serial++
      disposeListener?.()
      disposers.forEach((fn) => fn())
    },
  }
}
