#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target="$root/crates/nightshift-foreman/src"

check_tree() {
  local tree="$1"
  if rg -n 'std::process::Command|Command::new|send_approval|respond_approval|retry_attempt|automatic_retry|ag-loopctl|ag-effectd|docket execute' "$tree" >/dev/null; then
    return 1
  fi
}

check_tree "$target" || {
  echo "foreman boundary gate: forbidden subprocess, approval-response, retry, or target-actuator surface"
  exit 1
}

manifest="$root/crates/nightshiftd/Cargo.toml"
bin_count="$(rg -c '^\[\[bin\]\]$' "$manifest")"
source_bin_count="$(find "$root/crates/nightshiftd/src/bin" -maxdepth 1 -type f -name '*.rs' | wc -l)"
if [[ "$bin_count" != "2" || "$source_bin_count" != "2" || -e "$root/crates/nightshiftd/src/main.rs" ]]; then
  echo "foreman boundary gate: canonical nightshiftd production graph changed"
  exit 1
fi

if rg -n 'aggregate_result|aggregate_health' "$target" >/dev/null; then
  echo "foreman boundary gate: aggregate result surface present"
  exit 1
fi

if [[ "${1:-}" == "--self-test-inject" ]]; then
  fixture="$(mktemp -d)"
  trap 'rm -rf -- "$fixture"' EXIT
  cp -R "$target" "$fixture/src"
  printf '\nfn forbidden_fixture() { let _ = std::process::Command::new("docket"); }\n' >> "$fixture/src/lib.rs"
  if check_tree "$fixture/src"; then
    echo "foreman boundary gate negative control did not fail"
    exit 1
  fi
  echo "foreman boundary gate deterministic negative control: passed"
  exit 0
fi

echo "foreman boundary gate: passed"
