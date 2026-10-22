# Catalogue documentaire implémenté

Fichier généré par `cargo run --locked -- --commands-catalogue`.
Ce catalogue de phase 2 est partiel et ses signatures restent en cours de validation.
Les actions d’interface et les familles non migrées n’y figurent pas.
Les schémas JSON sont disponibles avec `--commands-catalogue-json`.

| Identifiant | Famille | Description |
| --- | --- | --- |
| `text.insert` | edit | Insert literal text or replace a selection. |
| `text.delete` | edit | Delete text within validated structural boundaries. |
| `text.select` | selection | Set a continuous, oriented text selection. |
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
| `letter.insert_postscript` | letter | Create or select the letter postscript. |
| `letter.insert_attachment` | letter | Insert an attachment mention into a letter. |
| `edit.undo` | history | Undo the previous document transaction. |
| `edit.redo` | history | Redo the next document transaction. |
| `document.inspect` | query | Read the current revision, selection and history availability. |
| `document.text` | query | Read editable text without Typst markup. |
| `document.structure` | query | Read blocks, text zones, roles and their opaque identities. |
| `metadata.get` | query | Read the document metadata. |
| `commands.list` | query | Read the implemented command and query catalogue with JSON schemas. |
| `search.find` | query | Find literal Unicode text without modifying the selection. |
