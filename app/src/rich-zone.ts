import {
  Editor,
  Extension,
  Node,
  type EditorOptions,
  type Extensions,
  type RawCommands,
} from '@tiptap/core'
import StarterKit from '@tiptap/starter-kit'
import Collaboration from '@tiptap/extension-collaboration'
import Caret from '@tiptap/extension-collaboration-caret'
import { ySyncPlugin, yUndoPluginKey, ySyncPluginKey } from '@tiptap/y-tiptap'
import type * as Y from 'yjs'
import type { Provider } from './provider'
import { bindEditingHistory, editingProvider, historyFor } from './editing-history'
import { bindEditorTarget, type RecordRoot } from './editor-targets'
import { bindCommandContext } from './command-context'
import { capabilityAvailability, editingAvailability } from './command-availability'
import { runFormatting } from './formatting'
import { syncNativeTextSelection } from './native-selection'
import { Plugin } from '@tiptap/pm/state'
import { ContextBand } from './band-layout'
import { DocumentInput, SoftBreak, TypstInlineInput } from './editing'
import { DocumentWhitespace } from './document-whitespace'
import { SharedInlineMarks } from './shared-inline-marks'
import { documentExtensions } from './document-zone-extensions'
import { inlineObjectKey } from './inline-objects'
import { widgetKey } from './widgets'
import type { WidgetPresentations } from './widget-frame'

/** Policy, not per-field feature lists. Every new rich field chooses one of these profiles. */
export const richZoneProfiles = {
  document: { content: 'block+', multiline: true, widgets: true, trailingParagraph: true },
  'rich-text': { content: 'paragraph+', multiline: true, widgets: false, trailingParagraph: false },
  'inline-text': {
    content: 'paragraph',
    multiline: false,
    widgets: false,
    trailingParagraph: false,
  },
} as const
export type RichZoneProfile = keyof typeof richZoneProfiles
export type EditorialProfile = Exclude<RichZoneProfile, 'document'>
export function richFieldProfile(kind: 'rich' | 'rich_inline'): EditorialProfile {
  return kind === 'rich_inline' ? 'inline-text' : 'rich-text'
}

type Binding =
  | { kind: 'document'; document: Y.Doc; presence?: Provider }
  | { kind: 'fragment'; fragment: Y.XmlFragment; historyOwner: Editor; target?: RecordRoot }
type OpenProperties = (pos: number, kind: string, remove?: boolean) => void
interface ZoneConfiguration {
  imageURL?: (path: string) => string
  properties?: OpenProperties
  presentations?: WidgetPresentations
  binding?: Binding
}

