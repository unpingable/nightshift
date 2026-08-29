use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Utc};
use nightshiftd::packet::NightshiftPacketV1;
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest as _, Sha256};
use thiserror::Error;

use crate::model::*;

const PACKET_FILE: &str = "packet.v1.json";
const RECEIPTS_FILE: &str = "run-receipts.v1.json";

#[derive(Debug, Error)]
pub enum CaseworkError {
    #[error("run directory is invalid: {0}")]
    InvalidRunDirectory(String),
    #[error("run input path is not a regular non-symlink file: {0}")]
    InvalidInputPath(String),
    #[error("cannot read run input {path}: {source}")]
    Read {
        path: String,
        source: std::io::Error,
    },
    #[error("packet validation failed: {0}")]
    Packet(String),
    #[error("receipt compatibility validation failed: {0}")]
    Receipt(String),
    #[error("duplicate run packet digest: {0}")]
    DuplicateRun(String),
    #[error("projection serialization failed: {0}")]
    Projection(String),
}

#[derive(Clone, Debug)]
pub struct LoadedRun {
    pub projection: CaseworkRunV1,
    pub packet_bytes: Vec<u8>,
    pub receipt_bytes: Vec<u8>,
}

#[derive(Debug, Deserialize)]
struct ReceiptDocument {
    schema: Value,
    packet_digest: Value,
    updated_at: Value,
    work_items: Value,
    human_questions: Value,
    repository_custody: Value,
}

#[derive(Debug, Deserialize)]
struct ReceiptItem {
    id: Value,
    state: Value,
    result_classification: Value,
    repositories: Value,
    tests: Value,
    evidence: Value,
    live_or_production_mutations: Value,
    remaining_trigger: Value,
    next_lawful_action: Value,
}

pub fn load_runs_at(
    run_dirs: &[PathBuf],
    evaluated_now: DateTime<Utc>,
) -> Result<BTreeMap<String, LoadedRun>, CaseworkError> {
    let mut runs = BTreeMap::new();
    for run_dir in run_dirs {
        let loaded = load_run_at(run_dir, evaluated_now)?;
        let run_id = loaded.projection.run_id.clone();
        if runs.insert(run_id.clone(), loaded).is_some() {
            return Err(CaseworkError::DuplicateRun(run_id));
        }
    }
    Ok(runs)
}

pub fn load_run_at(
    run_dir: &Path,
    evaluated_now: DateTime<Utc>,
) -> Result<LoadedRun, CaseworkError> {
    let metadata = fs::symlink_metadata(run_dir)
        .map_err(|error| CaseworkError::InvalidRunDirectory(error.to_string()))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(CaseworkError::InvalidRunDirectory(
            run_dir.display().to_string(),
        ));
    }
    let root = fs::canonicalize(run_dir)
        .map_err(|error| CaseworkError::InvalidRunDirectory(error.to_string()))?;
    let packet_path = exact_input_path(&root, PACKET_FILE)?;
    let receipts_path = exact_input_path(&root, RECEIPTS_FILE)?;
    let packet_bytes = read(&packet_path)?;
    let receipt_bytes = read(&receipts_path)?;

    let packet = NightshiftPacketV1::from_slice(&packet_bytes)
        .map_err(|error| CaseworkError::Packet(error.to_string()))?;
    packet
        .validate_integrity()
        .map_err(|error| CaseworkError::Packet(error.to_string()))?;
    let receipts = parse_receipts(&receipt_bytes, &packet)?;
    let projection = project(
        &packet,
        &packet_bytes,
        receipts,
        &receipt_bytes,
        evaluated_now,
    )?;
    Ok(LoadedRun {
        projection,
        packet_bytes,
        receipt_bytes,
    })
}

