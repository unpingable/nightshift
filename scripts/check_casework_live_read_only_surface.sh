#!/usr/bin/env bash
# Structural gate for the distinct query-only live Casework family.

set -uo pipefail

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
cd "$repo_root"

findings=0
hits=$(mktemp)
snapshot=$(mktemp)
trap 'rm -f "$hits" "$snapshot"' EXIT

fail() {
    printf 'casework-live-read-only: %s\n' "$1" >&2
    findings=$((findings + 1))
}

sources=(
    crates/nightshift-casework/src/live_capacity.rs
    crates/nightshift-casework/src/live_loader.rs
    crates/nightshift-casework/src/live_model.rs
    crates/nightshift-casework/src/server.rs
    ui/casework/src/LiveViews.tsx
    ui/casework/src/api.ts
)
for source in "${sources[@]}"; do
    printf '\n/* source: %s */\n' "$source" >>"$snapshot"
    awk '/^#\[cfg\(test\)\]/{exit} {print}' "$source" >>"$snapshot"
done

required_markers=(
    'nightshift.casework-live-run/v1'
    'read_only_run_snapshot'
    'NIGHTSHIFT-FOREMAN-JOURNAL-FRAMING-V1'
    'NOT_RECORDED_BY_FOREMAN'
    'EXACT_RECORDED_BY_FOREMAN'
    'capacity_admission_digest'
    'READ_ONLY_OPERATOR_PROJECTION'
    'NO_ACCEPTED_TERMINAL_OR_NOT_STARTED_RECEIPT'
    '/api/v1/active-runs'
)
for required in "${required_markers[@]}"; do
    if ! rg -q "$required" "$snapshot" schemas/nightshift.casework-live-run.v1.schema.json; then
        fail "required live custody marker is missing: $required"
    fi
done

if ! rg -q 'pragma_update\(None, "query_only", "ON"\)' crates/nightshift-foreman/src/store.rs ||
   ! rg -q 'PRAGMA query_only' crates/nightshift-foreman/src/store.rs; then
    fail "foreman owner does not enforce and verify SQLite query_only"
fi

if rg -n 'ForemanStore|TransactionBehavior::Immediate|initialize\(|pragma_update|journal_mode.*WAL' "$snapshot" >"$hits"; then
    fail "Casework imports a foreman writer or initialization surface:"
    cat "$hits" >&2
fi
if rg -n 'fs::(write|rename|remove_|create_dir)|File::create|OpenOptions|Command::new|TcpStream::connect' "$snapshot" >"$hits"; then
    fail "live projection contains a filesystem, subprocess, or outbound transition:"
    cat "$hits" >&2
fi
if rg -n -i 'aggregate[_ -]?(result|verdict|health)|overall[_ -]?health' crates/nightshift-casework/src/live_capacity.rs crates/nightshift-casework/src/live_loader.rs crates/nightshift-casework/src/live_model.rs ui/casework/src/LiveViews.tsx schemas/nightshift.casework-live-run.v1.schema.json >"$hits"; then
    fail "live projection contains an aggregate result or health field:"
    cat "$hits" >&2
fi
if rg -n '<(button|textarea)|contentEditable|method:[[:space:]]*"(POST|PUT|PATCH|DELETE)"' ui/casework/src/LiveViews.tsx ui/casework/src/api.ts >"$hits"; then
    fail "live UI contains a response or write control:"
    cat "$hits" >&2
fi
if ! rg -q 'if method != "GET"' crates/nightshift-casework/src/server.rs || ! rg -q '405, "Method Not Allowed"' crates/nightshift-casework/src/server.rs; then
    fail "all non-GET methods are not refused with 405"
fi
if ! rg -q 'navigation_id' crates/nightshift-casework/src/server.rs || ! rg -q 'live_sources.get\(navigation_id\)' crates/nightshift-casework/src/server.rs; then
    fail "live request routing is not restricted to startup-registered navigation identities"
fi

if [ "$#" -gt 0 ] && [ "$1" = "--self-test-inject" ]; then
    temporary=$(mktemp -d)
    self_hits=$(mktemp)
    trap 'rm -rf "$temporary"; rm -f "$hits" "$snapshot" "$self_hits"' EXIT
    mkdir -p "$temporary/crates/nightshift-casework/src" "$temporary/crates/nightshift-foreman/src" "$temporary/ui/casework/src" "$temporary/schemas" "$temporary/scripts"
    cp scripts/check_casework_live_read_only_surface.sh "$temporary/scripts/"
    cp crates/nightshift-casework/src/live_capacity.rs crates/nightshift-casework/src/live_loader.rs crates/nightshift-casework/src/live_model.rs crates/nightshift-casework/src/server.rs "$temporary/crates/nightshift-casework/src/"
    cp crates/nightshift-foreman/src/store.rs "$temporary/crates/nightshift-foreman/src/"
    cp ui/casework/src/LiveViews.tsx ui/casework/src/api.ts "$temporary/ui/casework/src/"
    cp schemas/nightshift.casework-live-run.v1.schema.json "$temporary/schemas/"
    printf '%s\n' 'fn injected_transition() {' '  let _ = ForemanStore::open("writer.sqlite");' '  let _ = std::process::Command::new("worker");' '  let _ = std::fs::write("state", b"x");' '}' >>"$temporary/crates/nightshift-casework/src/live_model.rs"
    printf '%s\n' 'export function Injected() {' '  return <button>retry and approve aggregate health</button>;' '}' >>"$temporary/ui/casework/src/LiveViews.tsx"
    if (cd "$temporary" && bash scripts/check_casework_live_read_only_surface.sh) >/dev/null 2>"$self_hits"; then
        printf 'self-test FAILED: injected live transitions passed\n' >&2
        exit 1
    fi
    for marker in writer filesystem aggregate control; do
        if ! rg -q "$marker" "$self_hits"; then
            printf 'self-test FAILED: injected %s boundary was not reported\n' "$marker" >&2
            cat "$self_hits" >&2
            exit 1
        fi
    done
    printf 'self-test PASSED: writer, filesystem, subprocess, aggregate, and UI-control injections were refused.\n' >&2
    exit 0
fi

if [ "$findings" -ne 0 ]; then
    printf 'casework-live-read-only: FAILED (%d finding(s))\n' "$findings" >&2
    exit 1
fi
printf 'casework-live-read-only: clean query-only live projection surface\n'
