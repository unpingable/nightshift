#!/usr/bin/env python3
"""Exercise exact Nightshift slot contention through independent processes.

The deterministic support child is a fixture, not Pulse. This produces local
development evidence only and does not establish systemd or power-loss behavior.
"""

from __future__ import annotations

import argparse
import concurrent.futures
import datetime as dt
import hashlib
import json
import os
import subprocess
import threading
from pathlib import Path
from typing import Any


REPO = Path(__file__).resolve().parents[2]


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False) + "\n").encode()


def object_id(value: dict[str, Any], field: str) -> str:
    preimage = dict(value)
    preimage.pop(field, None)
    return "sha256:" + hashlib.sha256(canonical_bytes(preimage).rstrip(b"\n")).hexdigest()


def sha256(data: bytes) -> str:
    return "sha256:" + hashlib.sha256(data).hexdigest()


def write_exclusive(path: Path, data: bytes) -> None:
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(descriptor, "wb") as stream:
        stream.write(data)
        stream.flush()
        os.fsync(stream.fileno())


def load_example(name: str) -> Any:
    path = REPO / "docs/operator/examples/diagnostic-posture-v1" / name
    return json.loads(path.read_bytes())


def request(occurrence: int) -> dict[str, Any]:
    policy = load_example("policy.json")
    inputs = load_example("inputs.json")
    recurrence = load_example("recurrence.json")
    minute = occurrence
    due = f"2026-07-27T20:{minute:02d}:00Z"
    evaluated = f"2026-07-27T20:{minute:02d}:10Z"
    latest = f"2026-07-27T20:{minute:02d}:30Z"
    slot: dict[str, Any] = {
        "schema": "nightshift.recurrence_slot.v1",
        "slot_id": "",
        "policy_id": policy["policy_id"],
        "configuration_version": "qualification-config-v1",
        "subject_id": policy["subject"]["id"],
        "scope_id": policy["subject"]["scope"]["digest"],
        "scheduler_clock_id": "nightshift-qualification-scheduler",
        "nominal_due_at": due,
        "latest_admissible": {
            "scheduler_clock_id": "nightshift-qualification-scheduler",
            "at": latest,
        },
        "occurrence": occurrence,
        "trigger": "scheduled",
    }
    slot["slot_id"] = object_id(slot, "slot_id")
    value: dict[str, Any] = {
        "schema": "nightshift.canonical_cycle_request.v1",
        "request_id": "",
        "slot": slot,
        "scheduler_clock_id": "nightshift-qualification-scheduler",
        "evaluated_at": evaluated,
        "observation_id": "sha256:" + chr(ord("d") + occurrence) * 64,
        "policy": policy,
        "inputs": inputs,
        "recurrence": recurrence,
    }
    value["request_id"] = object_id(value, "request_id")
    return value


