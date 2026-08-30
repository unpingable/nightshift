"""Executable qualification for the closed base-admission receipt."""

import copy
import hashlib
import json
import pathlib
import unittest

import jsonschema


ROOT = pathlib.Path(__file__).resolve().parents[1]
SCHEMA_PATH = ROOT / "schemas" / "nightshift.base-admission-receipt.v1.schema.json"
RECEIPT_PATH = (
    ROOT
    / "qualification"
    / "nightshift-casework-custody-convergence-20260829"
    / "base-admission-receipt.v1.json"
)
DOMAIN = b"nightshift.base-admission-receipt.digest/v1\0"


class BaseAdmissionReceiptSchemaTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.schema = json.loads(SCHEMA_PATH.read_bytes())
        cls.receipt = json.loads(RECEIPT_PATH.read_bytes())
        jsonschema.Draft202012Validator.check_schema(cls.schema)
        cls.validator = jsonschema.Draft202012Validator(
            cls.schema, format_checker=jsonschema.FormatChecker()
        )

    def test_checked_in_receipt_validates(self):
        self.validator.validate(self.receipt)

    def test_closed_schema_refuses_qualification_substitution(self):
        invalid = copy.deepcopy(self.receipt)
        invalid["implementation_equivalent_therefore_qualified"] = True
        with self.assertRaises(jsonschema.ValidationError):
            self.validator.validate(invalid)

    def test_receipt_digest_reproduces(self):
        preimage = copy.deepcopy(self.receipt)
        presented = preimage.pop("receipt_digest")
        canonical = json.dumps(
            preimage,
            ensure_ascii=False,
            separators=(",", ":"),
            sort_keys=True,
        ).encode("utf-8")
        digest = hashlib.sha256(DOMAIN + canonical).hexdigest()
        self.assertEqual(presented, f"sha256:{digest}")


if __name__ == "__main__":
    unittest.main()
