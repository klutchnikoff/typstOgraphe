import { afterEach, describe, expect, it } from 'vitest'
import { Editor } from '@tiptap/core'
import type { Capability } from '../src/commands'
import { hasSettings, settingsForNode } from '../src/capability-settings'
import { ParameterFields } from '../src/parameter-fields'
import { baseExtensions } from '../src/extensions'
import { widgetPresentations } from '../src/widget-frame'

function capability(node: string, parameters: Capability['parameters'] = {}): Capability {
  return {
    id: `test.${node}`,
    node,
    parameters,
    mark: null,
    label: node,
    label_key: node,
    icon: 'quote',
    group: 'test',
    letter_only: false,
  }
}
afterEach(() => document.body.replaceChildren())

describe('paramètres dérivés du contrat', () => {
  it('un paramètre optionnel suffit à créer le champ et activer les réglages', () => {
    const contract = capability('unknownWidget')
    expect(hasSettings(settingsForNode(contract.node!, [contract]))).toBe(false)
    contract.parameters.properties = {
      spacing: { type: 'number', default: 1.5, minimum: 0, maximum: 4, title: 'Espacement' },
    }
    const settings = settingsForNode(contract.node!, [contract])
    expect(hasSettings(settings)).toBe(true)
    expect(settings.fields[0]).toMatchObject({
      name: 'spacing',
      attribute: 'spacing',
      label: 'Espacement',
      control: 'number',
      required: false,
      schema: { minimum: 0, maximum: 4, default: 1.5 },
    })
    delete contract.parameters.properties.spacing
    expect(hasSettings(settingsForNode(contract.node!, [contract]))).toBe(false)
  })

  it('ne duplique ni le corps ni la structure ni les paramètres de création', () => {
    const contract = capability('unknownWidget', {
      properties: {
        source: { type: 'string' },
        attribution: { type: 'string', 'x-editor': { placement: 'body' } },
        rows: { type: 'integer', 'x-editor': { placement: 'structure' } },
        header: { type: 'boolean', 'x-editor': { placement: 'creation' } },
        resource: {
          type: 'string',
          'x-editor': { placement: 'panel', attribute: 'src', control: 'image' },
        },
      },
      required: ['source', 'resource'],
    })
    expect(settingsForNode(contract.node!, [contract]).fields).toMatchObject([
      { name: 'resource', attribute: 'src', control: 'image', required: true },
    ])
  })

  it('une action de conversion réelle reste disponible sans champ optionnel', () => {
    const a = capability('first'),
      b = capability('second')
    a.editor = b.editor = { conversion_group: 'alternatives' }
    expect(hasSettings(settingsForNode('first', [a]))).toBe(false)
    const settings = settingsForNode('first', [a, b])
    expect(settings.fields).toEqual([])
    expect(settings.conversions).toEqual([a, b])
    expect(hasSettings(settings)).toBe(true)
  })

  it('un type non pris en charge échoue explicitement au lieu de produire un panneau vide', () => {
    expect(() =>
      settingsForNode('custom', [
        capability('custom', {
          properties: { complex: { type: 'object' } },
        }),
      ]),
    ).toThrow('test.custom.complex')
  })
})

it('un nouveau champ du contrat traverse formulaire, attribut de nœud et sérialisation sans cas dédié', () => {
  const contract = capability('quote', {
    properties: {
      indent: { type: 'integer', minimum: 1, maximum: 8, default: 2, title: 'Retrait' },
    },
  })
  const presentation = widgetPresentations([contract])
  const editor = new Editor({
    element: document.createElement('div'),
    extensions: baseExtensions(() => '', undefined, presentation),
    content: {
      type: 'doc',
      content: [{ type: 'quote', content: [{ type: 'paragraph' }, { type: 'quoteAttribution' }] }],
    },
  })
  try {
    expect(editor.state.doc.firstChild!.attrs.indent).toBe(2)
    expect(editor.view.dom.querySelector('button[aria-label^="Paramètres"]')).toHaveProperty(
      'disabled',
      false,
    )
    const root = document.createElement('div')
    document.body.append(root)
    const fields = new ParameterFields(root, () => {})
    fields.render(presentation.quote!.settings.fields, {})
    const input = root.querySelector('input')!
    expect(input.value).toBe('2')
    input.value = '9'
    expect(fields.read()).toHaveProperty('error')
    expect(editor.state.doc.firstChild!.attrs.indent).toBe(2)
    expect(document.activeElement).toBe(input)
    input.value = '4'
    const result = fields.read()
    if (!result.attrs) throw new Error('Expected valid settings')
    editor.view.dispatch(editor.state.tr.setNodeMarkup(0, undefined, result.attrs))
    expect(editor.getJSON().content![0].attrs!.indent).toBe(4)
  } finally {
    editor.destroy()
  }
})

it('les contrôles génériques conservent les types et les noms des attributs', () => {
  const contract = capability('link', {
    properties: {
      url: {
        type: 'string',
        pattern: '^(https?://|mailto:).+',
        'x-editor': { placement: 'panel', attribute: 'href' },
      },
      visible: { type: 'boolean', default: false, 'x-editor': { placement: 'panel' } },
      tone: { type: 'string', enum: ['normal', 'muted'], default: 'normal' },
      text: { type: 'string', default: '', 'x-editor': { multiline: true } },
    },
    required: ['url', 'visible'],
  })
  const root = document.createElement('div')
  const fields = new ParameterFields(root, () => {})
  fields.render(settingsForNode('link', [contract]).fields, {})
  root.querySelector<HTMLInputElement>('input[type=text]')!.value = 'https://example.org'
  root.querySelector<HTMLInputElement>('input[type=checkbox]')!.checked = true
  root.querySelector('select')!.value = JSON.stringify('muted')
  root.querySelector('textarea')!.value = 'Deux lignes\nà conserver'
  expect(fields.read()).toEqual({
    attrs: {
      href: 'https://example.org',
      visible: true,
      tone: 'muted',
      text: 'Deux lignes\nà conserver',
    },
  })
  root.querySelector<HTMLInputElement>('input[type=checkbox]')!.checked = false
  expect(fields.read().attrs?.visible).toBe(false)
})
