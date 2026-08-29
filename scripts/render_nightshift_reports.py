#!/usr/bin/env python3
"""Render Nightshift reports from a sealed packet and explicit run receipts."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

RECEIPT_SCHEMA = "nightshift.run-receipts/v1"


def load_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path}: expected JSON object")
    return value


def validate(packet: dict[str, Any], receipts: dict[str, Any]) -> None:
    if packet.get("schema") != "nightshift.orientation-packet/v1":
        raise ValueError("foreign packet schema")
    if receipts.get("schema") != RECEIPT_SCHEMA:
        raise ValueError("foreign receipt schema")
    if receipts.get("packet_digest") != packet.get("packet_digest"):
        raise ValueError("receipt packet digest mismatch")
    packet_items = packet.get("work_items")
    receipt_items = receipts.get("work_items")
    if not isinstance(packet_items, list) or not isinstance(receipt_items, list):
        raise ValueError("work_items must be arrays")
    packet_ids = [item.get("id") for item in packet_items]
    receipt_ids = [item.get("id") for item in receipt_items]
    if len(packet_ids) != len(set(packet_ids)):
        raise ValueError("duplicate packet work item")
    if len(receipt_ids) != len(set(receipt_ids)):
        raise ValueError("duplicate receipt work item")
    unknown = sorted(set(receipt_ids) - set(packet_ids))
    missing = sorted(set(packet_ids) - set(receipt_ids))
    if unknown:
        raise ValueError(f"unknown receipt work item: {', '.join(unknown)}")
    if missing:
        raise ValueError(f"missing receipt work item: {', '.join(missing)}")
    required = {
        "id",
        "state",
        "result_classification",
        "repositories",
        "tests",
        "evidence",
        "live_or_production_mutations",
        "remaining_trigger",
        "next_lawful_action",
    }
    for item in receipt_items:
        absent = sorted(required - set(item))
        if absent:
            raise ValueError(f"{item.get('id')}: missing receipt fields: {', '.join(absent)}")
    if not isinstance(receipts.get("human_questions"), list):
        raise ValueError("human_questions must be an array")
    if not isinstance(receipts.get("repository_custody"), list):
        raise ValueError("repository_custody must be an array")


def md(value: Any) -> str:
    text = str(value)
    return text.replace("|", "\\|").replace("\n", "<br>")


def render_ledger(packet: dict[str, Any], receipts: dict[str, Any]) -> str:
    by_id = {item["id"]: item for item in receipts["work_items"]}
    lines = [
        "# Nightshift run ledger",
        "",
        f"- Packet: `{packet['packet_id']}`",
        f"- Packet digest: `{packet['packet_digest']}`",
        f"- Receipt snapshot: `{receipts['updated_at']}`",
        "- Aggregate verdict: none; every campaign retains its own classification.",
        "",
        "## Campaign DAG",
        "",
        "```text",
    ]
    for item in packet["work_items"]:
        deps = ", ".join(item["dependencies"]) or "root"
        lines.append(f"{item['id']} <- {deps}")
    lines.extend(
        [
            "```",
            "",
            "## Per-workstream state",
            "",
            "| Work item | Campaign | Dependencies | State | Classification |",
            "|---|---|---|---|---|",
        ]
    )
    for item in packet["work_items"]:
        receipt = by_id[item["id"]]
        deps = ", ".join(item["dependencies"]) or "none"
        campaign = f"{item['campaign']['codename']} / {item['campaign']['canonical_slug']}"
        lines.append(
            f"| {md(item['id'])} | {md(campaign)} | {md(deps)} | "
            f"{md(receipt['state'])} | {md(receipt['result_classification'])} |"
        )
    return "\n".join(lines) + "\n"


def render_questions(receipts: dict[str, Any]) -> str:
    lines = ["# Human questions", ""]
    questions = receipts["human_questions"]
    if not questions:
        lines.append("None.")
        return "\n".join(lines) + "\n"
    for index, question in enumerate(questions, 1):
        lines.extend(
            [
                f"## {index}. {question['work_item']}",
                "",
                f"- Exact question: {question['exact_question']}",
                f"- Evidence exhausted: {question['evidence_exhausted']}",
                f"- Safe default: {question['safe_default']}",
                f"- Consequences: {question['consequences']}",
                f"- Resume point: {question['resume_point']}",
                "",
            ]
        )
    return "\n".join(lines)


def render_morning(packet: dict[str, Any], receipts: dict[str, Any]) -> str:
    packet_by_id = {item["id"]: item for item in packet["work_items"]}
    lines = [
        "# Morning report",
        "",
        "> Generated from the sealed packet and explicit run receipts. It does not create a campaign result or confer authority.",
        "",
        f"- Packet digest: `{packet['packet_digest']}`",
        f"- Receipt snapshot: `{receipts['updated_at']}`",
        "",
    ]
    for receipt in receipts["work_items"]:
        item = packet_by_id[receipt["id"]]
        lines.extend(
            [
                f"## {item['campaign']['codename']} — `{item['campaign']['canonical_slug']}`",
                "",
                f"- Track: {item['track']}",
                f"- Predecessor/base: {', '.join(p['commit'] for p in item['predecessor_lineage']) or 'none'}",
                f"- State: {receipt['state']}",
                f"- Result classification: {receipt['result_classification']}",
                f"- Repositories: {json.dumps(receipt['repositories'], sort_keys=True)}",
                f"- Tests: {'; '.join(receipt['tests']) or 'none recorded'}",
                f"- Evidence: {'; '.join(receipt['evidence']) or 'none recorded'}",
                f"- Live/production mutations: {'; '.join(receipt['live_or_production_mutations']) or 'none'}",
                f"- Remaining trigger: {receipt['remaining_trigger']}",
                f"- Exact next lawful action: {receipt['next_lawful_action']}",
                "",
            ]
        )
    lines.extend(
        [
            "## Final repository custody",
            "",
            "| Repository | Branch/head | Push custody | Dirty | Live runtime | Secrets | Teardown |",
            "|---|---|---|---|---|---|---|",
        ]
    )
    for row in receipts["repository_custody"]:
        lines.append(
            f"| {md(row['repository'])} | {md(row['branch_head'])} | "
            f"{md(row['push_custody'])} | {md(row['dirty'])} | "
            f"{md(row['live_runtime'])} | {md(row['secrets'])} | {md(row['teardown'])} |"
        )
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--packet", type=Path, required=True)
    parser.add_argument("--receipts", type=Path, required=True)
    parser.add_argument("--output-directory", type=Path, required=True)
    args = parser.parse_args()

    packet = load_json(args.packet)
    receipts = load_json(args.receipts)
    validate(packet, receipts)
    args.output_directory.mkdir(parents=True, exist_ok=True)
    (args.output_directory / "NIGHTSHIFT-RUN-LEDGER.md").write_text(
        render_ledger(packet, receipts), encoding="utf-8"
    )
    (args.output_directory / "HUMAN-QUESTIONS.md").write_text(
        render_questions(receipts), encoding="utf-8"
    )
    (args.output_directory / "MORNING-REPORT.md").write_text(
        render_morning(packet, receipts), encoding="utf-8"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