fn exact_input_path(root: &Path, filename: &str) -> Result<PathBuf, CaseworkError> {
    let candidate = root.join(filename);
    let metadata = fs::symlink_metadata(&candidate)
        .map_err(|_| CaseworkError::InvalidInputPath(candidate.display().to_string()))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(CaseworkError::InvalidInputPath(
            candidate.display().to_string(),
        ));
    }
    let canonical = fs::canonicalize(&candidate)
        .map_err(|_| CaseworkError::InvalidInputPath(candidate.display().to_string()))?;
    if canonical.parent() != Some(root) {
        return Err(CaseworkError::InvalidInputPath(
            candidate.display().to_string(),
        ));
    }
    Ok(canonical)
}

fn read(path: &Path) -> Result<Vec<u8>, CaseworkError> {
    fs::read(path).map_err(|source| CaseworkError::Read {
        path: path.display().to_string(),
        source,
    })
}

#[derive(Debug)]
struct ParsedReceipts {
    updated_at: DateTime<Utc>,
    items: BTreeMap<String, ParsedReceiptItem>,
    questions: Vec<ParsedQuestion>,
    custody: Vec<ParsedCustody>,
}

#[derive(Debug)]
struct ParsedReceiptItem {
    state: String,
    classification: String,
    repositories: Vec<ResultRepositoryV1>,
    tests: Vec<String>,
    evidence: Vec<String>,
    mutations: Vec<String>,
    remaining_trigger: String,
    next_lawful_action: String,
}

#[derive(Debug)]
struct ParsedQuestion {
    work_item: String,
    exact_question: String,
    evidence_exhausted: String,
    safe_default: String,
    consequences: String,
    resume_point: String,
}

#[derive(Debug)]
struct ParsedCustody {
    repository: String,
    branch_head: String,
    push_custody: String,
    dirty: String,
    live_runtime: String,
    secrets: String,
    teardown: String,
}

fn parse_receipts(
    bytes: &[u8],
    packet: &NightshiftPacketV1,
) -> Result<ParsedReceipts, CaseworkError> {
    let value: Value = serde_json::from_slice(bytes).map_err(receipt_error)?;
    let doc: ReceiptDocument = serde_json::from_value(value).map_err(receipt_error)?;
    if string(&doc.schema, "schema")? != RUN_RECEIPTS_SCHEMA_V1 {
        return Err(receipt("foreign receipt schema"));
    }
    if string(&doc.packet_digest, "packet_digest")? != packet.packet_digest {
        return Err(receipt("receipt packet digest mismatch"));
    }
    let updated_at = DateTime::parse_from_rfc3339(string(&doc.updated_at, "updated_at")?)
        .map_err(|_| receipt("updated_at must be an RFC 3339 timestamp"))?
        .with_timezone(&Utc);
    let packet_ids: BTreeSet<&str> = packet
        .work_items
        .iter()
        .map(|item| item.id.as_str())
        .collect();
    let mut items = BTreeMap::new();
    for item_value in array(&doc.work_items, "work_items")? {
        let item: ReceiptItem =
            serde_json::from_value(item_value.clone()).map_err(receipt_error)?;
        let id = string(&item.id, "work_items.id")?.to_owned();
        if items.contains_key(&id) {
            return Err(receipt(format!("duplicate receipt work item: {id}")));
        }
        if !packet_ids.contains(id.as_str()) {
            return Err(receipt(format!("unknown receipt work item: {id}")));
        }
        items.insert(
            id,
            ParsedReceiptItem {
                state: string(&item.state, "work_items.state")?.to_owned(),
                classification: string(
                    &item.result_classification,
                    "work_items.result_classification",
                )?
                .to_owned(),
                repositories: parse_repositories(&item.repositories)?,
                tests: strings(&item.tests, "work_items.tests")?,
                evidence: strings(&item.evidence, "work_items.evidence")?,
                mutations: strings(
                    &item.live_or_production_mutations,
                    "work_items.live_or_production_mutations",
                )?,
                remaining_trigger: string(&item.remaining_trigger, "work_items.remaining_trigger")?
                    .to_owned(),
                next_lawful_action: string(
                    &item.next_lawful_action,
                    "work_items.next_lawful_action",
                )?
                .to_owned(),
            },
        );
    }
    let missing: Vec<_> = packet_ids
        .iter()
        .filter(|id| !items.contains_key(**id))
        .copied()
        .collect();
    if !missing.is_empty() {
        return Err(receipt(format!(
            "missing receipt work item: {}",
            missing.join(", ")
        )));
    }
    Ok(ParsedReceipts {
        updated_at,
        items,
        questions: parse_questions(&doc.human_questions, &packet_ids)?,
        custody: parse_custody(&doc.repository_custody)?,
    })
}

