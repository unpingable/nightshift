"""Executable closure checks for provider execution-availability contracts."""

import copy
import json
import unittest
from pathlib import Path

from jsonschema import Draft202012Validator, FormatChecker, ValidationError

ROOT = Path(__file__).resolve().parents[1]
NAMES = [
    "nightshift.provider-execution-availability-observation.v1.schema.json",
    "nightshift.provider-execution-availability-policy.v1.schema.json",
    "nightshift.foreman-execution-availability-requirement.v1.schema.json",
    "nightshift.provider-dispatch-occurrence.v1.schema.json",
    "nightshift.provider-admission-disposition.v1.schema.json",
    "nightshift.deferred-provider-dispatch.v1.schema.json",
]
D = "sha256:" + "0" * 64


def load(name):
    return json.loads((ROOT / "schemas" / name).read_bytes())


class ExecutionAvailabilitySchemaTest(unittest.TestCase):
    def test_schemas_are_draft_2020_12_and_top_level_closed(self):
        for name in NAMES:
            with self.subTest(name=name):
                schema = load(name)
                Draft202012Validator.check_schema(schema)
                self.assertEqual(
                    schema["$schema"], "https://json-schema.org/draft/2020-12/schema"
                )
                self.assertFalse(schema["additionalProperties"])

    def test_policy_vocabulary_and_non_authorizing_constants_are_closed(self):
        schema = load(NAMES[1])
        value = {
            "schema": "nightshift.provider-execution-availability-policy/v1",
            "policy_digest": D,
            "policy_id": "holding-policy",
            "maximum_dispatch_occurrences_per_attempt": 4,
            "backoff_seconds": [5, 10, 20, 40],
            "maximum_total_deferral_seconds": 600,
            "parked_resource_lock_policy": "RELEASE_AND_REACQUIRE",
            "provider_capacity_released_while_parked": True,
            "reconcile_indeterminate": True,
            "allow_ordered_model_fallback": True,
            "automatic_semantic_retry": False,
            "approval_response_authorized": False,
            "authority_effect": "LOCAL_AGENT_COMPUTE_SCHEDULING_ONLY",
        }
        validator = Draft202012Validator(schema)
        validator.validate(value)
        for field, replacement in (
            ("automatic_semantic_retry", True),
            ("approval_response_authorized", True),
            ("provider_capacity_released_while_parked", False),
        ):
            changed = copy.deepcopy(value)
            changed[field] = replacement
            with self.assertRaises(ValidationError):
                validator.validate(changed)
        changed = copy.deepcopy(value)
        changed["aggregate_provider_health"] = "green"
        with self.assertRaises(ValidationError):
            validator.validate(changed)

    def test_requirement_exact_owner_pins_and_nested_selection_are_closed(self):
        schema = load(NAMES[2])
        value = {
            "schema": "nightshift.foreman-execution-availability-requirement/v1",
            "requirement_digest": D,
            "packet_digest": D,
            "admission_digest": D,
            "profile_digest": D,
            "run_id": "run-holding",
            "adapter_id": "switchyard-codex",
            "adapter_protocol": "switchyard.codex-app-server/v2",
            "adapter_version": "2.0.0",
            "adapter_executable_identity": D,
            "owner_pins": {
                "codex_owner_head": "c36a8137638decf8b04a49611354a90f32c5a945",
                "switchyard_owner_head": "2ba25db66d8b29dd215bd87e05f4ea794024b3b7",
                "switchyard_schema_sha256": "sha256:131f1f6e0cf8cb0aea26ed225c584440c81ffedd443c68ace23adecbe493cf93",
                "deterministic_fixture_sha256": "sha256:cafa673ac58f60029fd6c1de229b4f57d9f42ba918b7ecb2a3bfb20cb2b41a31",
            },
            "policy_id": "holding-policy",
            "policy_digest": D,
            "work_item_model_selections": {
                "WORK-A": [
                    {"provider_id": "openai", "model_id": "gpt-5.6-sol", "model_class": "large"}
                ]
            },
            "admitted_at": "2026-08-31T12:00:00Z",
            "authority_effect": "LOCAL_AGENT_COMPUTE_SCHEDULING_ONLY",
        }
        validator = Draft202012Validator(schema, format_checker=FormatChecker())
        validator.validate(value)
        for path in (("owner_pins", "codex_owner_head"), ("selection", "extra")):
            changed = copy.deepcopy(value)
            if path[0] == "owner_pins":
                changed["owner_pins"][path[1]] = "0" * 40
            else:
                changed["work_item_model_selections"]["WORK-A"][0][path[1]] = True
            with self.assertRaises(ValidationError):
                validator.validate(changed)

    def test_schema_vocabulary_matches_runtime_closed_meanings(self):
        observation = load(NAMES[0])
        self.assertEqual(
            set(observation["properties"]["state"]["enum"]),
            {
                "AVAILABLE", "MODEL_AT_CAPACITY", "PROVIDER_UNAVAILABLE",
                "RATE_LIMITED", "AUTHENTICATION_REFUSED", "TRANSPORT_ERROR",
                "PROTOCOL_ERROR", "UNKNOWN",
            },
        )
        disposition = load(NAMES[4])
        self.assertEqual(
            set(disposition["properties"]["disposition"]["enum"]),
            {
                "NOT_ADMITTED_MODEL_AT_CAPACITY", "NOT_ADMITTED_PROVIDER_UNAVAILABLE",
                "NOT_ADMITTED_RATE_LIMITED", "AUTHENTICATION_REFUSED",
                "QUOTA_EXHAUSTED_FUEL_OWNED", "ADMISSION_INDETERMINATE",
                "EXECUTION_ADMITTED",
            },
        )
        self.assertEqual(
            disposition["properties"]["mapper_snapshot_schema"]["const"],
            "switchyard.codex-provider-admission-snapshot/v1",
        )


if __name__ == "__main__":
    unittest.main()
