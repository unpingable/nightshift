import projectionJson from "../../../../qualification/nightshift-casework-mvp-20260829/velvet-orrery.casework-run.v1.json";
import type {
  CaseworkLiveRun,
  CaseworkRun,
  LiveRunIndex,
  OperationalConditionIndex,
  RunIndex,
} from "../contract";

export const run = projectionJson as CaseworkRun;

export const index: RunIndex = {
  schema: "nightshift.casework-run-index/v1",
  runs: [{
    run_id: run.run_id,
    projection_digest: run.projection_digest,
    packet_id: run.packet.packet_id,
    packet_digest: run.packet.packet_digest,
    receipt_updated_at: run.receipts.updated_at,
    summary: run.summary,
    packet_integrity: run.packet.integrity,
    packet_currentness_at_receipt_snapshot: run.packet.currentness_at_receipt_snapshot,
    packet_currentness_now: run.packet.currentness_now,
  }],
};

export const packetBytes = '{"exact":"packet bytes"}\n';
export const receiptBytes = '{"exact":"receipt bytes"}\n';
export const liveRun: CaseworkLiveRun = {
  schema: "nightshift.casework-live-run/v1",
  projection_digest: "sha256:" + "1".repeat(64),
  navigation_id: "2".repeat(64),
  run_id: "run/live:fixture",
  evaluated_at: "2026-08-31T00:00:00+00:00",
  packet: {
    packet_id: "live-packet",
    packet_digest: "sha256:" + "3".repeat(64),
    exact_bytes_sha256: "sha256:" + "4".repeat(64),
    integrity: "VALID",
    created_at: "2026-08-30T00:00:00+00:00",
    current_until: "2026-09-01T00:00:00+00:00",
    currentness: "CURRENT",
  },
  admission: {
    admission_digest: "sha256:" + "5".repeat(64),
    exact_bytes_sha256: "sha256:" + "6".repeat(64),
    admitted_at: "2026-08-30T00:00:00+00:00",
    expires_at: "2026-09-01T00:00:00+00:00",
    currentness: "CURRENT",
    maximum_concurrent_workers: 2,
  },
  execution_profile: {
    profile_digest: "sha256:" + "7".repeat(64),
    exact_bytes_sha256: "sha256:" + "8".repeat(64),
    budget_policy_ref: "policy:fixture",
    capacity_binding_status: "POLICY_REFERENCE_ONLY_NO_RECORDED_DECISION",
  },
  foreman: {
    source_schema: "nightshift.foreman-live-run/v1",
    lifecycle: "OPEN",
    scheduler_state_counts: { WAITING_HUMAN: 1 },
    terminal_receipt_count: 0,
    not_started_receipt_count: 0,
    closed_final_receipts_digest: null,
  },
  work_items: [{
    work_item_id: "lane-a",
    track: "fixture",
    campaign_codename: "LANE-A",
    campaign_slug: "lane-a",
    dependencies: [],
    entry_predicates: ["exact evidence"],
    stop_conditions: ["identity mismatch"],
    scheduler_state: "WAITING_HUMAN",
    scheduler_state_recognized: true,
    dependency_terminality: {},
    resource_lock_keys: ["repository:fixture"],
    active_attempt_id: "attempt:one",
    adapter_id: "adapter",
    adapter_version: "adapter/v1",
    provider_model_class: "bounded",
    provider_identity: "provider:fixture",
    model_identity: "model:fixture",
    session_identity: "session:fixture",
    thread_identity: "thread:fixture",
    turn_identity: null,
    queue_identity: null,
    last_event_sequence: 2,
    last_event_digest: "sha256:" + "9".repeat(64),
    human_questions: [{
      navigation_id: "e".repeat(64),
      question_id: "question:one",
      question: "Which explicit input is required?",
      exhausted_evidence: "Local evidence exhausted.",
      safe_default: "Do not continue.",
      consequences: "Lane remains waiting.",
      resume_point: "Append a successor event.",
    }],
    accepted_receipt_kind: null,
    accepted_outcome: null,
    accepted_outcome_absent_reason: "NO_ACCEPTED_TERMINAL_OR_NOT_STARTED_RECEIPT",
  }],
  resource_claims: [{
    resource_lock_key: "repository:fixture",
    work_item_id: "lane-a",
    attempt_id: "attempt:one",
  }],
  events: [{
    sequence: 1,
    event_id: "event:one",
    work_item_id: null,
    attempt_id: null,
    kind: "internal",
    recorded_at: "2026-08-30T00:00:00+00:00",
    retained_raw_digest: "sha256:" + "a".repeat(64),
    exact_bytes_sha256: "sha256:" + "b".repeat(64),
    raw_length: 100,
  }],
  raw_sources: {
    packet_sha256: "sha256:" + "4".repeat(64),
    admission_sha256: "sha256:" + "6".repeat(64),
    profile_sha256: "sha256:" + "8".repeat(64),
    journal_framing_sha256: "sha256:" + "c".repeat(64),
    accepted_receipts_framing_sha256: "sha256:" + "d".repeat(64),
    final_snapshot_sha256: null,
  },
  sealed_case_run_id: null,
  provider_capacity: {
    status: "NOT_RECORDED_BY_FOREMAN",
    requirement: null,
    attempts: [],
    explanation: "No exact capacity decision was recorded.",
  },
  authority_effect: "READ_ONLY_OPERATOR_PROJECTION",
};

