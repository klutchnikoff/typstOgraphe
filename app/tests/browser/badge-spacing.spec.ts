import type { Editor } from '@tiptap/core'
import { expect, test } from '@playwright/test'
declare global {
  interface Window {
    __badgeSpacing?: Editor
  }
}

test.beforeEach(async ({ page, request }) => {
  await request.post('/__spike/invoke', { data: { command: 'reset', peer: 'main', args: {} } })
  await page.goto('/?client=main')
  await page.waitForFunction(() => window.__spike?.ready)
  await page.evaluate(async () => {
    // Use the actual rich editor and badge constructors in an isolated draft:
    // geometry does not require a document bibliography or a native publication.
    window.__spike!.editor.view.dom.hidden = true
    const element = document.createElement('div')
    element.dataset.badgeFixture = ''
    document.querySelector('#editor')!.append(element)
    const module = '/src/rich-zone.ts'
    const { createRichZone } = await import(module)
    const editor = createRichZone({ profile: 'document', label: 'Badge spacing', element })
    editor.view.dom.style.cssText = 'flex:none;width:320px;padding:0'
    window.__badgeSpacing = editor
    const referenceModule = '/src/citation-index.ts'
    const { setCitationEntries } = await import(referenceModule)
    setCitationEntries(editor, [
      {
        file_id: 'demo',
        key: 'imaginaire1200',
        file_name: 'demo.bib',
        title: 'Théorème de représentation',
        authors: 'Imaginaire',
        year: '1200',
        label: 'Imaginaire, 1200',
      },
    ])
    await document.fonts.ready
  })
})

for (const kind of ['footnote', 'citation'] as const) {
  test(`${kind} : espacement selon le texte précédent et alignement en début de ligne`, async ({
    page,
  }) => {
    for (const [prefix, flush] of [
      ['', true],
      ['Voir', false],
      ['Voir ', true],
      ['Voir\u00a0', true],
      ['Voir\n', true],
    ] as const) {
      const geometry = await page.evaluate(
        ({ kind, prefix }) => {
          const editor = window.__badgeSpacing!
          const content = prefix
            ? prefix.endsWith('\n')
              ? [{ type: 'text', text: prefix.slice(0, -1) }, { type: 'softBreak' }]
              : [{ type: 'text', text: prefix }]
            : []
          editor.commands.setContent({
            type: 'doc',
            content: [
              {
                type: 'paragraph',
                content: [
                  ...content,
                  {
                    type: kind,
                    ...(kind === 'citation'
                      ? {
                          attrs: {
                            items: [{ file_id: 'demo', key: 'imaginaire1200', supplement: '' }],
                            form: 'normal',
                          },
                        }
                      : {}),
                  },
                ],
              },
            ],
          })
          const badge = editor.view.dom.querySelector('.inline-object')!.getBoundingClientRect()
          const label = editor.view.dom
            .querySelector('.inline-object-label')!
            .getBoundingClientRect()
          const paragraph = editor.view.dom.querySelector('p')!.getBoundingClientRect()
          return {
            gap: label.left - badge.left,
            indent: label.left - paragraph.left,
            json: JSON.stringify(editor.getJSON()),
          }
        },
        { kind, prefix },
      )
      if (flush) expect(Math.abs(geometry.gap)).toBeLessThan(1)
      else expect(geometry.gap).toBeGreaterThan(2)
      if (!prefix || prefix.endsWith('\n')) expect(Math.abs(geometry.indent)).toBeLessThan(1)
      expect(geometry.json).not.toContain('inline-object-flush')
    }
    // A normal space can wrap: the badge must align with the visual line's start.
    const wrapped = await page.evaluate(async (kind) => {
      const editor = window.__badgeSpacing!
      editor.commands.setContent({
        type: 'doc',
        content: [
          {
            type: 'paragraph',
            content: [
              { type: 'text', text: 'Voir ' },
              {
                type: kind,
                ...(kind === 'citation'
                  ? {
                      attrs: {
                        items: [{ file_id: 'demo', key: 'imaginaire1200', supplement: '' }],
                        form: 'normal',
                      },
                    }
                  : {}),
              },
            ],
          },
        ],
      })
      await document.fonts.ready
      const label = editor.view.dom.querySelector('.inline-object-label')!
      const width = label.getBoundingClientRect().width + 4
      editor.view.dom.style.width = `${width}px`
      const rect = label.getBoundingClientRect()
      const paragraph = editor.view.dom.querySelector('p')!
      const range = document.createRange()
      range.setStart(paragraph.firstChild!, 0)
      range.setEnd(paragraph.firstChild!, 4)
      return {
        indent: rect.left - paragraph.getBoundingClientRect().left,
        secondLine: rect.top > range.getBoundingClientRect().bottom,
        fragments: label.getClientRects().length,
      }
    }, kind)
    expect(wrapped.secondLine).toBe(true)
    expect(wrapped.fragments).toBe(1)
    expect(Math.abs(wrapped.indent)).toBeLessThan(1)
  })
}
