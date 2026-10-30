# CLI structured contract

The CLI uses the same Yrs owner, typed targets, document commands and Unicode zones as Tauri.
`commands describe ID` is authoritative for parameter schemas. Offsets are UTF-16 inside a zone,
paths are projected child indices in the current revision, and inserted marks are explicit.

`new --type article --output article.typsto` creates a self-contained project without personal preferences.
`inspect`, `check`, `query`, `run`, `script`, `export` and `render` operate on structured `.typsto` files.
`run` previews changes unless `--save` or `--output` is given. A writable open locks before reading;
read-only copies may inspect the last saved snapshot while another writer owns the project.

`--select TEXT [--occurrence N]` resolves an exact match inside one zone. `--block N` addresses a
one-based top-level body block. Ambiguity, invalid Unicode boundaries and foreign fields are refused.
Scripts resolve selectors on the evolving isolated draft, validate only the final batch, and publish
one undoable change. File operations are forbidden as steps; optional final save is explicit.
Resource data is base64 in `resource.add`; models use their pure Typst source in `document.set_model`.

| ID | Family | Description |
| --- | --- | --- |
| `bibliography.search` | query | Search the project bibliography by title and author. |
| `block.heading_1` | edit | Level-one heading; final presentation belongs to the Typst model. |
| `block.heading_2` | edit | Level-two heading. |
| `block.heading_3` | edit | Level-three heading. |
| `block.insert` | edit | Edit a guarded tree with explicit nodes (quote, list, figure, postscript…). Discover examples via capabilities.describe. |
| `block.join` | edit | Join explicit adjacent left/right paragraphs or headings in the same container. Two nonempty paragraphs retain a single soft break at the seam (a Typst space); an existing break is reused. Joins involving headings concatenate directly. The nonempty left block keeps its kind and attributes. Remove an empty neighbor without converting its survivor. Widgets, cells and annotation boundaries are protected; the mandatory final editing slot is a no-op. |
| `block.paragraph` | edit | Ordinary prose. A soft return stays inside the paragraph; two returns create a paragraph. |
| `block.remove` | edit | Typed structured document operation; discover targets and parameters in params_schema. |
| `block.replace` | edit | Edit a guarded tree with explicit nodes (quote, list, figure, postscript…). Discover examples via capabilities.describe. |
| `block.splice` | edit | Edit a guarded tree with explicit nodes (quote, list, figure, postscript…). Discover examples via capabilities.describe. |
| `block.split` | edit | Typed structured document operation; discover targets and parameters in params_schema. |
| `capabilities.describe` | query | Describe an operation with its actual parameter schema. |
| `commands.available` | query | Check a concrete command through the same resolver and final validation as execution. |
| `commands.list` | query | Discover the common structured command and query catalogue. |
| `context.get` | query | Read this participant's private selection, typing styles, history and selection-level capability availability. Form parameters and adapter permissions still require execution validation. |
| `context.insert` | session | Insert literal prose at this participant's single-zone selection using its typing styles. For explicit edits use text.insert with target and marks. |
| `context.set` | session | Atomically set private directed selection and typing styles; never edit content, move another participant's focus or create history. |
| `context.style` | session | Switch a private typing style on/off; stopping it does not alter already written text. |
| `document.batch` | session | Apply typed documentary steps atomically, with one history entry and optional final save. A late failure publishes and saves nothing. |
| `document.close` | session | Close the project; unsaved work requires explicit discard_changes. |
| `document.export` | session | Export a confirmed snapshot to a new/empty directory; never mark the project saved. |
| `document.inspect` | query | Read the confirmed structured state, guard and project information. |
| `document.new` | session | Create a structured project of a built-in type; no personal preferences are implied. |
| `document.open` | session | Open a self-contained structured .typsto; writable access acquires a cooperative lock before reading. |
| `document.save` | session | Save the current project under its existing lock, checking external changes. |
| `document.save_as` | session | Save an independent copy; existing targets are refused. |
| `document.set_model` | edit | Set the model ID and full pure Typst source together. The model must implement this document type; publication validates its policy. |
| `document.set_signature` | edit | Typed structured document operation; discover targets and parameters in params_schema. |
| `document.source` | query | Read generated Typst without changing the source of truth. |
| `document.statistics` | query | Count Unicode words and signs in the body and optional target. |
| `document.structure` | query | Read trees and all typed text zones; paths are revision-relative, offsets UTF-16. |
| `document.text` | query | Read plain text of the body or a typed text target. |
| `document.validate` | query | Validate the structured draft and report export readiness separately. |
| `document_types.describe` | query | Describe metadata, rich header collections and capabilities of one type. |
| `document_types.list` | query | List built-in document types and their interchangeable visual models. |
| `edit.redo` | session | Redo this participant's contribution. |
| `edit.undo` | session | Undo this participant's last confirmed contribution. |
| `inline.replace` | edit | Replace a typed inline range with one node (for example footnote or citation). Discover the node via capabilities.describe. |
| `insert.table` | edit | Insert a table figure. header marks the first row as column headers; row_headers marks the first column as row headers. They are independent, including a one-cell table. No merged cells. |
| `metadata.get` | query | Read document metadata and field/collection definitions. |
| `metadata.set` | edit | Set a typed metadata, record field or document option. Values are validated against the type and model policy. |
| `models.list` | query | Read brief visual model cards, including the active embedded model. |
| `node.set_attrs` | edit | Typed structured document operation; discover targets and parameters in params_schema. |
| `record.add` | edit | Edit a header collection using stable record IDs. Forward references inside an atomic lot are checked only on its final state. |
| `record.move` | edit | Edit a header collection using stable record IDs. Forward references inside an atomic lot are checked only on its final state. |
| `record.remove` | edit | Edit a header collection using stable record IDs. Forward references inside an atomic lot are checked only on its final state. |
| `render.compile` | query | Compile confirmed content with embedded Typst and return page dimensions. |
| `render.locate` | query | Map a typed body target to preview or a zero-based PagePoint back to a body zone. Template decorations may have no target. |
| `render.page` | query | Render one one-based page to PNG without writing files. |
| `render.selection_boxes` | query | Read rendered highlight rectangles for a typed body range. Header/annotation geometry is unavailable. |
| `resource.add` | edit | Admit immutable image bytes (base64), validated before use; a batch may add the resource then insert its figure or signature. |
| `resources.list` | query | List admitted immutable image resources; no filesystem access. |
| `search.find` | query | Find exact text inside each zone, including rich header fields; no cross-zone matches. |
| `spelling.ignore` | edit | Typed structured document operation; discover targets and parameters in params_schema. |
| `spelling.issues` | query | Read local spelling issues as guarded typed ranges; never correct text. |
| `spelling.suggestions` | query | Get local suggestions for a literal word and language. |
| `table.column.insert` | edit | Typed structured document operation; discover targets and parameters in params_schema. |
| `table.column.remove` | edit | Typed structured document operation; discover targets and parameters in params_schema. |
| `table.header` | edit | Set a header axis on an existing tableFigure: axis columns (default) labels columns using the first row; axis rows labels rows using the first column. Text and other cells are preserved. Leading-track insertions and removals retain the chosen roles. |
| `table.row.insert` | edit | Typed structured document operation; discover targets and parameters in params_schema. |
| `table.row.remove` | edit | Typed structured document operation; discover targets and parameters in params_schema. |
| `text.bold` | edit | Apply or remove this style on a nonempty typed range (value on/off). Never changes future typing; use context.style for that. |
| `text.clear` | edit | Typed structured document operation; discover targets and parameters in params_schema. |
| `text.delete` | edit | Typed structured document operation; discover targets and parameters in params_schema. |
| `text.insert` | edit | Insert/replace literal prose in one typed range; marks are explicit and default to none. No Typst/Markdown input rules. |
| `text.italic` | edit | Apply or remove this style on a nonempty typed range (value on/off). Never changes future typing; use context.style for that. |
| `text.link` | edit | Editable label and http, https or mailto URL. Never fetch the URL. |
| `text.remove_link` | edit | Typed structured document operation; discover targets and parameters in params_schema. |