def invoke(argv: list[str], environment: dict[str, str]) -> dict[str, Any]:
    started = dt.datetime.now(dt.timezone.utc).isoformat()
    completed = subprocess.run(
        argv,
        cwd=REPO,
        env=environment,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    return {
        "argv": argv,
        "started_at_utc": started,
        "ended_at_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "exit_code": completed.returncode,
        "stdout": completed.stdout.decode("utf-8", "replace"),
        "stderr": completed.stderr.decode("utf-8", "replace"),
        "stdout_sha256": sha256(completed.stdout),
        "stderr_sha256": sha256(completed.stderr),
    }


def competing(argv: list[str], environment: dict[str, str], count: int) -> list[dict[str, Any]]:
    barrier = threading.Barrier(count)

    def one(index: int) -> dict[str, Any]:
        barrier.wait()
        result = invoke(argv, environment)
        result["writer"] = index
        return result

    with concurrent.futures.ThreadPoolExecutor(max_workers=count) as pool:
        futures = [pool.submit(one, index) for index in range(count)]
        return [future.result() for future in futures]


def invocation_count(directory: Path) -> int:
    return len(list(directory.glob("invocation-*.json")))


def find_cycle_id(value: Any) -> str | None:
    if isinstance(value, dict):
        if isinstance(value.get("cycle_id"), str):
            return value["cycle_id"]
        for child in value.values():
            found = find_cycle_id(child)
            if found:
                return found
    elif isinstance(value, list):
        for child in value:
            found = find_cycle_id(child)
            if found:
                return found
    return None


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--nightshift", type=Path, required=True)
    parser.add_argument("--resolver", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--writers", type=int, default=8)
    args = parser.parse_args()
    if args.writers < 2:
        parser.error("--writers must be at least 2")

    program = args.nightshift.resolve(strict=True)
    resolver = args.resolver.resolve(strict=True)
    args.output.mkdir(mode=0o700, parents=True, exist_ok=False)
    logs = args.output / "resolver-invocations"
    logs.mkdir(mode=0o700)
    database = args.output / "nightshift.sqlite"
    requests = [request(0), request(1)]
    request_paths = []
    for index, value in enumerate(requests):
        path = args.output / f"request-{index}.json"
        write_exclusive(path, canonical_bytes(value))
        request_paths.append(path)

    environment = dict(os.environ)
    environment["NIGHTSHIFT_QUAL_SUPPORT_MODE"] = "current"
    environment["NIGHTSHIFT_QUAL_RESOLVER_LOG_DIR"] = str(logs)

    def cycle_argv(path: Path) -> list[str]:
        return [
            str(program), "--store", str(database), "cycle", "run",
            "--request", str(path), "--present-evidence-resolver", str(resolver),
        ]

    first = competing(cycle_argv(request_paths[0]), environment, args.writers)
    first_successes = [item for item in first if item["exit_code"] == 0]
    count_after_first = invocation_count(logs)
    second = invoke(cycle_argv(request_paths[1]), environment)
    count_after_second = invocation_count(logs)
    duplicate = invoke(cycle_argv(request_paths[1]), environment)
    count_after_duplicate = invocation_count(logs)

    cycle_ids = []
    for successful in first_successes + ([second] if second["exit_code"] == 0 else []):
        try:
            cycle_id = find_cycle_id(json.loads(successful["stdout"]))
        except json.JSONDecodeError:
            cycle_id = None
        if cycle_id:
            cycle_ids.append(cycle_id)
    list_result = invoke([str(program), "--store", str(database), "cycle", "list"], environment)
    replays = [
        invoke([str(program), "--store", str(database), "cycle", "replay", "--cycle-id", cycle_id], environment)
        for cycle_id in cycle_ids
    ]

    passed = (
        len(first_successes) == 1
        and count_after_first == 1
        and second["exit_code"] == 0
        and count_after_second == 2
        and duplicate["exit_code"] != 0
        and count_after_duplicate == 2
        and len(set(cycle_ids)) == 2
        and list_result["exit_code"] == 0
        and all(item["exit_code"] == 0 for item in replays)
    )
    record = {
        "schema": "nightshift.multiprocess-cycle-development-evidence.v1",
        "authority_use": "none",
        "qualification_status": "not_assessed",
        "development_check": "passed" if passed else "failed",
        "fixture_is_not_pulse": True,
        "same_diagnostic_basis_across_distinct_slots": True,
        "limitations": [
            "No systemd timer manager was exercised.",
            "No Pulse receiver or deployed NQ/AG service was exercised.",
            "No abrupt power loss or filesystem fault was injected.",
            "Both cycles are posture-only and create no effect proposal."
        ],
        "program": {"path": str(program), "sha256": sha256(program.read_bytes())},
        "resolver": {"path": str(resolver), "sha256": sha256(resolver.read_bytes())},
        "writers": args.writers,
        "first_competition": first,
        "support_invocations_after_first": count_after_first,
        "distinct_slot": second,
        "support_invocations_after_distinct_slot": count_after_second,
        "duplicate_slot": duplicate,
        "support_invocations_after_duplicate": count_after_duplicate,
        "cycle_ids": cycle_ids,
        "list": list_result,
        "replays": replays,
    }
    write_exclusive(args.output / "result.json", canonical_bytes(record))
    print(json.dumps({"development_check": record["development_check"], "output": str(args.output)}))
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
