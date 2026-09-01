"""Executable closure checks for SECOND-WATCH bootstrap V1."""

import copy
import json
import unittest
from pathlib import Path

from jsonschema import Draft202012Validator, FormatChecker, ValidationError


ROOT = Path(__file__).resolve().parents[1]
SCHEMA_PATH = (
    ROOT / "schemas" / "nightshift.self-hosted-foreman-bootstrap.v1.schema.json"
)
D = "sha256:" + "0" * 64


def specimen():
    return {
        "schema": "nightshift.self-hosted-foreman-bootstrap/v1",
        "bootstrap_digest": D,
        "digest_preimage": (
            "domain prefix nightshift.self-hosted-foreman-bootstrap.digest/v1 NUL, "
            "then the bootstrap object with bootstrap_digest omitted as RFC8785-JCS"
        ),
        "campaign_codename": "SECOND-WATCH",
        "canonical_slug": "nightshift-self-hosted-foreman-bootstrap-v1",
        "track": "nightshift-self-hosting",
        "holding_result_head": "0dff82fa3522e59a6ce8e8161f6aed92cbacc061",
        "holding_qualified_subject": "57c165fb246a530bc9448afbe3a26c17a5118ebd",
        "durable_roadmap_head": "70e3b734e979173ae552efb322b48bf7fb0c028b",
        "midnight_result_head": "6160a7fac9845aaefefbc11847e55786b35749e6",
        "silicon_result_head": "f6e95c8a51982a9381c27c4792c8d9fd6f1daf47",
        "codex_owner_head": "c36a8137638decf8b04a49611354a90f32c5a945",
        "switchyard_owner_head": "2ba25db66d8b29dd215bd87e05f4ea794024b3b7",
        "bootstrap_occurrence_id": "bootstrap-second-watch-1",
        "run_id": "second-watch-run",
        "packet_id": "second-watch-fixture",
        "packet_digest": "sha256:" + "1" * 64,
        "predecessor_v2_packet_digest": (
            "sha256:1df7f47bb3ea70d0f987e756f34aaa62f7187a659ef0bcc8d7c8aa2e645431fc"
        ),
        "admission_digest": D,
        "profile_digest": D,
        "capacity_requirement_digest": D,
        "capacity_policy_digest": D,
        "execution_availability_requirement_digest": D,
        "execution_availability_policy_digest": D,
        "local_runtime_identity": "second-watch-local-runtime",
        "evaluated_at": "2026-08-31T12:00:01Z",
        "expected_work_item_count": 4,
        "initially_runnable_lane_count": 3,
        "presentation_only_question_work_item_id": "question",
        "maximum_driver_steps": 100,
        "maximum_wall_seconds": 600,
        "bootstrap_depth": 0,
        "parent_bootstrap_occurrence_id": None,
        "scheduler_owner": "NIGHTSHIFT_DURABLE_FOREMAN",
        "worker_adapter_mode": "CAMPAIGN_QUALIFICATION_DETERMINISTIC_FAKE",
        "wake_source_policy": (
            "QUALIFIED_LOCAL_REEVALUATION_NO_EVIDENCE_OR_AUTHORITY"
        ),
        "closeout_policy": "ALL_EXPLICIT_TERMINAL_OR_NOT_STARTED",
        "authority_effect": "LOCAL_AGENT_COMPUTE_SCHEDULING_ONLY",
        "target_effects_authorized": False,
        "approval_response_authorized": False,
        "protected_effect_authorized": False,
        "semantic_retry_authorized": False,
        "bootstrap_may_nest": False,
        "worker_may_invoke_bootstrap": False,
        "outer_conversation_scheduler": False,
        "timer_or_service_activation_authorized": False,
        "production_activation_authorized": False,
        "aggregate_result_created": False,
    }


class SelfHostedBootstrapSchemaTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.schema = json.loads(SCHEMA_PATH.read_bytes())
        Draft202012Validator.check_schema(cls.schema)
        cls.validator = Draft202012Validator(
            cls.schema, format_checker=FormatChecker()
        )

    def assert_refused(self, value):
        with self.assertRaises(ValidationError):
            self.validator.validate(value)

    def test_schema_is_closed_and_specimen_validates(self):
        self.validator.validate(specimen())
        self.assertEqual(
            self.schema["$schema"],
            "https://json-schema.org/draft/2020-12/schema",
        )

        def visit(value):
            if isinstance(value, dict):
                if value.get("type") == "object":
                    self.assertFalse(value.get("additionalProperties", True))
                for child in value.values():
                    visit(child)
            elif isinstance(value, list):
                for child in value:
                    visit(child)

        visit(self.schema)

    def test_unknown_field_and_owner_pin_substitution_refuse(self):
        changed = specimen()
        changed["approve"] = True
        self.assert_refused(changed)

        changed = specimen()
        changed["holding_result_head"] = "0" * 40
        self.assert_refused(changed)

        changed = specimen()
        changed["switchyard_owner_head"] = "0" * 40
        self.assert_refused(changed)

    def test_v2_packet_reuse_and_noncanonical_identity_refuse(self):
        changed = specimen()
        changed["packet_digest"] = changed["predecessor_v2_packet_digest"]
        self.assert_refused(changed)

        changed = specimen()
        changed["profile_digest"] = "sha256:" + "A" * 64
        self.assert_refused(changed)

        changed = specimen()
        changed["evaluated_at"] = "2026-08-31T08:00:01-04:00"
        self.assert_refused(changed)

    def test_authority_recursion_retry_and_activation_are_fixed_false(self):
        for field in (
            "target_effects_authorized",
            "approval_response_authorized",
            "protected_effect_authorized",
            "semantic_retry_authorized",
            "bootstrap_may_nest",
            "worker_may_invoke_bootstrap",
            "outer_conversation_scheduler",
            "timer_or_service_activation_authorized",
            "production_activation_authorized",
            "aggregate_result_created",
        ):
            changed = specimen()
            changed[field] = True
            self.assert_refused(changed)

    def test_bounds_and_depth_are_closed(self):
        for field, value in (
            ("expected_work_item_count", 2),
            ("initially_runnable_lane_count", 1),
            ("maximum_driver_steps", 0),
            ("maximum_wall_seconds", 86401),
            ("bootstrap_depth", 1),
        ):
            changed = specimen()
            changed[field] = value
            self.assert_refused(changed)

        changed = specimen()
        changed["parent_bootstrap_occurrence_id"] = "bootstrap-parent"
        self.assert_refused(changed)


def driver_step_specimen():
    return {
        "schema": "nightshift.self-hosted-foreman-driver-step/v1",
        "step_digest": D,
        "bootstrap_digest": D,
        "bootstrap_occurrence_id": "bootstrap-second-watch-1",
        "run_id": "second-watch-run",
        "step_ordinal": 1,
        "scheduler_process_occurrence_id": "scheduler-process-1",
        "observed_projection_digest": D,
        "disposition": "READY_WORK_PRESENT",
        "recorded_at": "2026-08-31T12:00:02Z",
        "worker_dispatch_authorized": False,
        "approval_response_authorized": False,
        "protected_effect_authorized": False,
        "semantic_retry_authorized": False,
        "aggregate_result_created": False,
    }


class SelfHostedDriverStepSchemaTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        path = (
            ROOT
            / "schemas"
            / "nightshift.self-hosted-foreman-driver-step.v1.schema.json"
        )
        cls.schema = json.loads(path.read_bytes())
        Draft202012Validator.check_schema(cls.schema)
        cls.validator = Draft202012Validator(
            cls.schema, format_checker=FormatChecker()
        )

    def assert_refused(self, value):
        with self.assertRaises(ValidationError):
            self.validator.validate(value)

    def test_exact_closed_non_authorizing_step_validates(self):
        self.validator.validate(driver_step_specimen())
        changed = driver_step_specimen()
        changed["dispatch"] = True
        self.assert_refused(changed)
        for field in (
            "worker_dispatch_authorized",
            "approval_response_authorized",
            "protected_effect_authorized",
            "semantic_retry_authorized",
            "aggregate_result_created",
        ):
            changed = driver_step_specimen()
            changed[field] = True
            self.assert_refused(changed)

    def test_digest_time_ordinal_and_disposition_bounds_refuse(self):
        for field, value in (
            ("step_digest", "sha256:" + "A" * 64),
            ("recorded_at", "2026-08-31T08:00:02-04:00"),
            ("step_ordinal", 0),
            ("step_ordinal", 1000001),
            ("disposition", "RETRY"),
        ):
            changed = driver_step_specimen()
            changed[field] = value
            self.assert_refused(changed)
if __name__ == "__main__":
    unittest.main()
