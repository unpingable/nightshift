import json
import pathlib
import unittest

from jsonschema import Draft202012Validator, FormatChecker, ValidationError


ROOT = pathlib.Path(__file__).resolve().parents[1]
DIGEST = "sha256:" + "0" * 64
UTC = "2026-08-30T03:00:00.123456789Z"


def load(name: str) -> dict:
    return json.loads((ROOT / "schemas" / name).read_text())


def lineage_fixture() -> dict:
    return {
        "schema": "nightshift.operational-observation-lineage/v1",
        "lineage_id": DIGEST,
        "monitor_result_head": "b2d52fe34f146774cbf5601819982c267c7fb082",
        "nq_result_head": "39b9f84f2f70955dd12e5cbfe798c740f9e52854",
        "monitor_custody": {
            "raw_bytes_sha256": DIGEST,
            "raw_bytes_length": 1,
            "semantic_digest": DIGEST,
        },
        "nq_custody": {
            "raw_bytes_sha256": DIGEST,
            "raw_bytes_length": 1,
            "semantic_digest": DIGEST,
        },
        "nq_profile_id": "profile:fixture",
        "nq_input_id": "input:fixture",
        "subject": {
            "kind": "service_instance",
            "namespace": "inventory:fixture",
            "basis_contract": "monitor.subject-basis.service-instance-registry/v1",
            "stable_basis": {
                "basis_type": "service_instance",
                "service_identity": DIGEST,
                "instance_identity": DIGEST,
            },
        },
        "subject_identity_digest": DIGEST,
        "producer": {
            "principal_id": "producer:fixture",
            "collector_id": "collector:fixture",
            "key_algorithm": "ed25519",
            "public_key_hex": "0" * 64,
            "public_key_digest": DIGEST,
            "producer_class": "instrumented_monitor",
        },
        "producer_identity_digest": DIGEST,
        "acquisition_outcome": "observation_produced",
        "acquisition_started_at": UTC,
        "acquisition_ended_at": UTC,
        "producer_observed_at": UTC,
        "receiver_custody_at": UTC,
        "nq_qualified_at": UTC,
        "nightshift_admitted_at": UTC,
        "epoch": "epoch:fixture",
        "sequence": 0,
        "predecessor_observation_digest": None,
        "payload_schema": "fixture.payload/v1",
        "claim_support": [{
            "claim_id": "claim:fixture",
            "proposition": "fixture proposition",
            "value_digest": DIGEST,
            "monitor_record_digest": DIGEST,
        }],
        "cannot_testify": [],
        "refusals": [],
        "contradictions": [],
        "nonclaims": [
            "lineage is temporal custody, not claim qualification",
            "currentness is not standing, authorization, or permission to act",
            "producer class does not establish evidentiary precedence",
            "re-observation may acquire testimony but cannot remediate a target",
        ],
    }


def evaluation_fixture() -> dict:
    return {
        "schema": "nightshift.operational-reobservation-evaluation/v1",
        "evaluation_id": DIGEST,
        "lineage_id": DIGEST,
        "profile_id": "profile:fixture",
        "profile_digest": DIGEST,
        "max_age_seconds": 60,
        "evaluated_at": UTC,
        "current_until": "2026-08-30T03:01:00.123456789Z",
        "exact_supported_claim_ids": ["claim:fixture"],
        "disposition": "current",
        "reobservation_trigger": "none",
        "next_lawful_action": "await_currentness_change",
        "grants_authority": False,
    }


class OperationalLineageSchemaTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.lineage_schema = load(
            "nightshift.operational-observation-lineage.v1.schema.json"
        )
        cls.evaluation_schema = load(
            "nightshift.operational-reobservation-evaluation.v1.schema.json"
        )
        Draft202012Validator.check_schema(cls.lineage_schema)
        Draft202012Validator.check_schema(cls.evaluation_schema)
        cls.lineage = Draft202012Validator(
            cls.lineage_schema, format_checker=FormatChecker()
        )
        cls.evaluation = Draft202012Validator(
            cls.evaluation_schema, format_checker=FormatChecker()
        )

    def test_golden_documents_validate(self) -> None:
        self.lineage.validate(lineage_fixture())
        self.evaluation.validate(evaluation_fixture())

    def test_subject_family_contract_and_basis_are_one_exact_alternative(self) -> None:
        wrong_kind = lineage_fixture()
        wrong_kind["subject"]["kind"] = "host"
        with self.assertRaises(ValidationError):
            self.lineage.validate(wrong_kind)

        wrong_basis = lineage_fixture()
        wrong_basis["subject"]["stable_basis"] = {
            "basis_type": "host",
            "machine_identity": DIGEST,
        }
        with self.assertRaises(ValidationError):
            self.lineage.validate(wrong_basis)

        unknown_basis_field = lineage_fixture()
        unknown_basis_field["subject"]["stable_basis"]["hostname"] = "fixture.example"
        with self.assertRaises(ValidationError):
            self.lineage.validate(unknown_basis_field)

    def test_runtime_token_bounds_and_controls_match_schemas(self) -> None:
        oversized_lineage = lineage_fixture()
        oversized_lineage["nq_profile_id"] = "p" * 1025
        with self.assertRaises(ValidationError):
            self.lineage.validate(oversized_lineage)

        controlled_lineage = lineage_fixture()
        controlled_lineage["nq_profile_id"] = "profile:\u0001fixture"
        with self.assertRaises(ValidationError):
            self.lineage.validate(controlled_lineage)

        oversized_evaluation = evaluation_fixture()
        oversized_evaluation["profile_id"] = "p" * 1025
        with self.assertRaises(ValidationError):
            self.evaluation.validate(oversized_evaluation)

        controlled_evaluation = evaluation_fixture()
        controlled_evaluation["profile_id"] = "profile:\u0001fixture"
        with self.assertRaises(ValidationError):
            self.evaluation.validate(controlled_evaluation)

    def test_unknown_fields_and_authority_substitution_refuse(self) -> None:
        unknown = lineage_fixture()
        unknown["aggregate_result"] = "green"
        with self.assertRaises(ValidationError):
            self.lineage.validate(unknown)

        authority = evaluation_fixture()
        authority["grants_authority"] = True
        with self.assertRaises(ValidationError):
            self.evaluation.validate(authority)


if __name__ == "__main__":
    unittest.main()
