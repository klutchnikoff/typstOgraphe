# Interface typstOgraphe. Ne pas traduire les données des courriers.

save-dialog = Save document
letter-filter = typstOgraphe document
default-filename = Document.typsto
replace-title = Replace the document?
letter-opened = Document opened.
open-dialog = Open document
all-files = All files
select-link-remove = Select the linked text to remove its link.
select-link = Select the text to link first.
link-one-paragraph = A link must stay within a single paragraph.
renderer-stopped = The rendering engine has stopped.
multiple-styles = Mixed styles
preview-title = Typst preview
preview-hint = Double-click a passage to find it in the editor.
preview-updating = Updating…
preview-failed = The document could not be typeset. The last successful preview is still displayed.
error-details = Details
preview-composing = Typesetting the document…
group-correspondence = Correspondence
field-recipient = Recipient
field-address = Address
field-subject = Subject
field-reference = Reference
field-place = Place
field-date = Date
group-sender = Sender
field-full-name = Full name
field-role = Role
field-service = Department
field-email = Email
link-title = Link
apply = Apply
link-invalid-url = Use an https://, http:// or mailto: address.
command-search = Search for a command…
export-title = Export document
export-folder = Destination folder
export-help = PDF, PNG pages and separate Typst files. Use Save to keep an editable document in a single file.
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
source-syntax-error = The Typst source contains a syntax error.
source-unreadable = Unreadable Typst document
source-heading-depth = Only heading levels 1–3 are supported.
source-computed-call = Computed function calls are not supported.
source-nested-blocks = Nested blocks are not supported in this prototype.
storage-file-too-large = The letter file exceeds the size limit.
storage-file-count = the archive must contain the five letter files and at most 1,019 images
storage-unknown-entry = unknown or duplicate file in the archive
storage-invalid-size = invalid archive size
storage-expanded-too-large = the expanded content exceeds 64 MiB
storage-unsupported = unsupported format, version or document type
storage-main-mismatch = the main.typ header does not match the letter metadata
storage-invalid-template = the Typst template is empty or contains a syntax error
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
cmd-institutional = Use the institutional letter template
cmd-institutional-short = Institutional letter
cmd-service = Use the department letter template
cmd-service-short = Department letter
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
source-unsupported-node = Construct outside the editable subset: { $kind }
source-unsupported-function = Unsupported function or signature: { $name }
storage-open-failed = Could not open the document: { $error }
storage-invalid = Invalid document: { $error }
storage-zip-unreadable = unreadable ZIP archive ({ $error })
storage-manifest-unreadable = unreadable manifest ({ $error })
storage-metadata-unreadable = unreadable metadata ({ $error })
storage-entry-missing = { $name } is missing or unreadable ({ $error })
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
cli-usage-render = Usage: --render-demo FOLDER
cli-usage-capture = Usage: --capture IMAGE.png
cli-usage-capture-recovery = Usage: --capture-recovery IMAGE.png RECOVERY_FOLDER
cli-usage-capture-search = Usage: --capture-search IMAGE.png QUERY
cli-unknown-option = Unknown option: { $option }
cli-capture-error = Screenshot: { $error }
cli-help =
    typstOgraphe
      cargo run
      cargo run -- LETTER.typsto
      cargo run -- --render-demo FOLDER
      cargo run -- --capture IMAGE.png
      cargo run -- --capture-recovery IMAGE.png RECOVERY_FOLDER
      cargo run -- --capture-search IMAGE.png QUERY
      cargo run -- --capture-spelling IMAGE.png
      cargo run -- --capture-profile IMAGE.png
      cargo run -- --capture-profile-letter IMAGE.png
preview-stats =
    { $pages ->
        [one] { $pages } page · { $milliseconds } ms
       *[other] { $pages } pages · { $milliseconds } ms
    }
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
cli-render-result =
    { $pages ->
        [one] { $pages } page
       *[other] { $pages } pages
    }, { $milliseconds } ms, export: { $path }

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
bullet-marker-invalid = Enter a single visible character.
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
field-sender-address = Sender’s address
field-phone = Phone
profile-help = These details are suggested when creating documents. Each document keeps its own copy, editable independently of the profile.
signature-label = Signature
signature-choose = Choose an image…
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
description-placeholder = Description

cmd-quote = Block quote
quote-attribution-placeholder = Attribution / source (optional)
source-quote-signature = A quote must be a text block, with an optional single-paragraph attribution.

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
source-image-signature = This figure is outside the editable subset: an image, alternative text and an optional caption are expected.

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
cmd-minutes-standard = Standard meeting minutes template
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
search-one-line = Replacement must be a single line, without structural separators.
search-too-large = This search is too large. Try a shorter text.
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
recovery-worker-stopped = The recovery service has stopped.
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
