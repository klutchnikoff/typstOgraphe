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
