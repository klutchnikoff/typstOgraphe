# CLI typstOgraphe

Document généré par `cargo run --locked -p typstographe-cli -- commands list --markdown`.

Le premier lot permet de consulter le catalogue, inspecter, vérifier et exporter un fichier `.typsto`, sans fenêtre ni préférences personnelles implicites.

```sh
typstographe-cli commands list --json
typstographe-cli commands describe metadata.set --json
typstographe-cli inspect courrier.typsto --json
typstographe-cli check courrier.typsto --for-export
typstographe-cli check courrier.typsto --spelling
typstographe-cli export courrier.typsto --format pdf --output export/
```

Les lectures utilisent la dernière version enregistrée, même si le document est ouvert dans l’application. L’export crée un répertoire ou utilise un répertoire vide, avec `--pages` pour ajouter des images PNG.

`--lang fr|en` choisit la langue d’interface ; sinon langue système, repli anglais. La langue du document est indépendante. `--json` produit un objet `{"version":1,"ok":true,"outcome":…}` ou `{"version":1,"ok":false,"error":{"code","message","params"}}`. Aucun autre texte sur stdout en mode JSON.

Codes de sortie : 0 succès, 1 contrôle négatif ou opération refusée, 2 arguments invalides, 3 entrée/sortie. Un contrôle négatif a `ok: true` (exécution réussie), `outcome.passed: false` et le code de sortie 1. L’orthographe est facultative ; avec `--spelling`, un dictionnaire indisponible ou une suspicion donne un contrôle négatif.

Création, `query`, `run`, scripts atomiques et images de pages dédiées : lots suivants. Les identités de session ne sont pas des références persistantes entre deux exécutions. MCP est prévu en phase 5.

## Catalogue du moteur

Le catalogue décrit les contrats du moteur, y compris les opérations qui ne disposent pas encore d’un déclencheur CLI. L’ouverture accepte l’option additive `read_only` (false par défaut).

| Identifiant | Famille | Description |
| --- | --- | --- |
| `document.batch` | batch | Apply bounded editing steps on a private session as one undoable transaction, with an optional single final atomic save. |
| `recovery.restore` | file | Restore a configured recovery reference as an unsaved copy, protecting current work. |
| `recovery.delete` | file | Explicitly delete one configured recovery copy without modifying document history. |
| `document.new` | file | Create an empty document from a supplied type/model and explicit initial values, protecting unsaved work. |
| `document.open` | file | Open an autonomous archive from an explicit path, protecting unsaved work. |
| `document.close` | file | Close the current document after an explicit decision about unsaved work. |
| `document.save` | file | Atomically save the guarded document to an explicit or current path without changing editing history. |
| `document.save_as` | file | Atomically save the guarded document to a required path, preserving the previous file. |
| `document.export` | file | Export PDF, autonomous Typst sources or both into a new or empty directory, with optional page PNGs. |
| `document.set_model` | edit | Validate and install Typst model source or a UTF-8 file with controlled dependencies in one guarded transaction. |
| `courrier.model.personal` | letter | Apply the supplied personal-letter model using the shared guarded model operation. |
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
| `document.validate` | query | Validate a guarded draft, model, resources and options; report errors separately from fields missing for export, without editing or installing a render. |
| `render.compile` | query | Compile a guarded document synchronously into a cached in-memory render reference and diagnostics, without rasterization, export or document edits. |
| `document_types.list` | query | List built-in types, supplied models, creation availability and documentary command capabilities without requiring an open document. |
| `document.source` | query | Read the committed documentary Typst source with its revision guard, without executing code. |
| `document.assets` | query | Inventory document/model resources retained by the save/export policy and packaged dependencies, without returning file contents or traversing undo history. |
| `render.locate` | query | Locate a logical text target in an identified render or return the guarded text target at a page point, without moving selection. |
| `render.selection_boxes` | query | Read page quadrilaterals for a guarded text selection on an identified render, in Typst points. |
| `document.statistics` | query | Read Unicode word and grapheme counts for the document and an optional text target, excluding structural markers and private fields. |
| `recovery.list` | query | List opaque references from the configured recovery service without opening documents. |
| `models.list` | query | List supplied models for the active document type without changing the document. |
| `models.presentation` | query | Read the current model page constraints and permitted bullet markers. |
| `spelling.issues` | query | Read revision-guarded spelling issues and dictionary availability using the configured personal words. |
| `spelling.suggestions` | query | Suggest corrections for an explicit word and language or a validated spelling issue, without changing selection. |
| `document.inspect` | query | Read session state, document type, documentary command capabilities, guarded selection and typing style; document context is null when closed. |
| `document.text` | query | Read editable text without Typst markup. |
| `document.structure` | query | Read blocks, text zones, roles and their opaque identities. |
| `metadata.get` | query | Read the document metadata. |
| `commands.list` | query | Read command/query schemas and context availability with reasons, distinguishing calls that still require parameters; no execution or file access. |
| `search.find` | query | Find literal Unicode text without modifying the selection. |
