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

check_query_only_tree() {
  local tree="$1"
  local store="$tree/store.rs"
  local cli="$tree/bin/nightshift_foreman.rs"
  rg -q 'mode=ro' "$store" || return 1
  rg -q 'immutable=1' "$store" || return 1
  if rg -q 'mode=rwc' "$store"; then
    return 1
  fi
  rg -q 'O_NOFOLLOW' "$store" || return 1
  rg -q '/proc/self/fd/' "$store" || return 1
  rg -q 'ForemanStore::open_read_only' "$cli" || return 1
  if rg -n 'ForemanStore::open\(db\)\?\.(projection|worker_brief|export_events|export_final)' "$cli" >/dev/null; then
    return 1
  fi
}

check_tree "$target" || {
  echo "foreman boundary gate: forbidden subprocess, approval-response, retry, or target-actuator surface"
  exit 1
}

check_query_only_tree "$target" || {
  echo "foreman boundary gate: query-only store custody is incomplete"
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
  query_fixture="$(mktemp -d)"
  trap 'rm -rf -- "$fixture" "$query_fixture"' EXIT
  cp -R "$target" "$query_fixture/src"
  perl -0pi -e 's/mode=ro/mode=rwc/g' "$query_fixture/src/store.rs"
  if check_query_only_tree "$query_fixture/src"; then
    echo "foreman query-only boundary negative control did not fail"
    exit 1
  fi
  echo "foreman boundary gate deterministic negative controls: passed"
  exit 0
fi

echo "foreman boundary gate: passed"
