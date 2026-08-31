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
        from switchyard.appserver import (  # noqa: PLC0415
            AcquisitionCut,
            AcquisitionEnvelope,
            ServerMessage,
        )

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
            if result == 0:
                base_binding = json.loads(
                    (fixture_dir / "switchyard-provider-completed.snapshot.v1.json")
                    .read_text(encoding="utf-8")
                )["binding"]

                def owner_message(method: str, params: dict) -> ServerMessage:
                    value = {"method": method, "params": params}
                    raw = json.dumps(
                        value, separators=(",", ":"), ensure_ascii=False
                    ).encode("utf-8") + b"\n"
                    return ServerMessage(value, raw_bytes=raw)

                def request(ordinal: int = 0, occurrence: str = "request-0") -> dict:
                    return {
                        "threadId": "thread-holding-1",
                        "turnId": "turn-holding-1",
                        "requestOccurrenceId": occurrence,
                        "samplingOrdinal": ordinal,
                        "requestOrder": ordinal,
                        "provider": "openai",
                        "model": "gpt-5.6-sol",
                        "startedAtMs": 1788163200000 + 1000 * ordinal,
                    }

                def response() -> dict:
                    value = request()
                    value.pop("startedAtMs")
                    value.update(
                        {"responseId": "response-0", "observedAtMs": 1788163200100}
                    )
                    return value

                def completion() -> dict:
                    return {
                        "threadId": "thread-holding-1",
                        "turnId": "turn-holding-1",
                        "responseId": "response-0",
                        "usage": None,
                    }

                def refusal() -> dict:
                    value = request()
                    value.pop("startedAtMs")
                    value.update(
                        {
                            "responseCreated": False,
                            "willRetry": False,
                            "refusalKind": "modelAtCapacity",
                            "codexErrorInfo": "serverOverloaded",
                            "retryAfterMs": 5000,
                            "diagnostic": "bounded mock capacity refusal",
                            "observedAtMs": 1788163200100,
                        }
                    )
                    return value

                approval_method = "item/commandExecution/requestApproval"
                approval = {
                    "threadId": "thread-holding-1",
                    "turnId": "turn-holding-1",
                }

                # Exact closed response shape is checked before execution identity.
                closed_shape = ProviderAdmissionMapper(copy.deepcopy(base_binding))
                closed_shape.consume_envelope(
                    AcquisitionEnvelope(
                        0,
                        "NOTIFICATION",
                        message=owner_message("rawResponse/completed", {}),
                    )
                )

                # Approval-named notifications are ordinary provider watermarks.
                watermark = ProviderAdmissionMapper(copy.deepcopy(base_binding))
                watermark.consume_envelope(
                    AcquisitionEnvelope(
                        0,
                        "NOTIFICATION",
                        message=owner_message(approval_method, approval),
                    )
                )

                # The same method after an exact approval remains lane-sensitive.
                waiting = ProviderAdmissionMapper(copy.deepcopy(base_binding))
                for ordinal, kind, method, params in (
                    (0, "NOTIFICATION", "providerRequest/started", request()),
                    (1, "NOTIFICATION", "rawResponse/started", response()),
                    (2, "NOTIFICATION", "rawResponse/completed", completion()),
                    (3, "SERVER_REQUEST", approval_method, approval),
                    (4, "NOTIFICATION", approval_method, approval),
                ):
                    waiting.consume_envelope(
                        AcquisitionEnvelope(
                            ordinal,
                            kind,
                            message=owner_message(method, params),
                        )
                    )

                # A later discrepancy invalidates a previously retained refusal.
                provider_discrepancy = ProviderAdmissionMapper(
                    copy.deepcopy(base_binding)
                )
                for ordinal, method, params in (
                    (0, "providerRequest/started", request()),
                    (1, "providerAdmission/refused", refusal()),
                    (2, "providerRequest/started", request()),
                ):
                    provider_discrepancy.consume_envelope(
                        AcquisitionEnvelope(
                            ordinal,
                            "NOTIFICATION",
                            message=owner_message(method, params),
                        )
                    )

                client_discrepancy = ProviderAdmissionMapper(copy.deepcopy(base_binding))
                client_discrepancy.consume_envelope(
                    AcquisitionEnvelope(
                        0,
                        "NOTIFICATION",
                        message=owner_message("providerRequest/started", request()),
                    )
                )

                turn_completed = {
                    "threadId": "thread-holding-1",
                    "turn": {"id": "turn-holding-1"},
                }
                before_boundary = ProviderAdmissionMapper(copy.deepcopy(base_binding))
                before_boundary.consume_envelope(
                    AcquisitionEnvelope(
                        0,
                        "NOTIFICATION",
                        message=owner_message("turn/completed", turn_completed),
                    )
                )

                def waiting_mapper() -> ProviderAdmissionMapper:
                    mapper = ProviderAdmissionMapper(copy.deepcopy(base_binding))
                    for ordinal, kind, method, params in (
                        (0, "NOTIFICATION", "providerRequest/started", request()),
                        (1, "NOTIFICATION", "rawResponse/started", response()),
                        (2, "NOTIFICATION", "rawResponse/completed", completion()),
                        (3, "SERVER_REQUEST", approval_method, approval),
                    ):
                        mapper.consume_envelope(
                            AcquisitionEnvelope(
                                ordinal,
                                kind,
                                message=owner_message(method, params),
                            )
                        )
                    return mapper

                waiting_completion = waiting_mapper()
                waiting_completion.consume_envelope(
                    AcquisitionEnvelope(
                        4,
                        "NOTIFICATION",
                        message=owner_message("turn/completed", turn_completed),
                    )
                )

                waiting_provider_activity = waiting_mapper()
                waiting_provider_activity.consume_envelope(
                    AcquisitionEnvelope(
                        4,
                        "NOTIFICATION",
                        message=owner_message("rawResponse/started", response()),
                    )
                )

                legacy_waiting = ProviderAdmissionMapper(
                    copy.deepcopy(base_binding), allow_unordered_fixture=True
                )
                legacy_waiting.consume(
                    owner_message("providerRequest/started", request())
                )
                legacy_waiting.consume(owner_message("rawResponse/started", response()))
                legacy_waiting.consume(
                    owner_message("rawResponse/completed", completion())
                )
                legacy_waiting.consume(
                    owner_message(approval_method, approval), server_request=True
                )
                legacy_waiting.consume(owner_message(approval_method, approval))
                client_discrepancy.consume_envelope(
                    AcquisitionEnvelope(
                        1,
                        "NOTIFICATION",
                        message=owner_message("providerAdmission/refused", refusal()),
                    )
                )
                client_wire = {
                    "id": 7,
                    "method": "thread/read",
                    "params": {"threadId": "thread-substituted"},
                }
                client_discrepancy.consume_envelope(
                    AcquisitionEnvelope(
                        2,
                        "CLIENT_REQUEST",
                        message=ServerMessage(
                            client_wire,
                            raw_bytes=json.dumps(
                                client_wire, separators=(",", ":")
                            ).encode("utf-8")
                            + b"\n",
                        ),
                        request_method="thread/read",
                    )
                )
        finally:
            ProviderAdmissionMapper.snapshot = original_snapshot
            for name, operation in original_operations.items():
                setattr(ProviderAdmissionMapper, name, operation)
        if result != 0:
            raise SystemExit(f"exact owner branch suite failed: {result}")
        captured_snapshots = [captured[digest] for digest in sorted(captured)]
        generic_replayable = []
        generic_helper_exceptions = []
        for snapshot in captured_snapshots:
            try:
                replayed = replay_snapshot(snapshot).snapshot()
            except ProviderAdmissionError as error:
                generic_helper_exceptions.append(
                    {
                        "snapshot_digest": snapshot["snapshot_digest"],
                        "admission_disposition": snapshot["admission_disposition"],
                        "mechanism_state": snapshot["mechanism_state"],
                        "reason": str(error),
                    }
                )
                continue
            if replayed == snapshot:
                generic_replayable.append(snapshot["snapshot_digest"])
            else:
                generic_helper_exceptions.append(
                    {
                        "snapshot_digest": snapshot["snapshot_digest"],
                        "admission_disposition": snapshot["admission_disposition"],
                        "mechanism_state": snapshot["mechanism_state"],
                        "reason": "generic owner replay changed retained snapshot",
                    }
                )
        snapshots = captured_snapshots
        inventory = {
            "closed_response_completed_shape": sum(
                any(
                    record["normalized"].get("detail")
                    == "closed response-completed fields do not match"
                    for record in snapshot["records"]
                )
                for snapshot in snapshots
            ),
            "notification_approval_watermark": sum(
                any(
                    record["acquisition_kind"] == "NOTIFICATION"
                    and record["method"] == "item/commandExecution/requestApproval"
                    and record["kind"] == "ACQUISITION_WATERMARK"
                    for record in snapshot["records"]
                )
                for snapshot in snapshots
            ),
            "waiting_approval_then_notification_watermark": sum(
                any(record["kind"] == "WAITING_APPROVAL" for record in snapshot["records"])
                and any(
                    record["acquisition_kind"] == "NOTIFICATION"
                    and record["method"] == "item/commandExecution/requestApproval"
                    and record["kind"] == "ACQUISITION_WATERMARK"
                    for record in snapshot["records"]
                )
                for snapshot in snapshots
            ),
            "refusal_then_discrepancy_indeterminate": sum(
                any(
                    record["kind"] == "PROVIDER_ADMISSION_REFUSED"
                    for record in snapshot["records"]
                )
                and any(
                    record["kind"] == "ADMISSION_DISCREPANCY"
                    for record in snapshot["records"]
                )
                and snapshot["admission_disposition"] == "ADMISSION_INDETERMINATE"
                and snapshot["acquisition_cut"]["clean"] is False
                for snapshot in snapshots
            ),
            "turn_completed_before_boundary_discrepancy": sum(
                any(
                    record["method"] == "turn/completed"
                    and record["normalized"].get("detail")
                    == "turn completed without exact provider execution identity"
                    for record in snapshot["records"]
                )
                for snapshot in snapshots
            ),
            "turn_completed_during_approval_discrepancy": sum(
                any(
                    record["method"] == "turn/completed"
                    and record["normalized"].get("detail")
                    == "turn completed before exact response/approval sequence closed"
                    for record in snapshot["records"]
                )
                for snapshot in snapshots
            ),
            "provider_activity_after_approval_discrepancy": sum(
                any(
                    record["method"] == "rawResponse/started"
                    and record["normalized"].get("detail")
                    == "provider activity followed unanswered approval"
                    for record in snapshot["records"]
                )
                for snapshot in snapshots
            ),
            "unordered_waiting_approval_then_watermark": sum(
                any(
                    record["acquisition_kind"] is None
                    and record["kind"] == "WAITING_APPROVAL"
                    for record in snapshot["records"]
                )
                and any(
                    record["acquisition_kind"] is None
                    and record["method"] == "item/commandExecution/requestApproval"
                    and record["kind"] == "ACQUISITION_WATERMARK"
                    for record in snapshot["records"]
                )
                for snapshot in snapshots
            ),
        }
        if any(count == 0 for count in inventory.values()):
            raise SystemExit(f"owner branch inventory incomplete: {inventory}")
        corpus = {
            "owner_head": OWNER_HEAD,
            "source_test": "tests/test_provider_admission.py",
            "owner_generated_terminal_snapshot_count": len(captured_snapshots),
            "owner_generic_replayable_snapshot_count": len(generic_replayable),
            "owner_generic_replayable_snapshot_digests": generic_replayable,
            "owner_generic_helper_exceptions": generic_helper_exceptions,
            "branch_inventory": inventory,
            "snapshots": snapshots,
        }
        output = fixture_dir / "switchyard-owner-terminal-corpus.v1.json"
        output.write_text(
            json.dumps(corpus, separators=(",", ":"), sort_keys=True) + "\n",
            encoding="utf-8",
        )
        print(
            f"refreshed_owner_terminal_snapshots={len(captured_snapshots)} "
            f"owner_generic_replayable={len(generic_replayable)} "
            f"generic_helper_exceptions={len(generic_helper_exceptions)} path={output}"
        )

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
