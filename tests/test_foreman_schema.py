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
            "nightshift.foreman-execution-profile.v2.schema.json",
            "nightshift.worker-adapter.v1.schema.json",
            "nightshift.worker-adapter.v2.schema.json",
        ]:
            Draft202012Validator.check_schema(schema(name))

        profile = {
            "schema": "nightshift.foreman-execution-profile/v2",
            "profile_digest": DIGEST,
            "packet_digest": DIGEST,
            "admission_digest": DIGEST,
            "adapters": {
                "fixture-adapter": {
                    "adapter_id": "fixture-adapter",
                    "protocol": "fixture.adapter/v1",
                    "adapter_version": "fixture.adapter/v1",
                    "executable_identity": DIGEST,
                    "bounded_arguments": [],
                }
            },
            "work_items": {
                "fixture-work": {
                    "adapter_id": "fixture-adapter",
                    "workspace_identity": "workspace:fixture",
                    "resource_lock_keys": ["repository:fixture"],
                    "provider_model_class": "bounded",
                }
            },
            "budget_policy_ref": "budget:fixture",
            "log_custody_root": "/tmp/fixture/log",
            "receipt_custody_root": "/tmp/fixture/receipts",
            "maximum_event_bytes": 65536,
            "maximum_receipt_bytes": 131072,
            "adapter_timeout_seconds": 60,
            "closeout_policy": "ALL_EXPLICIT_TERMINAL_OR_NOT_STARTED",
        }
        validator = Draft202012Validator(
            schema("nightshift.foreman-execution-profile.v2.schema.json")
        )
        validator.validate(profile)
        missing_version = copy.deepcopy(profile)
        del missing_version["adapters"]["fixture-adapter"]["adapter_version"]
        with self.assertRaises(ValidationError):
            validator.validate(missing_version)

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
            "schema": "nightshift.worker-start-request/v2",
            "request_digest": DIGEST,
            "adapter_id": "fixture-adapter",
            "adapter_version": "fixture.adapter/v1",
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
            schema("nightshift.worker-adapter.v2.schema.json")
        )
        validator.validate(document)
        verification = {
            "schema": "nightshift.adapter-contract-verification/v1",
            "capabilities_raw_digest": DIGEST,
            "profile_digest": DIGEST,
            "request_digest": DIGEST,
            "adapter_id": "fixture-adapter",
            "adapter_protocol": "fixture.adapter/v1",
            "adapter_version": "fixture.adapter/v1",
            "adapter_executable_identity": DIGEST,
            "disposition": "EXACT_PROFILE_CAPABILITIES_START_BINDING_VERIFIED",
        }
        validator.validate(verification)
        unknown = copy.deepcopy(document)
        excessive_timeout = copy.deepcopy(document)
        excessive_timeout["timeout_seconds"] = 86401
        with self.assertRaises(ValidationError):
            validator.validate(excessive_timeout)
        excessive_output = copy.deepcopy(document)
        excessive_output["maximum_output_bytes"] = 16777217
        with self.assertRaises(ValidationError):
            validator.validate(excessive_output)
        unknown["respond_to_approval"] = True
        with self.assertRaises(ValidationError):
            validator.validate(unknown)


    def test_adapter_extensions_use_the_recursively_closed_interoperable_subset(self):
        document = {
            "schema": "nightshift.worker-adapter-event/v1",
            "event_digest": DIGEST,
            "packet_digest": DIGEST,
            "run_id": "run-fixture",
            "work_item_id": "root-a",
            "attempt_id": "attempt-fixture",
            "adapter_id": "fixture-adapter",
            "adapter_version": "fixture.adapter/v1",
            "event_id": "event-fixture",
            "occurred_at": "2026-08-29T16:01:00Z",
            "kind": "checkpoint",
            "provider_identity": None,
            "model_identity": None,
            "session_identity": None,
            "thread_identity": None,
            "turn_identity": None,
            "queue_identity": None,
            "message": "bounded local progress",
            "human_question": None,
            "extensions": {"nested": {"safe_key": ["unicode value \u20ac", 0.0000001, None, True]}},
        }
        validator = Draft202012Validator(
            schema("nightshift.worker-adapter.v2.schema.json")
        )
        validator.validate(document)
        unicode_key = copy.deepcopy(document)
        unicode_key["extensions"] = {"nested": {"\U0001f600": "not admitted as an object key"}}
        with self.assertRaises(ValidationError):
            validator.validate(unicode_key)
        for unsafe_number in (9007199254740992, -9007199254740992, 1e20):
            numeric = copy.deepcopy(document)
            numeric["extensions"] = {"nested": {"unsafe_number": unsafe_number}}
            with self.assertRaises(ValidationError):
                validator.validate(numeric)

    def test_v2_worker_brief_is_closed_and_carries_exact_sources(self):
        document = {
            "schema": "nightshift.worker-brief-basis/v2",
            "packet_digest": DIGEST,
            "packet_source": {
                "retained_raw_digest": DIGEST,
                "encoding": "hex",
                "bytes_hex": "7b7d",
            },
            "work_item": {"contract": "nightshift.orientation-packet/v1#work-item", "canonical_json": "{\"dependencies\":[\"root-a\"],\"id\":\"dependent\"}"},
            "predecessor_receipts": {
                "root-a": {
                    "receipt_kind": "terminal",
                    "retained_raw_digest": DIGEST,
                    "encoding": "hex",
                    "bytes_hex": "7b2022657874656e73696f6e223a2074727565207d",
                }
            },
            "global_constraints": {"contract": "nightshift.orientation-packet/v1#global-constraints", "canonical_json": "{}"},
            "execution": {"contract": "nightshift.foreman-execution-profile/v2#work-item", "canonical_json": "{}"},
        }
        validator = Draft202012Validator(
            schema("nightshift.worker-adapter.v2.schema.json")
        )
        validator.validate(document)
        verification = {
            "schema": "nightshift.adapter-contract-verification/v1",
            "capabilities_raw_digest": DIGEST,
            "profile_digest": DIGEST,
            "request_digest": DIGEST,
            "adapter_id": "fixture-adapter",
            "adapter_protocol": "fixture.adapter/v1",
            "adapter_version": "fixture.adapter/v1",
            "adapter_executable_identity": DIGEST,
            "disposition": "EXACT_PROFILE_CAPABILITIES_START_BINDING_VERIFIED",
        }
        validator.validate(verification)
        unknown = copy.deepcopy(document)
        unknown["semantic_summary"] = "not admitted"
        with self.assertRaises(ValidationError):
            validator.validate(unknown)
        too_many = copy.deepcopy(document)
        too_many["predecessor_receipts"] = {
            f"p{index}": document["predecessor_receipts"]["root-a"]
            for index in range(1025)
        }
        with self.assertRaises(ValidationError):
            validator.validate(too_many)
        nested_unknown = copy.deepcopy(document)
        nested_unknown["work_item"]["interpreted_result"] = "not admitted"
        with self.assertRaises(ValidationError):
            validator.validate(nested_unknown)


if __name__ == "__main__":
    unittest.main()
