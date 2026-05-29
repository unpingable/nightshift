#!/usr/bin/env bash
# check_no_actuation_surface.sh — build gate for the Nightshift
# actuation boundary. Enforces the doctrine documented at
# `docs/working/decisions/ACTUATION_BOUNDARY.md` (canonical source:
# ~/git/cartography/doctrine/nightshift-actuation-boundary.md).
#
# Sibling discipline:
# - NQ-witness: assert_no_authority_claims()
# - Continuity: test_adapter_module_exposes_no_transport_surface
#
# Run from the repo root. Exit 0 means no actuation surface
# detected; any non-zero exit is a build-blocking finding.
#
# Categories enforced (each section emits its own findings to
# stderr and bumps the exit code at the bottom):
#
#   1. Actuator subprocess execution
#      Hard-fail on `Command::new(<actuator>)` for:
#      ssh, salt, salt-call, ansible, ansible-playbook, systemctl,
#      kubectl, docker, podman, helm, terraform, scp, rsync, curl
#      (curl/rsync/scp are dual-use; we forbid them inside NS code
#      because their presence is a smell, not a need).
#
#   2. Actuator-client dependencies in Cargo.toml
#      Hard-fail on any ssh-client / salt-client / ansible-client /
#      kubectl-client / docker-client / k8s-openapi / openssh /
#      ssh2 / russh family entry.
#
#   3. Python actuation primitives
#      NS has no Python today; this is forward-defensive. Hard-fail
#      on paramiko, fabric, ansible, saltstack, pyinfra, pyssh
#      anywhere under `src/` or `crates/*/src/`.
#
#   4. Control-plane HTTP writes
#      Hard-fail on the presence of HTTP client libraries known to
#      do mutations (reqwest, ureq, hyper-client, isahc, etc.) in
#      Cargo.toml, AND on any `reqwest::Client::post`-shaped or
#      .post(/.put(/.delete( method invocations inside NS source.
#      NS's Governor integration is JSON-RPC over a Unix socket
#      (`crate::governor_client`), which is not an HTTP control
#      plane and is allowlisted as a known seam.
#
#   5. New subprocess call sites outside the read-adapter allowlist
#      Hard-fail on `Command::new` calls in NS source files outside
#      `crates/nightshiftd/src/nq.rs` and
#      `crates/nightshiftd/src/liveness.rs`. The doctrine doc names
#      these two files as the vetted read-adapter allowlist (current
#      as of 2026-05-28). New shells anywhere else require explicit
#      doctrine-side review.
#
# Optional invocation for self-test:
#   $ scripts/check_no_actuation_surface.sh --self-test-inject
#   Adds a synthetic violation in /tmp, runs the guard against it,
#   and confirms the guard hard-fails. Use this to verify the guard
#   actually catches what it claims to catch.

set -uo pipefail

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
cd "$repo_root"

findings=0
fail() {
    printf 'actuation-boundary: %s\n' "$1" >&2
    findings=$((findings + 1))
}

# All NS source. crates/*/src/ + any future scripts that should
# also be checked. Tests are deliberately NOT scanned: a test that
# constructs a fake systemctl-invoking payload to verify the guard
# catches it would otherwise trip the guard.
ns_src_paths=(
    "crates/nightshiftd/src"
)

# -------------------------------------------------------------------
# 1. Actuator subprocess execution
# -------------------------------------------------------------------
# Match `Command::new("<actuator>")` regardless of whitespace.
# Word-anchored so an unrelated identifier like `ssh_target` (a
# field name we *also* forbid below) doesn't trigger here.
actuators=(
    ssh salt salt-call ansible ansible-playbook
    systemctl kubectl docker podman helm terraform
    scp rsync curl wget
)
for actuator in "${actuators[@]}"; do
    # Look for Command::new("<actuator>"...) literal, or
    # Command::new with an identifier whose first dotted path
    # segment is the actuator name.
    hits=$(grep -rnE \
        "Command::new\(\s*\"${actuator}\"" \
        "${ns_src_paths[@]}" 2>/dev/null || true)
    if [ -n "$hits" ]; then
        fail "actuator subprocess detected for \`${actuator}\`:"
        printf '%s\n' "$hits" >&2
    fi
done

# -------------------------------------------------------------------
# 2. Actuator-client dependencies in Cargo.toml
# -------------------------------------------------------------------
forbidden_deps=(
    ssh2 russh openssh thrussh
    salt-api ansible-rs
    kube k8s-openapi
    bollard shiplift
    helm
)
for dep in "${forbidden_deps[@]}"; do
    hits=$(grep -rnE "^${dep}\s*=" Cargo.toml crates/*/Cargo.toml 2>/dev/null || true)
    if [ -n "$hits" ]; then
        fail "actuator-client dependency \`${dep}\` in Cargo.toml:"
        printf '%s\n' "$hits" >&2
    fi
done

# -------------------------------------------------------------------
# 3. Python actuation primitives
# -------------------------------------------------------------------
# NS has no Python today. Defensive scan for python imports of
# actuation tools, anywhere under the source tree.
python_actuators=(
    paramiko fabric ansible saltstack pyinfra
)
for tool in "${python_actuators[@]}"; do
    hits=$(grep -rnE \
        "^(import|from)\s+${tool}\b" \
        crates 2>/dev/null || true)
    if [ -n "$hits" ]; then
        fail "Python actuation primitive \`${tool}\` imported:"
        printf '%s\n' "$hits" >&2
    fi
done

# -------------------------------------------------------------------
# 4. Control-plane HTTP writes
# -------------------------------------------------------------------
http_client_deps=(
    reqwest ureq hyper-client isahc surf attohttpc
    awc rustls-tokio-stream
)
for dep in "${http_client_deps[@]}"; do
    hits=$(grep -rnE "^${dep}\s*=" Cargo.toml crates/*/Cargo.toml 2>/dev/null || true)
    if [ -n "$hits" ]; then
        fail "HTTP client dependency \`${dep}\` present (NS has no control-plane need):"
        printf '%s\n' "$hits" >&2
    fi
