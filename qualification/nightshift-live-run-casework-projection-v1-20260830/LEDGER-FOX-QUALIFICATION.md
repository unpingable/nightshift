# LEDGER-FOX qualification

Campaign: `nightshift-live-run-casework-projection-v1`

Independent classification: `NIGHTSHIFT-LIVE-RUN-CASEWORK-PROJECTION-V1-QUALIFIED`

Qualified implementation subject: `ef6b11ef7514ab11074c297dc09cf796adea7e32`

Sealed packet: `sha256:1df7f47bb3ea70d0f987e756f34aaa62f7187a659ef0bcc8d7c8aa2e645431fc`

Exact base ancestry includes STILL-CIPHER `87e39aeaac07c74819a7e20a24cc905ad8929d63`. The accepted predecessor and every superseded correction remain immutable ancestors; no history was rewritten.

## Qualified result

LEDGER-FOX adds the distinct closed `nightshift.casework-live-run/v1` family beside the unchanged sealed `nightshift.casework-run/v1` family. It projects active or closed foreman state from an explicitly registered existing SQLite store through a descriptor-bound, `mode=ro`, `PRAGMA query_only=ON` read transaction. It creates no database, journal, WAL, SHM, scheduler transition, receipt, or capacity decision.

The read snapshot reopens and cross-binds exact packet, admission, profile, journal-event, accepted-receipt, and final-snapshot bytes before scheduler replay. Redundant run-table fields are checked against those exact records. A final snapshot is admitted only with one last `RunClosed` event whose digest, exact recorded time, ordering, terminal-evidence lower bound, replay result, and canonical reconstruction from accepted receipts all agree.

The loopback GET-only Casework API and browser render active runs separately from sealed receipt cases. Each work-item view keeps packet intent, live mechanism/attempt state, and accepted terminal/not-started outcome or explicit absence in separate regions. Lane-local question navigation derives from both work-item and question identity. Raw views retain exact packet, admission, profile, journal, accepted-receipt, event, and final-snapshot bytes. Scheduler state, provider-capacity references, and receipt classifications remain independent and are not an aggregate result.

## Qualification evidence

- Full locked Rust workspace, all targets: passed; documented environment-dependent tests remained ignored.
- Full workspace all-target/all-feature Clippy with warnings denied: passed.
- Rust formatting check: passed.
- Python schema and renderer suite: 15 passed.
- Casework UI suite: 24 passed; production TypeScript/Vite build passed.
- Foreman and Casework focused exact-archive replay: 50 passed with one documented browser-fixture emitter ignored.
- Canonical no-actuation, foreman read-only, sealed Casework read-only, Casework UI read-only, and live Casework query-only gates passed.
- Every gate's deterministic negative control passed.
- Independent exact-commit audit accepted `ef6b11ef7514ab11074c297dc09cf796adea7e32` with no acceptance-significant finding.

## Installed-browser journey

Google Chrome `143.0.7499.109` rendered the deterministic campaign-owned fixture from an explicit temporary root through a loopback-only Casework listener.

- The home page visibly separated Active foreman runs from Sealed receipt cases.
- Direct refresh rendered the active-run, work-item, event-timeline, and raw-source deep links.
- The work-item page retained the three required regions and showed no action control.
- The raw view rendered exact final-snapshot bytes and linked every exact event-byte route.
- All browser resource entries were served from `127.0.0.1`; no external application resource was used.
- Keyboard Tab focus reached the `Skip to casework` link.
- No approve, answer, dispatch, retry, resume, cancel, execute, merge, promote, or close control was present.

The browser profile, emitted fixture, SQLite copy, server, browser process, and both temporary loopback listeners were removed immediately after the journey.

## Independent limitations

- No capacity observation or decision exists in the fixture journal; the UI truthfully renders the execution profile's policy reference as `NOT_RECORDED_BY_FOREMAN` rather than inventing scheduler use.
- No production service, timer, default-branch merge, protected approval response, or target effect was performed.
- This qualification does not classify any other V2 campaign and creates no aggregate campaign verdict.

## Final custody

The qualified subject worktree was clean before qualification artifacts were added. Publication custody is the exact closeout commit on `campaign/ledger-fox-nightshift-live-run-casework-projection-v1-20260830`; the operator closeout resolves and reports that commit and its remote equality. No human question or teardown obligation remains.
