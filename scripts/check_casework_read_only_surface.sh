#!/usr/bin/env bash
# Structural gate for the separate Nightshift Casework read-only operator tool.

set -uo pipefail

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
cd "$repo_root"

findings=0
fail() {
    printf 'casework-read-only: %s\n' "$1" >&2
    findings=$((findings + 1))
}

manifest="crates/nightshift-casework/Cargo.toml"
production_src="crates/nightshift-casework/src"
source_snapshot=$(mktemp)
trap 'rm -f "$source_snapshot" /tmp/nightshift_casework_read_only_hits' EXIT
while IFS= read -r source; do
    printf '\n/* source: %s */\n' "$source" >>"$source_snapshot"
    awk '/^#\[cfg\(test\)\]/{exit} {print}' "$source" >>"$source_snapshot"
done < <(rg --files "$production_src" | sort)

if [ ! -f "$manifest" ] || [ ! -d "$production_src" ]; then
    fail "casework manifest or production source is missing"
fi
if ! rg -q '^autobins = false$' "$manifest"; then
    fail "automatic binary discovery is not disabled"
fi
if [ "$(rg -c '^\[\[bin\]\]$' "$manifest" || true)" -ne 1 ]     || ! rg -q '^name = "nightshift-casework"$' "$manifest"     || ! rg -q '^path = "src/bin/nightshift_casework.rs"$' "$manifest"; then
    fail "operator binary graph is not the exact single explicit target"
fi

if rg -n '(^|[^[:alnum:]_])(std::process|process::Command|Command::new)' "$source_snapshot" >/tmp/nightshift_casework_read_only_hits 2>/dev/null; then
    fail "operator tool opens a subprocess:"
    cat /tmp/nightshift_casework_read_only_hits >&2
fi
if rg -n '(CanonicalStore|canonical_store|rusqlite|ag_port|nq_admission|docket|switchyard)' "$source_snapshot" "$manifest" >/tmp/nightshift_casework_read_only_hits 2>/dev/null; then
    fail "operator tool references a canonical store or adjacent office:"
    cat /tmp/nightshift_casework_read_only_hits >&2
fi
if rg -n '^[[:space:]]*(reqwest|ureq|isahc|hyper|rusqlite|sqlx|tokio-postgres|postgres)[[:space:]]*=' "$manifest" >/tmp/nightshift_casework_read_only_hits 2>/dev/null; then
    fail "operator tool declares a client or database dependency:"
    cat /tmp/nightshift_casework_read_only_hits >&2
fi
if rg -n '(TcpStream::connect|UdpSocket|ToSocketAddrs|reqwest::|ureq::|hyper::client)' "$source_snapshot" >/tmp/nightshift_casework_read_only_hits 2>/dev/null; then
    fail "operator tool contains an outbound network path:"
    cat /tmp/nightshift_casework_read_only_hits >&2
fi
if rg -n 'fs::(write|remove_|rename|copy|create_dir|set_permissions)|OpenOptions|File::create|OFlags::(WRONLY|RDWR|CREATE|TRUNC|APPEND|TMPFILE)|\b(renameat|unlinkat|mkdirat)\b' "$source_snapshot" >/tmp/nightshift_casework_read_only_hits 2>/dev/null; then
    fail "operator tool contains a filesystem mutation path:"
    cat /tmp/nightshift_casework_read_only_hits >&2
fi
if [ "$(rg -c 'TcpListener::bind\(' "$source_snapshot" || true)" -ne 1 ] || ! rg -q 'TcpListener::bind\(address\)' crates/nightshift-casework/src/server.rs; then
    fail "listener bind is not the exact closed single site"
fi
if ! rg -q 'if !address\.ip\(\)\.is_loopback\(\)'     crates/nightshift-casework/src/server.rs     || ! rg -q 'if !local\.ip\(\)\.is_loopback\(\)'     crates/nightshift-casework/src/server.rs; then
    fail "both requested bind and resulting listener are not loopback checked"
fi
if ! rg -q 'if method != "GET"' crates/nightshift-casework/src/server.rs     || ! rg -q '405, "Method Not Allowed"' crates/nightshift-casework/src/server.rs; then
    fail "non-GET methods do not fail closed with 405"
