# typstographe-bibliography

Pure bibliographic values and operations, shared by project libraries (C.3) and
the future personal library (C.4). No filesystem, clock, network, randomness,
GUI, document engine or localization runtime. Callers allocate opaque identities
and own persistence, undo history, recovery and publication.

- Native Hayagriva records with independent stable IDs and editable citation keys.
- Immutable UTF-8 import originals and provenance, BibLaTeX/Hayagriva parsing,
  conversion diagnostics, nonblocking completeness hints and form descriptors.
- Unicode-aware fuzzy title/author search, summaries and available-key suggestions.
- Candidate evidence separate from contradictions and whole-group coherence.
- Immutable reconciliation plans, explicit same/distinct/skip decisions,
  gap-only enrichment, full-library guards and all-or-nothing application in memory.
- Deterministic Hayagriva projection retaining supported native fields and parents.

Public records serialize as `id`, `key`, `entry`, `retained_parents`, `provenance`.
Editing profiles are derived from the active native type and its parents; no
independent genre or profile is stored. The
`entry` is native Hayagriva data, not a flattened title/author/year schema.
Parent paths in `native_value` and field descriptors use arrays, including a
single parent. Field patches replace or remove **complete native fields**;
compound formatted text, publisher and URL controls must preserve their metadata.
`Record::edited` preserves every field outside the supplied patches and refuses
invalid paths, invalid native values and a changed base. Source imports report
unsupported fields; strict record decoding refuses unknown native root/parent
fields instead of dropping them. Shared checks also cover formatted text, people
and publisher members. Import diagnostics use the same paths. This is not an
exhaustive conversion-loss detector for every native composite.

`Record::from_native` preserves typed validation errors. Public preparation values
have JSON schemas; identities and fingerprints validate their advertised string
formats at runtime. Literal formatted-text shorthand is normalized before native
decoding to preserve casing and short titles with Hayagriva 0.10.1. Native
serialization still owns the projected YAML; import originals are unchanged.

Import identities must be fresh within the destination library and unique in a
batch. They remain stable while the plan is reviewed. Reimporting the same source
with fresh proposal identities does not duplicate existing works; source links
are deduplicated by original digest and key. Citation-key collisions remain
naming decisions, never identity evidence. Suggested names never rename an
existing record. Plans cannot be deserialized as trusted mutation instructions.

Apply returns a new `Library`; it never changes its input. A blocked or stale
plan returns an error. Skipping a conflicting incoming record rechecks the
remaining group. Same-work decisions can target another incoming record;
cycles and unrelated targets are rejected. Existing values remain intact even
after a same-work decision: differing fields are listed for a separate correction.

The prepared import contains review diagnostics, including conservative warnings
for unqualified BibLaTeX fields. The caller must present those before publication.
Hayagriva 0.10.1 converts `@software` to `misc`, while `repository` also covers
data. Prepared imports explicitly map `@software` to native `repository` and
report that choice before publication; original bytes stay immutable. Native
`repository` derives the software editing profile, consistent with the CSL
processor. Dataset conversions still require review rather than inventing an
unsupported native dataset type.

`Record::reclassified` changes the active native structure, preserves common
inherited values and retains inactive publication containers for later type
changes. Matching profiles preserve imported variants without normalization.
The form codec edits only visible changed controls. Returning to a previous
publication profile restores its container, never a different publication’s
title. Identity, citation key and provenance remain stable. The project adapter
materializes inactive containers separately from the active Hayagriva library.

Tests:

```sh
cargo test --locked --offline -p typstographe-bibliography
cargo clippy --locked --offline -p typstographe-bibliography --all-targets -- -D warnings
```

The [contract](../../docs/c3-bibliography-contract.md),
[read-only preparation queries](../../docs/c3-bibliography-preparation.md),
[validated interface](../../docs/c3-bibliography-interface.md) and
[Citekeep-derived corpus](../../fixtures/bibliography/README.md) explain the
integration contracts and attribution. See the
[native-profile decision](../../docs/c3-native-bibliography-profiles.md) for
guarded transitions, inactive publication data and CAS 16 qualification.
