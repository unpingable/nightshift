import copy
import json
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

    def test_every_object_definition_is_closed(self):
        self.assertFalse(self.schema["additionalProperties"])
        for name, definition in self.schema["$defs"].items():
            if definition.get("type") == "object":
                self.assertFalse(definition.get("additionalProperties", True), name)


if __name__ == "__main__":
    unittest.main()