fn parse_repositories(value: &Value) -> Result<Vec<ResultRepositoryV1>, CaseworkError> {
    array(value, "work_items.repositories")?
        .iter()
        .map(|row| {
            let object = object(row, "work_items.repositories row")?;
            Ok(ResultRepositoryV1 {
                repository: object_string(object, "repository", "work_items.repositories")?,
                branch: object_string(object, "branch", "work_items.repositories")?,
                head: object_string(object, "head", "work_items.repositories")?,
                push_status: object_string(object, "push_status", "work_items.repositories")?,
            })
        })
        .collect()
}

fn parse_questions(
    value: &Value,
    packet_ids: &BTreeSet<&str>,
) -> Result<Vec<ParsedQuestion>, CaseworkError> {
    array(value, "human_questions")?
        .iter()
        .map(|row| {
            let object = object(row, "human_questions row")?;
            let work_item = object_string(object, "work_item", "human_questions")?;
            if !packet_ids.contains(work_item.as_str()) {
                return Err(receipt(format!(
                    "human question links unknown work item: {work_item}"
                )));
            }
            Ok(ParsedQuestion {
                work_item,
                exact_question: object_string(object, "exact_question", "human_questions")?,
                evidence_exhausted: object_string(object, "evidence_exhausted", "human_questions")?,
                safe_default: object_string(object, "safe_default", "human_questions")?,
                consequences: object_string(object, "consequences", "human_questions")?,
                resume_point: object_string(object, "resume_point", "human_questions")?,
            })
        })
        .collect()
}

fn parse_custody(value: &Value) -> Result<Vec<ParsedCustody>, CaseworkError> {
    array(value, "repository_custody")?
        .iter()
        .map(|row| {
            let object = object(row, "repository_custody row")?;
            Ok(ParsedCustody {
                repository: object_string(object, "repository", "repository_custody")?,
                branch_head: object_string(object, "branch_head", "repository_custody")?,
                push_custody: object_string(object, "push_custody", "repository_custody")?,
                dirty: object_string(object, "dirty", "repository_custody")?,
                live_runtime: object_string(object, "live_runtime", "repository_custody")?,
                secrets: object_string(object, "secrets", "repository_custody")?,
                teardown: object_string(object, "teardown", "repository_custody")?,
            })
        })
        .collect()
}

