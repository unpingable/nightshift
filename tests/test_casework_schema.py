"""Executable qualification for the closed Nightshift Casework projection schema."""

import copy
import json
import pathlib
import unittest

import jsonschema


ROOT = pathlib.Path(__file__).resolve().parents[1]
SCHEMA_PATH = ROOT / "schemas" / "nightshift.casework-run.v1.schema.json"
GOLDEN_PATH = (
    ROOT
    / "qualification"
    / "nightshift-casework-mvp-20260829"
    / "velvet-orrery.casework-run.v1.json"
)


class CaseworkProjectionSchemaTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.schema = json.loads(SCHEMA_PATH.read_bytes())
        cls.golden = json.loads(GOLDEN_PATH.read_bytes())
        jsonschema.Draft202012Validator.check_schema(cls.schema)
        cls.validator = jsonschema.Draft202012Validator(cls.schema)

    def test_checked_in_golden_validates(self):
        self.validator.validate(self.golden)

    def test_closed_schema_refuses_aggregate_result_injection(self):
        invalid = copy.deepcopy(self.golden)
        invalid["aggregate_result"] = "invented"
        with self.assertRaises(jsonschema.ValidationError):
            self.validator.validate(invalid)


if __name__ == "__main__":
    unittest.main()
