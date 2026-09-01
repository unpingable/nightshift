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

export interface LiveProviderExecutionIdentity {
  provider_id: string;
  model_id: string;
  app_server_session_identity: string;
  thread_id: string;
  turn_id: string;
  first_response_id: string;
}

export interface LiveProviderExecutionRequirement {
  journal_sequence: number;
  requirement_digest: string;
  policy_id: string;
  policy_digest: string;
  provider_id: string;
  work_item_model_selections: Record<string, Array<{
    provider_id: string;
    model_id: string;
    model_class: string;
  }>>;
  adapter_id: string;
  adapter_protocol: string;
  adapter_version: string;
  adapter_executable_identity: string;
  codex_owner_head: string;
  provider_admission_owner_head: string;
  provider_admission_schema_sha256: string;
  deterministic_fixture_sha256: string;
  admitted_at: string;
  requirement_exact_bytes_sha256: string;
  policy_exact_bytes_sha256: string;
  parked_resource_lock_policy: string;
  allow_ordered_model_fallback: boolean;
  automatic_semantic_retry: false;
  approval_response_authorized: false;
  authority_effect: "READ_ONLY_MECHANISM_PROJECTION";
}

export interface LiveProviderDispatch {
  journal_sequence: number;
  journal_event_id: string;
  journal_exact_bytes_sha256: string;
  journal_retained_raw_digest: string;
  work_item_id: string;
  work_attempt_id: string;
  dispatch_occurrence_id: string;
  dispatch_ordinal: number;
  selected_model_ordinal: number;
  provider_id: string;
  model_id: string;
  model_class: string;
  adapter_id: string;
  adapter_version: string;
  adapter_protocol: string;
  adapter_process_occurrence_id: string;
  app_server_session_identity: string;
  worker_start_request_digest: string;
  worker_brief_digest: string;
  dispatch_digest: string;
  opened_at: string;
  start_request_exact_bytes_sha256: string;
  dispatch_exact_bytes_sha256: string;
  provider_execution_identity_absent_at_start: true;
}

export interface LiveProviderDisposition {
  journal_sequence: number;
  journal_event_id: string;
  journal_exact_bytes_sha256: string;
  journal_retained_raw_digest: string;
  work_item_id: string;
  work_attempt_id: string;
  dispatch_occurrence_id: string;
  dispatch_digest: string;
  disposition_digest: string;
  reconciles_disposition_digest: string | null;
  provider_id: string;
  model_id: string;
  availability_state: string;
  admission_disposition: string;
  mechanism_state: string;
  observed_at: string;
  evidence_received_at: string;
  expires_at: string;
  disposition_received_at: string;
  currentness: string;
  source_identity: string;
  source_version: string;
  response_created: boolean;
  acquisition_complete: boolean;
  provider_retry_after: string | null;
  provider_request_occurrence_id: string;
  provider_execution: LiveProviderExecutionIdentity | null;
  mapper_snapshot_schema: string;
  mapper_snapshot_digest: string;
  approval_response_sent: false;
  protected_effect_absent: true;
  observation_digest: string;
  observation_exact_bytes_sha256: string;
  disposition_exact_bytes_sha256: string;
}

export interface CaseworkLiveProviderExecution {
  schema: "nightshift.casework-live-provider-execution/v1";
  projection_digest: string;
  run_id: string;
  packet_digest: string;
  evaluated_at: string;
  status: "NOT_RECORDED_BY_FOREMAN" | "EXACT_RECORDED_FOREMAN_HISTORY";
  requirement: LiveProviderExecutionRequirement | null;
  dispatches: LiveProviderDispatch[];
  dispositions: LiveProviderDisposition[];
  deferrals: Array<{
    journal_sequence: number;
    journal_event_id: string;
    journal_exact_bytes_sha256: string;
    disposition_digest: string;
    deferred_dispatch_digest: string;
    work_item_id: string;
    work_attempt_id: string;
    last_dispatch_occurrence_id: string;
    provider_id: string;
    model_id: string;
    selected_model_ordinal: number;
    remaining_model_ordinals: number[];
    refusal_received_at: string;
    wake_basis: string;
    backoff_ordinal: number;
    backoff_seconds: number;
    provider_retry_after: string | null;
    wake_at: string;
    parked_resource_lock_policy: string;
    provider_capacity_released: boolean;
    deferred_exact_bytes_sha256: string;
  }>;
  wakes: Array<{
    journal_sequence: number;
    journal_event_id: string;
    journal_exact_bytes_sha256: string;
    work_item_id: string;
    work_attempt_id: string;
    wake_occurrence_id: string;
    deferred_dispatch_digest: string;
    next_dispatch_digest: string;
    recorded_at: string;
  }>;
  resumes: Array<{
    journal_sequence: number;
    journal_event_id: string;
    journal_exact_bytes_sha256: string;
    work_item_id: string;
    work_attempt_id: string;
    resume_occurrence_id: string;
    disposition_digest: string;
    adapter_process_occurrence_id: string;
    execution_identity: LiveProviderExecutionIdentity;
    recorded_at: string;
  }>;
  resource_transitions: Array<{
    journal_sequence: number;
    journal_event_id: string;
    journal_exact_bytes_sha256: string;
    transition: "RELEASED" | "REACQUIRED";
    work_item_id: string;
    work_attempt_id: string;
    dispatch_digest: string;
    disposition_digest: string | null;
    deferred_dispatch_digest: string | null;
    policy_digest: string;
    wake_occurrence_id: string | null;
    resource_lock_keys: string[];
    recorded_at: string;
  }>;
  independent_provider_capacity_status: "NOT_RECORDED_BY_FOREMAN" | "EXACT_RECORDED_BY_FOREMAN";
  explanation: string;
  authority_effect: "READ_ONLY_MECHANISM_PROJECTION";
}
export interface OperationalConditionIndexEntry {
  navigation_id: string;
  projection_digest: string;
  lineage_id: string;
  evaluation_id: string;
  subject_kind: string;
  subject_namespace: string;
  subject_identity_digest: string;
  disposition: string;
  reobservation_trigger: string;
  evaluated_at: string;
  question_count: number;
}

