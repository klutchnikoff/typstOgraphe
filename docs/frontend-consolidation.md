# Frontend consolidation and type-aware commands

Consolidation before C.4, 6 November 2026. This changes ownership and command
exposure, without replacing Tiptap, the shared form components or the Rust
bibliography implementation.

## Boundaries

| Module | Responsibility |
| --- | --- |
| `metadata-collections.ts` | Collection order, cards, private form lifecycle, recovery and guarded publication |
| `metadata-record-fields.ts` | Scalar/reference/rich controls, live versus private bindings, refresh and subscription cleanup |
| `bibliography-manager.ts` | Workflow, private draft, protected navigation, recovery, focus and publication guard |
| `bibliography-browser.ts` | Compact rows, accessible search/list controls and keyboard navigation; no queries or document edits |
| `bibliography-requests.ts` | Request lifetime tied to the document identity and generation; independent latest-response channels |
| `bibliography-publication.ts` | Prepare an atomic command batch from frozen draft data; no DOM, publication or recovery |
| `typstographe-bibliography` | Native Hayagriva profiles, field codecs, search, import and reconciliation rules |

The manager remains the integration point for `formDialog`, recovery and
`metadataCommands`. Only the shared document engine publishes edits. A failed
preparation or a stale document guard leaves the private form intact. Search,
import review and usage/navigation requests cannot overwrite another request
channel; closing the manager or switching documents invalidates all of them.
Late usage counts/errors are ignored after the selected notice changes.

`provider.ts` retains its tightly coupled coordination of confirmed revisions,
IME, history and editing contexts. `main.ts` composes the services. Neither is
split just to reduce a line count; extracting these state machines requires
its own behavior contract if their responsibilities expand.

## Type support versus availability

The registry derives command ownership from the common capability catalogue:
primary capability commands and their declared child actions share one support
predicate. Toolbar and palette consume `commandVisible`; all triggers also
check common availability before running.

- Unsupported capabilities are hidden; empty toolbar menus are hidden and closed.
- Supported commands remain visible when unavailable in the current selection.
- General application commands remain accessible, subject to their normal guards.
- A future type assembled from existing capabilities needs no toolbar/palette
  special case. Adding a new capability still requires its editor adapter.

Refresh keeps the palette query/focus. Its navigation and document subscription
are explicitly released when its sidebar view closes or is replaced.

## Checks

`scripts/check_file_length.py` checks Rust production items and all production
TypeScript/TSX under `app/src`, recursively, with the same blocking 800-line
limit. Rust test items and test files, frontend tests, fixtures and generated
subdirectories are excluded. Existing custom `--root` Rust checks remain
available; `--typescript-root` selects another frontend source tree. This is
part of the existing local and CI verification path.

Unit checks cover guarded publication, atomic creation/citation, preservation
of citation items/marks, clean notices, unreviewed imports, independent request
channels and lifetime invalidation. Command tests use an arbitrary capability
subset and verify filtering, child actions, contextual disabling, global
commands and query/focus preservation. Native-backed Chromium/WebKit recipes
cover bibliography, recovery, concurrent edits, private metadata dialogs,
ordering and Courrier → Article → Courrier, without screenshots or traces.

Local qualification: 492 frontend unit tests, 10 script checks, 28 Python checks,
production build/type checking and the module-size guard passed. The targeted
native-backed suite passed 58 journeys across Chromium/WebKit; the deliberately
late usage-response case passed separately in both engines (60 total). The
pre-push hook additionally checks the immutable commit before publication.

The broader pre-push run exposed an existing asynchronous setup race in the
palette recipe: Tiptap `focus()` schedules an animation frame. The recipe now
awaits actual editor focus before opening the palette and uses the platform
modifier. Both palette journeys passed ten repetitions per browser (40 runs).
