export interface RunSummary {
  work_item_count: number;
  state_counts: Record<string, number>;
  human_question_count: number;
  packet_custody_discrepancy_count: number;
}

export interface RunIndexEntry {
  run_id: string;
  projection_digest: string;
  packet_id: string;
  packet_digest: string;
  receipt_updated_at: string;
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
  derived_id: string;
  repository: string;
  branch_head: string;
  push_custody: string;
  dirty: string;
  live_runtime: string;
  secrets: string;
  teardown: string;
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
    state: string;
    result_classification: string;
    repositories: {
      canonical_json: string;
      recognized_rows: Array<{ repository: string; branch: string; head: string; push_status: string }> | null;
    };
    tests: string[];
    evidence: string[];
    live_or_production_mutations: string[];
    remaining_trigger: string;
    next_lawful_action: string;
  };
}

export interface HumanQuestion {
  derived_id: string;
  work_item: string;
  exact_question: string;
  evidence_exhausted: string;
  safe_default: string;
  consequences: string;
  resume_point: string;
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
    currentness_now: string;
    repository_custody: StartingCustody[];
  };
  receipts: {
    schema: string;
    updated_at: string;
    source_bytes_digest: string;
    validation: string;
  };
  summary: RunSummary;
  work_items: WorkItem[];
  human_questions: HumanQuestion[];
  final_repository_custody: FinalCustody[];
}
