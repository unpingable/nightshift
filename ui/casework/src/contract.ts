export interface CompatibleValue {
  recognized_string: string | null;
}

export interface RendererJoinedValue {
  recognized_strings: string[] | null;
}

export interface CompatibleTimestamp extends CompatibleValue {
  recognized_rfc3339: string | null;
}

export interface RunSummary {
  work_item_count: number;
  state_counts: Record<string, number>;
  unrecognized_state_count: number;
  human_question_count: number;
  packet_custody_discrepancy_count: number;
}

export interface RunIndexEntry {
  run_id: string;
  projection_digest: string;
  packet_id: string;
  packet_digest: string;
  receipt_updated_at: CompatibleTimestamp;
  summary: RunSummary;
  packet_integrity: string;
  packet_currentness_at_receipt_snapshot: string;
  packet_currentness_now: string;
}

export interface RunIndex {
  schema: string;
  runs: RunIndexEntry[];
}

export interface StartingCustody {
  derived_id: string;
  repository: string;
  path: string;
  branch: string;
  commit: string;
  remote: string | null;
  remote_commit: string | null;
  worktree_clean: boolean;
  discrepancy: string | null;
}

export interface FinalCustody {
  derived_id: string | null;
  navigation_id: string;
  source_ordinal: number;
  repository: CompatibleValue;
  branch_head: CompatibleValue;
  push_custody: CompatibleValue;
  dirty: CompatibleValue;
  live_runtime: CompatibleValue;
  secrets: CompatibleValue;
  teardown: CompatibleValue;
}

export interface WorkItem {
  derived_id: string;
  id: string;
  track: string;
  campaign: { codename: string; canonical_slug: string };
  predecessor_lineage: Array<{ campaign: string; classification: string; commit: string }>;
  dependencies: string[];
  exact_work_refs: Array<{
    contract_kind: string;
    contract_schema: string;
    repository: string;
    branch: string;
    commit: string;
    path: string;
    proposal_ref: string;
  }>;
  entry_predicates: string[];
  allowed_mutation_surfaces: string[];
  forbidden_actions: string[];
  acceptance_tests: string[];
  stop_conditions: string[];
  expected_receipts: string[];
  closeout_requirements: string[];
  model_routing: { class: string; reason: string; maximum_mutating_workers: number };
  outcome: {
    state: CompatibleValue;
    result_classification: CompatibleValue;
    repositories: {
      recognized_rows: Array<{ repository: string; branch: string; head: string; push_status: string }> | null;
    };
    tests: RendererJoinedValue;
    evidence: RendererJoinedValue;
    live_or_production_mutations: RendererJoinedValue;
    remaining_trigger: CompatibleValue;
    next_lawful_action: CompatibleValue;
  };
}

export interface HumanQuestion {
  derived_id: string | null;
  navigation_id: string;
  source_ordinal: number;
  work_item: CompatibleValue;
  linked_work_item: string | null;
  exact_question: CompatibleValue;
  evidence_exhausted: CompatibleValue;
  safe_default: CompatibleValue;
  consequences: CompatibleValue;
  resume_point: CompatibleValue;
}

export interface CaseworkRun {
  schema: string;
  projection_digest: string;
  run_id: string;
  packet: {
    packet_id: string;
    packet_digest: string;
    created_at: string;
    current_until: string;
    source_bytes_digest: string;
    integrity: string;
    currentness_at_receipt_snapshot: string;
    currentness_evaluated_at: string;
    currentness_now: string;
    repository_custody: StartingCustody[];
  };
  receipts: {
    schema: string;
    updated_at: CompatibleTimestamp;
    source_bytes_digest: string;
    validation: string;
  };
  summary: RunSummary;
  work_items: WorkItem[];
  human_questions: HumanQuestion[];
  final_repository_custody: FinalCustody[];
}

export interface LiveRunIndexEntry {
  navigation_id: string;
  run_id: string;
  projection_digest: string;
  packet_id: string;
  packet_digest: string;
  lifecycle: string;
  sealed_case_run_id: string | null;
  scheduler_state_counts: Record<string, number>;
}

export interface LiveRunIndex {
  schema: string;
  runs: LiveRunIndexEntry[];
}

export interface LiveQuestion {
  navigation_id: string;
  question_id: string;
  question: string;
  exhausted_evidence: string;
  safe_default: string;
  consequences: string;
  resume_point: string;
}

