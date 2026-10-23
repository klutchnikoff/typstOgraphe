# Catalogue documentaire implémenté

Fichier généré par `cargo run --locked -- --commands-catalogue`.
Ce catalogue de phase 2 est partiel et ses signatures restent en cours de validation.
Les actions d’interface et les familles non migrées n’y figurent pas.
Les schémas JSON sont disponibles avec `--commands-catalogue-json`.

| Identifiant | Famille | Description |
| --- | --- | --- |
| `metadata.set` | edit | Set an active document-type field, with a guarded value and optional typing history grouping. |
| `document.set_option` | edit | Set a typed document option while enforcing the model page and marker policy. |
| `spelling.next` | selection | Select the next spelling issue in the current document, wrapping around. |
| `spelling.previous` | selection | Select the previous spelling issue in the current document, wrapping around. |
| `spelling.correct` | edit | Correct a revision-guarded spelling issue while preserving inline styles. |
| `spelling.ignore` | edit | Accept a validated word for the document language in this document only. |
| `spelling.reset` | edit | Clear spelling exceptions for all languages of the guarded document. |
| `edit.find_next` | selection | Select the next literal match, wrapping around the editable document. |
| `edit.find_previous` | selection | Select the previous literal match, wrapping around the editable document. |
| `edit.replace_current` | edit | Replace a revision-guarded occurrence with expected text, then select the next match. |
| `edit.replace_all` | edit | Replace all literal matches from one guarded document revision in one transaction. |
| `text.insert` | edit | Insert text or replace a selection, with literal input by default and optional typing rules. |
| `text.delete` | edit | Delete text within validated structural boundaries. |
| `text.select` | selection | Select text or move to a body, annotation, description term or definition. |
| `text.bold` | edit | Set or toggle bold text or the typing style. |
| `text.italic` | edit | Set or toggle italic text or the typing style. |
| `text.clear` | edit | Remove inline formatting from text or the typing style. |
| `text.link` | edit | Apply an explicit URL to a text selection. |
| `text.remove_link` | edit | Remove links from a text selection. |
| `block.paragraph` | edit | Convert selected paragraphs to ordinary text. |
| `block.heading_1` | edit | Set level-one headings. |
| `block.heading_2` | edit | Set level-two headings. |
| `block.heading_3` | edit | Set level-three headings. |
| `block.bullet` | edit | Set an unordered list using the document marker policy. |
| `block.numbered` | edit | Set an ordered list. |
| `block.description` | edit | Set description items with term and definition zones. |
| `block.quote` | edit | Set a block quotation with an optional attribution. |
| `insert.image` | image | Insert a validated image with optional literal caption and alternative text. |
| `image.replace` | image | Replace an image resource while preserving its caption and alternative text. |
| `image.remove` | image | Remove an image block and its caption in one transaction. |
| `image.set_alt` | image | Set alternative text on a revision-guarded image block. |
| `document.set_signature` | letter | Set or remove the letter signature using explicit image data or a file. |
| `block.split` | edit | Split a quote, list item or letter note, or exit an empty item or optional annotation. |
| `block.exit` | edit | Create a paragraph after an annotated structure or letter note without replacing its content. |
| `letter.insert_postscript` | letter | Create or select the letter postscript. |
| `letter.insert_attachment` | letter | Insert an attachment mention into a letter. |
| `edit.undo` | history | Undo the previous document transaction. |
| `edit.redo` | history | Redo the next document transaction. |
| `models.presentation` | query | Read the current model page constraints and permitted bullet markers. |
| `spelling.issues` | query | Read revision-guarded spelling issues and dictionary availability using the configured personal words. |
| `spelling.suggestions` | query | Suggest corrections for an explicit word and language or a validated spelling issue, without changing selection. |
| `document.inspect` | query | Read the current revision, selection and history availability. |
| `document.text` | query | Read editable text without Typst markup. |
| `document.structure` | query | Read blocks, text zones, roles and their opaque identities. |
| `metadata.get` | query | Read the document metadata. |
| `commands.list` | query | Read the implemented command and query catalogue with JSON schemas. |
| `search.find` | query | Find literal Unicode text without modifying the selection. |
