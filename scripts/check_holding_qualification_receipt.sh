#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
schema="$root/schemas/nightshift.provider-execution-availability-and-deferred-dispatch-qualification.v1.schema.json"
receipt="$root/qualification/provider-execution-availability-and-deferred-dispatch-v1-20260831/holding-pattern.qualification.v1.json"
test_file="$root/tests/test_holding_qualification_receipt_schema.py"

for required in "$schema" "$receipt" "$test_file"; do
  if [[ ! -f "$required" ]]; then
    echo "HOLDING qualification receipt: missing checked contract artifact"
    exit 1
  fi
done

if [[ "${1:-}" == "--self-test-inject" ]]; then
  fixture="$(mktemp -d)"
  trap 'rm -rf -- "$fixture"' EXIT
  injected="$fixture/substituted-receipt.json"
  python3 -c '
import json
import pathlib
import sys
value = json.loads(pathlib.Path(sys.argv[1]).read_bytes())
value["aggregate_result"] = "QUALIFIED"
pathlib.Path(sys.argv[2]).write_text(json.dumps(value), encoding="utf-8")
' "$receipt" "$injected"
  if env NIGHTSHIFT_HOLDING_QUALIFICATION_RECEIPT="$injected" python3 "$test_file" >"$fixture/output" 2>&1; then
    echo "HOLDING qualification receipt negative control did not refuse substitution"
    exit 1
  fi
  if ! rg -q "Additional properties are not allowed" "$fixture/output"; then
    cat "$fixture/output"
    echo "HOLDING qualification receipt negative control failed without the closed-field disposition"
    exit 1
  fi
  echo "HOLDING qualification receipt deterministic substitution control: passed"
  exit 0
fi

python3 "$test_file"
echo "HOLDING qualification receipt: closed schema and exact receipt passed"