/** Schema, Yjs seeds and rendering use the very same profile as mounted zones. */
export function richZoneExtensions(
  profile: RichZoneProfile,
  configuration: ZoneConfiguration = {},
): Extensions {
  const policy = richZoneProfiles[profile]
  const binding = configuration.binding
  const native =
    binding?.kind === 'document'
      ? binding.presence?.nativeEditing
        ? binding.presence
        : undefined
      : binding?.kind === 'fragment'
        ? editingProvider(binding.historyOwner)
        : undefined
  const extensions: Extensions = [
    Node.create({ name: 'doc', topNode: true, content: policy.content }),
    ContextBand,
    SoftBreak,
    SharedInlineMarks,
    TypstInlineInput,
    ...(policy.multiline
      ? [
          DocumentInput,
          DocumentWhitespace.configure({ trailingParagraph: policy.trailingParagraph }),
        ]
      : [
          Extension.create({
            name: 'singleLineInput',
            priority: 1200,
            addKeyboardShortcuts() {
              return {
                Enter: () => !this.editor.view.composing,
                'Shift-Enter': () => !this.editor.view.composing,
              }
            },
          }),
        ]),
    StarterKit.configure({
      document: false,
      undoRedo: binding ? false : {},
      trailingNode: false,
      heading: policy.widgets ? { levels: [1, 2, 3] } : false,
      blockquote: false,
      bulletList: false,
      orderedList: false,
      listItem: false,
      code: false,
      codeBlock: false,
      strike: false,
      horizontalRule: false,
      link: { openOnClick: false },
    }),
    ...(policy.widgets
      ? documentExtensions(
          configuration.imageURL || (() => ''),
          configuration.properties,
          configuration.presentations,
        )
      : []),
  ]
  if (binding?.kind === 'document') {
    if (native)
      extensions.push(
        Extension.create({
          name: 'nativeDocumentSync',
          addProseMirrorPlugins: () => [ySyncPlugin(binding.document.getXmlFragment('default'))],
        }),
      )
    else extensions.push(Collaboration.configure({ document: binding.document, field: 'default' }))
    if (binding.presence) {
      const provider = binding.presence
      extensions.push(
        Caret.configure({
          provider,
          user: {
            name: provider.id === 'main' ? 'Camille' : 'Alex',
            color: provider.id === 'main' ? '#286b91' : '#99733b',
          },
        }),
      )
    }
  } else if (binding?.kind === 'fragment') {
    // A rich field shares the document undo owner; destroying the field must never destroy it.
    extensions.push(
      Extension.create({
        name: 'sharedZoneFragment',
        addProseMirrorPlugins: () => [ySyncPlugin(binding.fragment)],
        addCommands() {
          const change =
            (redo: boolean): RawCommands['undo'] =>
            () =>
            ({ tr, dispatch }) => {
              tr.setMeta('preventDispatch', true)
              if (!editingAvailability(this.editor, false).enabled) return false
              const history = historyFor(binding.historyOwner)
              if (!(redo ? history?.redoStack.length : history?.undoStack.length)) return false
              if (dispatch) {
                history!.stopCapturing()
                if (redo) history!.redo()
                else history!.undo()
              }
              return true
            }
          return { undo: change(false), redo: change(true) }
        },
        addKeyboardShortcuts() {
          const change = (redo = false) => {
            if (!editingAvailability(this.editor, false).enabled) return false
            if (redo) this.editor.commands.redo()
            else this.editor.commands.undo()
            return true
          }
          return {
            'Mod-z': () => change(),
            'Mod-Shift-z': () => change(true),
            'Mod-y': () => change(true),
          }
        },
      }),
    )
  }
  if (native)
    extensions.push(
      Extension.create({
        name: 'nativeEditingHistory',
        priority: 1500,
        addStorage: () => ({
          compositionToken: 0,
          compositionTimer: undefined as ReturnType<typeof setTimeout> | undefined,
        }),
        addProseMirrorPlugins() {
          return [
            new Plugin({
              props: {
                handleDOMEvents: {
                  compositionstart: () => {
                    clearTimeout(this.storage.compositionTimer)
                    this.storage.compositionToken = native.beginComposition()
                    return false
                  },
                  compositionend: () => {
                    // Let ProseMirror flush its DOM observer and the shared whitespace
                    // repair first. Preedit bytes have never left this Provider.
                    const token = this.storage.compositionToken
                    // A committed composition survives unmounting its zone. Its
                    // token cannot end a newer composition in another field.
                    this.storage.compositionTimer = setTimeout(
                      () => native.endComposition(token),
                      0,
                    )
                    return false
                  },
                },
              },
            }),
          ]
        },
        dispatchTransaction({ transaction, next }) {
          const remote = transaction.getMeta(ySyncPluginKey)?.isChangeOrigin
          const typing =
            transaction.docChanged &&
            transaction.steps.length === 1 &&
            transaction.steps[0].toJSON().stepType === 'replace' &&
            (transaction.steps[0].toJSON().slice?.content || []).every(
              (node: { type: string }) => node.type === 'text',
            )
          const boundary =
            !remote &&
            !typing &&
            (transaction.docChanged || transaction.selectionSet || transaction.storedMarksSet)
          if (boundary) native.stopCapturing()
          next(transaction)
          if (boundary) native.stopCapturing()
        },
        addCommands() {
          const change =
            (redo: boolean): RawCommands['undo'] =>
            () =>
            ({ tr, dispatch }) => {
              tr.setMeta('preventDispatch', true)
              if (!editingAvailability(this.editor, false).enabled || !native.canHistory(redo))
                return false
              if (dispatch) void native.history(redo ? 'redo' : 'undo').catch(native.onError)
              return true
            }
          return { undo: change(false), redo: change(true) }
        },
        addKeyboardShortcuts() {
          return {
            'Mod-z': () =>
              !editingAvailability(this.editor, false).enabled
                ? false
                : (this.editor.commands.undo(), true),
            'Mod-Shift-z': () =>
              !editingAvailability(this.editor, false).enabled
                ? false
                : (this.editor.commands.redo(), true),
            'Mod-y': () =>
              !editingAvailability(this.editor, false).enabled
                ? false
                : (this.editor.commands.redo(), true),
          }
        },
      }),
    )
  return extensions
}

type Lifecycle = Pick<
  Partial<EditorOptions>,
  | 'element'
  | 'content'
  | 'editable'
  | 'onUpdate'
  | 'onTransaction'
  | 'onSelectionUpdate'
  | 'onCreate'
  | 'onDestroy'
>
export interface RichZoneOptions extends Lifecycle, ZoneConfiguration {
  profile: RichZoneProfile
  label: string
  language?: string
  openLink?: (editor: Editor) => void
  openContext?: (editor: Editor) => boolean
}

