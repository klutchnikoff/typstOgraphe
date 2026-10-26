# Catalogue documentaire implémenté

Fichier généré par `cargo run --locked -p typstographe-egui -- --commands-catalogue`.
Identifiants et contrats d’entrée stabilisés à la clôture de la phase 2, le 24 octobre 2026.
Tout changement incompatible exige une décision explicite ; les ajouts compatibles restent possibles.
Les actions propres à l’interface ne figurent pas dans ce catalogue documentaire.
Les schémas JSON sont disponibles avec `--commands-catalogue-json`.
Les réponses complémentaires sont décrites dans [le contrat des lectures](engine-queries.md).

| Identifiant | Famille | Étape de lot | Description |
| --- | --- | --- | --- |
| `document.batch` | batch | non | Apply bounded editing steps on a private session as one undoable transaction, with an optional single final atomic save. |
| `recovery.restore` | file | non | Restore a configured recovery reference as an unsaved copy, protecting current work. |
| `recovery.delete` | file | non | Explicitly delete one configured recovery copy without modifying document history. |
| `document.new` | file | non | Create an empty document from a supplied type/model and explicit initial values, protecting unsaved work. |
| `document.open` | file | non | Open an autonomous archive from an explicit path, protecting unsaved work. |
| `document.close` | file | non | Close the current document after an explicit decision about unsaved work. |
| `document.save` | file | non | Atomically save the guarded document to an explicit or current path without changing editing history. |
| `document.save_as` | file | non | Atomically save the guarded document to a required path, preserving the previous file. |
| `document.export` | file | non | Export PDF, autonomous Typst sources or both into a new or empty directory, with optional page PNGs. |
| `document.set_model` | edit | oui | Validate and install Typst model source or a UTF-8 file with controlled dependencies in one guarded transaction. |
| `courrier.model.personal` | letter | oui | Apply the supplied personal-letter model using the shared guarded model operation. |
| `metadata.set` | edit | oui | Set an active document-type field, with a guarded value and optional typing history grouping. |
| `document.set_option` | edit | oui | Set a typed document option while enforcing the model page and marker policy. |
| `spelling.next` | selection | oui | Select the next spelling issue in the current document, wrapping around. |
| `spelling.previous` | selection | oui | Select the previous spelling issue in the current document, wrapping around. |
| `spelling.correct` | edit | oui | Correct a revision-guarded spelling issue while preserving inline styles. |
| `spelling.ignore` | edit | oui | Accept a validated word for the document language in this document only. |
| `spelling.reset` | edit | oui | Clear spelling exceptions for all languages of the guarded document. |
| `edit.find_next` | selection | oui | Select the next literal match, wrapping around the editable document. |
| `edit.find_previous` | selection | oui | Select the previous literal match, wrapping around the editable document. |
| `edit.replace_current` | edit | oui | Replace a revision-guarded occurrence with expected text, then select the next match. |
| `edit.replace_all` | edit | oui | Replace all literal matches from one guarded document revision in one transaction. |
| `text.insert` | edit | oui | Insert text or replace a selection, with literal input by default and optional typing rules. |
| `text.delete` | edit | oui | Delete text within validated structural boundaries. |
| `text.select` | selection | oui | Select text or move to a body, annotation, description term or definition. |
| `text.bold` | edit | oui | Set or toggle bold text or the typing style. |
| `text.italic` | edit | oui | Set or toggle italic text or the typing style. |
| `text.clear` | edit | oui | Remove inline formatting from text or the typing style. |
| `text.link` | edit | oui | Apply an explicit URL to a text selection. |
| `text.remove_link` | edit | oui | Remove links from a text selection. |
| `block.paragraph` | edit | oui | Convert selected paragraphs to ordinary text. |
| `block.heading_1` | edit | oui | Set level-one headings. |
| `block.heading_2` | edit | oui | Set level-two headings. |
| `block.heading_3` | edit | oui | Set level-three headings. |
| `block.bullet` | edit | oui | Set an unordered list using the document marker policy. |
| `block.numbered` | edit | oui | Set an ordered list. |
| `block.description` | edit | oui | Set description items with term and definition zones. |
| `block.quote` | edit | oui | Set a block quotation with an optional attribution. |
| `insert.image` | image | oui | Insert a validated image with optional literal caption and alternative text. |
| `image.replace` | image | oui | Replace an image resource while preserving its caption and alternative text. |
| `image.remove` | image | oui | Remove an image block and its caption in one transaction. |
| `image.set_alt` | image | oui | Set alternative text on a revision-guarded image block. |
| `document.set_signature` | letter | oui | Set or remove the letter signature using explicit image data or a file. |
| `block.split` | edit | oui | Split a quote, list item or letter note, or exit an empty item or optional annotation. |
| `block.exit` | edit | oui | Create a paragraph after an annotated structure or letter note without replacing its content. |
| `letter.insert_postscript` | letter | oui | Create or select the letter postscript. |
| `letter.insert_attachment` | letter | oui | Insert an attachment mention into a letter. |
| `edit.undo` | history | non | Undo the previous document transaction. |
| `edit.redo` | history | non | Redo the next document transaction. |
| `document.validate` | query | non | Validate a guarded draft, model, resources and options; report errors separately from fields missing for export, without editing or installing a render. |
| `render.compile` | query | non | Compile a guarded document synchronously into a cached in-memory render reference and diagnostics, without rasterization, export or document edits. |
| `document_types.list` | query | non | List built-in types, supplied models, creation availability and documentary command capabilities without requiring an open document. |
| `document.source` | query | non | Read the committed documentary Typst source with its revision guard, without executing code. |
| `document.assets` | query | non | Inventory document/model resources retained by the save/export policy and packaged dependencies, without returning file contents or traversing undo history. |
| `render.locate` | query | non | Locate a logical text target in an identified render or return the guarded text target at a page point, without moving selection. |
| `render.selection_boxes` | query | non | Read page quadrilaterals for a guarded text selection on an identified render, in Typst points. |
| `document.statistics` | query | non | Read Unicode word and grapheme counts for the document and an optional text target, excluding structural markers and private fields. |
| `recovery.list` | query | non | List opaque references from the configured recovery service without opening documents. |
| `models.list` | query | non | List supplied models for the active document type without changing the document. |
| `models.presentation` | query | non | Read the current model page constraints and permitted bullet markers. |
| `spelling.issues` | query | non | Read revision-guarded spelling issues and dictionary availability using the configured personal words. |
| `spelling.suggestions` | query | non | Suggest corrections for an explicit word and language or a validated spelling issue, without changing selection. |
| `document.inspect` | query | non | Read session state, document type, documentary command capabilities, guarded selection and typing style; document context is null when closed. |
| `document.text` | query | non | Read editable text without Typst markup. |
| `document.structure` | query | non | Read blocks, text zones, roles and their opaque identities. |
| `metadata.get` | query | non | Read the document metadata. |
| `commands.list` | query | non | Read command/query schemas and context availability with reasons, distinguishing calls that still require parameters; no execution or file access. |
| `search.find` | query | non | Find literal Unicode text without modifying the selection. |
