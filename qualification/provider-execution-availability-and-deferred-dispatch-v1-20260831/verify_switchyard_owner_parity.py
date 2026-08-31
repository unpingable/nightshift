#!/usr/bin/env python3
"""Replay every checked HOLDING mapper fixture through exact Switchyard 2ba.

This is a qualification-only read predicate.  Nightshift's Rust contract suite
opens the same fixture bytes independently; this predicate proves that the
accepted owner state machine produces the identical snapshot and result.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import subprocess
import sys


OWNER_HEAD = "2ba25db66d8b29dd215bd87e05f4ea794024b3b7"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--switchyard-root", type=Path, required=True)
    args = parser.parse_args()
    root = args.switchyard_root.resolve(strict=True)
    head = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    if head != OWNER_HEAD:
        raise SystemExit(f"Switchyard owner head is {head}, expected {OWNER_HEAD}")

    sys.path.insert(0, str(root / "src"))
    from switchyard.provider_admission import replay_snapshot  # noqa: PLC0415

    fixture_dir = Path(__file__).resolve().parent / "fixtures"
    fixtures = sorted(fixture_dir.glob("switchyard-*.snapshot.v1.json"))
    if not fixtures:
        raise SystemExit("no checked Switchyard snapshots")
    for fixture in fixtures:
        raw = fixture.read_bytes()
        value = json.loads(raw)
        replayed = replay_snapshot(value).snapshot()
        if replayed != value:
            raise SystemExit(f"owner replay changed {fixture.name}")
    print(f"owner={OWNER_HEAD} exact_snapshots={len(fixtures)} parity=ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
