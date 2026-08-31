#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
store="$root/crates/nightshift-foreman/src/store.rs"
casework="$root/crates/nightshift-casework/src/loader.rs"
renderer="$root/scripts/render_nightshift_reports.py"

check_contract() {
  local checked_store="$1"
  local checked_casework="$2"
  local checked_renderer="$3"
  rg -q 'struct FinalQuestion' "$checked_store" || return 1
  rg -q 'exact_question: String' "$checked_store" || return 1
  rg -q 'exact_question: question[.]question[.]clone[(][)]' "$checked_store" || return 1
  if rg -q '^[[:space:]]*question: question[.]question[.]clone[(][)]' "$checked_store"; then
    return 1
  fi
  rg -q '"exact_question"' "$checked_casework" || return 1
  rg -q "question\\['exact_question'\\]" "$checked_renderer" || return 1
}

if ! check_contract "$store" "$casework" "$renderer"; then
  echo "foreman final-question compatibility boundary: exact sealed vocabulary missing"
  exit 1
fi

if [[ "${1:-}" == "--self-test-inject" ]]; then
  fixture="$(mktemp -d)"
  trap 'rm -rf -- "$fixture"' EXIT
  mkdir -p "$fixture/foreman" "$fixture/casework" "$fixture/scripts"
  cp "$store" "$fixture/foreman/store.rs"
  cp "$casework" "$fixture/casework/loader.rs"
  cp "$renderer" "$fixture/scripts/render.py"
  perl -0pi -e 's/exact_question: question[.]question[.]clone[(][)]/question: question.question.clone()/g' \
    "$fixture/foreman/store.rs"
  if check_contract \
      "$fixture/foreman/store.rs" \
      "$fixture/casework/loader.rs" \
      "$fixture/scripts/render.py"; then
    echo "foreman final-question compatibility negative control did not fail"
    exit 1
  fi
  echo "foreman final-question compatibility deterministic negative control: passed"
  exit 0
fi

echo "foreman final-question compatibility boundary: passed"