fn project(
    packet: &NightshiftPacketV1,
    packet_bytes: &[u8],
    mut receipts: ParsedReceipts,
    receipt_bytes: &[u8],
    evaluated_now: DateTime<Utc>,
) -> Result<CaseworkRunV1, CaseworkError> {
    let run_id = packet
        .packet_digest
        .strip_prefix("sha256:")
        .expect("integrity validation requires this prefix")
        .to_owned();
    let packet_digest = packet.packet_digest.clone();
    let starting_custody = packet
        .repository_custody
        .iter()
        .enumerate()
        .map(|(index, row)| StartingCustodyV1 {
            derived_id: derived_id(
                "nightshift.casework.custody-row/v1",
                &[
                    &packet_digest,
                    "packet",
                    &row.repository,
                    &index.to_string(),
                ],
            ),
            repository: row.repository.clone(),
            path: row.path.clone(),
            branch: row.branch.clone(),
            commit: row.commit.clone(),
            remote: row.remote.clone(),
            remote_commit: row.remote_commit.clone(),
            worktree_clean: row.worktree_clean,
            discrepancy: row.discrepancy.clone(),
        })
        .collect::<Vec<_>>();
    let discrepancy_count = starting_custody
        .iter()
        .filter(|row| row.discrepancy.is_some())
        .count();
    let mut state_counts = BTreeMap::new();
    let mut work_items = Vec::with_capacity(packet.work_items.len());
    for item in &packet.work_items {
        let outcome = receipts
            .items
            .remove(&item.id)
            .expect("one-to-one receipt validation established linkage");
        *state_counts.entry(outcome.state.clone()).or_insert(0) += 1;
        work_items.push(CaseworkItemV1 {
            derived_id: derived_id(
                "nightshift.casework.work-item/v1",
                &[&packet_digest, &item.id],
            ),
            id: item.id.clone(),
            track: item.track.clone(),
            campaign: CampaignV1 {
                codename: item.campaign.codename.clone(),
                canonical_slug: item.campaign.canonical_slug.clone(),
            },
            predecessor_lineage: item
                .predecessor_lineage
                .iter()
                .map(|row| PredecessorV1 {
                    campaign: row.campaign.clone(),
                    classification: row.classification.clone(),
                    commit: row.commit.clone(),
                })
                .collect(),
            dependencies: item.dependencies.clone(),
            exact_work_refs: item
                .exact_work_refs
                .iter()
                .map(|row| ExactWorkRefV1 {
                    contract_kind: row.contract_kind.clone(),
                    contract_schema: row.contract_schema.clone(),
                    repository: row.repository.clone(),
                    branch: row.branch.clone(),
                    commit: row.commit.clone(),
                    path: row.path.clone(),
                    proposal_ref: row.proposal_ref.clone(),
                })
                .collect(),
            entry_predicates: item.entry_predicates.clone(),
            allowed_mutation_surfaces: item.allowed_mutation_surfaces.clone(),
            forbidden_actions: item.forbidden_actions.clone(),
            acceptance_tests: item.acceptance_tests.clone(),
            stop_conditions: item.stop_conditions.clone(),
            expected_receipts: item.expected_receipts.clone(),
            closeout_requirements: item.closeout_requirements.clone(),
            model_routing: ModelRoutingV1 {
                class: item.model_routing.class.clone(),
                reason: item.model_routing.reason.clone(),
                maximum_mutating_workers: item.model_routing.maximum_mutating_workers,
            },
            outcome: WorkItemOutcomeV1 {
                state: outcome.state,
                result_classification: outcome.classification,
                repositories: outcome.repositories,
                tests: outcome.tests,
                evidence: outcome.evidence,
                live_or_production_mutations: outcome.mutations,
                remaining_trigger: outcome.remaining_trigger,
                next_lawful_action: outcome.next_lawful_action,
            },
        });
    }
    let questions = receipts
        .questions
        .into_iter()
        .map(|question| HumanQuestionV1 {
            derived_id: derived_id(
                "nightshift.casework.question/v1",
                &[
                    &packet_digest,
                    &question.work_item,
                    &question.exact_question,
                ],
            ),
            work_item: question.work_item,
            exact_question: question.exact_question,
            evidence_exhausted: question.evidence_exhausted,
            safe_default: question.safe_default,
            consequences: question.consequences,
            resume_point: question.resume_point,
        })
        .collect::<Vec<_>>();
    let final_custody = receipts
        .custody
        .into_iter()
        .enumerate()
        .map(|(index, row)| FinalCustodyV1 {
            derived_id: derived_id(
                "nightshift.casework.custody-row/v1",
                &[
                    &packet_digest,
                    "receipts",
                    &row.repository,
                    &index.to_string(),
                ],
            ),
            repository: row.repository,
            branch_head: row.branch_head,
            push_custody: row.push_custody,
            dirty: row.dirty,
            live_runtime: row.live_runtime,
            secrets: row.secrets,
            teardown: row.teardown,
        })
        .collect();
    let snapshot_currentness = currentness(packet, receipts.updated_at);
    let now_currentness = currentness(packet, evaluated_now);
    let mut projection = CaseworkRunV1 {
        schema: CASEWORK_RUN_SCHEMA_V1.to_owned(),
        projection_digest: String::new(),
        run_id,
        packet: PacketCaseV1 {
            packet_id: packet.packet_id.clone(),
            packet_digest,
            created_at: packet.created_at.to_rfc3339(),
            current_until: packet.current_until.to_rfc3339(),
            source_bytes_digest: bytes_digest(packet_bytes),
            integrity: "VALID_PACKET_INTEGRITY".to_owned(),
            currentness_at_receipt_snapshot: snapshot_currentness.to_owned(),
            currentness_now: now_currentness.to_owned(),
            repository_custody: starting_custody,
        },
        receipts: ReceiptSnapshotV1 {
            schema: RUN_RECEIPTS_SCHEMA_V1.to_owned(),
            updated_at: receipts.updated_at.to_rfc3339(),
            source_bytes_digest: bytes_digest(receipt_bytes),
            validation: "VALID_RENDERER_COMPATIBLE_RECEIPT_SNAPSHOT".to_owned(),
        },
        summary: RunSummaryV1 {
            work_item_count: work_items.len(),
            state_counts,
            human_question_count: questions.len(),
            packet_custody_discrepancy_count: discrepancy_count,
        },
        work_items,
        human_questions: questions,
        final_repository_custody: final_custody,
    };
    projection.projection_digest = projection_digest(&projection)?;
    Ok(projection)
}