fi
if rg -n -i 'Access-Control-Allow-Origin|allow_origin|cors' "$source_snapshot" >/tmp/nightshift_casework_read_only_hits 2>/dev/null; then
    fail "operator tool contains a cross-origin permission surface:"
    cat /tmp/nightshift_casework_read_only_hits >&2
fi
for required in     'Content-Security-Policy'     'Cross-Origin-Resource-Policy: same-origin'     'X-Content-Type-Options: nosniff'     'Referrer-Policy: no-referrer'; do
    if ! rg -q "$required" crates/nightshift-casework/src/server.rs; then
        fail "required restrictive response header is missing: $required"
    fi
done
# The final MAP-CABINET tree adds the manifest-closed static UI loader. Count
# its no-follow opens separately from the exact run-input loader so the
# predecessor single-loader check remains true without ignoring new code.
if ! rg -q 'const PACKET_FILE: &str = "packet.v1.json"' crates/nightshift-casework/src/loader.rs || ! rg -q 'const RECEIPTS_FILE: &str = "run-receipts.v1.json"' crates/nightshift-casework/src/loader.rs || [ "$(rg -c '\bopenat\(' crates/nightshift-casework/src/loader.rs || true)" -ne 1 ] || ! rg -q 'OFlags::NOFOLLOW' crates/nightshift-casework/src/loader.rs || ! rg -q 'metadata\.is_file\(\)' crates/nightshift-casework/src/loader.rs; then
    fail "exact run filenames and directory-relative no-follow reads are not structurally pinned"
fi
if [ "$(rg -c '\bopenat\(' crates/nightshift-casework/src/static_ui.rs || true)" -ne 3 ] || ! rg -q 'OFlags::NOFOLLOW' crates/nightshift-casework/src/static_ui.rs || ! rg -q 'Requests never select a filesystem pathname' crates/nightshift-casework/src/static_ui.rs; then
    fail "compiled UI is not pinned to startup-only directory-relative no-follow reads"
fi

if [ "$#" -gt 0 ] && [ "$1" = "--self-test-inject" ]; then
    sandbox=$(mktemp -d)
    trap 'rm -rf "$sandbox" "$source_snapshot" /tmp/nightshift_casework_read_only_hits /tmp/nightshift_casework_selftest' EXIT
    mkdir -p "$sandbox/repo/crates"
    cp -a Cargo.toml Cargo.lock scripts "$sandbox/repo/"
    cp -a crates/nightshift-casework "$sandbox/repo/crates/nightshift-casework"
    printf '\nreqwest = "0.12"\n' >>"$sandbox/repo/crates/nightshift-casework/Cargo.toml"
    printf '%s\n' 'fn injected_boundary_failure() {' '    let _ = std::process::Command::new("local-helper");' '    let _ = std::net::TcpListener::bind("127.0.0.1:0");' '    let _ = std::fs::write("campaign-owned-fixture", b"x");' '    let _ = CanonicalStore::open("canonical.sqlite");' '    let _ = "Access-Control-Allow-Origin: *";' '}' >"$sandbox/repo/crates/nightshift-casework/src/_injected_boundary_failure.rs"

    if (cd "$sandbox/repo" && bash scripts/check_casework_read_only_surface.sh         >/dev/null 2>/tmp/nightshift_casework_selftest); then
        printf 'self-test FAILED: injected boundary transitions passed\n' >&2
        exit 1
    fi
    for marker in subprocess dependency bind filesystem canonical cross-origin; do
        if ! rg -q "$marker" /tmp/nightshift_casework_selftest; then
            printf 'self-test FAILED: injected %s boundary was not reported\n' "$marker" >&2
            cat /tmp/nightshift_casework_selftest >&2
            exit 1
        fi
    done
    printf 'self-test PASSED: alternate bind, filesystem mutation, canonical store, permissive CORS, subprocess, and client-dependency injections were refused.\n' >&2
    exit 0
fi

if [ "$findings" -ne 0 ]; then
    printf 'casework-read-only: FAILED (%d finding(s))\n' "$findings" >&2
    exit 1
fi
printf 'casework-read-only: clean read-only operator surface\n'
