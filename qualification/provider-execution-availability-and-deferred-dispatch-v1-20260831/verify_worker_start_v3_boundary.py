#!/usr/bin/env python3
"""Deterministic structural boundary for HOLDING worker-start V3."""

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

schema = json.loads(
    (ROOT / "schemas/nightshift.worker-start-request.v3.schema.json").read_bytes()
)
properties = schema["properties"]
assert schema["additionalProperties"] is False
assert properties["predecessor_schema"]["const"] == "nightshift.worker-start-request/v2"
assert properties["adapter_protocol"]["const"] == "switchyard.codex-app-server/v2"
assert properties["provider_admission_adapter_protocol"]["const"] == "switchyard.codex-app-server/v2"
assert properties["provider_admission_binding_schema"]["const"] == "switchyard.codex-provider-admission-binding/v1"
assert properties["provider_admission_evidence_schema"]["const"] == "switchyard.codex-provider-admission-evidence/v1"
assert properties["provider_admission_snapshot_schema"]["const"] == "switchyard.codex-provider-admission-snapshot/v1"
assert properties["provider_execution_id"]["type"] == "null"
assert properties["internal_provider_retry_count"]["const"] == 0
assert properties["semantic_retry"]["const"] is False
assert properties["approval_response_authorized"]["const"] is False
assert properties["authority_effect"]["const"] == "LOCAL_AGENT_COMPUTE_SCHEDULING_ONLY"
assert not ({"approve", "execute", "retry", "promote", "merge"} & set(properties))

v2 = json.loads((ROOT / "schemas/nightshift.worker-adapter.v2.schema.json").read_bytes())
assert (
    v2["$defs"]["capabilities"]["properties"]["expected_start_request_schema"]["const"]
    == "nightshift.worker-start-request/v2"
)

nightshiftd = (ROOT / "crates/nightshiftd/Cargo.toml").read_text()
assert len(re.findall(r"^\[\[bin\]\]$", nightshiftd, re.MULTILINE)) == 2
print("worker-start-v3-boundary=pass canonical-nightshiftd-binaries=2")
