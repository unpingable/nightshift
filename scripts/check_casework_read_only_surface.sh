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

if [ ! -f "$manifest" ] || [ ! -d "$production_src" ]; then
    fail "casework manifest or production source is missing"
fi
if ! rg -q '^autobins = false$' "$manifest"; then
    fail "automatic binary discovery is not disabled"
fi
if [ "$(rg -c '^\[\[bin\]\]$' "$manifest" || true)" -ne 1 ]     || ! rg -q '^name = "nightshift-casework"$' "$manifest"     || ! rg -q '^path = "src/bin/nightshift_casework.rs"$' "$manifest"; then
    fail "operator binary graph is not the exact single explicit target"
fi

if rg -n '(^|[^[:alnum:]_])(std::process|process::Command|Command::new)'     "$production_src" >/tmp/nightshift_casework_read_only_hits 2>/dev/null; then
    fail "operator tool opens a subprocess:"
    cat /tmp/nightshift_casework_read_only_hits >&2
fi
if rg -n '(CanonicalStore|canonical_store|rusqlite|ag_port|nq_admission|docket|switchyard)'     "$production_src" "$manifest" >/tmp/nightshift_casework_read_only_hits 2>/dev/null; then
    fail "operator tool references a canonical store or adjacent office:"
    cat /tmp/nightshift_casework_read_only_hits >&2
fi
if rg -n '^[[:space:]]*(reqwest|ureq|isahc|hyper|rusqlite)[[:space:]]*='     "$manifest" >/tmp/nightshift_casework_read_only_hits 2>/dev/null; then
    fail "operator tool declares a client or database dependency:"
    cat /tmp/nightshift_casework_read_only_hits >&2
fi
if rg -n '(TcpStream::connect|UdpSocket|ToSocketAddrs|reqwest::|ureq::)'     "$production_src" >/tmp/nightshift_casework_read_only_hits 2>/dev/null; then
    fail "operator tool contains an outbound network path:"
    cat /tmp/nightshift_casework_read_only_hits >&2
fi
if rg -n 'fs::(write|remove_|rename|copy|create_dir)|OpenOptions|File::create'     "$production_src" >/tmp/nightshift_casework_read_only_hits 2>/dev/null; then
    fail "operator tool contains a filesystem mutation path:"
    cat /tmp/nightshift_casework_read_only_hits >&2
fi
if ! rg -q 'if !address\.ip\(\)\.is_loopback\(\)'     crates/nightshift-casework/src/server.rs     || ! rg -q 'if !local\.ip\(\)\.is_loopback\(\)'     crates/nightshift-casework/src/server.rs; then
    fail "both requested bind and resulting listener are not loopback checked"
fi
if ! rg -q 'if method != "GET"' crates/nightshift-casework/src/server.rs     || ! rg -q '405, "Method Not Allowed"' crates/nightshift-casework/src/server.rs; then
    fail "non-GET methods do not fail closed with 405"
fi
if rg -n -i 'Access-Control-Allow-Origin|allow_origin|cors'     "$production_src" >/tmp/nightshift_casework_read_only_hits 2>/dev/null; then
    fail "operator tool contains a cross-origin permission surface:"
    cat /tmp/nightshift_casework_read_only_hits >&2
fi
for required in     'Content-Security-Policy'     'Cross-Origin-Resource-Policy: same-origin'     'X-Content-Type-Options: nosniff'     'Referrer-Policy: no-referrer'; do
    if ! rg -q "$required" crates/nightshift-casework/src/server.rs; then
        fail "required restrictive response header is missing: $required"
    fi
done
if ! rg -q 'const PACKET_FILE: &str = "packet.v1.json"'     crates/nightshift-casework/src/loader.rs     || ! rg -q 'const RECEIPTS_FILE: &str = "run-receipts.v1.json"'     crates/nightshift-casework/src/loader.rs     || ! rg -q 'fs::symlink_metadata' crates/nightshift-casework/src/loader.rs; then
    fail "exact run filenames or symlink refusal are not structurally pinned"
fi

if [ "$#" -gt 0 ] && [ "$1" = "--self-test-inject" ]; then
    sandbox=$(mktemp -d)
    trap 'rm -rf "$sandbox" /tmp/nightshift_casework_read_only_hits /tmp/nightshift_casework_selftest' EXIT
    mkdir -p "$sandbox/repo/crates"
    cp -a Cargo.toml Cargo.lock scripts "$sandbox/repo/"
    cp -a crates/nightshift-casework "$sandbox/repo/crates/nightshift-casework"
    printf '\nreqwest = "0.12"\n' >>"$sandbox/repo/crates/nightshift-casework/Cargo.toml"
    printf '%s\n'         'fn injected_boundary_failure() {'         '    let _ = std::process::Command::new("local-helper");'         '}' >"$sandbox/repo/crates/nightshift-casework/src/_injected_boundary_failure.rs"

    if (cd "$sandbox/repo" && bash scripts/check_casework_read_only_surface.sh         >/dev/null 2>/tmp/nightshift_casework_selftest); then
        printf 'self-test FAILED: injected write-capable surfaces passed\n' >&2
        exit 1
    fi
    for marker in subprocess dependency; do
        if ! rg -q "$marker" /tmp/nightshift_casework_selftest; then
            printf 'self-test FAILED: injected %s boundary was not reported\n' "$marker" >&2
            cat /tmp/nightshift_casework_selftest >&2
            exit 1
        fi
    done
    printf 'self-test PASSED: subprocess and client-dependency injections were refused.\n' >&2
    exit 0
fi

rm -f /tmp/nightshift_casework_read_only_hits
if [ "$findings" -ne 0 ]; then
    printf 'casework-read-only: FAILED (%d finding(s))\n' "$findings" >&2
    exit 1
fi
printf 'casework-read-only: clean read-only operator surface\n'
