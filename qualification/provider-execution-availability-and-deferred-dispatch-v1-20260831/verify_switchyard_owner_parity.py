#!/usr/bin/env python3
"""Replay every checked HOLDING mapper fixture through exact Switchyard 2ba.

This is a qualification-only read predicate.  Nightshift's Rust contract suite
opens the same fixture bytes independently; this predicate proves that the
accepted owner state machine produces the identical snapshot and result.
"""

from __future__ import annotations

import argparse
import copy
import json
from pathlib import Path
import subprocess
import sys


OWNER_HEAD = "2ba25db66d8b29dd215bd87e05f4ea794024b3b7"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--switchyard-root", type=Path, required=True)
    parser.add_argument("--refresh-owner-test-corpus", action="store_true")
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
    from switchyard.provider_admission import (  # noqa: PLC0415
        ProviderAdmissionError,
        ProviderAdmissionMapper,
        replay_snapshot,
    )

    fixture_dir = Path(__file__).resolve().parent / "fixtures"
    if args.refresh_owner_test_corpus:
        import pytest  # noqa: PLC0415
        from switchyard.appserver import AcquisitionCut  # noqa: PLC0415

        captured: dict[str, dict] = {}
        original_snapshot = ProviderAdmissionMapper.snapshot
        operation_names = (
            "consume_envelope",
            "consume",
            "consume_cut",
            "mark_acquisition_loss",
            "record_legacy_local_fact",
        )
        original_operations = {
            name: getattr(ProviderAdmissionMapper, name) for name in operation_names
        }
        collecting = False

        def capture_transition(mapper: ProviderAdmissionMapper) -> None:
            nonlocal collecting
            if collecting:
                return
            collecting = True
            try:
                value = original_snapshot(mapper)
                terminal = value
                if value["acquisition_cut"] is None:
                    closed = copy.deepcopy(mapper)
                    binding = value["binding"]
                    try:
                        closed.consume_cut(
                            AcquisitionCut(
                                True,
                                0,
                                "EXITED",
                                closed.expected_acquisition_ordinal,
                                binding["adapter_process_occurrence_id"],
                                binding["app_server_session_identity"],
                            )
                        )
                    except ProviderAdmissionError:
                        terminal = None
                    else:
                        terminal = original_snapshot(closed)
                exact_binding = terminal is not None and all(
                    terminal["binding"][field] == expected
                    for field, expected in {
                        "work_attempt_id": "attempt-holding-1",
                        "dispatch_occurrence_id": "dispatch-holding-1",
                        "adapter_process_occurrence_id": "adapter-process-holding-1",
                        "app_server_session_identity": "fixture-estate-holding-1",
                        "thread_id": "thread-holding-1",
                        "turn_id": "turn-holding-1",
                        "provider": "openai",
                        "model": "gpt-5.6-sol",
                        "executable_kind": "DETERMINISTIC_FIXTURE",
                        "app_server_executable_identity": "tests/fake_app_server.py",
                        "app_server_executable_sha256": (
                            "sha256:cafa673ac58f60029fd6c1de229b4f57d9f42ba918b7ecb2a3bfb20cb2b41a31"
                        ),
                    }.items()
                )
                if exact_binding:
                    captured[terminal["snapshot_digest"]] = copy.deepcopy(terminal)
            finally:
                collecting = False

        def capture_operation(name: str):
            original = original_operations[name]

            def wrapped(mapper: ProviderAdmissionMapper, *args, **kwargs):
                result = original(mapper, *args, **kwargs)
                capture_transition(mapper)
                return result

            return wrapped

        def capture_snapshot(mapper: ProviderAdmissionMapper) -> dict:
            value = original_snapshot(mapper)
            capture_transition(mapper)
            return value

        for name in operation_names:
            setattr(ProviderAdmissionMapper, name, capture_operation(name))
        ProviderAdmissionMapper.snapshot = capture_snapshot
        try:
            result = pytest.main([str(root / "tests/test_provider_admission.py"), "-q"])
        finally:
            ProviderAdmissionMapper.snapshot = original_snapshot
            for name, operation in original_operations.items():
                setattr(ProviderAdmissionMapper, name, operation)
        if result != 0:
            raise SystemExit(f"exact owner branch suite failed: {result}")
        corpus = {
            "owner_head": OWNER_HEAD,
            "source_test": "tests/test_provider_admission.py",
            "snapshots": [captured[digest] for digest in sorted(captured)],
        }
        output = fixture_dir / "switchyard-owner-terminal-corpus.v1.json"
        output.write_text(
            json.dumps(corpus, separators=(",", ":"), sort_keys=True) + "\n",
            encoding="utf-8",
        )
        print(f"refreshed_owner_terminal_snapshots={len(captured)} path={output}")

    fixtures = sorted(fixture_dir.glob("switchyard-*.snapshot.v1.json"))
    if not fixtures:
        raise SystemExit("no checked Switchyard snapshots")
    for fixture in fixtures:
        raw = fixture.read_bytes()
        value = json.loads(raw)
        replayed = replay_snapshot(value).snapshot()
        if replayed != value:
            raise SystemExit(f"owner replay changed {fixture.name}")
    corpus_path = fixture_dir / "switchyard-owner-terminal-corpus.v1.json"
    corpus = json.loads(corpus_path.read_bytes())
    if corpus["owner_head"] != OWNER_HEAD or not corpus["snapshots"]:
        raise SystemExit("foreign or empty exact owner terminal corpus")
    owner_replayable = 0
    for value in corpus["snapshots"]:
        try:
            replayed = replay_snapshot(value).snapshot()
        except ProviderAdmissionError:
            continue
        if replayed != value:
            raise SystemExit(
                f"owner replay changed corpus snapshot {value['snapshot_digest']}"
            )
        owner_replayable += 1
    print(
        f"owner={OWNER_HEAD} exact_snapshots={len(fixtures)} "
        f"owner_terminal_snapshots={len(corpus['snapshots'])} "
        f"owner_generic_replayable={owner_replayable} parity=ok"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
