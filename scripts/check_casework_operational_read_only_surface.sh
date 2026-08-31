#!/usr/bin/env bash
# Structural qualification gate for the distinct operational-condition Casework family.

set -uo pipefail

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
cd "$repo_root"

findings=0
hits=$(mktemp)
snapshot=$(mktemp)
trap 'rm -f "$hits" "$snapshot"' EXIT

fail() {
    printf 'casework-operational-read-only: %s\n' "$1" >&2
    findings=$((findings + 1))
}

sources=(
    crates/nightshift-casework/src/operational_loader.rs
    crates/nightshift-casework/src/operational_model.rs
    crates/nightshift-casework/src/server.rs
    ui/casework/src/OperationalViews.tsx
    ui/casework/src/api.ts
)
for source in "${sources[@]}"; do
    printf '\n/* source: %s */\n' "$source" >>"$snapshot"
    awk '/^#\[cfg\(test\)\]/{exit} {print}' "$source" >>"$snapshot"
done

required_markers=(
    'nightshift.casework-operational-condition/v1'
    'nightshift.casework-operational-condition-index/v1'
    'OperationalObservationLineageV1'
    'OperationalReobservationEvaluationV1'
    'read_only_projection_no_authority'
    'presentation_only'
    'next_lawful_action'
    '/api/v1/operational-conditions'
)
for required in "${required_markers[@]}"; do
    if ! rg -q "$required" "$snapshot" schemas/nightshift.casework-operational-condition.v1.schema.json schemas/nightshift.casework-operational-condition-index.v1.schema.json; then
        fail "required operational custody marker is missing: $required"
    fi
done

for fixed in monitor.v1.json nq.v1.json lineage.v1.json profile.v1.json evaluation.v1.json; do
    if ! rg -q "$fixed" crates/nightshift-casework/src/operational_loader.rs; then
        fail "fixed operational owner filename is missing: $fixed"
    fi
done
if ! rg -q 'OFlags::NOFOLLOW' crates/nightshift-casework/src/operational_loader.rs ||
   ! rg -Fq 'bytes.len() as u64 != admitted_size' crates/nightshift-casework/src/operational_loader.rs ||
   ! rg -Fq 'before.dev() == after.dev()' crates/nightshift-casework/src/operational_loader.rs ||
   ! rg -Fq 'before.ino() == after.ino()' crates/nightshift-casework/src/operational_loader.rs ||
   ! rg -Fq 'before.ctime() == after.ctime()' crates/nightshift-casework/src/operational_loader.rs; then
    fail "descriptor-bound exact-byte acquisition and stable metadata are not structurally pinned"
fi
if ! rg -q 'admit_operational_lineage' crates/nightshift-casework/src/operational_loader.rs ||
   ! rg -q 'validate_against' crates/nightshift-casework/src/operational_loader.rs ||
   ! rg -q 'same_temporal_branch' crates/nightshift-casework/src/operational_loader.rs; then
    fail "exact EPOCH derivation, evaluation binding, or subject-local history grouping is absent"
fi

if rg -n 'fs::(write|rename|remove_|create_dir)|File::create|OpenOptions|Command::new|TcpStream::connect|ForemanStore|CanonicalStore' "$snapshot" >"$hits"; then
    fail "operational projection contains a filesystem, subprocess, outbound, or owner-writer transition:"
    cat "$hits" >&2
fi
if rg -n -i 'aggregate[_ -]?(result|verdict|health)|overall[_ -]?health' crates/nightshift-casework/src/operational_loader.rs crates/nightshift-casework/src/operational_model.rs ui/casework/src/OperationalViews.tsx schemas/nightshift.casework-operational-condition.v1.schema.json schemas/nightshift.casework-operational-condition-index.v1.schema.json >"$hits"; then
    fail "operational projection contains a combined result or health field:"
    cat "$hits" >&2
fi
if rg -n -i '<(button|textarea)|contentEditable|method:[[:space:]]*"(POST|PUT|PATCH|DELETE)"' ui/casework/src/OperationalViews.tsx ui/casework/src/api.ts >"$hits"; then
    fail "operational UI contains a response or write control:"
    cat "$hits" >&2
