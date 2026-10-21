# Interface typstOgraphe. Ne pas traduire les données des courriers.

save-dialog = Save letter
letter-filter = typstOgraphe letter
default-filename = Letter.typsto
replace-title = Replace the letter?
letter-opened = Letter opened.
open-dialog = Open letter
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
preview-composing = Typesetting the letter…
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
export-title = Export letter
export-folder = Destination folder
export-help = PDF, PNG pages and separate Typst files. Use Save to keep an editable letter in a single file.
export-button = Export
export-required-fields = Fill in the recipient and subject before exporting.
export-pending = Exporting…
unsaved-title = Save changes?
unsaved-message = The letter has unsaved changes.
save = Save
discard = Don’t save
cancel = Cancel
file-error-title = Letter file
close = Close
untitled-letter = Untitled letter
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
cmd-new = New letter
cmd-open = Open letter…
cmd-save = Save
cmd-save-as = Save as…
cmd-export = Export letter…
cmd-export-short = Export
cmd-information = Letter information
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
letter-saved = Letter saved: { $path }
replace-message = The file { $path } already exists. Do you want to replace it?
letter-exported = Letter exported to { $path }
mixed-command = { $command } (mixed selection)
link-selected-text = Text: { $text }
command-shortcut = { $label } · { $shortcut }
window-title = { $name }{ $marker }
source-unsupported-node = Construct outside the editable subset: { $kind }
source-unsupported-function = Unsupported function or signature: { $name }
storage-open-failed = Could not open the letter: { $error }
storage-invalid = Invalid letter: { $error }
storage-zip-unreadable = unreadable ZIP archive ({ $error })
storage-manifest-unreadable = unreadable manifest ({ $error })
storage-metadata-unreadable = unreadable metadata ({ $error })
storage-entry-missing = { $name } is missing or unreadable ({ $error })
storage-entry-unreadable = { $name } is unreadable ({ $error })
storage-entry-too-large = { $name } exceeds the size limit
storage-temporary-failed = Could not create a temporary file: { $error }
storage-write-failed = Could not write the letter: { $error }
storage-replace-failed = Could not replace the letter: { $error }
export-write-failed = Could not write { $name }: { $error }
shortcut-unknown-key = Unknown key: { $key }
shortcut-unknown-command = Unknown command: { $id }
shortcut-duplicate = Shortcut defined more than once for { $command }
shortcut-conflict = Shortcut conflict: { $first } and { $second }
cli-usage-render = Usage: --render-demo FOLDER
cli-usage-capture = Usage: --capture IMAGE.png
cli-unknown-option = Unknown option: { $option }
cli-capture-error = Screenshot: { $error }
cli-help =
    typstOgraphe
      cargo run
      cargo run -- LETTER.typsto
      cargo run -- --render-demo FOLDER
      cargo run -- --capture IMAGE.png
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
