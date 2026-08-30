#!/usr/bin/env bash
# Structural gate for the read-only provider-capacity adapter and policy.

set -uo pipefail
repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
cd "$repo_root"
findings=0
fail() {
    printf 'provider-capacity-boundary: %s\n' "$1" >&2
    findings=$((findings + 1))
}
manifest="crates/nightshift-provider-capacity/Cargo.toml"
source_root="crates/nightshift-provider-capacity/src"
snapshot=$(mktemp)
hits=$(mktemp)
trap 'rm -f "$snapshot" "$hits"' EXIT

if [ ! -f "$manifest" ] || [ ! -d "$source_root" ]; then
    fail "manifest or production source is missing"
else
    while IFS= read -r source; do
        printf '\n/* source: %s */\n' "$source" >>"$snapshot"
        awk '/^#\[cfg\(test\)\]/{exit} {print}' "$source" >>"$snapshot"
    done < <(rg --files "$source_root" | sort)
fi
if ! rg -q '^autobins = false$' "$manifest"; then
    fail "automatic binary discovery is not disabled"
fi
if [ "$(rg -c '^\[\[bin\]\]$' "$manifest" || true)" -ne 1 ] ||
   ! rg -q '^name = "nightshift-provider-capacity"$' "$manifest"; then
    fail "operator binary graph is not the exact single explicit target"
fi
if [ "$(rg -c 'Command::new\("codex"\)' "$snapshot" || true)" -ne 1 ] ||
   ! rg -q '\.args\(\["app-server", "--listen", "stdio://"\]\)' "$snapshot"; then
    fail "probe command is not pinned to one foreground Codex App Server stdio process"
fi
if [ "$(rg -c '"account/rateLimits/read"' "$snapshot" || true)" -ne 2 ]; then
    fail "supported read method is not pinned in request and evidence"
fi
if rg -n 'rateLimitResetCredit|account/(login|logout)|thread/(start|resume)|turn/start|process/spawn' "$snapshot" >"$hits"; then
    fail "provider mutation, session, model-turn, or process API is present"
    cat "$hits" >&2
fi
if rg -n -i 'portable[_-]?pty|expectrl|chatgpt\.com|browser|cookie|auth\.json|config\.toml|sessions/' "$snapshot" >"$hits"; then
    fail "PTY, web, browser-session, configuration, or credential access is present"
    cat "$hits" >&2
fi
if rg -n 'Tcp(Stream|Listener)|UdpSocket|reqwest|ureq|hyper::client|WebSocket' "$snapshot" "$manifest" >"$hits"; then
    fail "independent network client or listener is present"
    cat "$hits" >&2
fi
if rg -n 'fs::(write|remove_|rename|copy|create_dir|set_permissions)|OpenOptions|File::create' "$snapshot" >"$hits"; then
    fail "provider adapter contains a filesystem mutation path"
    cat "$hits" >&2
fi
if rg -n -i 'aggregate[_ -]?(result|verdict|health)|overall[_ -]?(result|status|health)' "$snapshot" schemas/nightshift.provider-capacity-*.schema.json >"$hits"; then
    fail "aggregate classification or health synthesis is present"
    cat "$hits" >&2
fi
for schema in schemas/nightshift.provider-capacity-observation.v1.schema.json \
              schemas/nightshift.provider-capacity-policy.v1.schema.json \
              schemas/nightshift.provider-capacity-decision.v1.schema.json; do
    if [ ! -f "$schema" ] || ! rg -q '"additionalProperties": false' "$schema"; then
        fail "closed capacity schema is missing or open: $schema"
    fi
done
if rg --files deploy/systemd deploy/systemd/user 2>/dev/null | rg 'provider-capacity|nightshift-provider-capacity' >"$hits"; then
    fail "provider-capacity timer or service was installed"
    cat "$hits" >&2
fi

if [ "$#" -gt 0 ] && [ "$1" = "--self-test-inject" ]; then
    sandbox=$(mktemp -d)
    self_log=$(mktemp)
    trap 'rm -rf "$sandbox"; rm -f "$snapshot" "$hits" "$self_log"' EXIT
    mkdir -p "$sandbox/repo/crates" "$sandbox/repo/schemas" "$sandbox/repo/deploy/systemd/user"
    cp -a Cargo.toml Cargo.lock scripts "$sandbox/repo/"
    cp -a crates/nightshift-provider-capacity "$sandbox/repo/crates/"
    cp schemas/nightshift.provider-capacity-*.schema.json "$sandbox/repo/schemas/"
    printf '%s\n' \
      'fn injected_boundary_failure() {' \
      ' let _ = std::net::TcpStream::connect("127.0.0.1:9");' \
      ' let _ = "account/login/start";' \
      ' let _ = "turn/start";' \
      ' let _ = "auth.json";' \
      ' let _ = std::fs::write("provider-state", b"x");' \
      ' let _ = "aggregate health";' \
      '}' >"$sandbox/repo/crates/nightshift-provider-capacity/src/injected.rs"
    printf '%s\n' '[Timer]' 'OnCalendar=hourly' >"$sandbox/repo/deploy/systemd/user/nightshift-provider-capacity.timer"
    if (cd "$sandbox/repo" && bash scripts/check_provider_capacity_boundary.sh >/dev/null 2>"$self_log"); then
        printf 'self-test FAILED: deterministic boundary substitutions passed\n' >&2
        exit 1
    fi
    for marker in mutation configuration network filesystem aggregate timer; do
        if ! rg -q "$marker" "$self_log"; then
            printf 'self-test FAILED: %s substitution was not reported\n' "$marker" >&2
            cat "$self_log" >&2
            exit 1
        fi
    done
    printf 'self-test PASSED: provider mutation, session/configuration access, independent network, filesystem mutation, aggregate synthesis, and timer substitutions were refused.\n' >&2
    exit 0
fi
if [ "$findings" -ne 0 ]; then
    printf 'provider-capacity-boundary: FAILED (%d finding(s))\n' "$findings" >&2
    exit 1
fi
printf 'provider-capacity-boundary: clean read-only provider adapter and policy\n'