fi
if ! rg -Fq 'method != "GET" && !(method == "HEAD" && operational_family)' crates/nightshift-casework/src/server.rs ||
   ! rg -Fq 'operational_family = is_operational_condition_route(path)' crates/nightshift-casework/src/server.rs ||
   ! rg -q 'fn is_operational_condition_route' crates/nightshift-casework/src/server.rs ||
   ! rg -Fq 'head_only = method == "HEAD" && response.allow == "GET, HEAD"' crates/nightshift-casework/src/server.rs; then
    fail "GET/HEAD is not restricted to the exact operational route family"
fi
if ! rg -q 'old_head' crates/nightshift-casework/src/server.rs ||
   ! rg -q 'HEAD /healthz' crates/nightshift-casework/src/server.rs ||
   ! rg -q 'method not allowed' crates/nightshift-casework/src/server.rs; then
    fail "the predecessor GET-only method contract lacks its direct regression qualification"
fi
for kind in monitor nq lineage profile evaluation; do
    if ! rg -q "/raw/$kind" crates/nightshift-casework/src/server.rs ||
       ! rg -q "\"$kind\"" ui/casework/src/api.ts; then
        fail "fixed exact raw route is absent: $kind"
    fi
done
if ! rg -q '"additionalProperties": false' schemas/nightshift.casework-operational-condition.v1.schema.json ||
   rg -n '"grants_authority":[[:space:]]*\{[^}]*"const":[[:space:]]*true' schemas/nightshift.casework-operational-condition.v1.schema.json >"$hits"; then
    fail "operational schema is open or authorizing"
fi

if [ "$#" -gt 0 ] && [ "$1" = "--self-test-inject" ]; then
    temporary=$(mktemp -d)
    self_hits=$(mktemp)
    trap 'rm -rf "$temporary"; rm -f "$hits" "$snapshot" "$self_hits"' EXIT
    mkdir -p "$temporary/crates/nightshift-casework/src" "$temporary/ui/casework/src" "$temporary/schemas" "$temporary/scripts"
    cp scripts/check_casework_operational_read_only_surface.sh "$temporary/scripts/"
    cp crates/nightshift-casework/src/operational_loader.rs crates/nightshift-casework/src/operational_model.rs crates/nightshift-casework/src/server.rs "$temporary/crates/nightshift-casework/src/"
    cp ui/casework/src/OperationalViews.tsx ui/casework/src/api.ts "$temporary/ui/casework/src/"
    cp schemas/nightshift.casework-operational-condition.v1.schema.json schemas/nightshift.casework-operational-condition-index.v1.schema.json "$temporary/schemas/"
    printf '%s\n' 'fn injected_transition() {' '  let _ = std::process::Command::new("local-helper");' '  let _ = std::fs::write("state", b"x");' '  let _ = CanonicalStore::open("writer.sqlite");' '}' >>"$temporary/crates/nightshift-casework/src/operational_model.rs"
    printf '%s\n' 'export function Injected() {' '  return <button>Approve aggregate health and dispatch</button>;' '}' >>"$temporary/ui/casework/src/OperationalViews.tsx"
    if (cd "$temporary" && bash scripts/check_casework_operational_read_only_surface.sh) >/dev/null 2>"$self_hits"; then
        printf 'self-test FAILED: injected operational transitions passed\n' >&2
        exit 1
    fi
    for marker in filesystem combined control; do
        if ! rg -q "$marker" "$self_hits"; then
            printf 'self-test FAILED: injected %s boundary was not reported\n' "$marker" >&2
            cat "$self_hits" >&2
            exit 1
        fi
    done
    printf 'self-test PASSED: injected writer, filesystem, subprocess, combined-result, and UI-control transitions were refused.\n' >&2
    exit 0
fi

if [ "$findings" -ne 0 ]; then
    printf 'casework-operational-read-only: FAILED (%d finding(s))\n' "$findings" >&2
    exit 1
fi
printf 'casework-operational-read-only: clean operational projection surface\n'
