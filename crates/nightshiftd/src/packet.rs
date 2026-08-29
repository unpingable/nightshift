//! Deterministic, non-authorizing Nightshift orientation packets.
//!
//! `NightshiftPacketV1` schedules references to existing exact-work proposals.
//! It does not mint standing, authorization, approval, execution custody,
//! retries, outcomes, or settlement.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest as _, Sha256};
use thiserror::Error;

pub const NIGHTSHIFT_PACKET_SCHEMA_V1: &str = "nightshift.orientation-packet/v1";
pub const EXACT_WORK_PROPOSAL_SCHEMA_V1: &str = "ag.governed-loop.exact-work-proposal/v1";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NightshiftPacketV1 {
    pub schema: String,
    pub packet_id: String,
    pub packet_digest: String,
    pub created_at: DateTime<Utc>,
    pub current_until: DateTime<Utc>,
    pub authoring: AuthoringIdentityV1,
    pub canonicalization: CanonicalizationV1,
    pub source_evidence: Vec<SourceEvidenceRefV1>,
    pub repository_custody: Vec<RepositoryCustodyV1>,
    pub global_constraints: GlobalConstraintsV1,
    pub work_items: Vec<WorkItemV1>,
    pub worker_budget: WorkerBudgetV1,
    pub human_question_criteria: Vec<String>,
    pub switchyard: SwitchyardRegistrationV1,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoringIdentityV1 {
    pub agent: String,
    pub session: String,
    pub authority_basis: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalizationV1 {
    pub algorithm: String,
    pub digest_algorithm: String,
    pub digest_preimage: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceEvidenceRefV1 {
    pub repository: String,
    pub branch: String,
    pub commit: String,
    pub path: String,
    pub file_digest: String,
    pub predecessor_classification: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryCustodyV1 {
    pub repository: String,
    pub path: String,
    pub branch: String,
    pub commit: String,
    pub remote: Option<String>,
    pub remote_commit: Option<String>,
    pub worktree_clean: bool,
    pub discrepancy: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GlobalConstraintsV1 {
    pub allowed_actions: Vec<String>,
    pub forbidden_actions: Vec<String>,
    pub invariants: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkItemV1 {
    pub id: String,
    pub track: String,
    pub campaign: CampaignIdentityV1,
    pub predecessor_lineage: Vec<PredecessorRefV1>,
    pub dependencies: Vec<String>,
    pub exact_work_refs: Vec<ExactWorkRefV1>,
    pub entry_predicates: Vec<String>,
    pub allowed_mutation_surfaces: Vec<String>,
    pub forbidden_actions: Vec<String>,
    pub acceptance_tests: Vec<String>,
    pub stop_conditions: Vec<String>,
    pub expected_receipts: Vec<String>,
    pub closeout_requirements: Vec<String>,
    pub model_routing: ModelRoutingV1,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignIdentityV1 {
    pub codename: String,
    pub canonical_slug: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PredecessorRefV1 {
    pub campaign: String,
    pub classification: String,
    pub commit: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExactWorkRefV1 {
    pub contract_kind: String,
    pub contract_schema: String,
    pub repository: String,
    pub branch: String,
    pub commit: String,
    pub path: String,
    pub proposal_ref: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelRoutingV1 {
    pub class: String,
    pub reason: String,
    pub maximum_mutating_workers: u16,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerBudgetV1 {
    pub maximum_concurrent_mutating_workers: u16,
    pub recursive_worker_swarms_forbidden: bool,
    pub reserve_posture: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SwitchyardRegistrationV1 {
    pub alias: String,
    pub plan_ref: String,
    pub transport_fields: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PacketValidationReceiptV1 {
    pub schema: String,
    pub packet_id: String,
    pub packet_digest: String,
    pub evaluated_at: DateTime<Utc>,
    pub work_item_count: usize,
    pub disposition: String,
    pub authority_effect: String,
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum PacketError {
    #[error("packet JSON is not valid: {0}")]
    Json(String),
    #[error("packet schema is not nightshift.orientation-packet/v1")]
    ForeignSchema,
    #[error("packet field is invalid: {0}")]
    InvalidField(&'static str),
    #[error("packet digest mismatch")]
    DigestMismatch,
    #[error("packet is not current at the supplied evaluation time")]
    NotCurrent,
    #[error("unknown work item dependency: {0}")]
    UnknownWorkItem(String),
    #[error("work item dependency graph contains a cycle")]
    DependencyCycle,
}

impl NightshiftPacketV1 {
    pub fn from_slice(bytes: &[u8]) -> Result<Self, PacketError> {
        serde_json::from_slice(bytes).map_err(|error| PacketError::Json(error.to_string()))
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, PacketError> {
        serde_jcs::to_vec(self).map_err(|error| PacketError::Json(error.to_string()))
    }

    /// Return the content identity. The two derived locator fields are omitted
    /// from the preimage to avoid self-reference; changing either is still
    /// rejected because validation recomputes both values.
    pub fn computed_digest(&self) -> Result<String, PacketError> {
        let mut value =
            serde_json::to_value(self).map_err(|error| PacketError::Json(error.to_string()))?;
        let Value::Object(ref mut object) = value else {
            return Err(PacketError::InvalidField("packet"));
        };
        object.remove("packet_digest");
        let switchyard = object
            .get_mut("switchyard")
            .and_then(Value::as_object_mut)
            .ok_or(PacketError::InvalidField("switchyard"))?;
        switchyard.remove("plan_ref");
        let preimage =
            serde_jcs::to_vec(&value).map_err(|error| PacketError::Json(error.to_string()))?;
        Ok(format!("sha256:{:x}", Sha256::digest(preimage)))
    }

    pub fn seal(&mut self) -> Result<(), PacketError> {
        self.packet_digest = self.computed_digest()?;
        self.switchyard.plan_ref = format!(
            "nightshift-packet://{}",
            self.packet_digest
                .strip_prefix("sha256:")
                .ok_or(PacketError::InvalidField("packet_digest"))?
        );
        Ok(())
    }

    pub fn validate_at(
        &self,
        evaluated_at: DateTime<Utc>,
    ) -> Result<PacketValidationReceiptV1, PacketError> {
        if self.schema != NIGHTSHIFT_PACKET_SCHEMA_V1 {
            return Err(PacketError::ForeignSchema);
        }
        if !valid_id(&self.packet_id) {
            return Err(PacketError::InvalidField("packet_id"));
        }
        if self.canonicalization.algorithm != "RFC8785-JCS"
            || self.canonicalization.digest_algorithm != "SHA-256"
            || self.canonicalization.digest_preimage
                != "packet object with packet_digest and switchyard.plan_ref omitted"
        {
            return Err(PacketError::InvalidField("canonicalization"));
        }
        require_nonempty("authoring.agent", &self.authoring.agent)?;
        require_nonempty("authoring.session", &self.authoring.session)?;
        require_nonempty("authoring.authority_basis", &self.authoring.authority_basis)?;
        if self.created_at >= self.current_until
            || evaluated_at < self.created_at
            || evaluated_at > self.current_until
        {
            return Err(PacketError::NotCurrent);
        }
        require_digest(&self.packet_digest)?;
        if self.computed_digest()? != self.packet_digest {
            return Err(PacketError::DigestMismatch);
        }
        let expected_plan_ref = format!(
            "nightshift-packet://{}",
            self.packet_digest
                .strip_prefix("sha256:")
                .unwrap_or_default()
        );
        if self.switchyard.plan_ref != expected_plan_ref
            || self.switchyard.transport_fields != ["alias", "plan_ref", "nonce"]
        {
            return Err(PacketError::InvalidField("switchyard"));
        }
        require_nonempty("switchyard.alias", &self.switchyard.alias)?;
        if self.worker_budget.maximum_concurrent_mutating_workers == 0
            || self.worker_budget.maximum_concurrent_mutating_workers > 4
            || !self.worker_budget.recursive_worker_swarms_forbidden
        {
            return Err(PacketError::InvalidField("worker_budget"));
        }
        if self.source_evidence.is_empty()
            || self.repository_custody.is_empty()
            || self.work_items.is_empty()
            || self.global_constraints.allowed_actions.is_empty()
            || self.global_constraints.forbidden_actions.is_empty()
            || self.human_question_criteria.is_empty()
        {
            return Err(PacketError::InvalidField("required collection"));
        }

        for source in &self.source_evidence {
            require_nonempty("source_evidence.repository", &source.repository)?;
            require_nonempty("source_evidence.path", &source.path)?;
            require_commit(&source.commit)?;
            require_digest(&source.file_digest)?;
        }
        for custody in &self.repository_custody {
            require_nonempty("repository_custody.repository", &custody.repository)?;
            require_commit(&custody.commit)?;
            if let Some(remote_commit) = &custody.remote_commit {
                require_commit(remote_commit)?;
            }
        }

        let mut ids = BTreeSet::new();
        let mut campaigns = BTreeSet::new();
        for item in &self.work_items {
            if !valid_id(&item.id) || !ids.insert(item.id.clone()) {
                return Err(PacketError::InvalidField("work_items.id"));
            }
            if !valid_codename(&item.campaign.codename)
                || !valid_slug(&item.campaign.canonical_slug)
                || !campaigns.insert((
                    item.campaign.codename.clone(),
                    item.campaign.canonical_slug.clone(),
                ))
            {
                return Err(PacketError::InvalidField("work_items.campaign"));
            }
            if item.entry_predicates.is_empty()
                || item.allowed_mutation_surfaces.is_empty()
                || item.forbidden_actions.is_empty()
                || item.acceptance_tests.is_empty()
                || item.stop_conditions.is_empty()
                || item.expected_receipts.is_empty()
                || item.closeout_requirements.is_empty()
            {
                return Err(PacketError::InvalidField("work_items contract"));
            }
            if item.model_routing.maximum_mutating_workers > 1 {
                return Err(PacketError::InvalidField("work_items.model_routing"));
            }
            for predecessor in &item.predecessor_lineage {
                require_commit(&predecessor.commit)?;
            }
            for proposal in &item.exact_work_refs {
                let valid_contract = match proposal.contract_kind.as_str() {
                    "exact_work_proposal_v1" => {
                        proposal.contract_schema == EXACT_WORK_PROPOSAL_SCHEMA_V1
                    }
                    "repository_actual_equivalent" => !proposal.contract_schema.trim().is_empty(),
                    _ => false,
                };
                if !valid_contract {
                    return Err(PacketError::InvalidField("exact_work_refs.contract"));
                }
                require_commit(&proposal.commit)?;
                require_digest(&proposal.proposal_ref)?;
            }
        }
        validate_dag(&self.work_items, &ids)?;

        Ok(PacketValidationReceiptV1 {
            schema: "nightshift.orientation-packet-validation/v1".into(),
            packet_id: self.packet_id.clone(),
            packet_digest: self.packet_digest.clone(),
            evaluated_at,
            work_item_count: self.work_items.len(),
            disposition: "VALID_NON_AUTHORIZING_ORIENTATION_PACKET".into(),
            authority_effect: "NONE".into(),
        })
    }

    pub fn render_markdown(&self) -> String {
        let mut output = String::new();
        output.push_str("# Nightshift packet summary (non-authorizing)\n\n");
        output.push_str("> This rendering is an orientation and scheduling aid. It grants no authority, approval, retry, execution custody, or settlement.\n\n");
        output.push_str(&format!("- Packet: `{}`\n", self.packet_id));
        output.push_str(&format!("- Digest: `{}`\n", self.packet_digest));
        output.push_str(&format!(
            "- Current: `{}` through `{}`\n",
            self.created_at, self.current_until
        ));
        output.push_str(&format!(
            "- Switchyard alias: `{}`\n",
            self.switchyard.alias
        ));
        output.push_str(&format!(
            "- Immutable plan reference: `{}`\n\n",
            self.switchyard.plan_ref
        ));
        output.push_str("## Campaign DAG\n\n");
        for item in &self.work_items {
            let dependencies = if item.dependencies.is_empty() {
                "none".to_owned()
            } else {
                item.dependencies.join(", ")
            };
            output.push_str(&format!(
                "- `{}` — **{}** / `{}`; depends on: {}\n",
                item.id, item.campaign.codename, item.campaign.canonical_slug, dependencies
            ));
        }
        output
    }
}

fn validate_dag(items: &[WorkItemV1], ids: &BTreeSet<String>) -> Result<(), PacketError> {
    let graph: BTreeMap<&str, Vec<&str>> = items
        .iter()
        .map(|item| {
            (
                item.id.as_str(),
                item.dependencies.iter().map(String::as_str).collect(),
            )
        })
        .collect();
    for dependencies in graph.values() {
        for dependency in dependencies {
            if !ids.contains(*dependency) {
                return Err(PacketError::UnknownWorkItem((*dependency).to_owned()));
            }
        }
    }
    fn visit<'a>(
        id: &'a str,
        graph: &BTreeMap<&'a str, Vec<&'a str>>,
        visiting: &mut BTreeSet<&'a str>,
        visited: &mut BTreeSet<&'a str>,
    ) -> Result<(), PacketError> {
        if visited.contains(id) {
            return Ok(());
        }
        if !visiting.insert(id) {
            return Err(PacketError::DependencyCycle);
        }
        for dependency in graph.get(id).into_iter().flatten() {
            visit(dependency, graph, visiting, visited)?;
        }
        visiting.remove(id);
        visited.insert(id);
        Ok(())
    }
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    for id in graph.keys() {
        visit(id, &graph, &mut visiting, &mut visited)?;
    }
    Ok(())
}

fn require_nonempty(field: &'static str, value: &str) -> Result<(), PacketError> {
    if value.trim().is_empty() {
        Err(PacketError::InvalidField(field))
    } else {
        Ok(())
    }
}

fn require_digest(value: &str) -> Result<(), PacketError> {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return Err(PacketError::InvalidField("digest"));
    };
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(PacketError::InvalidField("digest"));
    }
    Ok(())
}

fn require_commit(value: &str) -> Result<(), PacketError> {
    if value.len() != 40
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(PacketError::InvalidField("commit"));
    }
    Ok(())
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn valid_codename(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'-')
}

fn valid_slug(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 160
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}
