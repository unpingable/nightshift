#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
foreman="$root/crates/nightshift-foreman/src"
store="$foreman/store.rs"

check_mechanism() {
  local tree="$1"
  local candidate="$tree/store.rs"
  rg -q 'MAXIMUM_EXECUTION_AVAILABILITY_HISTORY_BYTES: u64 = 16 \* 1024 \* 1024' "$candidate" || return 1
  rg -q 'validate_execution_availability_history_size' "$candidate" || return 1
  rg -q 'prepare_provider_attempt' "$candidate" || return 1
  rg -q 'availability-required run refuses legacy V2 start path' "$candidate" || return 1
  rg -q 'TransactionBehavior::Immediate' "$candidate" || return 1
  if rg -n 'std::process::Command|TcpListener|UdpSocket|thread::sleep|send_approval|respond_approval|approval_response_authorized:\s*true|automatic_semantic_retry:\s*true' "$tree" >/dev/null; then
    return 1
  fi
}

check_mechanism "$foreman" || {
  echo "HOLDING mechanism boundary: missing metadata-first/atomic law or forbidden activation surface"
  exit 1
}

manifest="$root/crates/nightshiftd/Cargo.toml"
bin_count="$(rg -c '^\[\[bin\]\]$' "$manifest")"
source_bin_count="$(find "$root/crates/nightshiftd/src/bin" -maxdepth 1 -type f -name '*.rs' | wc -l)"
if [[ "$bin_count" != "2" || "$source_bin_count" != "2" || -e "$root/crates/nightshiftd/src/main.rs" ]]; then
  echo "HOLDING mechanism boundary: canonical nightshiftd production graph changed"
  exit 1
fi

if [[ "${1:-}" == "--self-test-inject" ]]; then
  fixture="$(mktemp -d)"
  trap 'rm -rf -- "$fixture"' EXIT
  cp -R "$foreman" "$fixture/src"
  printf '\nfn forbidden_fixture() { let _ = std::process::Command::new("provider"); }\n' >> "$fixture/src/lib.rs"
  if check_mechanism "$fixture/src"; then
    echo "HOLDING mechanism boundary deterministic negative control did not fail"
    exit 1
  fi
  echo "HOLDING mechanism boundary deterministic negative control: passed"
  exit 0
fi

echo "HOLDING mechanism boundary: passed"
