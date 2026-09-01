import copy
import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

from jsonschema import Draft202012Validator, FormatChecker, ValidationError

ROOT = Path(__file__).resolve().parents[1]


class CaseworkProviderExecutionSchemaTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.schema = json.loads(
            (ROOT / "schemas/nightshift.casework-live-provider-execution.v1.schema.json").read_text()
        )
        Draft202012Validator.check_schema(cls.schema)
        cls.validator = Draft202012Validator(cls.schema, format_checker=FormatChecker())
        cls.absent = {
            "schema": "nightshift.casework-live-provider-execution/v1",
            "projection_digest": "sha256:" + "1" * 64,
            "run_id": "run-1",
            "packet_digest": "sha256:" + "2" * 64,
            "evaluated_at": "2026-08-31T12:00:00Z",
            "status": "NOT_RECORDED_BY_FOREMAN",
            "requirement": None,
            "dispatches": [], "dispositions": [], "deferrals": [], "wakes": [],
            "resumes": [], "resource_transitions": [],
            "independent_provider_capacity_status": "NOT_RECORDED_BY_FOREMAN",
            "explanation": "No exact owner history is recorded.",
            "authority_effect": "READ_ONLY_MECHANISM_PROJECTION",
        }

    def test_absence_is_closed_and_extensions_refuse(self):
        self.validator.validate(self.absent)
        mutated = copy.deepcopy(self.absent)
        mutated["aggregate_result"] = "SUCCESS"
        with self.assertRaises(ValidationError):
            self.validator.validate(mutated)

    def test_absence_cannot_carry_mechanism_records(self):
        mutated = copy.deepcopy(self.absent)
        mutated["dispatches"] = [{}]
        with self.assertRaises(ValidationError):
            self.validator.validate(mutated)

    def test_independent_capacity_status_is_exactly_closed(self):
        for status in ("NOT_RECORDED_BY_FOREMAN", "EXACT_RECORDED_BY_FOREMAN"):
            projected = copy.deepcopy(self.absent)
            projected["independent_provider_capacity_status"] = status
            self.validator.validate(projected)

        mutated = copy.deepcopy(self.absent)
        mutated["independent_provider_capacity_status"] = "UNKNOWN"
        with self.assertRaises(ValidationError):
            self.validator.validate(mutated)

    def test_fuel_owned_quota_is_not_a_provider_admission_disposition(self):
        disposition_schema = {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$ref": "#/$defs/disposition",
            "$defs": self.schema["$defs"],
        }
        validator = Draft202012Validator(
            disposition_schema, format_checker=FormatChecker()
        )
        digest = "sha256:" + "3" * 64
        disposition = {
            "journal_sequence": 1,
            "journal_event_id": "event-1",
            "journal_exact_bytes_sha256": digest,
            "journal_retained_raw_digest": digest,
            "work_item_id": "work-1",
            "work_attempt_id": "attempt-1",
            "dispatch_occurrence_id": "dispatch-1",
            "dispatch_digest": digest,
            "disposition_digest": digest,
            "reconciles_disposition_digest": None,
            "provider_id": "openai",
            "model_id": "model-1",
            "availability_state": "UNKNOWN",
            "admission_disposition": "ADMISSION_INDETERMINATE",
            "mechanism_state": "ADMISSION_INDETERMINATE",
            "observed_at": "2026-08-31T12:00:00Z",
            "evidence_received_at": "2026-08-31T12:00:01Z",
            "expires_at": "2026-08-31T12:05:00Z",
            "disposition_received_at": "2026-08-31T12:00:01Z",
            "currentness": "CURRENT",
            "source_identity": "source-1",
            "source_version": "v1",
            "response_created": False,
            "acquisition_complete": False,
            "provider_retry_after": None,
            "provider_request_occurrence_id": "request-1",
            "provider_execution": None,
            "mapper_snapshot_schema": "switchyard.codex-provider-admission-snapshot/v1",
            "mapper_snapshot_digest": digest,
            "approval_response_sent": False,
            "protected_effect_absent": True,
            "observation_digest": digest,
            "observation_exact_bytes_sha256": digest,
            "disposition_exact_bytes_sha256": digest,
        }
        validator.validate(disposition)
        disposition["admission_disposition"] = "QUOTA_EXHAUSTED_FUEL_OWNED"
        with self.assertRaises(ValidationError):
            validator.validate(disposition)

    def test_recorded_requirement_and_resource_edges_are_closed(self):
        digest = "sha256:" + "4" * 64
        requirement = {
            "journal_sequence": 2,
            "requirement_digest": digest,
            "policy_id": "holding-policy",
            "policy_digest": digest,
            "provider_id": "openai",
            "work_item_model_selections": {
                "work-1": [
                    {
                        "provider_id": "openai",
                        "model_id": "model-primary",
                        "model_class": "large",
                    },
                    {
                        "provider_id": "openai",
                        "model_id": "model-fallback",
                        "model_class": "large",
                    },
                ]
            },
            "adapter_id": "switchyard-codex",
            "adapter_protocol": "switchyard.codex-app-server/v2",
            "adapter_version": "2.0.0",
            "adapter_executable_identity": digest,
            "codex_owner_head": "a" * 40,
            "provider_admission_owner_head": "b" * 40,
            "provider_admission_schema_sha256": digest,
            "deterministic_fixture_sha256": digest,
            "admitted_at": "2026-08-31T12:00:00Z",
            "requirement_exact_bytes_sha256": digest,
            "policy_exact_bytes_sha256": digest,
            "parked_resource_lock_policy": "RELEASE_AND_REACQUIRE",
            "allow_ordered_model_fallback": True,
            "automatic_semantic_retry": False,
            "approval_response_authorized": False,
            "authority_effect": "READ_ONLY_MECHANISM_PROJECTION",
        }
        released = {
            "journal_sequence": 5,
            "journal_event_id": "provider-resources-released",
            "journal_exact_bytes_sha256": digest,
            "transition": "RELEASED",
            "work_item_id": "work-1",
            "work_attempt_id": "attempt-1",
            "dispatch_digest": digest,
            "disposition_digest": digest,
            "deferred_dispatch_digest": None,
            "policy_digest": digest,
            "wake_occurrence_id": None,
            "resource_lock_keys": ["repo:work-1"],
            "recorded_at": "2026-08-31T12:00:02Z",
        }
        recorded = copy.deepcopy(self.absent)
        recorded["status"] = "EXACT_RECORDED_FOREMAN_HISTORY"
        recorded["requirement"] = requirement
        recorded["resource_transitions"] = [released]
        self.validator.validate(recorded)

        mutated = copy.deepcopy(recorded)
        mutated["requirement"]["work_item_model_selections"]["work-1"][1][
            "classification"
        ] = "invented"
        with self.assertRaises(ValidationError):
            self.validator.validate(mutated)

        mutated = copy.deepcopy(recorded)
        mutated["resource_transitions"][0]["disposition_digest"] = None
        with self.assertRaises(ValidationError):
            self.validator.validate(mutated)

        reacquired = copy.deepcopy(released)
        reacquired.update(
            {
                "transition": "REACQUIRED",
                "disposition_digest": None,
                "deferred_dispatch_digest": digest,
                "wake_occurrence_id": "wake-1",
            }
        )
        recorded["resource_transitions"] = [reacquired]
        self.validator.validate(recorded)
        recorded["resource_transitions"][0]["deferred_dispatch_digest"] = None
        with self.assertRaises(ValidationError):
            self.validator.validate(recorded)

    def test_runtime_recorded_projection_obeys_owner_selection_bound(self):
        owner_schema = json.loads(
            (
                ROOT
                / "schemas/nightshift.foreman-execution-availability-requirement.v1.schema.json"
            ).read_text()
        )
        owner_bound = owner_schema["properties"]["work_item_model_selections"][
            "maxProperties"
        ]
        casework_bound = self.schema["$defs"]["requirement"]["properties"][
            "work_item_model_selections"
        ]["maxProperties"]
        self.assertEqual(owner_bound, 1024)
        self.assertEqual(casework_bound, owner_bound)

        with tempfile.TemporaryDirectory() as directory:
            environment = os.environ.copy()
            environment["NIGHTSHIFT_CASEWORK_PROVIDER_EXECUTION_FIXTURE_DIR"] = directory
            completed = subprocess.run(
                [
                    "cargo",
                    "test",
                    "--locked",
                    "-p",
                    "nightshift-casework",
                    "--lib",
                    "emit_provider_execution_schema_fixture",
                    "--",
                    "--ignored",
                ],
                cwd=ROOT,
                env=environment,
                check=False,
                capture_output=True,
                text=True,
            )
            self.assertEqual(
                completed.returncode,
                0,
                completed.stdout + "\n" + completed.stderr,
            )
            runtime_projection = json.loads(
                (Path(directory) / "provider-execution.v1.json").read_text()
            )

        self.validator.validate(runtime_projection)
        selections = runtime_projection["requirement"]["work_item_model_selections"]
        self.assertGreater(len(selections), 0)
        self.assertLessEqual(len(selections), owner_bound)

        oversized = copy.deepcopy(runtime_projection)
        selection = next(iter(selections.values()))
        oversized["requirement"]["work_item_model_selections"] = {
            f"work-{index:04d}": copy.deepcopy(selection) for index in range(1025)
        }
        with self.assertRaises(ValidationError):
            self.validator.validate(oversized)

    def test_every_object_definition_is_closed(self):
        self.assertFalse(self.schema["additionalProperties"])
        for name, definition in self.schema["$defs"].items():
            if definition.get("type") == "object":
                self.assertFalse(definition.get("additionalProperties", True), name)


if __name__ == "__main__":
    unittest.main()
