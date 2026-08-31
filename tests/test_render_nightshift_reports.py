import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "render_nightshift_reports.py"
SPEC = importlib.util.spec_from_file_location("render_nightshift_reports", SCRIPT)
REPORTS = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(REPORTS)


class ReportRendererTests(unittest.TestCase):
    def setUp(self):
        self.packet = json.loads(
            (
                ROOT
                / "qualification"
                / "nightshift-packet-v1"
                / "fixtures"
                / "positive.v1.json"
            ).read_text(encoding="utf-8")
        )
        self.receipts = {
            "schema": "nightshift.run-receipts/v1",
            "packet_digest": self.packet["packet_digest"],
            "updated_at": "2026-08-29T17:30:00Z",
            "work_items": [
                {
                    "id": "fixture-work",
                    "state": "QUALIFIED",
                    "result_classification": "FIXTURE-QUALIFIED",
                    "repositories": [],
                    "tests": ["fixture"],
                    "evidence": ["fixture"],
                    "live_or_production_mutations": [],
                    "remaining_trigger": "none",
                    "next_lawful_action": "none",
                }
            ],
            "human_questions": [],
            "repository_custody": [
                {
                    "repository": "fixture",
                    "branch_head": "main@aaaaaaaa",
                    "push_custody": "not applicable",
                    "dirty": "no",
                    "live_runtime": "none",
                    "secrets": "none",
                    "teardown": "none",
                }
            ],
        }

    def test_receipts_render_all_three_reports(self):
        REPORTS.validate(self.packet, self.receipts)
        self.assertIn("fixture-work", REPORTS.render_ledger(self.packet, self.receipts))
        self.assertIn("FIXTURE-QUALIFIED", REPORTS.render_morning(self.packet, self.receipts))
        self.assertEqual(REPORTS.render_questions(self.receipts), "# Human questions\n\nNone.\n")

    def test_exact_human_question_contract_renders_and_legacy_key_refuses(self):
        self.receipts["human_questions"] = [
            {
                "work_item": "fixture-work",
                "exact_question": "What exact authority is present?",
                "evidence_exhausted": "No authority artifact was retained.",
                "safe_default": "Do not continue.",
                "consequences": "The affected lane remains blocked.",
                "resume_point": "Evaluate a successor after exact authority exists.",
            }
        ]
        REPORTS.validate(self.packet, self.receipts)
        rendered = REPORTS.render_questions(self.receipts)
        self.assertIn("- Exact question: What exact authority is present?", rendered)
        self.assertIn("- Evidence exhausted: No authority artifact was retained.", rendered)
        self.assertIn("- Safe default: Do not continue.", rendered)
        self.assertIn("- Consequences: The affected lane remains blocked.", rendered)
        self.assertIn(
            "- Resume point: Evaluate a successor after exact authority exists.",
            rendered,
        )

        self.receipts["human_questions"][0]["question"] = self.receipts[
            "human_questions"
        ][0].pop("exact_question")

        with self.assertRaises(KeyError):
            REPORTS.render_questions(self.receipts)
    def test_unknown_and_missing_work_items_fail_closed(self):
        self.receipts["work_items"][0]["id"] = "unknown"
        with self.assertRaisesRegex(ValueError, "unknown receipt work item"):
            REPORTS.validate(self.packet, self.receipts)
        self.receipts["work_items"] = []
        with self.assertRaisesRegex(ValueError, "missing receipt work item"):
            REPORTS.validate(self.packet, self.receipts)

    def test_cli_writes_only_derived_reports(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            packet_path = output / "packet.json"
            receipts_path = output / "receipts.json"
            packet_path.write_text(json.dumps(self.packet), encoding="utf-8")
            receipts_path.write_text(json.dumps(self.receipts), encoding="utf-8")
            REPORTS.validate(
                REPORTS.load_json(packet_path), REPORTS.load_json(receipts_path)
            )
            (output / "NIGHTSHIFT-RUN-LEDGER.md").write_text(
                REPORTS.render_ledger(self.packet, self.receipts), encoding="utf-8"
            )
            self.assertTrue((output / "NIGHTSHIFT-RUN-LEDGER.md").is_file())


if __name__ == "__main__":
    unittest.main()
