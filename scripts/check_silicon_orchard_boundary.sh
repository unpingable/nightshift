#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
campaign="$root/qualification/ecad-operational-observation-golden-journey-v1-20260831"
bundle="$campaign/source/monitor-bundle.v1.json"
nq="$campaign/source/nq-artifact.v1.json"
conditions="$campaign/conditions"

check_closed() {
  local monitor="$1"
  local qualification="$2"
  [[ "$(jq -r '.schema' "$monitor")" == "monitor.ecad-golden-fixture/v1" ]]
  [[ "$(jq '.entries | length' "$monitor")" == "13" ]]
  [[ "$(jq '.inputs | length' "$qualification")" == "13" ]]
  [[ "$(jq '.contradictions | length' "$qualification")" == "2" ]]
  jq -e 'all(.entries[];
    .signed_monitor_record.body.grants_authority == false and
    ((.exact_payload == null) or .exact_payload.grants_authority == false))' "$monitor" >/dev/null
  ! rg -n -i 'aggregate_(health|result|verdict)|target_effect_authorized' "$monitor" "$qualification" >/dev/null
}

check_closed "$bundle" "$nq"
for scenario in nominal exit-zero-missing-output digest-mismatch wrong-revision wrong-tool wrong-pdk license-no-response worker-loss scheduler-contradiction-a scheduler-contradiction-b stale-artifact delayed-duplicate-delivery agent-contradiction; do
  jq -e --arg scenario "$scenario" '.entries | any(.scenario == $scenario)' "$bundle" >/dev/null
  for file in monitor.v1.json nq.v1.json lineage.v1.json profile.v1.json evaluation.v1.json; do
    [[ -f "$conditions/$scenario/$file" ]]
  done
done

manifest="$root/crates/nightshiftd/Cargo.toml"
[[ "$(rg -c '^\[\[bin\]\]$' "$manifest")" == "2" ]]
[[ "$(find "$root/crates/nightshiftd/src/bin" -maxdepth 1 -type f -name '*.rs' | wc -l)" == "2" ]]

if [[ "${1:-}" == "--self-test-inject" ]]; then
  temporary="$(mktemp -d)"
  trap 'rm -rf -- "$temporary"' EXIT
  cp "$bundle" "$temporary/monitor.json"
  cp "$nq" "$temporary/nq.json"
  printf '\n{"aggregate_health":"green"}\n' >> "$temporary/nq.json"
  if check_closed "$temporary/monitor.json" "$temporary/nq.json"; then
    echo "SILICON boundary self-test did not refuse aggregate injection" >&2
    exit 1
  fi
  echo "SILICON boundary deterministic negative control: passed"
  exit 0
fi

echo "SILICON boundary: closed evidence-only ECAD corpus"