export interface LiveWorkItem {
  work_item_id: string;
  track: string;
  campaign_codename: string;
  campaign_slug: string;
  dependencies: string[];
  entry_predicates: string[];
  stop_conditions: string[];
  scheduler_state: string;
  scheduler_state_recognized: boolean;
  dependency_terminality: Record<string, boolean>;
  resource_lock_keys: string[];
  active_attempt_id: string | null;
  adapter_id: string;
  adapter_version: string;
  provider_model_class: string;
  provider_identity: string | null;
  model_identity: string | null;
  session_identity: string | null;
  thread_identity: string | null;
  turn_identity: string | null;
  queue_identity: string | null;
  last_event_sequence: number | null;
  last_event_digest: string | null;
  human_questions: LiveQuestion[];
  accepted_receipt_kind: string | null;
  accepted_outcome: {
    state: string;
    result_classification: string;
    receipt_digest: string;
  } | null;
  accepted_outcome_absent_reason: string | null;
}

export interface CaseworkLiveRun {
  schema: string;
  projection_digest: string;
  navigation_id: string;
  run_id: string;
  evaluated_at: string;
  packet: {
    packet_id: string;
    packet_digest: string;
    exact_bytes_sha256: string;
    integrity: string;
    created_at: string;
    current_until: string;
    currentness: string;
  };
  admission: {
    admission_digest: string;
    exact_bytes_sha256: string;
    admitted_at: string;
    expires_at: string;
    currentness: string;
    maximum_concurrent_workers: number;
  };
  execution_profile: {
    profile_digest: string;
    exact_bytes_sha256: string;
    budget_policy_ref: string;
    capacity_binding_status: string;
  };
  foreman: {
    source_schema: string;
    lifecycle: string;
    scheduler_state_counts: Record<string, number>;
    terminal_receipt_count: number;
    not_started_receipt_count: number;
    closed_final_receipts_digest: string | null;
  };
  work_items: LiveWorkItem[];
  resource_claims: Array<{
    resource_lock_key: string;
    work_item_id: string;
    attempt_id: string;
  }>;
  events: Array<{
    sequence: number;
    event_id: string;
    work_item_id: string | null;
    attempt_id: string | null;
    kind: string;
    recorded_at: string;
    retained_raw_digest: string;
    exact_bytes_sha256: string;
    raw_length: number;
  }>;
  raw_sources: {
    packet_sha256: string;
    admission_sha256: string;
    profile_sha256: string;
    journal_framing_sha256: string;
    accepted_receipts_framing_sha256: string;
    final_snapshot_sha256: string | null;
  };
  sealed_case_run_id: string | null;
  provider_capacity: {
    status: string;
    requirement: null | {
      capacity_requirement_digest: string;
      exact_bytes_sha256: string;
      recorded_at: string;
      policy_id: string;
      provider_id: string;
      model_cost_classes: Record<string, "CHEAP" | "EXPENSIVE">;
      authority_effect: "LOCAL_AGENT_COMPUTE_SCHEDULING_ONLY";
    };
    attempts: Array<{
      journal_sequence: number;
      work_item_id: string;
      attempt_id: string;
      recorded_at: string;
      provider_id: string;
      packet_model_class: string;
      profile_model_class: string;
      cost_class: "CHEAP" | "EXPENSIVE";
      capacity_state: "ABUNDANT" | "NORMAL" | "CONSERVE" | "CRITICAL" | "UNKNOWN";
      admission_disposition: "ORDINARY_BOUNDED" | "CHEAP_BOUNDED_ONLY" | "NO_NEW_WORK";
      source_class: "AUTHORITATIVE" | "OBSERVED" | "INFERRED" | "UNKNOWN";
      confidence: "UNKNOWN" | "LOW" | "MEDIUM" | "HIGH";
      observation_disposition: "USABLE" | "UNKNOWN";
      observed_at: string;
      expires_at: string;
      decision_at: string;
      evaluated_at: string;
      currentness: "NOT_YET_CURRENT" | "CURRENT" | "EXPIRED";
      capacity_admission_digest: string;
      observation_digest: string;
      policy_digest: string;
      decision_digest: string;
      admission_exact_bytes_sha256: string;
      observation_exact_bytes_sha256: string;
      policy_exact_bytes_sha256: string;
      decision_exact_bytes_sha256: string;
    }>;
    explanation: string;
  };
  authority_effect: string;
}
