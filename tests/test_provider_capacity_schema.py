"""Closed-schema and digest qualification for provider capacity V1."""

import copy
import hashlib
import json
import pathlib
import unittest

import jsonschema


ROOT = pathlib.Path(__file__).resolve().parents[1]
QUAL = (
    ROOT
    / "qualification"
    / "provider-capacity-observation-and-scheduling-policy-v1-20260830"
)
CASES = {
    "observation": (
        ROOT / "schemas" / "nightshift.provider-capacity-observation.v1.schema.json",
        QUAL / "live-codex-observation.v1.json",
        b"nightshift.provider-capacity-observation.digest/v1\0",
        "observation_digest",
    ),
    "policy": (
        ROOT / "schemas" / "nightshift.provider-capacity-policy.v1.schema.json",
        QUAL / "default-policy.v1.json",
        b"nightshift.provider-capacity-policy.digest/v1\0",
        "policy_digest",
    ),
    "decision": (
        ROOT / "schemas" / "nightshift.provider-capacity-decision.v1.schema.json",
        QUAL / "live-codex-decision.v1.json",
        b"nightshift.provider-capacity-decision.digest/v1\0",
        "decision_digest",
    ),
}


class ProviderCapacitySchemaTests(unittest.TestCase):
    def test_checked_in_records_validate_and_digests_reproduce(self):
        for name, (schema_path, record_path, domain, digest_field) in CASES.items():
            with self.subTest(name=name):
                schema = json.loads(schema_path.read_bytes())
                record = json.loads(record_path.read_bytes())
                jsonschema.Draft202012Validator.check_schema(schema)
                jsonschema.Draft202012Validator(
                    schema, format_checker=jsonschema.FormatChecker()
                ).validate(record)

                preimage = copy.deepcopy(record)
                presented = preimage.pop(digest_field)
                canonical = json.dumps(
                    preimage,
                    ensure_ascii=False,
                    separators=(",", ":"),
                    sort_keys=True,
                ).encode("utf-8")
                expected = hashlib.sha256(domain + canonical).hexdigest()
                self.assertEqual(presented, f"sha256:{expected}")

    def test_schemas_refuse_semantic_extension(self):
        for name, (schema_path, record_path, _, _) in CASES.items():
            with self.subTest(name=name):
                schema = json.loads(schema_path.read_bytes())
                record = json.loads(record_path.read_bytes())
                record["aggregate_result"] = "invented"
                with self.assertRaises(jsonschema.ValidationError):
                    jsonschema.Draft202012Validator(schema).validate(record)

    def test_decision_binds_exact_observation_and_policy(self):
        observation = json.loads((QUAL / "live-codex-observation.v1.json").read_bytes())
        policy = json.loads((QUAL / "default-policy.v1.json").read_bytes())
        decision = json.loads((QUAL / "live-codex-decision.v1.json").read_bytes())
        self.assertEqual(
            decision["observation_digest"], observation["observation_digest"]
        )
        self.assertEqual(decision["policy_digest"], policy["policy_digest"])

    def test_policy_requires_explicit_unique_window_types(self):
        schema_path, record_path, _, _ = CASES["policy"]
        schema = json.loads(schema_path.read_bytes())
        policy = json.loads(record_path.read_bytes())
        policy.pop("required_window_types")
        with self.assertRaises(jsonschema.ValidationError):
            jsonschema.Draft202012Validator(schema).validate(policy)

        policy = json.loads(record_path.read_bytes())
        policy["required_window_types"] = ["WEEKLY", "WEEKLY"]
        with self.assertRaises(jsonschema.ValidationError):
            jsonschema.Draft202012Validator(schema).validate(policy)

    def test_decision_schema_refuses_state_admission_substitution(self):
        schema_path, record_path, domain, digest_field = CASES["decision"]
        schema = json.loads(schema_path.read_bytes())
        decision = json.loads(record_path.read_bytes())
        decision.update(
            {
                "state": "CRITICAL",
                "admission": "ORDINARY_BOUNDED",
                "allow_new_expensive_work": True,
                "allow_new_speculative_work": True,
                "reason_codes": ["MINIMUM_REMAINING_WINDOW_CRITICAL"],
            }
        )
        decision.pop(digest_field)
        canonical = json.dumps(
            decision,
            ensure_ascii=False,
            separators=(",", ":"),
            sort_keys=True,
        ).encode("utf-8")
        decision[digest_field] = f"sha256:{hashlib.sha256(domain + canonical).hexdigest()}"
        with self.assertRaises(jsonschema.ValidationError):
            jsonschema.Draft202012Validator(schema).validate(decision)


if __name__ == "__main__":
    unittest.main()
