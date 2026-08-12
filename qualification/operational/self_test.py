#!/usr/bin/env python3
"""Self-test Nightshift operational metadata and deterministic support fixture."""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parent
FIXTURE = ROOT / "fixtures/pulse-support-resolver"
EXPECTED = {
    "nightshift.systemd_missed_catch_up": "requires_trusted_host",
    "nightshift.timer_restart": "requires_trusted_host",
    "nightshift.wall_clock_rollback": "requires_fault_injection_environment",
    "nightshift.pulse_receiver_currentness": "requires_deployed_service",
    "nightshift.deployed_nq_ag_correspondence": "requires_deployed_service",
    "nightshift.sqlite_filesystem_power_loss": "requires_fault_injection_environment",
}


def canonical(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()


def object_id(value: dict[str, Any], field: str) -> str:
    preimage = dict(value)
    preimage.pop(field, None)
    return "sha256:" + hashlib.sha256(canonical(preimage)).hexdigest()


def query() -> dict[str, Any]:
    value = {
        "schema": "nightshift.present_evidence_query.v1",
        "query_id": "",
        "observation_cycle_id": "cycle:00000000-0000-4000-8000-000000000001",
        "request_nonce": "qualification-nonce",
        "observation_id": "sha256:" + "a" * 64,
        "diagnostic_inputs_id": "sha256:" + "b" * 64,
        "subject_id": "qualification-subject",
        "scope_id": "sha256:" + "c" * 64,
        "artifact_ids": ["sha256:" + "d" * 64],
    }
    value["query_id"] = object_id(value, "query_id")
    return value


def invoke(mode: str, value: dict[str, Any]) -> subprocess.CompletedProcess[bytes]:
    environment = dict(os.environ)
    environment["NIGHTSHIFT_QUAL_SUPPORT_MODE"] = mode
    return subprocess.run(
        [str(FIXTURE)],
        input=canonical(value),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=environment,
        check=False,
    )


def main() -> int:
    gates = json.loads((ROOT / "gates.json").read_bytes())
    assert gates["qualification_status"] == "not_assessed"
    observed = {item["id"]: item["classification"] for item in gates["gates"]}
    assert observed == EXPECTED
    for name in ["trusted-host-plan.json", "deployed-service-plan.json", "fault-injection-plan.json"]:
        plan = json.loads((ROOT / name).read_bytes())
        assert plan["qualification_status"] == "not_assessed"
        assert plan["pass_interpretation_owner"] == "later qualification campaign"
        assert plan["preconditions"] and plan["procedure"] and plan["forbidden_shortcuts"]

    exact_query = query()
    responses = {}
    for mode in ["current", "expired", "unknown", "unsupported", "contradictory", "blind"]:
        result = invoke(mode, exact_query)
        assert result.returncode == 0, result.stderr.decode("utf-8", "replace")
        response = json.loads(result.stdout)
        assert response["standing"] == mode
        assert response["query_id"] == exact_query["query_id"]
        assert response["observation_cycle_id"] == exact_query["observation_cycle_id"]
        assert response["support_id"] == object_id(response, "support_id")
        responses[mode] = response
    assert responses["current"]["expiry"]["tick"] > responses["current"]["evaluated_at"]["tick"]
    assert responses["expired"]["expiry"]["tick"] == responses["expired"]["evaluated_at"]["tick"]
    assert "expiry" not in responses["unknown"]
    assert responses["contradictory"]["contradiction_refs"]

    malformed = dict(exact_query)
    malformed["unexpected"] = True
    assert invoke("current", malformed).returncode != 0
    print(json.dumps({
        "schema": "nightshift.operational-harness-self-test.v1",
        "qualification_status": "not_assessed",
        "development_check": "passed",
        "gate_count": len(EXPECTED),
        "fixture_is_not_pulse": True,
        "fixture_modes": sorted(responses),
    }, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
