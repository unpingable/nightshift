"""Executable closure checks for the HOLDING-PATTERN qualification receipt."""

import copy
import json
import os
import unittest
from pathlib import Path

from jsonschema import Draft202012Validator, ValidationError


ROOT = Path(__file__).resolve().parents[1]
SCHEMA_PATH = (
    ROOT
    / "schemas"
    / "nightshift.provider-execution-availability-and-deferred-dispatch-qualification.v1.schema.json"
)
DEFAULT_RECEIPT_PATH = (
    ROOT
    / "qualification"
    / "provider-execution-availability-and-deferred-dispatch-v1-20260831"
    / "holding-pattern.qualification.v1.json"
)


class HoldingQualificationReceiptSchemaTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.schema = json.loads(SCHEMA_PATH.read_bytes())
        receipt_path = Path(
            os.environ.get(
                "NIGHTSHIFT_HOLDING_QUALIFICATION_RECEIPT",
                DEFAULT_RECEIPT_PATH,
            )
        )
        cls.receipt = json.loads(receipt_path.read_bytes())
        Draft202012Validator.check_schema(cls.schema)
        cls.validator = Draft202012Validator(cls.schema)

    def assert_refused(self, value):
        with self.assertRaises(ValidationError):
            self.validator.validate(value)

    def test_checked_in_receipt_validates_and_schema_is_closed(self):
        self.validator.validate(self.receipt)
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

    def test_unknown_root_and_nested_fields_refuse(self):
        changed = copy.deepcopy(self.receipt)
        changed["aggregate_result"] = "QUALIFIED"
        self.assert_refused(changed)

        changed = copy.deepcopy(self.receipt)
        changed["closeout"]["retry_authorized"] = True
        self.assert_refused(changed)

    def test_exact_owner_pins_and_independent_classifications_refuse_substitution(self):
        changed = copy.deepcopy(self.receipt)
        changed["exact_accepted_subjects"]["codex_provider_admission_owner"] = "0" * 40
        self.assert_refused(changed)

        changed = copy.deepcopy(self.receipt)
        changed["independent_classifications"]["real_provider_lifecycle"] = "QUALIFIED"
        self.assert_refused(changed)

        changed = copy.deepcopy(self.receipt)
        changed["independent_classifications"]["fuel_independence"] = "AGGREGATED"
        self.assert_refused(changed)

    def test_matrix_has_exactly_twenty_unique_ordered_cases(self):
        self.assertEqual(
            [row["ordinal"] for row in self.receipt["twenty_case_matrix"]],
            list(range(1, 21)),
        )
        self.validator.validate(self.receipt)

        changed = copy.deepcopy(self.receipt)
        changed["twenty_case_matrix"].pop()
        self.assert_refused(changed)

        changed = copy.deepcopy(self.receipt)
        changed["twenty_case_matrix"][0], changed["twenty_case_matrix"][1] = (
            changed["twenty_case_matrix"][1],
            changed["twenty_case_matrix"][0],
        )
        self.assert_refused(changed)

        changed = copy.deepcopy(self.receipt)
        changed["twenty_case_matrix"][1] = copy.deepcopy(
            changed["twenty_case_matrix"][0]
        )
        self.assert_refused(changed)

    def test_not_run_closeout_and_human_question_vocabulary_is_closed(self):
        for field in (
            "real_provider_lifecycle",
            "real_codex_app_server_executable_identity",
            "live_timer_or_service",
            "production_or_default_route",
            "installed_browser",
        ):
            self.assertEqual(
                self.receipt["independent_classifications"][field],
                "NOT_RUN",
            )
        self.assertEqual(self.receipt["human_questions"], [])

        changed = copy.deepcopy(self.receipt)
        changed["human_questions"] = ["unlinked-question"]
        self.assert_refused(changed)

        changed = copy.deepcopy(self.receipt)
        changed["closeout"]["second_watch_started"] = True
        self.assert_refused(changed)

    def test_non_authorizing_and_no_retry_constants_are_exact(self):
        for path in (
            ("mechanism_law", "semantic_retry"),
            ("mechanism_law", "approval_response_authorized"),
            ("mechanism_law", "indeterminate_auto_redispatch"),
            ("mechanism_law", "aggregate_result_created"),
            ("casework", "controls_present"),
            ("casework", "aggregate_present"),
            ("closeout", "approval_response_sent"),
            ("closeout", "protected_effect_occurred"),
        ):
            changed = copy.deepcopy(self.receipt)
            changed[path[0]][path[1]] = True
            self.assert_refused(changed)


if __name__ == "__main__":
    unittest.main()
