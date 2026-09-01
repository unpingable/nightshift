import projectionJson from "../../../../qualification/nightshift-casework-mvp-20260829/velvet-orrery.casework-run.v1.json";
import type {
  CaseworkLiveProviderExecution,
  CaseworkLiveRun,
  CaseworkRun,
  LiveProviderDisposition,
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

const sha = (digit: string) => `sha256:${digit.repeat(64)}`;
const executionIdentity = {
  provider_id: "provider:execution-fixture",
  model_id: "model:fallback",
  app_server_session_identity: "session:execution-fixture",
  thread_id: "thread:execution-fixture",
  turn_id: "turn:execution-fixture",
  first_response_id: "response:execution-fixture",
};

function disposition(mechanismState: string, index: number): LiveProviderDisposition {
  const executionWasAdmitted = !["PARKED_NOT_ADMITTED", "ADMISSION_INDETERMINATE"].includes(mechanismState);
  return {
    journal_sequence: 20 + index,
    journal_event_id: `event:disposition:${index}`,
    journal_exact_bytes_sha256: sha("1"),
    journal_retained_raw_digest: sha("2"),
    work_item_id: "lane-a",
    work_attempt_id: "attempt:provider",
    dispatch_occurrence_id: `dispatch:${index}`,
    dispatch_digest: sha("3"),
    disposition_digest: sha(String((index + 4) % 10)),
    reconciles_disposition_digest: index === 1 ? sha("4") : null,
    provider_id: "provider:execution-fixture",
    model_id: executionWasAdmitted ? "model:fallback" : "model:primary",
    availability_state: mechanismState === "PARKED_NOT_ADMITTED" ? "MODEL_AT_CAPACITY" : executionWasAdmitted ? "AVAILABLE" : "UNKNOWN",
    admission_disposition: mechanismState === "PARKED_NOT_ADMITTED" ? "NOT_ADMITTED_MODEL_AT_CAPACITY" : executionWasAdmitted ? "EXECUTION_ADMITTED" : "ADMISSION_INDETERMINATE",
    mechanism_state: mechanismState,
    observed_at: `2026-08-31T00:00:0${index}Z`,
    evidence_received_at: `2026-08-31T00:01:0${index}Z`,
    expires_at: `2026-08-31T00:10:0${index}Z`,
    disposition_received_at: `2026-08-31T00:01:0${index}Z`,
    currentness: index === 0 ? "EXPIRED" : "CURRENT",
    source_identity: "switchyard:fixture",
    source_version: "v2",
    response_created: executionWasAdmitted,
    acquisition_complete: ["PARKED_NOT_ADMITTED", "PROVIDER_COMPLETED"].includes(mechanismState),
    provider_retry_after: mechanismState === "PARKED_NOT_ADMITTED" ? "2026-08-31T00:02:00Z" : null,
    provider_request_occurrence_id: `provider-request:${index}`,
    provider_execution: executionWasAdmitted ? executionIdentity : null,
    mapper_snapshot_schema: "switchyard.codex-provider-admission-snapshot/v1",
    mapper_snapshot_digest: sha("a"),
    approval_response_sent: false,
    protected_effect_absent: true,
    observation_digest: sha("b"),
    observation_exact_bytes_sha256: sha("c"),
    disposition_exact_bytes_sha256: sha("d"),
  };
}

export const providerExecutionAbsent: CaseworkLiveProviderExecution = {
  schema: "nightshift.casework-live-provider-execution/v1",
  projection_digest: sha("1"),
  run_id: liveRun.run_id,
  packet_digest: liveRun.packet.packet_digest,
  evaluated_at: liveRun.evaluated_at,
  status: "NOT_RECORDED_BY_FOREMAN",
  requirement: null,
  dispatches: [],
  dispositions: [],
  deferrals: [],
  wakes: [],
  resumes: [],
  resource_transitions: [],
  independent_provider_capacity_status: "NOT_RECORDED_BY_FOREMAN",
  explanation: "No provider-execution availability requirement is recorded in this foreman journal.",
  authority_effect: "READ_ONLY_MECHANISM_PROJECTION",
};

export const providerExecution: CaseworkLiveProviderExecution = {
  ...providerExecutionAbsent,
  projection_digest: sha("e"),
  status: "EXACT_RECORDED_FOREMAN_HISTORY",
  independent_provider_capacity_status: "EXACT_RECORDED_BY_FOREMAN",
  explanation: "Exact foreman provider-execution mechanism history; provider capacity remains independent.",
  requirement: {
    journal_sequence: 2,
    requirement_digest: sha("0"),
    policy_id: "policy:holding",
    policy_digest: sha("1"),
    provider_id: "provider:execution-fixture",
    work_item_model_selections: {
      "lane-a": [
        { provider_id: "provider:execution-fixture", model_id: "model:primary", model_class: "large" },
        { provider_id: "provider:execution-fixture", model_id: "model:fallback", model_class: "large" },
      ],
    },
    adapter_id: "switchyard-codex",
    adapter_protocol: "switchyard.codex-app-server/v2",
    adapter_version: "2.0.0",
    adapter_executable_identity: sha("2"),
    codex_owner_head: "3".repeat(40),
    provider_admission_owner_head: "4".repeat(40),
    provider_admission_schema_sha256: sha("5"),
    deterministic_fixture_sha256: sha("6"),
    admitted_at: "2026-08-31T00:00:00Z",
    requirement_exact_bytes_sha256: sha("7"),
    policy_exact_bytes_sha256: sha("8"),
    parked_resource_lock_policy: "RELEASE_AND_REACQUIRE",
    allow_ordered_model_fallback: true,
    automatic_semantic_retry: false,
    approval_response_authorized: false,
    authority_effect: "READ_ONLY_MECHANISM_PROJECTION",
  },
  dispatches: [0, 1].map((ordinal) => ({
    journal_sequence: 3 + ordinal,
    journal_event_id: `event:dispatch:${ordinal}`,
    journal_exact_bytes_sha256: sha("9"),
    journal_retained_raw_digest: sha("a"),
    work_item_id: "lane-a",
    work_attempt_id: "attempt:provider",
    dispatch_occurrence_id: `dispatch:${ordinal}`,
    dispatch_ordinal: ordinal + 1,
    selected_model_ordinal: ordinal,
    provider_id: "provider:execution-fixture",
    model_id: ordinal === 0 ? "model:primary" : "model:fallback",
    model_class: "large",
    adapter_id: "switchyard-codex",
    adapter_version: "2.0.0",
    adapter_protocol: "switchyard.codex-app-server/v2",
    adapter_process_occurrence_id: `process:${ordinal}`,
    app_server_session_identity: `session:${ordinal}`,
    worker_start_request_digest: sha("b"),
    worker_brief_digest: sha("c"),
    dispatch_digest: ordinal === 0 ? sha("3") : sha("4"),
    opened_at: `2026-08-31T00:02:0${ordinal}Z`,
    start_request_exact_bytes_sha256: sha("d"),
    dispatch_exact_bytes_sha256: sha("e"),
    provider_execution_identity_absent_at_start: true as const,
  })),
  dispositions: [
    "PARKED_NOT_ADMITTED",
    "ADMISSION_INDETERMINATE",
    "EXECUTION_ADMITTED",
    "WAITING_APPROVAL",
    "POST_ADMISSION_INTERRUPTED",
    "PROVIDER_COMPLETED",
  ].map(disposition),
  deferrals: [{
    journal_sequence: 30,
    journal_event_id: "event:deferral",
    journal_exact_bytes_sha256: sha("1"),
    disposition_digest: sha("4"),
    deferred_dispatch_digest: sha("2"),
    work_item_id: "lane-a",
    work_attempt_id: "attempt:provider",
    last_dispatch_occurrence_id: "dispatch:0",
    provider_id: "provider:execution-fixture",
    model_id: "model:primary",
    selected_model_ordinal: 0,
    remaining_model_ordinals: [1],
    refusal_received_at: "2026-08-31T00:01:00Z",
    wake_basis: "PROVIDER_RETRY_AFTER",
    backoff_ordinal: 0,
    backoff_seconds: 5,
    provider_retry_after: "2026-08-31T00:02:00Z",
    wake_at: "2026-08-31T00:02:00Z",
    parked_resource_lock_policy: "RELEASE_AND_REACQUIRE",
    provider_capacity_released: true,
    deferred_exact_bytes_sha256: sha("3"),
  }],
  wakes: [{
    journal_sequence: 32,
    journal_event_id: "event:wake",
    journal_exact_bytes_sha256: sha("4"),
    work_item_id: "lane-a",
    work_attempt_id: "attempt:provider",
    wake_occurrence_id: "wake:one",
    deferred_dispatch_digest: sha("2"),
    next_dispatch_digest: sha("4"),
    recorded_at: "2026-08-31T00:02:00Z",
  }],
  resumes: [{
    journal_sequence: 40,
    journal_event_id: "event:resume",
    journal_exact_bytes_sha256: sha("5"),
    work_item_id: "lane-a",
    work_attempt_id: "attempt:provider",
    resume_occurrence_id: "resume:one",
    disposition_digest: sha("8"),
    adapter_process_occurrence_id: "process:resume",
    execution_identity: executionIdentity,
    recorded_at: "2026-08-31T00:06:00Z",
  }],
  resource_transitions: [{
    journal_sequence: 31,
    journal_event_id: "event:resources-released",
    journal_exact_bytes_sha256: sha("6"),
    transition: "RELEASED",
    work_item_id: "lane-a",
    work_attempt_id: "attempt:provider",
    dispatch_digest: sha("3"),
    disposition_digest: sha("4"),
    deferred_dispatch_digest: null,
    policy_digest: sha("1"),
    wake_occurrence_id: null,
    resource_lock_keys: ["repository:fixture"],
    recorded_at: "2026-08-31T00:01:00Z",
  }, {
    journal_sequence: 33,
    journal_event_id: "event:resources-reacquired",
    journal_exact_bytes_sha256: sha("7"),
    transition: "REACQUIRED",
    work_item_id: "lane-a",
    work_attempt_id: "attempt:provider",
    dispatch_digest: sha("4"),
    disposition_digest: null,
    deferred_dispatch_digest: sha("2"),
    policy_digest: sha("1"),
    wake_occurrence_id: "wake:one",
    resource_lock_keys: ["repository:fixture"],
    recorded_at: "2026-08-31T00:02:00Z",
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
  providerExecution: CaseworkLiveProviderExecution = providerExecutionAbsent,
) {
  const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
    const path = String(input);
    if (path === "/api/v1/runs") return new Response(JSON.stringify(index), { status: 200 });
    if (path === "/api/v1/active-runs") return new Response(JSON.stringify(liveRunIndex), { status: 200 });
    if (path === "/api/v1/operational-conditions") return new Response(JSON.stringify(operationalRunIndex), { status: 200 });
    if (path.endsWith("/provider-execution")) return new Response(JSON.stringify(providerExecution), { status: 200 });
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
