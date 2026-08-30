import copy
import json
import pathlib
import unittest

from jsonschema import Draft202012Validator, ValidationError


ROOT = pathlib.Path(__file__).resolve().parents[1]
DIGEST = "sha256:" + "a" * 64


def schema(name):
    with (ROOT / "schemas" / name).open(encoding="utf-8") as handle:
        return json.load(handle)


class ForemanSchemaTests(unittest.TestCase):
    def test_all_contracts_are_valid_draft_2020_12(self):
        for name in [
            "nightshift.foreman-admission.v1.schema.json",
            "nightshift.foreman-execution-profile.v1.schema.json",
            "nightshift.worker-adapter.v1.schema.json",
        ]:
            Draft202012Validator.check_schema(schema(name))

    def test_admission_is_closed_and_authority_effect_is_fixed(self):
        document = {
            "schema": "nightshift.foreman-admission/v1",
            "admission_digest": DIGEST,
            "run_id": "run-fixture",
            "packet_digest": DIGEST,
            "operator_basis_digest": DIGEST,
            "admitted_at": "2026-08-29T16:00:00Z",
            "expires_at": "2026-08-29T17:00:00Z",
            "local_runtime_identity": "runtime-fixture",
            "maximum_concurrent_workers": 2,
            "allowed_adapter_ids": ["fixture-adapter"],
            "allowed_provider_model_classes": ["bounded"],
            "maximum_new_attempts_per_work_item": 1,
            "authority_effect": "LOCAL_AGENT_COMPUTE_SCHEDULING_ONLY",
            "target_effects_authorized": False,
        }
        validator = Draft202012Validator(
            schema("nightshift.foreman-admission.v1.schema.json")
        )
        validator.validate(document)
        widened = copy.deepcopy(document)
        widened["target_effects_authorized"] = True
        with self.assertRaises(ValidationError):
            validator.validate(widened)
        unknown = copy.deepcopy(document)
        unknown["approval"] = True
        with self.assertRaises(ValidationError):
            validator.validate(unknown)

    def test_adapter_start_request_has_no_approval_response_extension(self):
        document = {
            "schema": "nightshift.worker-start-request/v1",
            "request_digest": DIGEST,
            "adapter_protocol": "fixture.adapter/v1",
            "packet_digest": DIGEST,
            "run_id": "run-fixture",
            "work_item_id": "root-a",
            "attempt_id": "attempt-fixture",
            "worker_brief_digest": DIGEST,
            "workspace_identity": "workspace:root-a",
            "provider_model_class": "bounded",
            "timeout_seconds": 60,
            "maximum_output_bytes": 65536,
            "recursive_worker_swarms_forbidden": True,
            "approval_policy": "SURFACE_ONLY_NO_RESPONSE",
            "expected_receipt_schema": "nightshift.worker-terminal-receipt/v1",
        }
        validator = Draft202012Validator(
            schema("nightshift.worker-adapter.v1.schema.json")
        )
        validator.validate(document)
        unknown = copy.deepcopy(document)
        unknown["respond_to_approval"] = True
        with self.assertRaises(ValidationError):
            validator.validate(unknown)


if __name__ == "__main__":
    unittest.main()
