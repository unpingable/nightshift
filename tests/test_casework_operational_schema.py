"""Executable qualification for closed operational-condition Casework schemas."""

import copy
import json
import pathlib
import unittest

from jsonschema import (
    Draft202012Validator,
    FormatChecker,
    RefResolver,
    ValidationError,
)

from test_operational_lineage_schema import (
    DIGEST,
    evaluation_fixture,
    lineage_fixture,
)


ROOT = pathlib.Path(__file__).resolve().parents[1]
NAVIGATION_ID = "a" * 64


def load(name: str) -> dict:
    return json.loads((ROOT / "schemas" / name).read_text())

def absolute_owner_refs(value, base: str):
    if isinstance(value, dict):
        return {
            key: base + child
            if key == "$ref" and isinstance(child, str) and child.startswith("#")
            else absolute_owner_refs(child, base)
            for key, child in value.items()
        }
    if isinstance(value, list):
        return [absolute_owner_refs(child, base) for child in value]
    return value


def raw_source() -> dict:
    return {
        "exact_bytes_sha256": DIGEST,
        "exact_bytes_length": 1,
        "validation": "exact_owner_contract_valid",
    }


def condition_fixture() -> dict:
    lineage = lineage_fixture()
    evaluation = evaluation_fixture()
    return {
        "schema": "nightshift.casework-operational-condition/v1",
        "projection_digest": DIGEST,
        "navigation_id": NAVIGATION_ID,
        "subject": copy.deepcopy(lineage["subject"]),
        "subject_identity_digest": lineage["subject_identity_digest"],
        "producer": copy.deepcopy(lineage["producer"]),
        "producer_identity_digest": lineage["producer_identity_digest"],
        "acquisition_outcome": lineage["acquisition_outcome"],
        "lineage": lineage,
        "evaluation": evaluation,
        "profile": {
            "profile_id": evaluation["profile_id"],
            "max_age_seconds": evaluation["max_age_seconds"],
        },
        "questions": [],
        "raw_sources": {
            name: raw_source()
            for name in ["monitor", "nq", "lineage", "profile", "evaluation"]
        },
        "authority_effect": "read_only_projection_no_authority",
    }


def index_fixture() -> dict:
    condition = condition_fixture()
    evaluation = condition["evaluation"]
    lineage = condition["lineage"]
    return {
        "schema": "nightshift.casework-operational-condition-index/v1",
        "conditions": [{
            "navigation_id": condition["navigation_id"],
            "projection_digest": condition["projection_digest"],
            "lineage_id": lineage["lineage_id"],
            "evaluation_id": evaluation["evaluation_id"],
            "subject_kind": lineage["subject"]["kind"],
            "subject_namespace": lineage["subject"]["namespace"],
            "subject_identity_digest": lineage["subject_identity_digest"],
            "disposition": evaluation["disposition"],
            "reobservation_trigger": evaluation["reobservation_trigger"],
            "evaluated_at": evaluation["evaluated_at"],
            "question_count": 0,
        }],
    }


class OperationalCaseworkSchemaTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        lineage_schema = absolute_owner_refs(
            load("nightshift.operational-observation-lineage.v1.schema.json"),
            "urn:nightshift:operational-observation-lineage:v1",
        )
        evaluation_schema = absolute_owner_refs(
            load("nightshift.operational-reobservation-evaluation.v1.schema.json"),
            "urn:nightshift:operational-reobservation-evaluation:v1",
        )
        cls.condition_schema = load(
            "nightshift.casework-operational-condition.v1.schema.json"
        )
        cls.index_schema = load(
            "nightshift.casework-operational-condition-index.v1.schema.json"
        )
        for schema in [
            lineage_schema,
            evaluation_schema,
            cls.condition_schema,
            cls.index_schema,
        ]:
            Draft202012Validator.check_schema(schema)
        resolver = RefResolver.from_schema(
            cls.condition_schema,
            store={
                lineage_schema["$id"]: lineage_schema,
                evaluation_schema["$id"]: evaluation_schema,
            },
        )
        cls.condition = Draft202012Validator(
            cls.condition_schema,
            resolver=resolver,
            format_checker=FormatChecker(),
        )
        cls.index = Draft202012Validator(
            cls.index_schema,
            format_checker=FormatChecker(),
        )

    def test_closed_condition_and_distinct_index_validate(self) -> None:
        self.condition.validate(condition_fixture())
        self.index.validate(index_fixture())

    def test_authority_aggregate_and_control_substitutions_refuse(self) -> None:
        for field, value in [
            ("aggregate_health", "healthy"),
            ("approve", True),
            ("execute", True),
            ("retry", True),
            ("remediate", True),
        ]:
            invalid = condition_fixture()
            invalid[field] = value
            with self.assertRaises(ValidationError):
                self.condition.validate(invalid)

        authority = condition_fixture()
        authority["authority_effect"] = "authorize_repair"
        with self.assertRaises(ValidationError):
            self.condition.validate(authority)

    def test_duplicate_owner_fields_and_raw_contracts_stay_closed(self) -> None:
        wrong_subject = condition_fixture()
        wrong_subject["subject"]["kind"] = "host"
        with self.assertRaises(ValidationError):
            self.condition.validate(wrong_subject)

        oversized_raw = condition_fixture()
        oversized_raw["raw_sources"]["monitor"]["exact_bytes_length"] = 1048577
        with self.assertRaises(ValidationError):
            self.condition.validate(oversized_raw)

        unknown_raw = condition_fixture()
        unknown_raw["raw_sources"]["nq"]["extension"] = "raw-only"
        with self.assertRaises(ValidationError):
            self.condition.validate(unknown_raw)

    def test_question_is_typed_bound_and_presentation_only(self) -> None:
        condition = condition_fixture()
        condition["questions"] = [{
            "navigation_id": "b" * 64,
            "question_id": "b" * 64,
            "question": "NQ cannot testify to claim claim:fixture",
            "source_index": 0,
            "source": {
                "source_kind": "cannot_testify",
                "finding": {
                    "claim_id": "claim:fixture",
                    "reason": "profile claim absent from exact observation payload",
                },
            },
            "next_lawful_action": "request_reobservation",
            "presentation_only": True,
        }]
        self.condition.validate(condition)

        disposition = copy.deepcopy(condition)
        disposition["questions"][0]["answer"] = "ignore"
        with self.assertRaises(ValidationError):
            self.condition.validate(disposition)

        unbound = copy.deepcopy(condition)
        unbound["questions"][0]["presentation_only"] = False
        with self.assertRaises(ValidationError):
            self.condition.validate(unbound)

        unknown_finding = copy.deepcopy(condition)
        unknown_finding["questions"][0]["source"]["finding"]["confidence"] = 1
        with self.assertRaises(ValidationError):
            self.condition.validate(unknown_finding)


if __name__ == "__main__":
    unittest.main()
