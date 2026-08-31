#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
owner="$root/crates/nightshiftd/src/operational_lineage.rs"
lineage_schema="$root/schemas/nightshift.operational-observation-lineage.v1.schema.json"
evaluation_schema="$root/schemas/nightshift.operational-reobservation-evaluation.v1.schema.json"

check_owner() {
  local source="$1"
  if rg -n 'std::process|Command::new|TcpListener|TcpStream|rusqlite|ag-loopctl|ag-effectd|docket|casework|automatic_retry|retry_attempt|dispatch_command|approval_response' "$source" >/dev/null; then
    return 1
  fi
  rg -q 'FIELD_CLOCK_MONITOR_RESULT_HEAD' "$source" || return 1
  rg -q 'FIELD_CLOCK_NQ_RESULT_HEAD' "$source" || return 1
  rg -q 'grants_authority: false' "$source" || return 1
  if rg -n 'aggregate_(result|health|verdict)' "$source" >/dev/null; then
    return 1
  fi
}

check_schema() {
  local schema="$1"
  rg -q '"additionalProperties": false' "$schema" || return 1
  if rg -n '"const": true|"aggregate_(result|health|verdict)"' "$schema" >/dev/null; then
    return 1
  fi
}

check_owner "$owner" || {
  echo "operational-lineage boundary gate: owner gained an office call, actuator, aggregate, or unpinned boundary"
  exit 1
}
check_schema "$lineage_schema" || {
  echo "operational-lineage boundary gate: immutable lineage schema is open or authorizing"
  exit 1
}
check_schema "$evaluation_schema" || {
  echo "operational-lineage boundary gate: evaluation schema is open or authorizing"
  exit 1
}

manifest="$root/crates/nightshiftd/Cargo.toml"
bin_count="$(rg -c '^\[\[bin\]\]$' "$manifest")"
source_bin_count="$(find "$root/crates/nightshiftd/src/bin" -maxdepth 1 -type f -name '*.rs' | wc -l)"
if [[ "$bin_count" != "2" || "$source_bin_count" != "2" || -e "$root/crates/nightshiftd/src/main.rs" ]]; then
  echo "operational-lineage boundary gate: canonical nightshiftd production graph changed"
  exit 1
fi

if [[ "${1:-}" == "--self-test-inject" ]]; then
  fixture="$(mktemp -d)"
  trap 'rm -rf -- "$fixture"' EXIT
  cp "$owner" "$fixture/owner.rs"
  printf '\nfn injected_office_call() { let _ = std::process::Command::new("docket"); }\n' >> "$fixture/owner.rs"
  if check_owner "$fixture/owner.rs"; then
    echo "operational-lineage boundary gate: office-call negative control did not fail"
    exit 1
  fi
  cp "$evaluation_schema" "$fixture/evaluation.json"
  perl -0pi -e 's/"const": false/"const": true/' "$fixture/evaluation.json"
  if check_schema "$fixture/evaluation.json"; then
    echo "operational-lineage boundary gate: authority negative control did not fail"
    exit 1
  fi
  echo "operational-lineage boundary deterministic negative controls: passed"
  exit 0
fi

echo "operational-lineage boundary gate: passed"
