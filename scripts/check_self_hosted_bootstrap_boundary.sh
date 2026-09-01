#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source_file="${NIGHTSHIFT_SECOND_WATCH_BOOTSTRAP_SOURCE:-$root/crates/nightshift-foreman/src/bootstrap.rs}"
schema="$root/schemas/nightshift.self-hosted-foreman-bootstrap.v1.schema.json"
architecture="$root/docs/architecture/SELF_HOSTED_FOREMAN_BOOTSTRAP_V1.md"
topology="$root/qualification/nightshift-self-hosted-foreman-bootstrap-v1-20260831/PRE-MUTATION-TOPOLOGY.md"
schema_test="$root/tests/test_self_hosted_bootstrap_schema.py"

for required in "$source_file" "$schema" "$architecture" "$topology" "$schema_test"; do
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

if rg -q 'std::process|Command::new|TcpListener|TcpStream|reqwest|hyper::|tokio::' "$source_file"; then
  echo "SECOND-WATCH boundary: contract module gained process, network, or listener machinery"
  exit 1
fi

if rg -q 'SelfHostedForemanBootstrap|SELF_HOSTED_FOREMAN_BOOTSTRAP' "$root/crates/nightshift-foreman/src/store.rs" "$root/crates/nightshift-foreman/src/bin/nightshift_foreman.rs"; then
  echo "SECOND-WATCH boundary: runtime/store activated before contract audit"
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
  echo "SECOND-WATCH boundary deterministic substitution control: passed"
  exit 0
fi

python3 "$schema_test"
echo "SECOND-WATCH boundary: contract, schema, and held-runtime checks passed"
