# Nightshift Casework console MVP

Nightshift Casework is a separate, read-only operator surface over one or more
explicit run directories. Each case is derived only from exact sealed
`nightshift.orientation-packet/v1` bytes and its exact
`nightshift.run-receipts/v1` snapshot. Generated Markdown reports are not UI
inputs.

## Information architecture

The run index lists packet identity, receipt snapshot time, exact state counts,
human-question count, and packet custody discrepancies. It intentionally has
no aggregate score or campaign result.

The run case is a dense ledger. Its state and classification cells display the
projection strings verbatim. Exact-state, track, and human-question filters are
client-side views over the loaded projection; they create no new
classification.

Work-item records pair bounded packet intent with recorded receipt outcome in
parallel columns. Human-question records expose question, exhausted evidence,
safe default, consequences, resume point, and linked work item. Starting packet
custody and final receipt custody remain separate. Raw artifact views display
the exact response text and its projection-recorded digest and validation
disposition.

Stable client routes are:

```text
/runs/{packet-digest-hex}
/runs/{packet-digest-hex}/work-items/{work-item-id}
/runs/{packet-digest-hex}/questions/{question-id}
/runs/{packet-digest-hex}/custody
/runs/{packet-digest-hex}/raw
```

Derived route identifiers are navigation keys only. Browser history changes
do not write to the casework backend.

## Read-only behavior

The browser issues same-origin `GET` requests only to the accepted casework
API. It has no answer, disposition, agent-control, or case-mutation operation.
The only form elements are three local ledger filters. Source text can contain
operational verbs because exact packet and receipt values must remain visible;
those values are never rendered as controls.

Receipt fields that the historical renderer accepted as loose JSON remain
compatibility values in the closed projection. The UI displays only a recognized
typed value where the backend provides one. An unrecognized shape stays solely
in the exact raw receipts; semantic views label it as unrecognized and link to
those bytes. The browser does not infer meaning from an unrecognized value.

## Accessibility and layout

The application uses landmarks, headings, tables, definition lists, explicit
labels, and native links/selects. A skip link and visible focus treatment cover
keyboard navigation. Exact states and classifications use text rather than
color as their identity. Dense two-column records collapse to one column below
tablet width, and wide ledgers retain horizontal scrolling rather than dropping
fields. Reduced-motion preferences disable smooth scrolling.

No remote font, image, analytics, telemetry, CDN, or runtime asset is loaded.

## Development and qualification

From `ui/casework`:

```bash
npm ci
npm test
npm run build
```

The Vite development listener is loopback-only and proxies `/api` and
`/healthz` to a loopback casework API at port 8080. It is development machinery,
not a production service. For local operator use, the casework binary accepts
an explicit compiled `--ui-dir`, preloads the manifest-closed asset set with
directory-relative no-follow opens, and then serves only those in-memory bytes.
No HTTP request selects a filesystem pathname.

After building the UI, a local integrated run is:

```bash
cargo run --locked -p nightshift-casework --bin nightshift-casework -- \
  --run-dir qualification/nightshift-packet-v1/velvet-orrery \
  --ui-dir ui/casework/dist \
  --bind 127.0.0.1:8080 \
  --evaluated-at 2026-08-31T00:00:00Z
```

The committed VELVET-ORRERY casework projection is the frontend golden fixture.
Unit journeys cover all stable routes, the 14-item ledger, RIVER-CLERK,
GLASSHOPPER, six questions, custody separation, raw-byte display, unknown-state
fidelity, filtering, and the absence of mutation controls.
