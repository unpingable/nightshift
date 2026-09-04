"""Executable closure checks for the SECOND-WATCH qualification receipt."""

import copy
import json
import os
import unittest
from pathlib import Path

from jsonschema import Draft202012Validator, ValidationError


ROOT = Path(__file__).resolve().parents[1]
SCHEMA_PATH = ROOT / "schemas/nightshift.self-hosted-foreman-bootstrap-qualification.v1.schema.json"
DEFAULT_RECEIPT_PATH = (
    ROOT
    / "qualification"
    / "nightshift-self-hosted-foreman-bootstrap-v1-20260831"
    / "second-watch.qualification.v1.json"
)


class SecondWatchQualificationReceiptSchemaTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.schema = json.loads(SCHEMA_PATH.read_bytes())
        receipt_path = Path(
            os.environ.get(
                "NIGHTSHIFT_SECOND_WATCH_QUALIFICATION_RECEIPT",
                DEFAULT_RECEIPT_PATH,
            )
        )
        cls.receipt = json.loads(receipt_path.read_bytes())
        Draft202012Validator.check_schema(cls.schema)
        cls.validator = Draft202012Validator(cls.schema)

    def assert_refused(self, value):
        with self.assertRaises(ValidationError):
            self.validator.validate(value)

    def test_checked_in_receipt_matches_closed_schema(self):
        self.validator.validate(self.receipt)
        self.assertFalse(self.schema["additionalProperties"])
        self.assertEqual(self.schema["const"], self.receipt)

    def test_unknown_root_and_nested_fields_refuse(self):
        changed = copy.deepcopy(self.receipt)
        changed["aggregate_result"] = "QUALIFIED"
        self.assert_refused(changed)
        changed = copy.deepcopy(self.receipt)
        changed["closeout"]["retry_authorized"] = True
        self.assert_refused(changed)

    def test_exact_subjects_and_classifications_refuse_substitution(self):
        changed = copy.deepcopy(self.receipt)
        changed["qualified_subject_commit"] = "0" * 40
        self.assert_refused(changed)
        changed = copy.deepcopy(self.receipt)
        changed["independent_classifications"]["real_provider_lifecycle"] = "QUALIFIED"
        self.assert_refused(changed)

    def test_golden_counts_and_digests_are_exact(self):
        self.assertEqual(self.receipt["golden_journey"]["provider_dispatch_count"], 5)
        self.assertEqual(
            self.receipt["qualification"]["qualified_subject_executable_schema_discovery_passed"],
            70,
        )
        changed = copy.deepcopy(self.receipt)
        changed["golden_journey"]["packet_sha256"] = "sha256:" + "0" * 64
        self.assert_refused(changed)

    def test_no_retry_authority_or_aggregate_can_be_added(self):
        for section, field in (
            ("mechanism_law", "semantic_retry"),
            ("mechanism_law", "approval_response_authorized"),
            ("mechanism_law", "protected_effect_authorized"),
            ("mechanism_law", "aggregate_result_created"),
        ):
            changed = copy.deepcopy(self.receipt)
            changed[section][field] = True
            self.assert_refused(changed)

    def test_closeout_and_human_questions_are_exact(self):
        self.assertEqual(self.receipt["human_questions"], [])
        for field, value in self.receipt["closeout"].items():
            self.assertFalse(value, field)
        changed = copy.deepcopy(self.receipt)
        changed["human_questions"] = ["unlinked-question"]
        self.assert_refused(changed)


if __name__ == "__main__":
    unittest.main()