/** Sole runtime construction point: policy and interaction defaults cannot be forgotten by a view. */
export function createRichZone(options: RichZoneOptions): Editor {
  const {
    profile,
    label,
    language,
    openLink,
    openContext,
    imageURL,
    properties,
    presentations,
    binding,
    ...lifecycle
  } = options
  const policy = richZoneProfiles[profile]
  const native =
    binding?.kind === 'document'
      ? binding.presence?.nativeEditing
        ? binding.presence
        : undefined
      : binding?.kind === 'fragment'
        ? editingProvider(binding.historyOwner)
        : undefined
  const history = () =>
    binding?.kind === 'fragment'
      ? yUndoPluginKey.getState(binding.historyOwner.state)?.undoManager
      : undefined
  if (binding?.kind === 'fragment' && !native) {
    const owner = history()
    if (!owner || owner.doc !== binding.fragment.doc)
      throw new Error('A shared rich zone requires an initialized undo owner on the same Y.Doc.')
    // A containing scope (normally settings) already covers all its fields.
    let covered: (typeof owner.scope)[number] | null = binding.fragment
    while (covered && !owner.scope.includes(covered)) covered = covered.parent
    if (!covered) owner.addToScope(binding.fragment)
  }
  let editor!: Editor
  editor = new Editor({
    ...lifecycle,
    extensions: [
      ...richZoneExtensions(profile, { imageURL, properties, presentations, binding }),
      Extension.create({
        name: 'richZoneInteractions',
        priority: 1600,
        addKeyboardShortcuts() {
          return {
            'Mod-b': () => {
              if (!editingAvailability(this.editor).enabled) return false
              runFormatting(this.editor, 'text.bold')
              return true
            },
            'Mod-i': () => {
              if (!editingAvailability(this.editor).enabled) return false
              runFormatting(this.editor, 'text.italic')
              return true
            },
            ...(policy.widgets
              ? Object.fromEntries(
                  ([0, 1, 2, 3] as const).map((level) => [
                    `Mod-Alt-${level}`,
                    () => {
                      if (!editingAvailability(this.editor).enabled) return false
                      runFormatting(
                        this.editor,
                        level ? `block.heading_${level}` : 'block.paragraph',
                      )
                      return true
                    },
                  ]),
                )
              : {}),
            'Mod-k': () => {
              if (!openLink || !editingAvailability(this.editor).enabled) return false
              if (capabilityAvailability(this.editor, 'text.link', 'link').enabled)
                openLink(this.editor)
              return true
            },
          }
        },
      }),
    ],
    editorProps: {
      attributes: {
        role: 'textbox',
        'aria-label': label,
        'aria-multiline': String(policy.multiline),
        'data-rich-zone-profile': profile,
        ...(language ? { lang: language } : {}),
        // The app owns proofing; native services remain available on explicit request.
        spellcheck: 'false',
        autocorrect: 'off',
      },
      handleKeyDown: (view, event) => {
        if (event.isComposing || view.composing) return false
        syncNativeTextSelection(editor)
        const owner = editor
        if (policy.widgets) {
          if (inlineObjectKey(owner, event, properties || (() => {}))) return true
          if (widgetKey(owner, event)) return true
        }
        if ((event.ctrlKey || event.metaKey) && event.key === 'Enter') {
          if (openContext) {
            event.preventDefault()
            openContext(owner)
            return true
          }
          if (openLink && owner.isActive('link')) {
            event.preventDefault()
            openLink(owner)
            return true
          }
        }
        return false
      },
    },
    onFocus: () => historyFor(editor)?.stopCapturing(),
    onBlur: () => historyFor(editor)?.stopCapturing(),
  })
  if (binding?.kind === 'document') bindEditorTarget(editor, { kind: 'body' })
  else if (binding?.kind === 'fragment' && binding.target) bindEditorTarget(editor, binding.target)
  if (native) bindEditingHistory(editor, native)
  bindCommandContext(
    editor,
    binding?.kind === 'fragment' ? binding.historyOwner : editor,
    openLink
      ? () => {
          if (capabilityAvailability(editor, 'text.link', 'link').enabled) openLink(editor)
        }
      : undefined,
  )
  return editor
}
const ownShortcuts = new Set([
  'history.undo',
  'history.redo',
  'text.bold',
  'text.italic',
  'text.link',
  'context.open',
  'block.remove',
])
/** Text formatting runs in the view's key handler, after ProseMirror has flushed
 * pending DOM input. Named fields also own their local history/context keys.
 */
export function richZoneOwnsShortcut(target: EventTarget | null, command: string) {
  if (!(target instanceof Element) || !ownShortcuts.has(command)) return false
  const zone = target.closest<HTMLElement>('[data-rich-zone-profile]')
  return (
    !!zone &&
    (zone.dataset.richZoneProfile !== 'document' ||
      ['text.bold', 'text.italic', 'text.link'].includes(command))
  )
}
