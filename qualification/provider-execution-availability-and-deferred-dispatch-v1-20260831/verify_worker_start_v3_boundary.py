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
reserved_fake_id = "nightshift:holding-pattern-deterministic-fake-adapter"
switchyard_protocol = "switchyard.codex-app-server/v2"
fake_v1_protocol = "nightshift.holding-deterministic-provider-admission-evidence/v1"
fake_v2_protocol = "nightshift.holding-deterministic-provider-admission-evidence/v2"
family_fields = (
    "adapter_protocol",
    "provider_admission_adapter_protocol",
    "provider_admission_binding_schema",
    "provider_admission_evidence_schema",
    "provider_admission_snapshot_schema",
)

expected_families = (
    {
        "adapter_id": {"not": {"const": reserved_fake_id}},
        "adapter_protocol": {"const": switchyard_protocol},
        "provider_admission_adapter_protocol": {"const": switchyard_protocol},
        "provider_admission_binding_schema": {"const": "switchyard.codex-provider-admission-binding/v1"},
        "provider_admission_evidence_schema": {"const": "switchyard.codex-provider-admission-evidence/v1"},
        "provider_admission_snapshot_schema": {"const": "switchyard.codex-provider-admission-snapshot/v1"},
    },
    {
        "adapter_id": {"const": reserved_fake_id},
        "adapter_version": {"const": "v1"},
        **{field: {"const": fake_v1_protocol} for field in family_fields},
    },
    {
        "adapter_id": {"const": reserved_fake_id},
        "adapter_version": {"const": "v2"},
        **{field: {"const": fake_v2_protocol} for field in family_fields},
    },
)
branches = schema["allOf"][0]["oneOf"]
assert tuple(branch["properties"] for branch in branches) == expected_families


def branch_matches(branch, candidate):
    for field, rule in branch["properties"].items():
        if "const" in rule and candidate.get(field) != rule["const"]:
            return False
        if "not" in rule and candidate.get(field) == rule["not"]["const"]:
            return False
    return True


family_candidates = (
    {
        "adapter_id": "switchyard-codex",
        "adapter_version": "2.0.0",
        "adapter_protocol": switchyard_protocol,
        "provider_admission_adapter_protocol": switchyard_protocol,
        "provider_admission_binding_schema": "switchyard.codex-provider-admission-binding/v1",
        "provider_admission_evidence_schema": "switchyard.codex-provider-admission-evidence/v1",
        "provider_admission_snapshot_schema": "switchyard.codex-provider-admission-snapshot/v1",
    },
    {
        "adapter_id": reserved_fake_id,
        "adapter_version": "v1",
        **{field: fake_v1_protocol for field in family_fields},
    },
    {
        "adapter_id": reserved_fake_id,
        "adapter_version": "v2",
        **{field: fake_v2_protocol for field in family_fields},
    },
)
for candidate in family_candidates:
    assert sum(branch_matches(branch, candidate) for branch in branches) == 1

# Deterministic mixed-family substitutions must match no owner-family branch.
for source_index, replacement_index, field in (
    (0, 1, "adapter_id"),
    (1, 2, "provider_admission_evidence_schema"),
    (2, 0, "provider_admission_binding_schema"),
):
    substituted = dict(family_candidates[source_index])
    substituted[field] = family_candidates[replacement_index][field]
    assert not any(branch_matches(branch, substituted) for branch in branches)
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
