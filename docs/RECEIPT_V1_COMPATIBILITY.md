# Nightshift run-receipt V1 compatibility

`nightshift-casework` consumes exact `nightshift.run-receipts/v1` bytes. It
does not treat the generated ledger, human-question report, or morning report
as source evidence.

## Compatibility baseline

The compatibility baseline is a receipt snapshot that
`scripts/render_nightshift_reports.py` can successfully render, plus the
explicit casework linkage checks below. The adapter requires:

- a top-level object with exact string schema and packet digest, a present
  `updated_at` value, and arrays named `work_items`, `human_questions`,
  and `repository_custody`;
- exactly one receipt item for every packet work-item id, with no duplicate,
  unknown, or missing id;
- all renderer-known work-item fields;
- complete question and custody rows; and
- present question `work_item` and custody `repository` fields; recognized exact
  strings receive linkage/identity semantics only when applicable.

Renderer-compatible values deliberately remain broader than the semantic
projection:

- `state`, `result_classification`, `remaining_trigger`,
  `next_lawful_action`, `updated_at`, and question/custody display cells
  may be any present JSON value;
- `tests`, `evidence`, and `live_or_production_mutations` accept arrays of
  strings, strings, or objects, matching the renderer's join behavior; and
- `repositories` retains the renderer's broad JSON compatibility.

The closed projection promotes only recognized shapes. Strings become
`recognized_string`; string arrays become `recognized_strings`; demonstrated
four-string repository rows become `recognized_rows`. All of those members
are null for unrecognized shapes. An RFC 3339 string `updated_at` additionally
provides `recognized_rfc3339`; another shape leaves snapshot currentness
`UNAVAILABLE`. Opaque JSON and renderer text are not copied into semantic
projection fields. The exact raw receipt bytes retain them for inspection.

Unknown top-level, work-item, question, custody, and repository-row extension
fields remain inspectable in the exact raw receipt bytes. They are not copied
into the closed casework model and acquire no meaning.

## Deliberate casework checks

Complete question and custody structures are required because the product exposes
them as records. Missing keys and non-object rows fail closed. Renderer-accepted
non-string or unlinked identity cells load with nullable recognized/linkable
semantics and retain deterministic ordinal navigation identities.

Recognized strings are projected verbatim. Duplicate exact questions preserve
the same base identity derived from packet, work item, and exact question; each
source row also receives a distinct ordinal-derived navigation identity so no
renderer-accepted row is lost.

The adapter defines no taxonomy, aggregate result, approval, execution request,
or inferred custody disposition.
