# Interface typstOgraphe. Ne pas traduire les données des courriers.

save-dialog = Save document
native-menu-about = About typstOgraphe
native-menu-hide = Hide typstOgraphe
native-menu-hide-others = Hide Others
native-menu-cut = Cut
native-menu-copy = Copy
native-menu-paste = Paste
native-menu-select-all = Select All
file-dialog-unavailable = The system could not open the file dialog. No file was opened or saved. Please try again.
letter-filter = typstOgraphe document
default-filename = Document.typsto
replace-title = Replace the document?
letter-opened = Document opened.
open-dialog = Open document
all-files = All files
renderer-stopped = The rendering engine has stopped.
multiple-styles = Mixed styles
preview-title = Typst preview
preview-hint = Double-click a passage to find it in the editor.
preview-updating = Updating…
preview-failed = The document could not be typeset. The last successful preview is still displayed.
error-details = Details
preview-composing = Typesetting the document…
group-editorial-header = Header
group-recipient = Recipient
group-letter-information = Information
date-today = Today
date-today-help = Insert the current local date and keep it in this document.
date-input-hint = DD/MM/YYYY or today
field-recipient = Recipient
field-address = Address
field-subject = Subject
field-reference = Reference
field-place = Place
field-date = Date
group-contact-details = Contact details
group-professional-details = Organisation
group-sender = Sender
field-full-name = Full name
field-role = Role
field-service = Department
field-email = Email
link-title = Link
apply = Apply
command-search = Search for a command…
export-title = Export document
export-folder = Destination folder
export-help = Choose a new or empty folder. Use Save to keep an editable document in a single file.
export-button = Export
export-required-fields = Fill in the recipient and subject before exporting.
export-pending = Exporting…
unsaved-title = Save changes?
unsaved-message = The document has unsaved changes.
save = Save
discard = Don’t save
cancel = Cancel
file-error-title = Document file
close = Close
untitled-letter = Untitled document
file-menu = File
letter-type = Letter
embedded-template = Document template
storage-file-too-large = The letter file exceeds the size limit.
storage-file-count = The archive contains too many files or its structure is incomplete (4,096 files maximum).
storage-unknown-entry = unknown or duplicate file in the archive
storage-expanded-too-large = the expanded content exceeds 64 MiB
storage-unsupported = unsupported format, version or document type
storage-main-mismatch = the main.typ header does not match the letter metadata
storage-letter-too-large = The letter exceeds the size limit (64 MiB).
export-nonempty = The folder is not empty. Choose a new folder to preserve the existing files.
shortcut-modifier-required = Use Ctrl, ⌘ or Alt, or a function key, to preserve text entry.
cmd-new = New document
cmd-open = Open document…
cmd-save = Save
cmd-save-as = Save as…
cmd-export = Export document…
cmd-export-short = Export
cmd-information = Document information
cmd-information-short = Information
cmd-close = Close
cmd-palette = Command palette
cmd-bold = Bold
cmd-bold-short = B
cmd-italic = Italic
cmd-italic-short = I
cmd-link = Insert or edit a link
cmd-link-short = Link
cmd-remove-link = Remove link
cmd-clear = Clear formatting
cmd-paragraph = Paragraph
cmd-heading-1 = Heading 1
cmd-heading-2 = Heading 2
cmd-heading-3 = Heading 3
cmd-bullet = Bulleted list
cmd-bullet-short = • List
cmd-numbered = Numbered list
cmd-numbered-short = 1. List
cmd-description = Description list
cmd-description-short = Description
cmd-undo = Undo
cmd-redo = Redo
letter-saved = Document saved: { $path }
replace-message = The file { $path } already exists. Do you want to replace it?
letter-exported = Document exported to { $path }
mixed-command = { $command } (mixed selection)
link-selected-text = Text: { $text }
command-shortcut = { $label } · { $shortcut }
window-title = { $name }{ $marker }
storage-open-failed = Could not open the document: { $error }
storage-zip-unreadable = unreadable ZIP archive ({ $error })
storage-manifest-unreadable = unreadable manifest ({ $error })
storage-metadata-unreadable = unreadable metadata ({ $error })
storage-entry-unreadable = { $name } is unreadable ({ $error })
storage-entry-too-large = { $name } exceeds the size limit
storage-temporary-failed = Could not create a temporary file: { $error }
storage-write-failed = Could not write the document: { $error }
storage-replace-failed = Could not replace the document: { $error }
export-write-failed = Could not write { $name }: { $error }
shortcut-unknown-key = Unknown key: { $key }
shortcut-unknown-command = Unknown command: { $id }
shortcut-duplicate = Shortcut defined more than once for { $command }
shortcut-conflict = Shortcut conflict: { $first } and { $second }
cli-render-help = Render one page of a saved document as PNG
cli-mcp-help = Run the local MCP server over stdio with explicit file permissions
cli-mcp-read-only-help = Allow queries, read-only opening and closing only
mcp-file-access = Document file access is refused. Check the declared read/write roots, parent components, symlinks and lock companion.
mcp-read-only = This MCP server is read-only.
mcp-document-required = Supply the document reference returned by document.inspect or the previous edit.
mcp-document-foreign = This document reference belongs to another MCP session or a replaced document.
mcp-stdio-required = Start the MCP server through the stdio entry point.
mcp-completed = { $id } completed; inspect the outcome for any explicitly requested file operation.
cli-page-help = Page number, starting at 1
cli-png-help = New PNG file; existing destinations are refused
cli-render-page = The requested page does not exist in this document.
cli-render-io = Could not write the PNG file. Check the destination folder and its permissions.
cli-rendered = Page { $page } rendered: { $path }
cli-usage-capture = Usage: --capture IMAGE.png
cli-usage-capture-recovery = Usage: --capture-recovery IMAGE.png RECOVERY_FOLDER
cli-usage-capture-search = Usage: --capture-search IMAGE.png QUERY
cli-unknown-option = Unknown option: { $option }
cli-capture-error = Screenshot: { $error }
cli-help =
    typstOgraphe
      cargo run
      cargo run -- LETTER.typsto
      cargo run -- --capture IMAGE.png
      cargo run -- --capture-new-document IMAGE.png
      cargo run -- --capture-recovery IMAGE.png RECOVERY_FOLDER
      cargo run -- --capture-search IMAGE.png QUERY
      cargo run -- --capture-assistance IMAGE.png
      cargo run -- --capture-spelling IMAGE.png
      cargo run -- --capture-profile IMAGE.png
      cargo run -- --capture-profile-letter IMAGE.png