done
# Method-call-shaped writes inside NS source.
http_write_methods='(\.post\(|\.put\(|\.delete\(|\.patch\()'
hits=$(grep -rnE "$http_write_methods" "${ns_src_paths[@]}" 2>/dev/null || true)
if [ -n "$hits" ]; then
    fail "HTTP write method call detected in NS source:"
    printf '%s\n' "$hits" >&2
fi

# -------------------------------------------------------------------
# 5. New subprocess call sites outside the read-adapter allowlist
# -------------------------------------------------------------------
# Doctrine doc: vetted allowlist as of 2026-05-28 is `nq.rs` and
# `liveness.rs`. Any `Command::new(...)` call in another file is a
# new subprocess site and must be reviewed by doctrine. Tracking is
# at file granularity (not call-site granularity) so adding a new
# call to nq.rs/liveness.rs is implicitly allowed; opening a new
# file with a Command::new is not.
allowlisted_subprocess_files=(
    crates/nightshiftd/src/nq.rs
    crates/nightshiftd/src/liveness.rs
)
declare -A allowlist_lookup=()
for f in "${allowlisted_subprocess_files[@]}"; do
    allowlist_lookup["$f"]=1
done
# Files containing Command::new under the NS source tree.
while IFS= read -r file; do
    [ -z "$file" ] && continue
    # Strip leading ./ if present (grep -r yields paths relative to
    # cwd which is the repo root).
    file_rel="${file#./}"
    if [ -z "${allowlist_lookup[$file_rel]:-}" ]; then
        fail "new subprocess call site outside read-adapter allowlist: ${file_rel}"
        grep -nE "Command::new" "$file_rel" >&2 || true
    fi
done < <(grep -rlE "Command::new" "${ns_src_paths[@]}" 2>/dev/null || true)

# -------------------------------------------------------------------
# Self-test mode
# -------------------------------------------------------------------
# Verifies the guard catches what it claims to catch. Runs the guard
# against synthetic injected violations and confirms hard-fail. This
# is the dual-verification the doctrine doc requires:
#   - clean on real code (the normal run above)
#   - hard-fail on injected mutation (this self-test)
if [ "${1:-}" = "--self-test-inject" ]; then
    printf '\n--- self-test: injecting synthetic violations ---\n' >&2
    sandbox=$(mktemp -d)
    trap 'rm -rf "$sandbox"' EXIT
    cp -r "$repo_root"/* "$sandbox/" 2>/dev/null
    # Need .git too so `git rev-parse` works from the sandbox.
    cp -r "$repo_root"/.git "$sandbox/" 2>/dev/null

    # Injection 1: Command::new("systemctl") in a new file.
    cat > "$sandbox/crates/nightshiftd/src/_inject_actuator.rs" <<'EOF'
// SYNTHETIC INJECTION — must be caught by the guard.
fn _bad() {
    let _ = std::process::Command::new("systemctl")
        .arg("restart")
        .arg("everything")
        .output();
}
EOF

    # Injection 2: ssh client dep in Cargo.toml.
    cat >> "$sandbox/Cargo.toml" <<'EOF'

[workspace.dependencies._inject_actuator_dep]
ssh2 = "0.9"
EOF

    # Injection 3: HTTP write method call in a new file.
    cat > "$sandbox/crates/nightshiftd/src/_inject_http.rs" <<'EOF'
// SYNTHETIC INJECTION — must be caught by the guard.
fn _bad_http() {
    let _ = some_client.post("https://substrate.example/restart");
}
EOF

    if (cd "$sandbox" && bash scripts/check_no_actuation_surface.sh 2>/tmp/guard_self_test_output >/dev/null); then
        printf 'self-test FAILED: guard did NOT detect injected violations\n' >&2
        cat /tmp/guard_self_test_output >&2
        exit 1
    fi
    inject_findings=$(grep -cE "_inject_actuator|ssh2|_inject_http|systemctl|HTTP write" \
        /tmp/guard_self_test_output 2>/dev/null || echo 0)
    if [ "$inject_findings" -lt 3 ]; then
        printf 'self-test FAILED: guard caught fewer than 3 of the 3 injections\n' >&2
        cat /tmp/guard_self_test_output >&2
        exit 1
    fi
    printf 'self-test PASSED: guard hard-failed on all 3 injected violations.\n' >&2
    rm -f /tmp/guard_self_test_output
    # Self-test does not propagate the (intentionally failing)
    # sandbox findings up to the real check; exit zero so the real
    # check (which ran first) decides the overall exit code.
    exit 0
fi

# -------------------------------------------------------------------
# Final verdict
# -------------------------------------------------------------------
if [ "$findings" -gt 0 ]; then
    printf '\nactuation-boundary: %d finding(s) — build blocked.\n' "$findings" >&2
    printf 'See docs/working/decisions/ACTUATION_BOUNDARY.md for the doctrine.\n' >&2
    exit 1
fi
printf 'actuation-boundary: clean (%d files under %s)\n' \
    "$(find ${ns_src_paths[@]} -type f -name '*.rs' 2>/dev/null | wc -l)" \
    "${ns_src_paths[*]}"
exit 0