export const liveIndex: LiveRunIndex = {
  schema: "nightshift.casework-live-run-index/v1",
  runs: [{
    navigation_id: liveRun.navigation_id,
    run_id: liveRun.run_id,
    projection_digest: liveRun.projection_digest,
    packet_id: liveRun.packet.packet_id,
    packet_digest: liveRun.packet.packet_digest,
    lifecycle: liveRun.foreman.lifecycle,
    sealed_case_run_id: liveRun.sealed_case_run_id,
    scheduler_state_counts: liveRun.foreman.scheduler_state_counts,
  }],
};

export const operationalIndex: OperationalConditionIndex = {
  schema: "nightshift.casework-operational-condition-index/v1",
  conditions: [],
};

export function installApiMock(
  caseworkRun: CaseworkRun = run,
  liveRunIndex: LiveRunIndex = liveIndex,
  activeRun: CaseworkLiveRun = liveRun,
  operationalRunIndex: OperationalConditionIndex = operationalIndex,
) {
  const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
    const path = String(input);
    if (path === "/api/v1/runs") return new Response(JSON.stringify(index), { status: 200 });
    if (path === "/api/v1/active-runs") return new Response(JSON.stringify(liveRunIndex), { status: 200 });
    if (path === "/api/v1/operational-conditions") return new Response(JSON.stringify(operationalRunIndex), { status: 200 });
    if (path.endsWith("/raw/final")) return new Response('{"exact":"final snapshot bytes"}\n', { status: 200 });
    if (/\/events\/\d+\/raw$/.test(path)) return new Response('{"exact":"event bytes"}\n', { status: 200 });
    if (path.includes("/api/v1/active-runs/") && path.includes("/raw/")) return new Response('{"exact":"live bytes"}\n', { status: 200 });
    if (path.startsWith("/api/v1/active-runs/")) return new Response(JSON.stringify(activeRun), { status: 200 });
    if (path.endsWith("/raw/packet")) return new Response(packetBytes, { status: 200 });
    if (path.endsWith("/raw/receipts")) return new Response(receiptBytes, { status: 200 });
    if (path.startsWith("/api/v1/runs/")) return new Response(JSON.stringify(caseworkRun), { status: 200 });
    return new Response("not found", { status: 404 });
  });
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

export function at(path: string) {
  window.history.replaceState(null, "", path);
}
