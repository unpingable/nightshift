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
