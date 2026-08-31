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
  [[ "$(jq '.entries | length' "$monitor")" == "20" ]]
  [[ "$(jq -r '.schema' "$qualification")" == "nq.ecad-qualification-bundle/v1" ]]
  [[ "$(jq '.cases | length' "$qualification")" == "20" ]]
  [[ "$(jq '.claim_deck.required_claims | length' "$qualification")" == "27" ]]
  [[ "$(jq -r '.monitor_fixture_head' "$qualification")" == "b00609c8d13c09f68debc884ec1a56b77f1234ce" ]]
  jq -e 'all(.entries[];
    .signed_monitor_record.body.grants_authority == false and
    ((.exact_payload == null) or .exact_payload.grants_authority == false))' "$monitor" >/dev/null
  jq -e '
    (.entries | map(.signed_monitor_record.body.producer.producer_class) | unique | length) == 8 and
    (.entries | map(.signed_monitor_record.body.producer.public_key_digest) | unique | length) == 8 and
    (.distant_traversal.partition_attempt_record.outcome == "partition") and
    (.distant_traversal.first_custody_attempt_record.outcome == "custody_confirmed") and
    (.distant_traversal.retry_custody_attempt_record.outcome == "custody_confirmed") and
    (.distant_traversal.retained_sender_attempts | length) == 3 and
    (.distant_traversal.sender_pending_ids_after_reopen | length) == 0 and
    (.distant_traversal.duplicate_converged == true) and
    (.distant_traversal.grants_authority == false)' "$monitor" >/dev/null
  jq -e '
    (.claim_deck.requires_full_evidence == true) and
    (.claim_deck.process_exit_alone_is_sufficient == false) and
    (.claim_deck.grants_authority == false) and
    (.cases | map(select(.scenario == "nominal"))[0].eligibility.disposition == "evidence_eligible") and
    (.cases | map(select(.scenario == "exit-zero-missing-output"))[0].eligibility.disposition == "evidence_not_established") and
    (.cases | map(select(.scenario == "healthy-wrong-subject"))[0].eligibility.disposition == "refused") and
    (.distant_intake_binding.retained_receiver_inbox != null) and
    (.distant_intake_binding.retained_receiver_receipt != null) and
    (.distant_intake_binding.retained_receiver_lineage != null) and
    (.distant_intake_binding.retained_sender_delivered != null) and
    (.distant_intake_binding.retained_sender_attempts | length) == 3 and
    (.distant_intake_binding.grants_authority == false)' "$qualification" >/dev/null
  ! rg -n -i 'aggregate_(health|result|verdict)|target_effect_authorized' "$monitor" "$qualification" >/dev/null
}

check_closed "$bundle" "$nq"
for scenario in nominal exit-zero-missing-output digest-mismatch wrong-design-revision wrong-revision wrong-tool wrong-pdk license-unavailable-before-start license-no-response healthy-wrong-subject worker-loss repository-custody-historical repository-custody-successor scheduler-running-source worker-absent-source scheduler-contradiction-a scheduler-contradiction-b stale-artifact delayed-duplicate-delivery agent-contradiction; do
  jq -e --arg scenario "$scenario" '.entries | any(.scenario == $scenario)' "$bundle" >/dev/null
  jq -e --arg scenario "$scenario" '.cases | any(.scenario == $scenario)' "$nq" >/dev/null
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
