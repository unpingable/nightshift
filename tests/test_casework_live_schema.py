"""Executable qualification for the closed live Casework projection schema."""

import copy
import json
import pathlib
import unittest

from jsonschema import Draft202012Validator, ValidationError


ROOT = pathlib.Path(__file__).resolve().parents[1]
DIGEST = "sha256:" + "a" * 64


class LiveCaseworkSchemaTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.schema = json.loads(
            (ROOT / "schemas" / "nightshift.casework-live-run.v1.schema.json").read_bytes()
        )
        Draft202012Validator.check_schema(cls.schema)
        cls.validator = Draft202012Validator(
            cls.schema,
            format_checker=Draft202012Validator.FORMAT_CHECKER,
        )
        cls.document = {
            "schema": "nightshift.casework-live-run/v1",
            "projection_digest": DIGEST,
            "navigation_id": "b" * 64,
            "run_id": "run/live:fixture",
            "evaluated_at": "2026-08-31T00:00:00Z",
            "packet": {
                "packet_id": "packet",
                "packet_digest": DIGEST,
                "exact_bytes_sha256": DIGEST,
                "integrity": "VALID",
                "created_at": "2026-08-30T00:00:00Z",
                "current_until": "2026-09-01T00:00:00Z",
                "currentness": "CURRENT",
            },
            "admission": {
                "admission_digest": DIGEST,
                "exact_bytes_sha256": DIGEST,
                "admitted_at": "2026-08-30T00:00:00Z",
                "expires_at": "2026-09-01T00:00:00Z",
                "currentness": "CURRENT",
                "maximum_concurrent_workers": 2,
            },
            "execution_profile": {
                "profile_digest": DIGEST,
                "exact_bytes_sha256": DIGEST,
                "budget_policy_ref": "policy:fixture",
                "capacity_binding_status": "POLICY_REFERENCE_ONLY_NO_RECORDED_DECISION",
            },
            "foreman": {
                "source_schema": "nightshift.foreman-live-run/v1",
                "lifecycle": "OPEN",
                "scheduler_state_counts": {"READY_ENTRY_EVALUATION": 1},
                "terminal_receipt_count": 0,
                "not_started_receipt_count": 0,
                "closed_final_receipts_digest": None,
            },
            "work_items": [{
                "work_item_id": "lane",
                "track": "fixture",
                "campaign_codename": "LANE",
                "campaign_slug": "lane",
                "dependencies": [],
                "entry_predicates": [],
                "stop_conditions": [],
                "scheduler_state": "READY_ENTRY_EVALUATION",
                "scheduler_state_recognized": True,
                "dependency_terminality": {},
                "resource_lock_keys": [],
                "active_attempt_id": None,
                "adapter_id": "adapter",
                "adapter_version": "adapter/v1",
                "provider_model_class": "bounded",
                "provider_identity": None,
                "model_identity": None,
                "session_identity": None,
                "thread_identity": None,
                "turn_identity": None,
                "queue_identity": None,
                "last_event_sequence": 1,
                "last_event_digest": DIGEST,
                "human_questions": [],
                "accepted_receipt_kind": None,
                "accepted_outcome": None,
                "accepted_outcome_absent_reason": "NO_ACCEPTED_TERMINAL_OR_NOT_STARTED_RECEIPT",
            }],
            "resource_claims": [],
            "events": [{
                "sequence": 1,
                "event_id": "event",
                "work_item_id": None,
                "attempt_id": None,
                "kind": "internal",
                "recorded_at": "2026-08-30T00:00:00Z",
                "retained_raw_digest": DIGEST,
                "exact_bytes_sha256": DIGEST,
                "raw_length": 1,
            }],
            "raw_sources": {
                "packet_sha256": DIGEST,
                "admission_sha256": DIGEST,
                "profile_sha256": DIGEST,
                "journal_framing_sha256": DIGEST,
                "accepted_receipts_framing_sha256": DIGEST,
                "final_snapshot_sha256": None,
            },
            "sealed_case_run_id": None,
            "provider_capacity": {
                "status": "NOT_RECORDED_BY_FOREMAN",
                "observation_digest": None,
                "policy_digest": None,
                "decision_digest": None,
                "explanation": "No exact capacity decision is present.",
            },
            "authority_effect": "READ_ONLY_OPERATOR_PROJECTION",
        }

    def test_closed_live_projection_validates(self):
        self.validator.validate(self.document)

    def test_result_or_authority_injection_is_refused(self):
        for field, value in [
            ("aggregate_result", "invented"),
            ("approve", True),
            ("execute", True),
            ("retry", True),
        ]:
            invalid = copy.deepcopy(self.document)
            invalid[field] = value
            with self.assertRaises(ValidationError):
                self.validator.validate(invalid)

    def test_capacity_is_explicitly_unbound(self):
        invalid = copy.deepcopy(self.document)
        invalid["provider_capacity"]["status"] = "ABUNDANT"
        invalid["provider_capacity"]["decision_digest"] = DIGEST
        with self.assertRaises(ValidationError):
            self.validator.validate(invalid)

    def test_terminal_and_not_started_kinds_remain_distinct(self):
        terminal = copy.deepcopy(self.document)
        item = terminal["work_items"][0]
        item["accepted_receipt_kind"] = "terminal"
        item["accepted_outcome"] = {
            "state": "EXACT",
            "result_classification": "INDEPENDENT",
            "receipt_digest": DIGEST,
        }
        item["accepted_outcome_absent_reason"] = None
        self.validator.validate(terminal)
        terminal["work_items"][0]["accepted_receipt_kind"] = "other"
        with self.assertRaises(ValidationError):
            self.validator.validate(terminal)

        contradictory = copy.deepcopy(self.document)
        contradictory["work_items"][0]["accepted_receipt_kind"] = "terminal"
        with self.assertRaises(ValidationError):
            self.validator.validate(contradictory)

        contradictory = copy.deepcopy(terminal)
        contradictory["work_items"][0]["accepted_receipt_kind"] = None
        with self.assertRaises(ValidationError):
            self.validator.validate(contradictory)


if __name__ == "__main__":
    unittest.main()