export interface OperationalConditionIndex {
  schema: string;
  conditions: OperationalConditionIndexEntry[];
}

export interface OperationalSubject {
  kind: string;
  namespace: string;
  basis_contract: string;
  stable_basis: { basis_type: string } & Record<string, string>;
}

export interface OperationalProducer {
  principal_id: string;
  collector_id: string;
  key_algorithm: string;
  public_key_hex: string;
  public_key_digest: string;
  producer_class: string;
}

export interface OperationalCustody {
  raw_bytes_sha256: string;
  raw_bytes_length: number;
  semantic_digest: string;
}

export interface OperationalClaimSupport {
  claim_id: string;
  proposition: string;
  value_digest: string;
  monitor_record_digest: string;
}

export interface OperationalCannotTestify {
  claim_id: string;
  reason: string;
}

export interface OperationalRefusal {
  code: string;
  exact_basis_digest: string;
  detail: string;
}

export interface OperationalContradiction {
  subject_identity_digest: string;
  claim_id: string;
  first_input_id: string;
  first_value_digest: string;
  second_input_id: string;
  second_value_digest: string;
}

export interface OperationalLineage {
  schema: string;
  lineage_id: string;
  monitor_result_head: string;
  nq_result_head: string;
  monitor_custody: OperationalCustody;
  nq_custody: OperationalCustody;
  nq_profile_id: string;
  nq_input_id: string;
  subject: OperationalSubject;
  subject_identity_digest: string;
  producer: OperationalProducer;
  producer_identity_digest: string;
  acquisition_outcome: string;
  acquisition_started_at: string;
  acquisition_ended_at: string;
  producer_observed_at: string | null;
  receiver_custody_at: string;
  nq_qualified_at: string;
  nightshift_admitted_at: string;
  epoch: string;
  sequence: number;
  predecessor_observation_digest: string | null;
  payload_schema: string | null;
  claim_support: OperationalClaimSupport[];
  cannot_testify: OperationalCannotTestify[];
  refusals: OperationalRefusal[];
  contradictions: OperationalContradiction[];
  nonclaims: string[];
}

export interface OperationalEvaluation {
  schema: string;
  evaluation_id: string;
  lineage_id: string;
  profile_id: string;
  profile_digest: string;
  max_age_seconds: number;
  evaluated_at: string;
  current_until: string | null;
  exact_supported_claim_ids: string[];
  disposition: string;
  reobservation_trigger: string;
  next_lawful_action: string;
  grants_authority: false;
}

export interface OperationalQuestion {
  navigation_id: string;
  question_id: string;
  question: string;
  source_index: number;
  source:
    | { source_kind: "cannot_testify"; finding: OperationalCannotTestify }
    | { source_kind: "refusal"; finding: OperationalRefusal }
    | { source_kind: "contradiction"; finding: OperationalContradiction };
  next_lawful_action: string;
  presentation_only: true;
}

export interface OperationalRawSource {
  exact_bytes_sha256: string;
  exact_bytes_length: number;
  validation: string;
}

export interface CaseworkOperationalCondition {
  schema: string;
  projection_digest: string;
  navigation_id: string;
  subject: OperationalSubject;
  subject_identity_digest: string;
  producer: OperationalProducer;
  producer_identity_digest: string;
  acquisition_outcome: string;
  lineage: OperationalLineage;
  evaluation: OperationalEvaluation;
  profile: {
    profile_id: string;
    max_age_seconds: number;
  };
  questions: OperationalQuestion[];
  raw_sources: {
    monitor: OperationalRawSource;
    nq: OperationalRawSource;
    lineage: OperationalRawSource;
    profile: OperationalRawSource;
    evaluation: OperationalRawSource;
  };
  authority_effect: "read_only_projection_no_authority";
}