preview-stats =
    { $pages ->
        [one] { $pages } page
       *[other] { $pages } pages
    }
preview-render-time = Rendered in { $milliseconds } ms
document-stats =
    Document: { $words ->
        [one] { $words } word
       *[other] { $words } words
    } · { $signs ->
        [one] { $signs } character
       *[other] { $signs } characters
    }
selection-stats =
    Selection: { $words ->
        [one] { $words } word
       *[other] { $words } words
    } · { $signs ->
        [one] { $signs } character
       *[other] { $signs } characters
    }
statistics-hint =
    { $without_spaces ->
        [one] { $without_spaces } character excluding spaces.
       *[other] { $without_spaces } characters excluding spaces.
    }
    Characters include spaces; paragraph breaks, structural tabs and image objects are excluded.
    Words use Unicode segmentation (can't counts as one word, well-being as two).
    Scope: authored text, headings, captions and attributions, excluding metadata and content added by the template.
edit-menu = Edit
view-menu = View
cmd-quit = Quit typstOgraphe
mac-services = Services
mac-hide = Hide typstOgraphe
mac-hide-others = Hide Others
mac-show-all = Show All
native-menu-error = Could not update the menus: { $error }

group-general = General
document-settings-heading = Letter settings:
document-settings-short = Settings:
document-settings-close = Collapse settings
field-document-type = Document type
field-template = Template
field-paper = Paper size
field-orientation = Orientation
field-document-language = Document language
field-bullet-marker = List markers
bullet-marker-automatic = Automatic (template)
bullet-marker-model-controlled = The template applies its house style and the document language conventions.
bullet-marker-model-choices = The template creator defines the allowed choices. Automatic follows its house style and the document language.
bullet-marker-custom = Custom character…
model-invalid-editor-options = Invalid template editor-options: bullet-markers must be an array of visible characters, and custom-bullet-marker a boolean.
model-bullet-marker-not-allowed = This list marker is not allowed by the template. Restore automatic mode in the document information.
orientation-portrait = Portrait
orientation-landscape = Landscape
paper-letter = US Letter
paper-legal = US Legal
language-french = French
language-english = English
language-german = German
language-spanish = Spanish
language-italian = Italian
language-portuguese = Portuguese
language-dutch = Dutch

cmd-preferences = Preferences…
preferences-title = Preferences
preferences-profile = Personal profile
preferences-appearance = Appearance
preferences-spelling = Spelling
field-organisation = Organisation
field-sender-address = Address
field-phone = Phone
profile-help = These details are suggested when creating documents. Each document keeps its own copy, editable independently of the profile.
signature-label = Signature
signature-choose = Choose an image…
signature-from-profile = Profile signature
signature-from-profile-help = Use the signature saved in preferences. The letter keeps its own copy; this action can be undone.
signature-profile-unavailable = No profile signature is available. Choose an image in Preferences → Personal profile.
signature-none = No signature.
profile-signature-help = The application keeps the image and copies it into new letters. The template controls its presentation.
signature-document-help = This signature belongs only to this letter. Changing it does not change the personal profile.
profile-invalid-signature = The stored signature reference is invalid. Choose an image or remove the signature.
profile-signature-error = Could not load or store the signature: { $error }
profile-no-directory = The preferences directory is unavailable. The signature cannot be stored there.
profile-signature-unavailable = The profile signature is unavailable. Choose an image or remove the signature in preferences before creating a letter.
cli-usage-capture-profile = Usage: --capture-profile IMAGE.png or --capture-profile-letter IMAGE.png
editor-font = Editor font
editor-font-size = Text size
appearance = Appearance
theme-system = System
theme-light = Light
theme-dark = Dark
preferences-help = Settings apply immediately and are saved for future sessions. Font and size affect only the editor; the template controls the document’s appearance.
column-divider-hint = Drag to resize the columns. Double-click to restore equal widths.

description-edit-hint = Enter the term, then Tab or a colon followed by a space to enter its description. Shift+Tab returns to the term.
bullet-marker-restore-automatic = Restore template choice

list-menu = Lists
heading-menu = Headings
heading-menu-hint = Headings and return to body text
end-menu = Letter ending
end-menu-hint = Postscript and attachments
description-placeholder = Description

cmd-quote = Block quote
quote-attribution-placeholder = Attribution / source (optional)
insert-menu = Insert
cmd-insert-image = Insert image…
image-files = Images (PNG, JPEG, SVG)
image-label = Image
image-replace = Replace…
image-information = Information
image-remove = Delete
image-caption-placeholder = Add a caption…
image-alt-label = Alternative text
image-alt-help = Describe what the image shows for people who cannot see it. This text is separate from the caption and does not appear in the letter.
image-apply = Apply
image-cancel = Cancel
image-too-large = An image cannot exceed 16 MiB.
image-format-unsupported = This image format is not supported. Choose a PNG, JPEG or SVG file.
image-resource-missing = Image missing from document: { $path }
image-invalid-svg = The SVG file is unreadable or invalid.

minutes-type = Meeting minutes
minutes-settings-heading = Minutes settings:
group-meeting = Meeting
group-attendees = Attendees
field-meeting-title = Title
field-meeting-place = Location / video call
field-minutes-author = Written by
field-attendees = Attendees
field-apologies = Apologies
new-document-title = Create a document
new-document-create = Create
cmd-import-model = Import a template…
model-file-filter = Typst template
model-import-error = Cannot use this template: { $error }
model-too-large = The template exceeds the size limit (1 MiB).
model-contract-missing = The template must export the { $function } function.
cmd-recover-documents = Recover documents…
cmd-find = Find…
cmd-find-replace = Find and replace…
cmd-find-next = Next occurrence
cmd-find-previous = Previous occurrence
cmd-replace-current = Replace
cmd-replace-all = Replace all
search-placeholder = Find…
search-replacement-placeholder = Replace with…
search-toggle-replace = Show or hide replacement
search-match-case = Match case
search-close = Close search (Escape)
search-active-result = { $current } / { $total }
search-results =
    { $count ->
        [one] { $count } occurrence
       *[other] { $count } occurrences
    }
search-replaced =
    { $count ->
        [one] { $count } replacement made.
       *[other] { $count } replacements made.
    }
recovery-title = Documents to recover
recovery-help = Working copies have been preserved. Recover opens a copy to save separately, without replacing the original file.
recovery-date = Copy from { $date } (UTC)
recovery-restore = Recover
recovery-later = Later
recovery-delete = Delete copy
recovery-document-name = { $name } (recovered)
recovery-filename-suffix = recovered
recovery-restored = Document recovered. Use Save as to keep it.
recovery-unavailable = Recovery protection unavailable — hover for details.
recovery-no-directory = The application data directory could not be located.
recovery-load-failed = Unable to recover this copy: { $error }
recovery-delete-failed = Unable to delete this copy: { $error }
cmd-spelling-toggle = Check spelling while typing
cmd-spelling-next = Next spelling issue
cmd-spelling-previous = Previous spelling issue
cmd-spelling-suggestions = Spelling suggestions…
cli-usage-capture-spelling = Usage: --capture-spelling IMAGE.png
cmd-spelling-ignore = Ignore this word in the document
cmd-spelling-learn = Add to personal dictionary
cmd-spelling-reset = Reset ignored words in the document
spelling-count = Spelling: { $count }
spelling-checking = Checking spelling…
spelling-hint = Words to check. Click to select the next one; right-click an underlined word for suggestions. Local analysis uses the document language.
spelling-no-dictionary = Spelling unavailable ({ $language })
spelling-dictionaries = Bundled dictionaries: classical French and American English. No text is sent to an external service.
spelling-error = Spelling unavailable
spelling-no-issue-here = No spelling issue at this position.
spelling-no-suggestions = No suggestions.
spelling-loading-suggestions = Looking up suggestions…
spelling-personal-dictionary = Personal dictionary
spelling-personal-help = Add words by right-clicking in the editor. Words are saved in preferences, per language.
spelling-forget-word = Remove

welcome-help = Create a document or open a .typsto file.
type-no-model = No template is available for this document type.
cmd-personal-letter = Use the personal letter template
model-personal-letter = Personal letter
model-fixed-page = This template fixes the paper size and orientation for a window envelope.
cli-usage-capture-creation = Usage: --capture-new-document IMAGE.png

package-resource-missing = Missing Typst dependency: { $path }
package-inventory-invalid = The Typst capability and package inventory is invalid or inconsistent.

cmd-insert-postscript = Insert a postscript
cmd-insert-attachment = Add an enclosure mention
postscript-placeholder = Postscript text…
attachment-placeholder = Enclosed document…
edit-stale-draft = The document has changed since this edit began. Please try again.
engine-invalid-params = This command has invalid parameters.
engine-unavailable = This command is not available in the current context.

engine-image-error = Unable to process this image: { $error }

export-format = Format
export-pdf = PDF
export-typst = Typst
export-both = PDF + Typst
export-page-images = Include PNG pages (diagnostic)

file-already-exists = The file already exists. Confirm its replacement in Save As.

engine-batch-step-error = Step { $step }: { $error }
engine-batch-step-limit = A batch must contain between 1 and 256 steps.
engine-batch-size-limit = The batch exceeds the allowed size limit.
engine-edit-context-closed = The editing context is no longer available.
engine-batch-step-forbidden = This action is not allowed in an editing batch.

core-external-error = { $error }
paper-a3 = A3
paper-a4 = A4
paper-a5 = A5
spelling-no-dictionary-detail = No bundled dictionary for language { $language }.

# Accès concurrent et CLI documentaire.
file-locked-title = Document already open
file-locked = This document is already open for editing. Close it in the other session, or open the saved version read-only.
file-open-read-only = Open read-only
file-read-only = This document is read-only. Save a copy to edit it.
file-read-only-title = { $name } (read-only)
file-changed = The file has changed outside this session. Reopen it or save your work to another file.
file-access-error = Cannot access the document or acquire its write lock.
engine-unknown-command = Unknown command identifier.
cli-about = Inspect, check and export Typst documents without opening a window
cli-json-help = Produce a stable JSON response
cli-lang-help = Choose the interface language
cli-commands-help = Discover the document engine catalogue
cli-inspect-help = Read metadata, structure and Unicode statistics
cli-check-help = Validate a saved document
cli-for-export-help = Also require fields needed for export
cli-spelling-help = Also check spelling with the bundled dictionaries
cli-export-help = Export into a new or empty directory
cli-output-help = Destination directory (absent or empty)
cli-pages-help = Also export page images as PNG
cli-list-help = List engine commands and queries
cli-query-help = List queries only
cli-markdown-help = Generate the CLI reference in Markdown
cli-describe-help = Describe an identifier and its JSON parameter schema
cli-check-passed = Check passed.
cli-check-failed = Check failed.
cli-dictionary-unavailable = No spelling dictionary is available for this document language.
cli-spelling-issue = Possible spelling error: { $word }
cli-exported = Export completed: { $path }
cli-new-help = Create a document from a built-in type and a supplied model
cli-type-help = Type identifier from document_types.list
cli-model-help = Model identifier (optional when the type has exactly one model)
cli-document-output-help = New .typsto destination; existing files are refused
cli-params-help = Engine parameters as a JSON object
cli-params-file-help = Read a UTF-8 JSON object from a file, or - for standard input
cli-created = Document created: { $path }
cli-params-object = Parameters must be a JSON object.
cli-params-too-large = Parameters exceed the 4 MiB limit.
cli-params-io = Cannot read the parameter file or standard input.
cli-params-conflict = A parameter is supplied both in JSON and through a command option.
cli-query-call-help = Execute a registry query on a saved document or discover global data
cli-run-help = Apply a documentary command in memory; saving must be explicit
cli-save-help = Lock and save the source document after successful execution
cli-select-help = Select this exact, case-sensitive text in the current document
cli-occurrence-help = Choose a matching occurrence, starting at 1 (required if ambiguous)
cli-block-help = Target a document block by its position, starting at 1
cli-applied = Command executed: { $id }
cli-not-saved = Preview only: no document file was saved. Use --save or --output to keep changes.
cli-session-reference = Session identifiers cannot be reused between CLI processes. Omit guards and render references; use --select or --block for targets.
cli-call-family = Use query for queries and run for commands.
cli-call-unavailable = This operation is unavailable in a single-call session. Use dedicated file commands; batches, history and recovery require a different context.
cli-target-options = Use select or block, with positive numbers; occurrence requires select.
cli-script-help = Apply a versioned JSON script as one atomic document transaction
cli-script-file-help = UTF-8 JSON script, or - for standard input
cli-script-io = Cannot read the script or its base directory.
cli-script-version = Unsupported script version. Only version 1 is supported.
cli-script-save = A new document has no current file. Choose save.kind as and a new path.
cli-script-completed =
    { $steps ->
        [one] Script completed: { $steps } step.
       *[other] Script completed: { $steps } steps.
    }
cli-mcp-read-root-help = Authorize reading under this existing directory (repeatable)
cli-mcp-write-root-help = Authorize writing under this existing directory (repeatable; does not grant reads)
mcp-root = MCP roots must be existing, accessible directories.

# Local assistance — phase 6
cmd-assistance = Agent assistance…
cmd-share-selection = Share selection with the agent
assistance-selection-shared = Selection shared with the agent. Your subsequent movements remain independent.
cli-mcp-attach-help = Connect MCP to the document authorized by an application invitation, without opening a document file.
assistance-title = Agent assistance
assistance-help = Allow a local agent to read the open document and propose edits, even if unsaved. You decide whether to accept or reject each entire lot.
assistance-read-only = Proposals change nothing until you accept. Optional permissions below apply only to the published document. No access to preferences or other documents.
assistance-save-allow = Allow saving the current file
assistance-save-first = Save the document in the application first, with write access.
assistance-save-granted = Saving authorized: { $path }
assistance-export-choose = Choose an authorized export folder…
assistance-export-granted = Authorized export folder: { $path }
assistance-export-remove = Remove this permission
assistance-authorize = Create an invitation for this document
assistance-copy = Copy MCP configuration
assistance-revoke = Revoke invitation and disconnect
assistance-ready = Agent: invitation ready
assistance-connected = Agent connected · proposals require your approval
assistance-disconnected = The agent connection has ended. Create a new invitation, then reconnect the MCP server in your assistant.
assistance-error = Cannot create the local channel: { $error }
cmd-review-proposal = Review proposal
cmd-close-review = Return to writing
cmd-accept-proposal = Accept entire lot
cmd-reject-proposal = Reject entire lot
cmd-reveal-agent-change = Show changed passage
review-title = Review proposal
review-pending = A proposal is ready to review
review-stale = Stale proposal
review-stale-help = The document has changed. The agent must prepare a new proposal.
review-busy = Finish the active input or dialogue before deciding.
review-accepted = Lot accepted · one undo step · document not saved
review-accepted-unchanged = Lot accepted · no document change
review-rejected = Lot rejected · no proposed edits published
review-count = Changed passages: { $passages } · information: { $informations }
review-agent-summary = Agent comment: { $summary }
review-content = Content
review-information = Information
review-preview = Proposed preview
review-current = Current
review-base = Base version
review-proposed = Proposed
review-added = Underlined: added or modified
review-removed = Struck through: removed or modified
review-whole-lot = Decision applies to the entire lot
review-field = Information
review-empty = Not provided
review-paper = Paper size
review-language = Document language
review-orientation = Orientation
review-marker = List marker
review-dictionary = Spelling exceptions
review-structure = Text structure
review-model = Model
review-custom-model = Custom model
review-model-changed = Presentation changed — see proposed preview
review-signature = Signature
review-handwritten = Space for a handwritten signature
review-signature-image = Signature image
review-signature-new = New signature image
review-resource = Image: { $name }
review-resource-present = Resource present
cli-usage-capture-assistance = Usage: --capture-assistance IMAGE.png

widget-list = List
widget-quote = Quotation
widget-postscript = P.S.
widget-attachments = Enclosures
widget-settings = Block settings
widget-direct-parameters = All parameters are already available in the block.
widget-delete = Delete
cmd-remove-widget = Delete entire block
cmd-contextual-action = Contextual action
contextual-correct = Correct “{ $word }”
contextual-focus-band = Return to panel: { $title }
widget-close = Close the panel
widget-image-settings = Image settings
widget-list-settings = List settings
widget-delete-title = Delete “{ $kind }”?
widget-delete-list-help = The entire list and its sublists will be deleted. You can undo this action.
widget-delete-help = The block and its annotations will be deleted. You can undo this action.
widget-caption-direct = Edit the caption directly in the block.
widget-list-scope = These settings apply to the entire list.
widget-bullets = Bulleted
widget-numbered = Numbered
widget-descriptions = Descriptions
widget-spelling = Spelling
cli-usage-capture-widgets = Usage: --capture-widgets image.png image|list|list-empty|remove|spelling|link|quote|notes|context-list|context-spelling[-dark]
cli-mcp-attach-active-help = Connect MCP to the single available authorized invitation, using a fixed configuration across connections.
assistance-fixed-configuration = Save this MCP configuration once in your assistant. For a new invitation, reconnect the server without changing its arguments.

cmd-insert-table = Insert table
cmd-insert-footnote = Insert footnote

# Interface structurée Tauri
web-new-document = New document
web-open = Open…
web-save = Save
web-save-as = Save as…
web-export = Export
web-close-document = Close document
web-file = File
web-edit = Edit
web-view = View
web-window = Window
web-cancel = Cancel
web-redo = Redo
web-preferences = Preferences
web-commands = Commands
web-palette = Command palette
web-find-replace = Find and replace
web-find = Find
web-find-command = Find a command…
web-find-command-label = Find a command
web-replace-with = Replace with
web-replace-all = Replace all
web-ignore-case =  Ignore case
web-headings = Headings
web-lists = Lists
web-letter-notes = End of letter
web-second-window = Second window
web-toggle-connection = Connect / disconnect
web-connected = Connected
web-offline = Offline
web-general = General
web-model = Model
web-document-type = Document type
web-create = Create
web-untitled = Untitled
web-today = Today
web-paper = Paper size
web-document-language = Document language
web-appearance = Appearance
web-system = System
web-light = Light
web-dark = Dark
web-serif = Serif
web-sans = Sans serif
web-mono = Monospace
web-editor-font = Editor font
web-font-size = Size
web-no-signature = No signature image
web-default-signature = Signature from preferences
web-document-signature = Document signature
web-profile-signature = Default signature
web-save-preferences = Save preferences
web-preferences-saved = Preferences saved. Existing documents keep their own data.
web-document-saved = Document saved.
web-resume = Resume session
web-keep-changes = Keep changes?
web-discard-explanation = Save the document before replacing it, or discard the changes in this session.
web-discard = Discard
web-settings = Settings
web-remove = Delete
web-close = Close
web-apply = Apply
web-close-panel = Close panel
web-close-settings = Close settings
web-paragraph = Paragraph
web-bold = Bold
web-italic = Italic
web-link = Link
web-link-address = Link address
web-alt-text = Alternative text
web-footnote-text = Footnote text
web-footnote = Footnote
web-note-badge = Note
web-remove-footnote = Delete footnote
web-image-settings = Image settings
web-block-settings = Block settings
web-list-start = Start number
web-model-controls-appearance = The model defines the presentation.
web-quote = Quote
web-descriptions = Descriptions
web-bullet-list = Bullet list
web-numbered-list = Numbered list
web-term = Term
web-definition = Definition
web-caption-placeholder = Optional caption
web-attribution-placeholder = Attribution / source (optional)
web-postscript-placeholder = Postscript text…
web-enclosure-placeholder = Enclosed document…
web-add-row = Add row
web-remove-row = Remove row
web-add-column = Add column
web-remove-column = Remove column
web-remove-table = Delete table
web-assistance = Writing assistance
web-authorize-mcp = Authorize MCP connection
web-disconnect-agent = Disconnect agent
web-current = Current
web-proposal = Proposal
web-reject-lot = Reject all changes
web-accept-lot = Accept all changes
web-lot-accepted = Changes accepted. You can undo the entire lot in one step.
web-proposal-accepted = Proposal accepted.
web-proposal-rejected = Proposal rejected.
web-proposal-stale = Stale proposal.
web-proposal-ended = Stale or completed proposal. The agent must read the document again.
web-ignore-document = Ignore in this document
web-add-personal-word = Add to personal dictionary
web-changed-word = The word has changed: reopen the correction.
web-rendering = Updating…
web-preparing = Preparing…
web-loading = Loading…
web-invalid-content = Content needs correction · last valid preview retained
web-local-edits-offline = Local changes · offline
web-preview = Typst preview
web-invalid-link = Use an http, https or mailto address.
web-invalid-list-start = Start number must be between 1 and 10,000.
web-changed-block = The block has changed.
web-remote-block-changed = This block was edited elsewhere. Close and reopen its settings.
web-reconnect-save = Reconnect this window before saving.
web-reconnect-export = Reconnect this window before exporting.
web-native-agent-only = Agent connections are available in the Tauri application.
web-agent-introduction = Connect your agent to this document. Its proposals require your approval before any changes are published.
web-fixed-configuration = Fixed configuration: reconnect the server in your tool after a new invitation.
web-agent-connected = Connection authorized. The agent can read the published document and prepare a lot of changes.
web-finish-input = Finish the current input before deciding.
web-words = words
web-characters = characters
web-selection = Selection
web-without-spaces = Without spaces
web-welcome-title = Content first.
web-welcome-description = Write naturally. Your model handles the presentation.
web-session-saved = Session saved
web-exported = PDF and Typst sources exported
web-document-body = Document body
web-column-width = Column widths

web-you = You

# Bandeaux contextuels de l’éditeur structuré
web-list-type = List type
web-remove-block = Delete block
web-remove-block-help = The entire block will be deleted. You can undo this deletion.
web-image-replacement-pending = Image replaced — apply to confirm
web-close-correction = Close correction
web-table = Table

web-confirm = Confirm
web-no-extra-settings = No additional settings to edit.
web-invalid-setting = Check the value of this field.

cli-catalogue-export-help = Export the static document catalogue without opening a document.

web-recovery = Crash recovery
web-no-recovery = No recovery copies.
web-delete-recovery = Delete copy
web-recover = Recover
web-formatting-changed = Formatting changed
web-structure-changed = Structure changed
web-settings-changed = Settings changed
web-spelling-enabled = Spell checking
web-delete-signature = Remove signature
web-find-next = Next occurrence
web-find-previous = Previous occurrence
web-clear-formatting = Clear formatting
web-unlink = Remove link
web-toggle-spelling = Toggle spell checking
web-spelling-next = Next spelling error
web-spelling-previous = Previous spelling error
web-spelling-reset = Recheck ignored words
web-document-settings = Document settings
web-embedded-model = Embedded model
web-import-model = Import a Typst model
web-model-too-large = Model is too large.
web-model-import-stale = The document has changed; import the model again.
web-list-first-number = First number
web-heading-level = Heading level

web-palette-no-results = No matching commands.

web-snapshot-document-replaced = The document has changed; start the operation again.
web-clear-date = Clear date

article-type = Article
model-working-article = Working article
article-settings-heading = Article settings:
field-article-authors = Authors
field-article-affiliations = Affiliations
field-article-localizations = Titles, abstracts and keywords
article-author = Author
article-affiliation = Affiliation
article-localization = Language entry
field-orcid = ORCID
field-corresponding = Corresponding author
field-official-affiliation = Official affiliation
field-language = Language
field-article-title = Title
field-short-title = Short title
field-subtitle = Subtitle
field-abstract = Abstract
field-keywords = Keywords
web-add = Add
web-record-edit = Edit
web-move-up = Move up
web-move-down = Move down
web-keyword-lines = One keyword per line; phrases may contain spaces.
web-detach-affiliation = Also remove links to this affiliation.
web-keep-primary-localization = Keep the primary language entry.
web-affiliation-in-use = This affiliation is used. Confirm removing its links.

group-article-titles = Titles
group-article-abstracts = Abstracts
web-language-already-present = This language is already present.
web-record-changed = This item was changed or deleted elsewhere. Close and reopen the form.
web-record-reference-removed = A selected reference was deleted. Check the links before confirming.
web-deleted-record = Deleted item
field-bibliography = Bibliography
field-reference-file = Bibliography file
field-reference-source = File contents
field-citation-style = Citation style
field-show-outline = Show table of contents
cmd-insert-citation = Cite a reference
web-citation-form = Citation form
web-reference-files = Bibliography files
web-add-reference-files = Add files
web-remove-reference-file = Remove bibliography file
web-document-changed-retry = The document changed. Please try again.
web-remove-reference = Remove
web-reference-too-large = File too large (maximum 2 MiB).
web-replace-reference = Replace
web-search-reference-placeholder = Search by title or author…
web-search-reference = Search references
web-references = References
web-standard-citation = Standard
web-prose-citation = In the sentence
web-no-reference = No matching references.
web-locator-placeholder = Pages or chapter (optional)
web-locator = Pages or chapter
web-remove-citation-item = Remove from citation
web-citation = Citation
web-bibliographic-citation = Bibliographic citation
web-chicago-author-date = Chicago author–date
web-ieee = IEEE (numeric)
web-delete-citation = Delete citation

web-table-rows = Rows

web-table-columns = Columns

web-table-row = Row

web-table-column = Column
web-table-row-actions = Row actions
web-table-column-actions = Column actions
web-table-before-row-short = Above
web-table-after-row-short = Below
web-table-before-column-short = Left
web-table-after-column-short = Right

web-table-header = First row: column headers
web-table-row-headers = First column: row headers

web-table-edit = Edit table

web-table-invalid-dimensions = Invalid table dimensions.

web-table-row-before = Insert row above

web-table-row-after = Insert row below

web-table-row-remove = Delete this row

web-table-column-before = Insert column left

web-table-column-after = Insert column right

web-table-column-remove = Delete this column

web-edit-failed = Unable to apply this change.

# Structured projects: translate causes at the adapter boundary.
storage-project-error = Unable to read or save this project: { $error }
storage-unexpected-entry = Unexpected file in the archive: { $name }.
storage-duplicate-entry = A file occurs more than once in the archive: { $name }.
storage-missing-entry = A required project file is missing: { $name }.
storage-object-integrity = Stored content does not match its fingerprint: { $name }.
storage-project-index-invalid = The project version or reference index is invalid.
storage-snapshot-limit = The project already contains the maximum of { $limit } versions. Delete a version before creating another one.
storage-unknown-snapshot = This version does not exist in the project: { $id }.
content-invalid-target = The selected position is no longer available. Select the passage again.
content-invalid-parameter = Invalid value for the parameter “{ $parameter }”.
content-command-unavailable = The command “{ $command }” cannot be applied to this selection.
content-contract-version = This document uses an unsupported structured format (version { $version }).
content-shared-state-invalid = The saved editing state is incomplete or unreadable.
content-signature-missing = The signature image is missing from the project.
content-document-invalid = The document contains invalid or unsupported content.
content-settings-invalid = The document settings are invalid.
content-document-mismatch = The saved document and its editing state do not match.
content-settings-mismatch = The saved document settings and editing state do not match.
content-source-mismatch = The saved Typst source does not match the structured document.
content-bibliography-mismatch = The bibliography files do not match the document’s references.
content-state-mismatch = The project’s saved data is inconsistent.
content-context-pending = This editing context has a rejected private draft. Review it before continuing.
content-context-selection = Select a text zone in this editing context first.
content-history-conflict = This undo or redo would remove another participant’s contribution. The project has not changed; the private draft is retained.

web-sync-suspended = Synchronization is paused; your draft is preserved.
web-unsynced-draft = Unsynced draft
web-return-confirmed = Return to confirmed version
web-export-draft = Export draft
web-discard-draft-question = Discard this draft and reload the confirmed version? Export it first if you wish to keep it.
web-discard-draft = Discard draft

content-type-unavailable = The capability “{ $command }” is not available for this document type.

# Shared command availability
web-command-no-selection = Place the cursor in an editing area.
web-command-unsupported-type = This command is unavailable for this document type.
web-command-text-required = This command requires a compatible text area.
web-command-selection-required = Select text or place the cursor in a link.
web-command-body-required = This command is only available in the document body.
web-command-top-level-required = This command is unavailable inside this widget.
web-command-caret-required = Place the cursor without selecting text.
web-command-read-only = This area is read-only.
web-command-composing = Finish the current text composition.
web-command-disconnected = Reconnect this document before applying a command.
web-command-pending-draft = Keep or discard the rejected draft before continuing.
web-command-no-document = Open or create a document.
web-command-modal = Complete or cancel the open form.
web-command-unavailable = This command is unavailable in this context.

# Optional local synchronization services
sync-enable = Enable synchronization
sync-disable = Stop synchronization
sync-enabled = Synchronization active
sync-pending = Waiting for input to finish…
sync-solo = Individual mode · validation and history active
sync-close-windows = Close the other windows before stopping synchronization.

web-private-recovery-failed = The recovery copy could not be saved. Your work remains in this window.

# Draft recovery and explicit close decisions.
web-recovery-ui-1 = Work to resume
web-recovery-ui-2 = Input to resume
web-recovery-ui-3 = Resume
web-recovery-ui-4 = Unsaved changes
web-recovery-ui-5 = This recovery copy cannot be read.
web-recovery-ui-6 = Delete this recovery copy?
web-recovery-ui-7 = Its unsaved content will no longer be recoverable. The original file will remain unchanged.
web-recovery-ui-8 = Each draft stays private until you confirm it.
web-recovery-ui-9 = All recovered input has been reviewed.
web-recovery-ui-10 = This input is kept separate from the document.
web-recovery-ui-11 = Recovered input · awaiting confirmation
web-recovery-ui-12 = Discard this input
web-recovery-ui-13 = Discard this input?
web-recovery-ui-14 = Discard input
web-recovery-ui-15 = The current document content will remain unchanged.
web-recovery-ui-16 = Resume input
web-recovery-ui-17 = View
web-recovery-ui-18 = The target has changed or is no longer available. Your input is preserved.
web-recovery-ui-19 = Recovered content
web-recovery-ui-20 = Export draft…
web-recovery-ui-21 = Unapplied text
web-recovery-ui-22 = Header field
web-recovery-ui-23 = Header item
web-recovery-ui-24 = Unfinished input
web-recovery-ui-25 = Keep your work before closing?
web-recovery-ui-26 = Save changes?
web-recovery-ui-27 = The document and unfinished input will be kept on this computer.
web-recovery-ui-28 = The .typsto file will not be changed.
web-recovery-ui-29 = This document contains unsaved changes.
web-recovery-ui-30 = Discard and close
web-recovery-ui-31 = Close without saving
web-recovery-ui-32 = Keep and close
web-recovery-ui-33 = Save and close
web-recovery-ui-34 = Retry
web-recovery-ui-35 = Another window could not preserve its work. Please try again.
web-recovery-ui-36 = Recovered document · independent copy
web-recovery-ui-37 = Open a recovery draft…
recovery-unreadable = Unreadable copy
web-recovery-progress-1 = Preserving work in the other windows…
web-recovery-progress-2 = Operation cancelled.
web-recovery-error-1 = This recovery copy is invalid.
web-recovery-error-2 = The recovery folder is unavailable.
web-recovery-error-3 = This recovery copy is no longer available.
web-recovery-error-4 = This copy belongs to a window that is still open.
web-recovery-error-5 = This recovery copy is too large.
web-recovery-error-6 = Some recovered input cannot be interpreted.
web-recovery-error-7 = Another operation is in progress. Please try again.
web-recovery-error-8 = Other windows must preserve their work before continuing.
web-recovery-error-9 = Other windows have not finished preserving their work.
web-recovery-error-10 = A window’s recovery copy could not be confirmed.
web-recovery-error-11 = This input is no longer available.
web-recovery-error-12 = recovered

web-recovery-close-scope = The other windows will close. Their pending input will be kept.

storage-version-index-changed = The version list has changed. Refresh it and try again.

web-versions-0 = Versions

web-versions-1 = Create a version

web-versions-2 = Version name

web-versions-3 = Automatic dated name

web-versions-4 = Comma-separated tags

web-versions-5 = Optional description

web-versions-6 = Version content

web-versions-7 = Document information

web-versions-8 = Embedded model

web-versions-9 = Edit version information

web-versions-10 = Close version

web-versions-11 = Version display

web-versions-12 = Content

web-versions-13 = Preview

web-versions-14 = Page

web-versions-15 = Delete this version

web-versions-16 = Delete this version?

web-versions-17 = Version information

web-versions-18 = Only this version will be deleted. The current document and other versions will be kept.

web-versions-19 = No saved versions.

web-versions-20 = Manual versions


web-versions-22 = The version limit has been reached. Delete a version to create a new one.

web-versions-23 = Versions saved.

web-versions-24 = The project file has already been chosen.

web-versions-25 = Choose where to save the project to create a version.

web-versions-26 = Expected the .typsto extension.

web-versions-28 = Yes

web-versions-29 = No

cli-versions-help = Create, relabel or delete a retained version and atomically save the project; use versions.change parameters.

web-versions-existing-path = The project file has already been chosen.

web-versions-path-required = Choose where to save the project to create a version.

web-versions-extension = Expected the .typsto extension.

web-version-creator = Created by

# Shared snapshot comparison and local human decisions.
web-comparison-compare = Compare
web-comparison-frozen-order = Frozen states · before on the left, after on the right
web-comparison-previous = Previous difference
web-comparison-next = Next difference
web-comparison-before = Before
web-comparison-after = After
web-comparison-empty = No content
web-comparison-stale-notice = The document changed elsewhere. Close this comparison and open a new one before applying the remaining changes.
web-comparison-version-before = Before · Saved version
web-comparison-current-after = After · Frozen current document
web-comparison-base-before = Before · Base state
web-comparison-proposal-after = After · Proposal
web-comparison-document = Document and linked elements
web-comparison-passage = Passage
web-comparison-pending = Awaiting decision
web-comparison-reused = Copied into the document
web-comparison-applied = Applied to the document
web-comparison-kept = Current passage kept
web-comparison-refused = Rejected
web-comparison-linked = Linked elements · one decision
web-comparison-keep = Keep the current passage
web-comparison-reuse = Use this passage from the version
web-comparison-apply = Apply this change
web-comparison-identical = No differences.
web-comparison-progress = decisions made · displayed states unchanged
web-comparison-reuse-rest = Use the remaining changes
web-comparison-apply-rest = Apply the remaining changes
web-comparison-bytes = bytes
web-comparison-absent = Absent
web-comparison-read-only-notice = Read only · two saved versions
web-comparison-closed = Proposal completed or connection revoked.
web-comparison-against = Compare with
web-comparison-current = Current document · frozen state
web-comparison-review = Review the proposal
web-comparison-files = Changed files
web-comparison-modified = Changed
web-comparison-previews = Previews of both states
web-comparison-changes = Show changes
web-comparison-loading-previews = Loading previews…
web-comparison-unavailable = This comparison or proposal is no longer available.
web-comparison-stale = The document has changed. Open a new comparison.
web-comparison-composing = Finish the current text composition before deciding.
web-comparison-read-only = Comparing two saved versions is read only.
web-comparison-use-review = This proposal is open in the comparator. Use its local decisions.
web-comparison-stale-proposal = The document changed elsewhere. Ask the agent for a new proposal.

web-composition-select = Select
web-composition-choose-passage = Choose the passage version
web-composition-select-before = Select the red version
web-composition-select-after = Select the green version
web-composition-choices-applied = Choices applied
web-composition-choice-draft = Provisional choice
web-composition-applied = Choices applied. You can save the document.
web-composition-draft-notice = Provisional choices · nothing changes until confirmation.
web-composition-select-all = Select all
web-composition-select-all-before = Select everything on the left
web-composition-select-all-after = Select everything on the right
web-comparison-review-composition = Compare the composition with the current document before applying it.
web-comparison-context-1 = Keep current
web-comparison-context-2 = Use all
web-comparison-context-3 = Keep all
web-comparison-context-4 = Use this passage
web-comparison-context-5 = Keep the current passage
web-comparison-context-6 = Use everything from this version
web-comparison-context-7 = Keep everything from the current document
web-comparison-context-8 = Reject
web-comparison-context-9 = Accept
web-comparison-context-10 = Reject all
web-comparison-context-11 = Accept all
web-comparison-context-12 = Reject this change
web-comparison-context-13 = Accept this change
web-comparison-context-14 = Choose this version
web-comparison-context-15 = Choose everything here
web-comparison-context-16 = Choose from “{ "{name}" }”
web-comparison-context-17 = Choose everything from “{ "{name}" }”
web-comparison-context-18 = Before · Frozen current document
web-comparison-context-19 = After · Prepared composition
web-comparison-context-20 = Use this passage from the composition
web-comparison-context-21 = Use the entire composition
web-comparison-context-22 = Compare the composition with the current document
web-comparison-context-23 = The composition stays private until it is compared with the current document.
web-comparison-context-24 = Use { "{count}" } passage
web-comparison-context-25 = Use { "{count}" } passages
web-comparison-context-26 = No passages to use
web-comparison-context-27 = { "{count}" } passage to use
web-comparison-context-28 = { "{count}" } passages to use
web-comparison-context-29 = { "{count}" } passage kept
web-comparison-context-30 = { "{count}" } passages kept
web-comparison-context-31 = { "{count}" } change accepted
web-comparison-context-32 = { "{count}" } changes accepted
web-comparison-context-33 = { "{count}" } change rejected
web-comparison-context-34 = { "{count}" } changes rejected
web-comparison-context-35 = Apply { "{count}" } change
web-comparison-context-36 = Apply { "{count}" } changes
web-comparison-context-37 = Reject the proposal
web-comparison-context-38 = Finish without changes
web-comparison-context-39 = Keep the current document
web-comparison-context-40 = Review the composition before changing the current document.
web-comparison-context-41 = Edit the composition
web-comparison-context-42 = Review the composition
web-comparison-context-43 = The proposal was rejected. The document is unchanged.
web-comparison-context-44 = The proposal was completed without changes.
web-comparison-context-45 = The current document was kept.
