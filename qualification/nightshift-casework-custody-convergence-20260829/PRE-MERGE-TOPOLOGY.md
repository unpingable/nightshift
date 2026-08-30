# MORTISE-OWL pre-merge topology record

- Codename: `MORTISE-OWL`
- Canonical slug: `nightshift-casework-qualified-head-custody-convergence-v1`
- Track: `nightshift-repository-custody`
- Repository remote: `git@github-unpingable:unpingable/nightshift.git`
- Starting integration head: `ad36932be2d4aceeff71b848bad534aae1c3c938`
- Exact INDEX-WREN result head: `83ad130468c940848d3d0fb89ed9f48925baabd8`
- Exact merge base: `16e09ab97d12cfd3d1afa787610a6902b56f0967`

No codename or canonical-slug collision was found in relevant local working
trees or locally reachable Nightshift and Cartography history.

## Exact topology

```text
16e09ab INDEX-WREN contract freeze
├── c2912e0 → 7f2989c → f50e5c4 → 83ad130 INDEX-WREN result
└── 4571a38 → b3cd6e2 → 8ab22ad → cfc6d92 → 01666c4
    → 5eb7cdb → 1d6fec2 → 020b405 → 2e69f89
    → 0542129 → 4865d22 → ad36932 MAP-CABINET result
```

INDEX-WREN qualified implementation:
`f50e5c4ee64ae16d29fa5b39dedf166ad9dca4f9`.

INDEX-WREN result/closeout:
`83ad130468c940848d3d0fb89ed9f48925baabd8`.

MAP-CABINET result/closeout:
`ad36932be2d4aceeff71b848bad534aae1c3c938`.

## Verified implementation provenance

The four INDEX-side patches have MAP-side equivalents:

| INDEX-WREN | MAP adoption | Relationship |
| --- | --- | --- |
| `c2912e0` | `b3cd6e2` | stable patch equivalent |
| `7f2989c` | `cfc6d92` | stable patch equivalent |
| `f50e5c4` | `1d6fec2` | stable patch equivalent |
| `83ad130` | `2e69f89` | stable patch ID `b90fac77b7469dbbe264caeb90623f754f9df7e1` |

These relationships establish implementation provenance only. Before this
campaign, they did not establish exact INDEX-WREN result ancestry or inherited
qualification.

## Side-specific material

INDEX-WREN uniquely contributes its exact four-commit history and original
result-head custody. Its immutable versions of
`INDEX-WREN-QUALIFICATION.md`,
`index-wren.qualification.v1.json`, and the structural gate remain
addressable at the exact result head.

MAP-CABINET uniquely contributes the React/Vite UI, manifest-closed static
delivery, static-delivery tests, UI/backend structural integration, browser
qualification, accessibility corrections, self-description case, generated
reports, screenshot, and MAP closeout records.

The tree difference from INDEX result to MAP result is the MAP product and
qualification surface plus later time-scoped publication facts. No competing
backend projection schema, digest law, packet law, API route meaning, or
receipt compatibility law was found.

## Merge conflict inventory and resolutions

Three add/add conflicts arose:

1. `INDEX-WREN-QUALIFICATION.md`: retained both the earlier local
   implementation-checkpoint custody and the later remote-verified campaign
   closeout custody as distinct instants.
2. `index-wren.qualification.v1.json`: retained the remote closeout summary
   and added an ordered custody history preserving both instants.
3. `check_casework_read_only_surface.sh`: retained the exact INDEX run-input
   no-follow check and MAP's separate manifest-closed static-UI no-follow
   checks.

No runtime semantic choice was required. Exact predecessor versions remain
reachable through their immutable parent commits.