fn currentness(packet: &NightshiftPacketV1, instant: DateTime<Utc>) -> &'static str {
    if instant < packet.created_at {
        "NOT_YET_CURRENT"
    } else if instant > packet.current_until {
        "EXPIRED"
    } else {
        "CURRENT"
    }
}

fn derived_id(domain: &str, components: &[&str]) -> String {
    let canonical = serde_jcs::to_vec(components).expect("strings have a JCS representation");
    let mut digest = Sha256::new();
    digest.update(domain.as_bytes());
    digest.update([0]);
    digest.update(canonical);
    format!("sha256:{:x}", digest.finalize())
}

fn projection_digest(projection: &CaseworkRunV1) -> Result<String, CaseworkError> {
    let mut value = serde_json::to_value(projection)
        .map_err(|error| CaseworkError::Projection(error.to_string()))?;
    value
        .as_object_mut()
        .expect("projection serializes as an object")
        .remove("projection_digest");
    let canonical =
        serde_jcs::to_vec(&value).map_err(|error| CaseworkError::Projection(error.to_string()))?;
    Ok(bytes_digest(&canonical))
}

fn bytes_digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn receipt_error(error: impl std::fmt::Display) -> CaseworkError {
    receipt(error.to_string())
}

fn receipt(message: impl Into<String>) -> CaseworkError {
    CaseworkError::Receipt(message.into())
}

fn string<'a>(value: &'a Value, field: &str) -> Result<&'a str, CaseworkError> {
    value
        .as_str()
        .ok_or_else(|| receipt(format!("{field} must be a string")))
}

fn strings(value: &Value, field: &str) -> Result<Vec<String>, CaseworkError> {
    array(value, field)?
        .iter()
        .map(|value| string(value, field).map(ToOwned::to_owned))
        .collect()
}

fn array<'a>(value: &'a Value, field: &str) -> Result<&'a [Value], CaseworkError> {
    value
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| receipt(format!("{field} must be an array")))
}

fn object<'a>(
    value: &'a Value,
    field: &str,
) -> Result<&'a serde_json::Map<String, Value>, CaseworkError> {
    value
        .as_object()
        .ok_or_else(|| receipt(format!("{field} must be an object")))
}

fn object_string(
    object: &serde_json::Map<String, Value>,
    key: &str,
    field: &str,
) -> Result<String, CaseworkError> {
    string(
        object
            .get(key)
            .ok_or_else(|| receipt(format!("{field} missing required field {key}")))?,
        &format!("{field}.{key}"),
    )
    .map(ToOwned::to_owned)
}
