# MAP-CABINET browser trace

Campaign: MAP-CABINET

Canonical slug: `nightshift-read-only-casework-console-mvp-v1`

Browser: installed `Google Chrome 143.0.7499.109`; no browser or browser
driver was downloaded.

Backend contract: `f50e5c4ee64ae16d29fa5b39dedf166ad9dca4f9`

Projection digest:
`sha256:aa2e823cf8d8f323af1ed2e6a1cfc27dc84e8193f3915de75a03a348654651e8`

## Local integrated run

The production Vite build was preloaded from `ui/casework/dist` by the
casework binary and served at the temporary loopback address
`127.0.0.1:38175`. HTTP checks observed:

```text
healthz: 200 {"status":"ok"}
declared run route: 200 text/html; charset=utf-8
declared work-item route: 200 text/html; charset=utf-8
unlisted asset: 404
POST /: 405
```

Headless Chrome loaded the stable run and RIVER-CLERK work-item URLs through
the backend's SPA fallback. The rendered DOM contained:

```text
Showing 14 of 14 exact work items
Human questions · 6
GLASSHOPPER
CLOSEOUT-COMPLETE-NOT-QUALIFIED
CLOSEOUT-COMPLETE-CAMPAIGN-NOT-QUALIFIED
RIVER-CLERK
TERMINAL-NOT-QUALIFIED
NOT-QUALIFIED-IDENTITY-CONTRACT-SUCCESSOR-REQUIRED
separately authorized versioned NQ prepared-occurrence contract binding NQ plan and Docket executor-plan identities without circularity
```

The rendered run and RIVER DOM contained no `button`, `textarea`,
`contenteditable`, or aggregate-verdict element. The browser loaded only the
manifest-listed local assets `index-eeFaUpgO.js` and `index-3ba0qEVX.css`.

## Visual artifact

`map-cabinet-run-case.png` is a 1440 by 1000 RGB PNG of the backend-served run
ledger. Its SHA-256 is
`71e14a329cbbf7b070be042648a042badc263c6242503020af67a2c368f75a39`.

## Teardown

The Chrome processes exited, temporary browser profiles and DOM captures were
removed, the casework process received an interrupt, and both qualification
ports were verified without a remaining listener.
