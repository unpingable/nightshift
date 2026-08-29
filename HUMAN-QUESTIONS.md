# Human questions

## 1. switchyard-transport

- Exact question: What is the authoritative private remote/destination and canonical branch policy for /data/git/switchyard?
- Evidence exhausted: The configured origin is /data/git/switchyard/switchyard-codex-session-foreman-mvp.bundle, that bundle is absent, git ls-remote fails, and Cartography has no Switchyard repository row.
- Safe default: Do not invent or change a remote; retain the clean local campaign commit and report sole-local custody.
- Consequences: Providing the registered private destination permits a campaign-branch push and SHA verification. Leaving it unspecified keeps Interlock and QUIET-BRIDGE locally custodied only.
- Resume point: Verify destination identity against Cartography/repository policy, configure a non-destructive remote, push master Interlock and the QUIET-BRIDGE campaign branch, then verify exact SHAs.
