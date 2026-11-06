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
| `bibliography.apply` | edit | Publish one guarded native-library change in memory, never save. expected is the library fingerprint from bibliography.inspect. Edit, replace, rename and remove also require the record fingerprint from bibliography.record. Edit profile changes native type/parents; no independent genre is stored. Replace accepts a prepared native record while preserving identity/provenance, including retained inactive parents. Import requires the reviewed preparation fingerprint and explicit same/distinct/skip decisions; unresolved plans are refused. Use one atomic batch with inline.replace for create-and-cite. Undo restores notices, citations and provenance together. |
| `bibliography.browse` | query | Search all project notices by title/authors with compact summaries, derived profile and completeness. No 80-item suggestion limit. |
| `bibliography.form` | query | Project native record fields into simple editable controls, preserving unexposed metadata. The supplied record is a frozen draft base, never a write instruction. |
| `bibliography.inspect` | query | Read active native records with stable identities, editable keys and library fingerprint, plus immutable source summaries. Article-only. Use bibliography.apply for guarded, undoable changes; create-and-cite uses one atomic batch with inline.replace. Imports require the reviewed preparation fingerprint and explicit conflict decisions; no direct file write occurs. |
| `bibliography.prepare_form` | query | Validate changed form controls against a frozen native record, then optionally change its native classification with profile. Returns the proposed native record and its derived editing profile, preserving inactive publication containers. Does not publish. Use bibliography.apply (replace) with the original record fingerprint for a prepared form, or create for a new notice; a sequence of private form changes still publishes one undoable edit. |
| `bibliography.prepare_import` | query | Prepare an in-memory BibLaTeX/Hayagriva import with stable proposal identities, source digest and conversion diagnostics. UTF-8 contents are supplied explicitly, never a disk path. Does not add active references, create a citation, publish, save or change undo history. Returned IDs identify candidates: reconciliation can keep an existing target ID. Inspect diagnostics and plan; pass the report fingerprint, library_fingerprint and explicit decisions to bibliography.apply, then read inspect/search for active citation targets. |
| `bibliography.prepare_record` | query | Prepare a new native bibliographic notice with a proposed available key and nonblocking completeness diagnostics. Does not edit active project notices or create a citation. Native fields overlay the profile's starting structure; the returned profile is derived from the resulting native entry; unexposed fields are retained. Explicit occupied keys and invalid fields are refused. Publish through bibliography.apply (create) before using record_id in a citation, or do both in one atomic batch. |
| `bibliography.profiles` | query | Describe common editing profiles derived from native Hayagriva types and active parents. Profiles are never stored as independent classifications. Article-only; completeness recommendations are not mandatory bibliographic fields. |
| `bibliography.record` | query | Read one native record by stable id, its content fingerprint and completeness diagnostics. Use its fingerprint as the record guard for edit, rename or remove within bibliography.apply. |
| `bibliography.review_import` | query | Resolve explicit decisions against the frozen library fingerprint; return the reviewed plan without publication. No silent rebase of import decisions. |
| `bibliography.search` | query | Search the project bibliography by title and author. |
| `bibliography.usage` | query | Return typed tree targets of a notice’s citations in the document body, including widget text. Header profiles currently exclude citation objects. |
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
| `document.set_model` | edit | Set model_id, full pure Typst source and YAML manifest together. Every option regime and supported language is validated. Retained choices survive model switches; metadata.get reports effective values and substitutions. |
| `document.source` | query | Read generated Typst without changing the source of truth. |
| `document.statistics` | query | Count Unicode words and signs in the body and optional target. |
| `document.structure` | query | Read trees and all typed text zones; paths are revision-relative, offsets UTF-16. |
| `document.text` | query | Read plain text of the body or a typed text target. |
| `document.validate` | query | Validate the structured draft and report export readiness separately. |
| `document_types.describe` | query | Describe one type by ID. Integrated types include actual metadata, values_schema, capabilities and models; types without an implemented contract return only a product profile. Always check can_create. |
| `document_types.list` | query | List integrated document contracts and models, plus grouped product profiles. Discovery entries marked planned cannot be created; capability needs in profiles are intentions, not supported operations. |
| `edit.redo` | session | Redo this participant's contribution. |
| `edit.undo` | session | Undo this participant's last confirmed contribution. |
| `inline.replace` | edit | Replace one typed rich range with a footnote, citation, crossReference, editable mathInline, softBreak (a Typst space) or hardBreak (forced line break). Breaks are unavailable in single-line header fields and plain fields. A break is one UTF-16 unit; paragraph separation uses block.split. No implicit CR/LF interpretation in text.insert. Editable mathInline has nested plain text content: use text.insert on its own zone, not a parent offset inside its envelope. Formula delimiters are input conventions, not stored source. Discover objects via capabilities.describe. |
| `insert.table` | edit | Insert a table figure. header marks the first row as column headers; row_headers marks the first column as row headers. They are independent, including a one-cell table. No merged cells. |
| `list.item.exit` | edit | Remove an entirely empty item and create/reuse a continuation paragraph after its list. Remove the whole list only when it was the last item. A badge is content: an item containing a note cannot be exited as empty. |
| `list.item.join` | edit | Join adjacent bullet/numbered items of the same list, concatenating the last left paragraph and first right paragraph without an added space or break. Keep the left item and prefix. An empty right item is removed. Refuse description term/definition boundaries and different lists. |
| `list.item.split` | edit | Split a flat bullet/numbered list item at one paragraph range, or a description item at its definition range. Preserve the left item and prefix; the suffix and following paragraphs become a new item. Description terms remain separate, and the new term is empty. Empty-item exit is a separate operation. |
| `metadata.get` | query | Read metadata, fields and resolved model_options: choice (null = follow model), effective, regime, allowed/default and substituted warning. Language remains documentary data. |
| `metadata.set` | edit | Set a typed metadata, record field or document option. Values are validated against the type and model policy. Date fields accept null, {kind: calendar, iso: YYYY-MM-DD}, or {kind: custom, text: literal text}; never a raw string. Today must be resolved to a fixed local date by the caller. Discover documentary values_schema with document_types.describe. |
| `models.list` | query | Read brief visual model cards, including the active embedded model. |
| `node.set_attrs` | edit | Typed structured document operation; discover targets and parameters in params_schema. |
| `record.add` | edit | Edit a header collection using stable record IDs. Forward references inside an atomic lot are checked only on its final state. |
| `record.move` | edit | Edit a header collection using stable record IDs. Forward references inside an atomic lot are checked only on its final state. |
| `record.remove` | edit | Edit a header collection using stable record IDs. Forward references inside an atomic lot are checked only on its final state. |
| `references.list` | query | List cross-reference targets in document order with stable opaque IDs, kinds, titles and current paths. IDs survive title edits; paths are revision-scoped. Use inline.replace with a crossReference node (target, include_label, include_page). Labels and page formatting belong to the model; at least one component is required. Deleting a referenced target requires removing or retargeting its references in the same atomic batch. |
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
| `versions.change` | session | Create, relabel or delete a retained version and atomically save the current confirmed project under its existing lock. Requires the latest versions.list history token. Does not change text, private input or undo history; cannot delete internal review references. |
| `versions.list` | query | List retained versions with their immutable IDs, creation dates and editable labels. Internal review references are excluded. Read history for index-guarded mutations. |
| `versions.read` | query | Read one immutable version, including structured content, metadata and original image resources. Never changes the active document. |
