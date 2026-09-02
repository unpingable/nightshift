#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source_file="${NIGHTSHIFT_SECOND_WATCH_BOOTSTRAP_SOURCE:-$root/crates/nightshift-foreman/src/bootstrap.rs}"
store_file="$root/crates/nightshift-foreman/src/store.rs"
cli_file="$root/crates/nightshift-foreman/src/bin/nightshift_foreman.rs"
driver_schema="$root/schemas/nightshift.self-hosted-foreman-driver-step.v1.schema.json"
schema="$root/schemas/nightshift.self-hosted-foreman-bootstrap.v1.schema.json"
architecture="$root/docs/architecture/SELF_HOSTED_FOREMAN_BOOTSTRAP_V1.md"
topology="$root/qualification/nightshift-self-hosted-foreman-bootstrap-v1-20260831/PRE-MUTATION-TOPOLOGY.md"
schema_test="$root/tests/test_self_hosted_bootstrap_schema.py"

for required in "$source_file" "$store_file" "$cli_file" "$schema" "$driver_schema" "$architecture" "$topology" "$schema_test"; do
  if [[ ! -f "$required" ]]; then
    echo "SECOND-WATCH boundary: missing contract artifact"
    exit 1
  fi
done

required_source=(
  'nightshift.self-hosted-foreman-bootstrap/v1'
  'nightshift.self-hosted-foreman-bootstrap.digest/v1'
  '0dff82fa3522e59a6ce8e8161f6aed92cbacc061'
  'ACCEPTED_CODEX_PROVIDER_ADMISSION_OWNER_HEAD'
  'ACCEPTED_SWITCHYARD_PROVIDER_ADMISSION_OWNER_HEAD'
  'HOLDING_QUALIFICATION_PRODUCER_ID'
  'DETERMINISTIC_PROVIDER_ADMISSION_EVIDENCE_SCHEMA_V1'
  'HOLDING_QUALIFICATION_PRODUCER_VERSION'
  'HOLDING_QUALIFICATION_EXECUTABLE_SHA256'
  '> packet.worker_budget.maximum_concurrent_mutating_workers'
  'adapter.adapter_id != HOLDING_QUALIFICATION_PRODUCER_ID'
  'adapter.protocol != DETERMINISTIC_PROVIDER_ADMISSION_EVIDENCE_SCHEMA_V1'
  'adapter.adapter_version != HOLDING_QUALIFICATION_PRODUCER_VERSION'
  'adapter.executable_identity != HOLDING_QUALIFICATION_EXECUTABLE_SHA256'
  '|| !adapter.bounded_arguments.is_empty()'
  '|| self.approval_response_authorized'
  '|| self.protected_effect_authorized'
  '|| self.semantic_retry_authorized'
  '|| self.bootstrap_may_nest'
  '|| self.worker_may_invoke_bootstrap'
  '|| self.outer_conversation_scheduler'
  '|| self.timer_or_service_activation_authorized'
  '|| self.production_activation_authorized'
  '|| self.aggregate_result_created'
)

for needle in "${required_source[@]}"; do
  if ! rg -Fq -- "$needle" "$source_file"; then
    echo "SECOND-WATCH boundary: missing closed law $needle"
    exit 1
  fi
done

required_store=(
  'pub fn admit_self_hosted_at_path'
  'pub fn self_hosted_bootstrap'
  'pub fn advance_self_hosted_driver'
  'let (packet, admission, profile, _) = load_contracts(connection, run_id)?'
  'previous_recorded_at.is_some_and'
  'SelfHostedDriverDispositionV1::AllItemsExplicitTerminal'
  'CREATE TABLE IF NOT EXISTS self_hosted_bootstraps'
  'CREATE TABLE IF NOT EXISTS self_hosted_driver_steps'
  'self_hosted_bootstraps_no_update'
  'self_hosted_bootstraps_no_delete'
  'self_hosted_driver_steps_no_update'
  'self_hosted_driver_steps_no_delete'
  'worker_dispatch_authorized: false'
  'approval_response_authorized: false'
  'protected_effect_authorized: false'
  'semantic_retry_authorized: false'
  'aggregate_result_created: false'
)

for needle in "${required_store[@]}"; do
  if ! rg -Fq -- "$needle" "$store_file"; then
    echo "SECOND-WATCH boundary: missing append-only runtime law $needle"
    exit 1
  fi
done

required_cli=(
  'BootstrapAdmit'
  'BootstrapStep'
  'BootstrapStatus'
  'O_NOFOLLOW'
  'read_bounded_existing'
)

for needle in "${required_cli[@]}"; do
  if ! rg -Fq -- "$needle" "$cli_file"; then
    echo "SECOND-WATCH boundary: missing bounded CLI law $needle"
    exit 1
  fi
done

if rg -q 'std::process|Command::new|TcpListener|TcpStream|reqwest|hyper::|tokio::' "$source_file" "$store_file" "$cli_file"; then
  echo "SECOND-WATCH boundary: runtime stage gained process, network, or listener machinery"
  exit 1
fi


if [[ "${1:-}" == "--self-test-inject" ]]; then
  fixture="$(mktemp -d)"
  trap 'rm -rf -- "$fixture"' EXIT
  injected="$fixture/bootstrap.rs"
  sed 's/|| self.approval_response_authorized/|| false/' "$source_file" >"$injected"
  if env NIGHTSHIFT_SECOND_WATCH_BOOTSTRAP_SOURCE="$injected" "$0" >"$fixture/output" 2>&1; then
    echo "SECOND-WATCH boundary negative control did not refuse missing approval guard"
    exit 1
  fi
  if ! rg -q 'missing closed law.*approval_response_authorized' "$fixture/output"; then
    cat "$fixture/output"
    echo "SECOND-WATCH boundary negative control failed without exact disposition"
    exit 1
  fi
  injected_adapter="$fixture/bootstrap-adapter.rs"
  sed 's/adapter.adapter_id != HOLDING_QUALIFICATION_PRODUCER_ID/false/' "$source_file" >"$injected_adapter"
  if env NIGHTSHIFT_SECOND_WATCH_BOOTSTRAP_SOURCE="$injected_adapter" "$0" >"$fixture/adapter-output" 2>&1; then
    echo "SECOND-WATCH boundary negative control did not refuse missing adapter guard"
    exit 1
  fi
  if ! rg -q 'missing closed law.*adapter.adapter_id' "$fixture/adapter-output"; then
    cat "$fixture/adapter-output"
    echo "SECOND-WATCH adapter negative control failed without exact disposition"
    exit 1
  fi
  echo "SECOND-WATCH boundary deterministic substitution control: passed"
  exit 0
fi

python3 "$schema_test"
echo "SECOND-WATCH boundary: contract, schema, append-only store, and bounded CLI checks passed"
