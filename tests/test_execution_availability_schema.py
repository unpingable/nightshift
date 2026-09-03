"""Executable closure checks for provider execution-availability contracts."""

import copy
import hashlib
import subprocess
import sys
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
    "nightshift.provider-admission-disposition.v2.schema.json",
    "nightshift.provider-admission-disposition.v3.schema.json",
    "nightshift.holding-deterministic-provider-admission-evidence.v1.schema.json",
    "nightshift.holding-deterministic-provider-admission-evidence.v2.schema.json",
    "nightshift.deferred-provider-dispatch.v1.schema.json",
]
SWITCHYARD_SCHEMA = "vendor/switchyard.codex-provider-admission.v1.schema.json"
FIXTURES = ROOT / "qualification" / "provider-execution-availability-and-deferred-dispatch-v1-20260831" / "fixtures"
D = "sha256:" + "0" * 64


def load(name):
    return json.loads((ROOT / "schemas" / name).read_bytes())


class ExecutionAvailabilitySchemaTest(unittest.TestCase):
    def test_schemas_are_draft_2020_12_and_top_level_closed(self):
        for name in [*NAMES, SWITCHYARD_SCHEMA]:
            with self.subTest(name=name):
                schema = load(name)
                Draft202012Validator.check_schema(schema)
                self.assertEqual(
                    schema["$schema"], "https://json-schema.org/draft/2020-12/schema"
                )
                if name != SWITCHYARD_SCHEMA:
                    self.assertFalse(schema["additionalProperties"])

    def test_exact_accepted_switchyard_vectors_satisfy_vendored_schema(self):
        schema = load(SWITCHYARD_SCHEMA)
        validator = Draft202012Validator(schema)
        for path in sorted(FIXTURES.glob("switchyard-*.snapshot.v1.json")):
            with self.subTest(path=path.name):
                validator.validate(json.loads(path.read_bytes()))

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
        self.assertEqual(
            disposition["$defs"]["snapshot"]["properties"]["representation"]["const"],
            "RFC8785_SWITCHYARD_MAPPER_SNAPSHOT",
        )
        self.assertEqual(
            disposition["$defs"]["snapshot"]["properties"]["byte_length"]["maximum"],
            16 * 1024 * 1024,
        )

    def test_second_watch_fake_v2_schema_closes_completed_execution(self):
        evidence = {
            "schema": "nightshift.holding-deterministic-provider-admission-evidence/v2",
            "evidence_digest": D,
            "producer_id": "nightshift:holding-pattern-deterministic-fake-adapter",
            "producer_version": "v2",
            "executable_id": "campaign:second-watch:deterministic-fake-adapter:v2",
            "executable_sha256": "sha256:bcfea17f0aff021d6b69f2b3d924e7606bf74941671a5a7af13e2d1e3d43edd4",
            "work_attempt_id": "attempt-1",
            "dispatch_occurrence_id": "dispatch-1",
            "provider_request_occurrence_id": "request-1",
            "provider_id": "fixture-provider",
            "model_id": "fixture-model",
            "outcome": "EXECUTION_COMPLETED",
            "response_created": True,
            "non_admission_proven": False,
            "retry_after": None,
            "observed_at": "2026-08-31T12:01:01Z",
            "received_at": "2026-08-31T12:01:02Z",
            "provider_execution": {
                "provider_id": "fixture-provider",
                "model_id": "fixture-model",
                "app_server_session_identity": "estate-1",
                "thread_id": "thread-1",
                "turn_id": "turn-1",
                "first_response_id": "response-1",
            },
            "raw_evidence": {
                "representation": "EXACT_PROVIDER_AVAILABILITY_SOURCE_BYTES",
                "byte_length": 2,
                "sha256": D,
                "encoding": "hex",
                "bytes_hex": "7b7d",
            },
            "authority_effect": "QUALIFICATION_MECHANISM_EVIDENCE_ONLY",
        }
        validator = Draft202012Validator(
            load("nightshift.holding-deterministic-provider-admission-evidence.v2.schema.json"),
            format_checker=FormatChecker(),
        )
        validator.validate(evidence)
        for field, replacement in (
            ("producer_version", "v1"),
            ("executable_sha256", "sha256:" + "1" * 64),
            ("outcome", "RATE_LIMITED"),
            ("response_created", False),
            ("non_admission_proven", True),
            ("provider_execution", None),
        ):
            changed = copy.deepcopy(evidence)
            changed[field] = replacement
            with self.assertRaises(ValidationError):
                validator.validate(changed)

        unavailable = copy.deepcopy(evidence)
        unavailable.update(
            outcome="PROVIDER_UNAVAILABLE",
            response_created=False,
            non_admission_proven=True,
            retry_after="2026-08-31T12:01:07Z",
            provider_execution=None,
        )
        validator.validate(unavailable)
        missing_retry_after = copy.deepcopy(unavailable)
        missing_retry_after["retry_after"] = None
        with self.assertRaises(ValidationError):
            validator.validate(missing_retry_after)
        completed_with_retry_after = copy.deepcopy(evidence)
        completed_with_retry_after["retry_after"] = "2026-08-31T12:01:07Z"
        with self.assertRaises(ValidationError):
            validator.validate(completed_with_retry_after)

    def test_second_watch_fake_v2_executable_is_exact_and_closed(self):
        path = FIXTURES / "deterministic-fake-adapter-v2.py"
        self.assertEqual(
            hashlib.sha256(path.read_bytes()).hexdigest(),
            "bcfea17f0aff021d6b69f2b3d924e7606bf74941671a5a7af13e2d1e3d43edd4",
        )
        for value in (
            {
                "outcome": "PROVIDER_UNAVAILABLE",
                "response_created": False,
                "non_admission_proven": True,
                "retry_after": "2026-08-31T12:01:07Z",
                "observed_at": "2026-08-31T12:01:01Z",
                "provider_execution": None,
            },
            {
                "outcome": "EXECUTION_COMPLETED",
                "response_created": True,
                "non_admission_proven": False,
                "retry_after": None,
                "observed_at": "2026-08-31T12:01:01Z",
                "provider_execution": {
                    "provider_id": "fixture-provider",
                    "model_id": "fixture-model",
                    "app_server_session_identity": "estate-1",
                    "thread_id": "thread-1",
                    "turn_id": "turn-1",
                    "first_response_id": "response-1",
                },
            },
        ):
            raw = json.dumps(value, ensure_ascii=False).encode()
            completed = subprocess.run(
                [sys.executable, str(path)],
                input=raw,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                check=True,
            )
            self.assertEqual(json.loads(completed.stdout), value)
        invalid_cases = (
            {
                "outcome": "PROVIDER_UNAVAILABLE",
                "response_created": False,
                "non_admission_proven": True,
                "retry_after": None,
                "observed_at": "2026-08-31T12:01:01Z",
                "provider_execution": None,
            },
            {
                "outcome": "EXECUTION_COMPLETED",
                "response_created": True,
                "non_admission_proven": False,
                "retry_after": "2026-08-31T12:01:07Z",
                "observed_at": "2026-08-31T12:01:01Z",
                "provider_execution": {
                    "provider_id": "fixture-provider",
                    "model_id": "fixture-model",
                    "app_server_session_identity": "estate-1",
                    "thread_id": "thread-1",
                    "turn_id": "turn-1",
                    "first_response_id": "response-1",
                },
            },
        )
        for invalid in invalid_cases:
            completed = subprocess.run(
                [sys.executable, str(path)],
                input=json.dumps(invalid).encode(),
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                check=False,
            )
            self.assertNotEqual(completed.returncode, 0)

    def test_second_watch_disposition_v3_schema_closes_owner_family(self):
        execution = {
            "provider_id": "fixture-provider",
            "model_id": "fixture-model",
            "app_server_session_identity": "estate-1",
            "thread_id": "thread-1",
            "turn_id": "turn-1",
            "first_response_id": "response-1",
        }
        disposition = {
            "schema": "nightshift.provider-admission-disposition/v3",
            "disposition_digest": D,
            "dispatch_digest": D,
            "requirement_digest": D,
            "policy_digest": D,
            "packet_digest": D,
            "run_id": "run-1",
            "work_item_id": "work-1",
            "work_attempt_id": "attempt-1",
            "dispatch_occurrence_id": "dispatch-1",
            "provider_id": "fixture-provider",
            "model_id": "fixture-model",
            "provider_request_occurrence_id": "request-1",
            "adapter_process_occurrence_id": "process-1",
            "app_server_session_identity": "estate-1",
            "thread_id": "thread-1",
            "turn_id": "turn-1",
            "disposition": "EXECUTION_ADMITTED",
            "mechanism_state": "PROVIDER_COMPLETED",
            "received_at": "2026-08-31T12:01:02Z",
            "response_created": True,
            "will_retry": False,
            "acquisition_complete": True,
            "provider_retry_after": None,
            "provider_execution": execution,
            "mapper_snapshot_schema": "nightshift.holding-deterministic-provider-admission-evidence/v2",
            "mapper_snapshot_digest": D,
            "mapper_snapshot": {
                "representation": "RFC8785_NIGHTSHIFT_QUALIFICATION_FAKE_ADAPTER_EVIDENCE",
                "byte_length": 2,
                "sha256": D,
                "encoding": "hex",
                "bytes_hex": "7b7d",
            },
            "approval_response_sent": False,
            "protected_effect_absent": True,
            "authority_effect": "SCHEDULING_MECHANISM_EVIDENCE_ONLY",
        }
        validator = Draft202012Validator(
            load("nightshift.provider-admission-disposition.v3.schema.json"),
            format_checker=FormatChecker(),
        )
        validator.validate(disposition)
        for field, replacement in (
            ("schema", "nightshift.provider-admission-disposition/v2"),
            ("mapper_snapshot_schema", "switchyard.codex-provider-admission-snapshot/v1"),
            ("mechanism_state", "PARKED_NOT_ADMITTED"),
            ("response_created", False),
            ("provider_execution", None),
        ):
            changed = copy.deepcopy(disposition)
            changed[field] = replacement
            with self.assertRaises(ValidationError):
                validator.validate(changed)

        unavailable = copy.deepcopy(disposition)
        unavailable.update(
            disposition="NOT_ADMITTED_PROVIDER_UNAVAILABLE",
            mechanism_state="PARKED_NOT_ADMITTED",
            response_created=False,
            provider_retry_after="2026-08-31T12:01:07Z",
            provider_execution=None,
        )
        validator.validate(unavailable)
        unavailable["provider_retry_after"] = None
        with self.assertRaises(ValidationError):
            validator.validate(unavailable)

    def test_canonical_time_lowercase_digest_and_evidence_representation(self):
        requirement = {
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
                "WORK-A": [{"provider_id": "openai", "model_id": "gpt-5.6-sol", "model_class": "large"}]
            },
            "admitted_at": "2026-08-31T12:00:00.123Z",
            "authority_effect": "LOCAL_AGENT_COMPUTE_SCHEDULING_ONLY",
        }
        validator = Draft202012Validator(load(NAMES[2]), format_checker=FormatChecker())
        validator.validate(requirement)
        for timestamp in (
            "2026-08-31T08:00:00-04:00",
            "2026-08-31T12:00:00.123000Z",
        ):
            changed = copy.deepcopy(requirement)
            changed["admitted_at"] = timestamp
            with self.assertRaises(ValidationError):
                validator.validate(changed)
        changed = copy.deepcopy(requirement)
        changed["packet_digest"] = "sha256:" + "A" * 64
        with self.assertRaises(ValidationError):
            validator.validate(changed)

        observation_schema = load(NAMES[0])
        self.assertEqual(
            set(observation_schema["$defs"]["evidence"]["properties"]["representation"]["enum"]),
            {
                "EXACT_PROVIDER_AVAILABILITY_SOURCE_BYTES",
                "EXACT_WIRE_BYTES_INCLUDING_LINE_TERMINATOR",
                "EXACT_ACQUIRED_FRAME_BYTES_INCLUDING_LINE_TERMINATOR",
            },
        )

    def test_qualification_owner_schema_is_closed_and_executable_identity_is_exact(self):
        schema = load(
            "nightshift.holding-deterministic-provider-admission-evidence.v1.schema.json"
        )
        value = {
            "schema": "nightshift.holding-deterministic-provider-admission-evidence/v1",
            "evidence_digest": D,
            "producer_id": "nightshift:holding-pattern-deterministic-fake-adapter",
            "producer_version": "v1",
            "executable_id": "campaign:holding-pattern:deterministic-fake-adapter:v1",
            "executable_sha256": "sha256:e8a310d46cb40b0aef6399a8da6c97ac99f0fc5eab6a78c5e7007600d5cbfa82",
            "work_attempt_id": "attempt-1",
            "dispatch_occurrence_id": "dispatch-1",
            "provider_request_occurrence_id": "request-1",
            "provider_id": "fixture-provider",
            "model_id": "fixture-model",
            "outcome": "RATE_LIMITED",
            "response_created": False,
            "non_admission_proven": True,
            "retry_after": "2026-08-31T12:01:07Z",
            "observed_at": "2026-08-31T12:01:01Z",
            "received_at": "2026-08-31T12:01:02Z",
            "raw_evidence": {
                "representation": "EXACT_PROVIDER_AVAILABILITY_SOURCE_BYTES",
                "byte_length": 2,
                "sha256": D,
                "encoding": "hex",
                "bytes_hex": "7b7d",
            },
            "authority_effect": "QUALIFICATION_MECHANISM_EVIDENCE_ONLY",
        }
        validator = Draft202012Validator(schema, format_checker=FormatChecker())
        validator.validate(value)
        for field, replacement in (
            ("producer_id", "switchyard:provider-admission"),
            ("executable_sha256", D),
            ("outcome", "MODEL_AT_CAPACITY"),
        ):
            changed = copy.deepcopy(value)
            changed[field] = replacement
            with self.assertRaises(ValidationError):
                validator.validate(changed)
        changed = copy.deepcopy(value)
        changed["unknown"] = True
        with self.assertRaises(ValidationError):
            validator.validate(changed)


if __name__ == "__main__":
    unittest.main()
