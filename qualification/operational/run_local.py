#!/usr/bin/env python3
"""Run bounded Nightshift local development evidence with exact logs."""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
import subprocess
from pathlib import Path
from typing import Any


REPO = Path(__file__).resolve().parents[2]


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False) + "\n").encode()


def sha256(data: bytes) -> str:
    return "sha256:" + hashlib.sha256(data).hexdigest()


def write_exclusive(path: Path, data: bytes) -> None:
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(descriptor, "wb") as stream:
        stream.write(data)
        stream.flush()
        os.fsync(stream.fileno())


def git_status() -> str:
    completed = subprocess.run(
        ["git", "status", "--porcelain=v1", "--untracked-files=all"],
        cwd=REPO,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=True,
    )
    return completed.stdout.decode("utf-8", "replace").strip()


def run_step(index: int, name: str, argv: list[str], output: Path) -> dict[str, Any]:
    started = dt.datetime.now(dt.timezone.utc).isoformat()
    completed = subprocess.run(
        argv,
        cwd=REPO,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    stem = f"{index:02d}-{name}"
    write_exclusive(output / f"{stem}.stdout", completed.stdout)
    write_exclusive(output / f"{stem}.stderr", completed.stderr)
    record = {
        "name": name,
        "argv": argv,
        "started_at_utc": started,
        "ended_at_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "exit_code": completed.returncode,
        "stdout_sha256": sha256(completed.stdout),
        "stderr_sha256": sha256(completed.stderr),
    }
    write_exclusive(output / f"{stem}.json", canonical_bytes(record))
    return record


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--allow-dirty-development", action="store_true")
    args = parser.parse_args()

    status = git_status()
    if status and not args.allow_dirty_development:
        parser.error("source tree is dirty; commit it or pass --allow-dirty-development")
    args.output.mkdir(mode=0o700, parents=True, exist_ok=False)
    preflight = subprocess.run(
        [
            "python3",
            str(REPO / "qualification/operational/preflight.py"),
            "--profile", "local",
            "--output", str(args.output / "preflight"),
        ],
        cwd=REPO,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    write_exclusive(args.output / "preflight.stdout", preflight.stdout)
    write_exclusive(args.output / "preflight.stderr", preflight.stderr)

    commands = [
        ("harness-self-test", ["python3", "qualification/operational/self_test.py"]),
        ("build-nightshift", ["cargo", "build", "--locked", "--bin", "nightshift"]),
        ("currentness", ["cargo", "test", "--locked", "--lib", "currentness::tests::"]),
        ("canonical-store", ["cargo", "test", "--locked", "--lib", "canonical_store::tests::"]),
        ("canonical-runtime", ["cargo", "test", "--locked", "--lib", "canonical_runtime::tests::"]),
        ("ag-port", ["cargo", "test", "--locked", "--lib", "ag_port::tests::"]),
        (
            "multiprocess-cycle",
            [
                "python3", "qualification/operational/multiprocess_cycle.py",
                "--nightshift", "target/debug/nightshift",
                "--resolver", "qualification/operational/fixtures/pulse-support-resolver",
                "--output", str(args.output / "multiprocess"),
            ],
        ),
        ("structural-exclusivity", ["bash", "scripts/check_no_actuation_surface.sh"]),
        ("structural-guard-self-test", ["bash", "scripts/check_no_actuation_surface.sh", "--self-test-inject"]),
    ]
    records = []
    if preflight.returncode == 0:
        for index, (name, argv) in enumerate(commands, 1):
            record = run_step(index, name, argv, args.output)
            records.append(record)
            if record["exit_code"] != 0:
                break

    passed = preflight.returncode == 0 and len(records) == len(commands) and all(
        record["exit_code"] == 0 for record in records
    )
    result = {
        "schema": "nightshift.operational-local-development-run.v1",
        "authority_use": "none",
        "qualification_status": "not_assessed",
        "development_check": "passed" if passed else "failed",
        "dirty_development_mode": bool(status),
        "source_status_sha256": sha256((status + "\n").encode()),
        "limitations": [
            "No real systemd timer or deployed service was exercised.",
            "The support child is a deterministic fixture, not Pulse.",
            "No wall-clock rollback, abrupt power loss, or block-I/O fault was injected.",
            "A passing result is development evidence only."
        ],
        "preflight_exit_code": preflight.returncode,
        "steps": records,
    }
    write_exclusive(args.output / "result.json", canonical_bytes(result))
    print(json.dumps({"development_check": result["development_check"], "output": str(args.output)}))
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
