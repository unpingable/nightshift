//! WO-9 cross-boundary integration: the real Nightshift canonical runtime and
//! store feed the real `nightshift-observation-resolver` subprocess, which
//! feeds the real AG governed loop driven through the production `ag-loopctl`
//! CLI, with the real `ag-standing-resolver` subprocess on the standing gate
//! and the real Docket/`ag-effectd` pair behind the one-use spend.
//!
//! No fixture stands in for a serialized boundary: observation and standing
//! resolutions are produced by the production binaries from real persisted
//! state, AG state transitions happen only through `ag-loopctl`, and the
//! effect happens only through Docket custody and the executor plan.
//!
//! These tests are `#[ignore]`d in the default suite because they need
//! adjacent-repository binaries (the normal workspace suite does not build
//! adjacent repositories). Run them with:
//!
//! ```sh
//! AG_LOOPCTL_BIN=/path/to/ag-loopctl \
//! AG_STANDING_RESOLVER_BIN=/path/to/ag-standing-resolver \
//! AG_DOCKET_BIN=/path/to/docket \
//! AG_EFFECTD_BIN=/path/to/ag-effectd \
//! cargo test -p nightshiftd --test ag_governed_integration -- --include-ignored
//! ```
//!
//! The always-on `condition_present_fixture_is_real_and_resealed` test needs
//! no external binary and runs in the default suite.

use std::io::Write as _;
use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use chrono::{DateTime, Utc};
use nightshiftd::ag_port::{AgOccurrencePortV1, AgOpenModeV1, AgOpenOccurrenceRequestV1};
use nightshiftd::canonical_runtime::{
    ag_executor_plan_identity, CanonicalCycleRequestV1, CanonicalRuntime, CycleRunOutcomeV1,
    PrecompiledWorkflowProposalV2,
};
use nightshiftd::canonical_store::{
    AgOccurrenceReferenceV1, AgProgramCounterV1, CanonicalStore, RecurrenceSlotV1,
    RecurrenceTriggerV1,
};
use nightshiftd::currentness::{
    PresentEvidencePortV1, PresentEvidenceQueryV1, QualifiedSupportV1, SupportExpiryV1,
    SupportReceiverInstantV1, SupportStandingV1,
};
use nightshiftd::decision_basis::normalize_posture;
use nightshiftd::diagnostic_posture::{
    ConditionAxis, DiagnosticInputs, PosturePolicy, RecurrenceEvidence,
};
use sha2::{Digest as _, Sha256};

/// The identity AG is configured to expect from the observation resolver.
const OBSERVATION_RESOLVER_ID: &str = "nightshift-observation-resolver/v1";
/// The identity AG is configured to expect from the standing resolver.
const STANDING_RESOLVER_ID: &str = "ag-standing-resolver/integration-v1";
/// The standing resolver's answer lease and AG's kernel maximum.
const STANDING_TTL_MS: u64 = 60_000;
/// The observation evidence window configured for the resolver. The checked-in
/// Nightshift specimen is evaluated at 2026-07-27 while `ag-loopctl` reads the
/// wall clock, so the deployment TTL must span the gap; the kernel's freshness
/// law (`now < fresh_until`) is still exercised exactly.
const OBSERVATION_TTL_MS: u64 = 1_000_000_000_000_000;
/// The AG subject digest compiled into every test proposal (`digest('b')`).
const SUBJECT_DIGEST: &str =
    "sha256:6262626262626262626262626262626262626262626262626262626262626262";
/// Test-only issuer credential for the Docket trust path. This is published
/// test data, not a deployment secret.
const ISSUER_PKCS8_HEX: &str = "3051020101300506032b657004220420c226c22f628685cd349518c28eff015fd216a106bb49534286dceed3202b1c0e81210028d8b71d122a31cfd39f26313275119934a021918f5d37d100ad2f27acbaf776";
const ISSUER_PUBLIC_KEY_B64URL: &str = "KNi3HRIqMc_TnyYxMnURmTSgIZGPXTfRAK0vJ6y693Y";
const ISSUER_PRINCIPAL: &str = "ag-wo9-integration";
const ISSUER_KEY_ID: &str = "key-1";

const WORK_SCHEMA: &str = "ag-effectd.docket-executor-work/v1";
const CATALOG_SCHEMA: &str = "ag.governed-loop.exact-work-catalog/v1";
const PROPOSAL_SCHEMA: &str = "ag.governed-loop.exact-work-proposal/v1";
const MANDATE_STORE_SCHEMA: &str = "ag.governed-loop.standing-mandate-store/v1";
const PLAN_SCHEMA: &str = "ag-effectd.docket-executor-plan/v1";
const MANDATE_DIGEST_DOMAIN: &str = "ag.governed-loop.standing-mandate/v1";
const CATALOG_DIGEST_DOMAIN: &str = "ag.governed-loop.exact-work-catalog/v1";
const PLAN_DIGEST_DOMAIN: &str = "ag-effectd.docket-executor-plan/v1";

fn digest(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

fn campaign() -> String {
    digest('a')
}

fn program() -> String {
    digest('2')
}

fn occurrence_uuid(occurrence: u64) -> String {
    format!("00000000-0000-4000-8000-{occurrence:012}")
}

fn time(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .unwrap()
        .with_timezone(&Utc)
}

fn sha256_digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

/// Canonical-JSON content digest, the nightshift-side sealing convention.
fn digest_value(value: &serde_json::Value) -> String {
    sha256_digest(&serde_jcs::to_vec(value).unwrap())
}

/// Byte-exact mirror of `ag_primitives::Digest::hash_domain`, reimplemented
/// here so cross-boundary agreement is proven rather than assumed.
fn ag_hash_domain(domain: &str, payload: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"ag-ng\0digest\0v1\0");
    hasher.update((domain.len() as u128).to_be_bytes());
    hasher.update(domain.as_bytes());
    hasher.update((payload.len() as u128).to_be_bytes());
    hasher.update(payload);
    format!("sha256:{:x}", hasher.finalize())
}

fn ag_digest_value(domain: &str, value: &serde_json::Value) -> String {
    ag_hash_domain(domain, &serde_jcs::to_vec(value).unwrap())
}

fn wall_now_ms() -> u64 {
    u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap()
}

fn current_uid_gid() -> (u32, u32) {
    let status = std::fs::read_to_string("/proc/self/status").unwrap();
    let mut uid = None;
    let mut gid = None;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("Uid:") {
            uid = Some(rest.split_whitespace().next().unwrap().parse().unwrap());
        }
        if let Some(rest) = line.strip_prefix("Gid:") {
            gid = Some(rest.split_whitespace().next().unwrap().parse().unwrap());
        }
    }
    (uid.unwrap(), gid.unwrap())
}

// --- Nightshift store fixtures (real canonical runtime, real store) ---

fn example_policy_inputs_recurrence() -> (PosturePolicy, DiagnosticInputs, RecurrenceEvidence) {
    (
        serde_json::from_str(include_str!(
            "../../../docs/operator/examples/diagnostic-posture-v1/policy.json"
        ))
        .unwrap(),
        serde_json::from_str(include_str!(
            "../../../docs/operator/examples/diagnostic-posture-v1/inputs.json"
        ))
        .unwrap(),
        serde_json::from_str(include_str!(
            "../../../docs/operator/examples/diagnostic-posture-v1/recurrence.json"
        ))
        .unwrap(),
    )
}

/// The checked-in specimen with its one claim's condition flipped to
/// `present` in both the delivered artifact and the recurrence reference,
/// with every content-derived identity honestly resealed. Everything else —
/// times, slots, obligations, support binding — is byte-identical to the
/// clean specimen.
fn condition_present_inputs_recurrence() -> (DiagnosticInputs, RecurrenceEvidence) {
    let mut inputs_value: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/operator/examples/diagnostic-posture-v1/inputs.json"
    ))
    .unwrap();
    let artifact = &mut inputs_value["inputs"][0]["artifact"];
    artifact["claims"][0]["condition_effect"] = serde_json::json!("present");
    artifact["claims"][0]["proposition"] = serde_json::json!("host load pressure is present");
    artifact["outcome"]["condition"] = serde_json::json!("present");
    artifact["outcome"]["summary"] =
        serde_json::json!("complete current testimony places load above threshold");
    let mut preimage = artifact.clone();
    preimage.as_object_mut().unwrap().remove("artifact_id");
    let resealed = digest_value(&preimage);
    artifact["artifact_id"] = serde_json::json!(resealed);
    let mut inputs: DiagnosticInputs = serde_json::from_value(inputs_value).unwrap();
    inputs.inputs_id = inputs.computed_inputs_id().unwrap();

    let mut recurrence_value: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/operator/examples/diagnostic-posture-v1/recurrence.json"
    ))
    .unwrap();
    let reference = &mut recurrence_value["records"][0]["evidence"]["artifact"];
    reference["artifact_id"] = serde_json::json!(resealed);
    reference["claim"]["condition_effect"] = serde_json::json!("present");
    reference["claim"]["proposition"] = serde_json::json!("host load pressure is present");
    let mut recurrence: RecurrenceEvidence = serde_json::from_value(recurrence_value).unwrap();
    recurrence.recurrence_id = recurrence.computed_recurrence_id().unwrap();
    (inputs, recurrence)
}

struct SupportPort;

impl PresentEvidencePortV1 for SupportPort {
    fn resolve(&mut self, query: &PresentEvidenceQueryV1) -> Result<QualifiedSupportV1, String> {
        let mut support = QualifiedSupportV1 {
            schema: nightshiftd::currentness::QUALIFIED_SUPPORT_SCHEMA_V1.into(),
            support_id: String::new(),
            authority_id: "pulse-receiver-1".into(),
            query_id: query.query_id.clone(),
            observation_cycle_id: query.observation_cycle_id.clone(),
            request_nonce: query.request_nonce.clone(),
            observation_id: query.observation_id.clone(),
            diagnostic_inputs_id: query.diagnostic_inputs_id.clone(),
            subject_id: query.subject_id.clone(),
            scope_id: query.scope_id.clone(),
            artifact_ids: query.artifact_ids.clone(),
            evaluated_at: SupportReceiverInstantV1 {
                clock_id: "pulse-receiver-clock-1".into(),
                tick: 100,
            },
            expiry: Some(SupportExpiryV1 {
                clock_id: "pulse-receiver-clock-1".into(),
                tick: 101,
            }),
            standing: SupportStandingV1::Current,
            evidence_refs: vec![digest('9')],
            contradiction_refs: Vec::new(),
        };
        support.support_id = support.computed_support_id()?;
        support.validate_for(query)?;
        Ok(support)
    }
}

struct FakeAg;

impl AgOccurrencePortV1 for FakeAg {
    fn open_occurrence(
        &mut self,
        request: &AgOpenOccurrenceRequestV1,
    ) -> Result<AgOccurrenceReferenceV1, String> {
        let exact_snapshot = serde_json::json!({
            "campaign": request.campaign_id,
            "occurrence": request.occurrence_id,
            "program_counter": AgProgramCounterV1::ProposalRecorded,
        });
        Ok(AgOccurrenceReferenceV1 {
            schema: nightshiftd::canonical_store::AG_REFERENCE_SCHEMA_V1.into(),
            campaign_id: request.campaign_id.clone(),
            occurrence_id: request.occurrence_id.clone(),
            state_digest: digest('7'),
            snapshot_digest: digest_value(&exact_snapshot),
            program_counter: AgProgramCounterV1::ProposalRecorded,
            docket_attempt_id: None,
            settlement_id: None,
            external_decision_request_id: None,
            exact_snapshot,
        })
    }

    fn status(&mut self, _: &str, _: &str) -> Result<AgOccurrenceReferenceV1, String> {
        Err("integration tests never sync AG status".into())
    }
}

/// One sealed canonical cycle request. `with_proposal` attaches the
/// precompiled workflow proposal, which makes the runtime persist the typed
/// intent binding the Nightshift subject to `SUBJECT_DIGEST` and open the AG
/// occurrence through the port.
fn cycle_request(
    policy: &PosturePolicy,
    inputs: &DiagnosticInputs,
    recurrence: &RecurrenceEvidence,
    occurrence: u64,
    observation_id: &str,
    with_proposal: bool,
    plan: &serde_json::Value,
) -> CanonicalCycleRequestV1 {
    let slot = RecurrenceSlotV1::new(
        policy.policy_id.clone(),
        "config-v1".into(),
        policy.subject.id.clone(),
        policy.subject.scope.digest.clone(),
        "nightshift-scheduler-1".into(),
        time("2026-07-27T20:00:00Z") + chrono::Duration::minutes(occurrence as i64),
        time("2026-07-27T20:00:30Z") + chrono::Duration::minutes(occurrence as i64),
        occurrence,
        RecurrenceTriggerV1::Scheduled,
        None,
    )
    .unwrap();
    let immutable_parameters = serde_json::json!({"resource_id":"resource-1"});
    CanonicalCycleRequestV1 {
        schema: String::new(),
        request_id: String::new(),
        slot,
        scheduler_clock_id: "nightshift-scheduler-1".into(),
        evaluated_at: time("2026-07-27T20:00:10Z")
            + chrono::Duration::minutes(occurrence as i64),
        observation_id: observation_id.into(),
        policy: policy.clone(),
        inputs: inputs.clone(),
        recurrence: recurrence.clone(),
        temporal_policy: None,
        proposal: with_proposal.then(|| {
            let expected_ag_work = ag_executor_plan_identity(plan).unwrap();
            PrecompiledWorkflowProposalV2 {
                schema: nightshiftd::canonical_runtime::PRECOMPILED_PROPOSAL_SCHEMA_V2.into(),
                workflow_id: "workflow:host-care".into(),
                intent_kind: "inspect_exact_resource".into(),
                subject_digest: SUBJECT_DIGEST.into(),
                immutable_parameters,
                ag_executor_plan: plan.clone(),
                campaign_id: campaign(),
                occurrence_id: occurrence_uuid(0),
                mode: AgOpenModeV1::Genesis {
                    genesis: serde_json::json!({
                        "campaign": campaign(),
                        "occurrence": occurrence_uuid(0),
                        "program": program(),
                        "expected_ag_work": expected_ag_work,
                        "residuals": [],
                        "budget": {"retry_limit":1,"retries_used":0,"probe_limit":1,"probes_used":0,"escalation_limit":1,"escalations_used":0}
                    }),
                },
                proposal_input: serde_json::json!({
                    "observation": observation_id,
                    "proposal": {
                        "schema": PROPOSAL_SCHEMA,
                        "campaign": campaign(),
                        "subject": SUBJECT_DIGEST,
                        "scope": policy.subject.scope.digest,
                        "work_schema": WORK_SCHEMA,
                        "work": expected_ag_work,
                        "repair": null
                    },
                    "class":"initial"
                }),
            }
        }),
    }
    .seal()
    .unwrap()
}

fn run_cycle(store: &mut CanonicalStore, request: CanonicalCycleRequestV1) -> CycleRunOutcomeV1 {
    let mut support = SupportPort;
    let mut ag = FakeAg;
    CanonicalRuntime::new(store, &mut support, &mut ag)
        .run_cycle(request)
        .unwrap()
}

// --- AG-side document builders ---

fn genesis_json(expected_ag_work: &str) -> serde_json::Value {
    serde_json::json!({
        "campaign": campaign(),
        "occurrence": occurrence_uuid(0),
        "program": program(),
        "expected_ag_work": expected_ag_work,
        "residuals": [],
        "budget": {"retry_limit":1,"retries_used":0,"probe_limit":1,"probes_used":0,"escalation_limit":1,"escalations_used":0}
    })
}

fn proposal_input_json(observation_id: &str, scope: &str, work: &str) -> serde_json::Value {
    serde_json::json!({
        "observation": observation_id,
        "proposal": {
            "schema": PROPOSAL_SCHEMA,
            "campaign": campaign(),
            "subject": SUBJECT_DIGEST,
            "scope": scope,
            "work_schema": WORK_SCHEMA,
            "work": work,
            "repair": null
        },
        "class": "initial"
    })
}

fn catalog_json(scope: &str, required: &[&str]) -> serde_json::Value {
    serde_json::json!({
        "schema": CATALOG_SCHEMA,
        "entries": {
            WORK_SCHEMA: {
                "work_schema": WORK_SCHEMA,
                "subject": SUBJECT_DIGEST,
                "scope": scope,
                "precondition": {"required": required, "forbidden": []}
            }
        }
    })
}

fn mandate_json(scope: &str, generation: u64, status: &str, valid_until: u64) -> serde_json::Value {
    serde_json::json!({
        "subject": SUBJECT_DIGEST,
        "scope": scope,
        "generation": generation,
        "status": status,
        "valid_until_unix_ms": valid_until
    })
}

fn mandate_store_json(mut mandates: Vec<serde_json::Value>) -> serde_json::Value {
    mandates.sort_by_key(|mandate| mandate["generation"].as_u64().unwrap());
    serde_json::json!({
        "schema": MANDATE_STORE_SCHEMA,
        "mandates": mandates
    })
}

/// The content-derived mandate identity, recomputed from the mandate document
/// with the byte-exact mirror of AG's digest construction.
fn mandate_ref(mandate: &serde_json::Value) -> String {
    ag_digest_value(MANDATE_DIGEST_DOMAIN, mandate)
}

fn write_jcs(path: &Path, value: &serde_json::Value) {
    std::fs::write(path, serde_jcs::to_vec(value).unwrap()).unwrap();
}

fn write_wrapper(path: &Path, body: &str) {
    std::fs::write(path, body).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
}

// --- External binaries ---

struct Bins {
    loopctl: PathBuf,
    standing_resolver: PathBuf,
    docket: PathBuf,
    effectd: PathBuf,
}

fn bins() -> Bins {
    let bins = Bins {
        loopctl: PathBuf::from(std::env::var_os("AG_LOOPCTL_BIN").expect("AG_LOOPCTL_BIN")),
        standing_resolver: PathBuf::from(
            std::env::var_os("AG_STANDING_RESOLVER_BIN").expect("AG_STANDING_RESOLVER_BIN"),
        ),
        docket: PathBuf::from(std::env::var_os("AG_DOCKET_BIN").expect("AG_DOCKET_BIN")),
        effectd: PathBuf::from(std::env::var_os("AG_EFFECTD_BIN").expect("AG_EFFECTD_BIN")),
    };
    assert!(bins.loopctl.is_absolute());
    assert!(bins.standing_resolver.is_absolute());
    assert!(bins.docket.is_absolute());
    assert!(bins.effectd.is_absolute());
    bins
}

fn observation_wrapper(root: &Path, ns_store: &Path) -> PathBuf {
    let wrapper = root.join("observation-resolver.sh");
    write_wrapper(
        &wrapper,
        &format!(
            "#!/bin/sh\nexec \"{}\" --store \"{}\" --resolver-id \"{}\" --default-ttl-ms {}\n",
            env!("CARGO_BIN_EXE_nightshift-observation-resolver"),
            ns_store.display(),
            OBSERVATION_RESOLVER_ID,
            OBSERVATION_TTL_MS,
        ),
    );
    wrapper
}

fn standing_wrapper(bins: &Bins, root: &Path, mandate_store: &Path) -> PathBuf {
    let wrapper = root.join("standing-resolver.sh");
    write_wrapper(
        &wrapper,
        &format!(
            "#!/bin/sh\nexec \"{}\" --mandate-store \"{}\" --resolver-id \"{}\" --answer-ttl-ms {}\n",
            bins.standing_resolver.display(),
            mandate_store.display(),
            STANDING_RESOLVER_ID,
            STANDING_TTL_MS,
        ),
    );
    wrapper
}

/// One direct probe of the real standing-resolver binary, used to derive
/// expected provenance from the production component rather than from test
/// constants.
fn probe_standing_mandate_ref(
    bins: &Bins,
    mandate_store: &Path,
    scope: &str,
    proposal_ref: &str,
) -> String {
    let request = serde_json::json!({
        "schema": "ag.governed-loop.standing-request/v1",
        "key": {"campaign": campaign(), "occurrence": occurrence_uuid(0)},
        "observation": digest('d'),
        "proposal": proposal_ref,
        "subject": SUBJECT_DIGEST,
        "scope": scope,
        "now_unix_ms": wall_now_ms()
    });
    let mut child = Command::new(&bins.standing_resolver)
        .arg("--mandate-store")
        .arg(mandate_store)
        .arg("--resolver-id")
        .arg(STANDING_RESOLVER_ID)
        .arg("--answer-ttl-ms")
        .arg(STANDING_TTL_MS.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(serde_json::to_string(&request).unwrap().as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "standing probe failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    body["mandate"].as_str().unwrap().to_owned()
}

fn loopctl(bins: &Bins, args: &[String]) -> Output {
    Command::new(&bins.loopctl)
        .args(args)
        .output()
        .expect("spawn ag-loopctl")
}

fn loopctl_ok(bins: &Bins, args: &[String]) -> serde_json::Value {
    let output = loopctl(bins, args);
    assert!(
        output.status.success(),
        "ag-loopctl {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("ag-loopctl stdout is one JSON document")
}

fn loopctl_fail(bins: &Bins, args: &[String]) -> Output {
    let output = loopctl(bins, args);
    assert!(
        !output.status.success(),
        "ag-loopctl {args:?} unexpectedly succeeded: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    output
}

fn str_args(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| (*part).to_owned()).collect()
}

fn gate_args(catalog: &Path, observation: &Path, standing: &Path) -> Vec<String> {
    vec![
        "--catalog".to_owned(),
        catalog.display().to_string(),
        "--observation-resolver".to_owned(),
        observation.display().to_string(),
        "--expected-observation-resolver-id".to_owned(),
        OBSERVATION_RESOLVER_ID.to_owned(),
        "--standing-resolver".to_owned(),
        standing.display().to_string(),
        "--expected-standing-resolver-id".to_owned(),
        STANDING_RESOLVER_ID.to_owned(),
        "--max-standing-ttl-ms".to_owned(),
        STANDING_TTL_MS.to_string(),
    ]
}

fn docket_args(
    root: &Path,
    trust: &Path,
    docket_standing: &Path,
    plan: &Path,
    issuer_key: &Path,
    bins: &Bins,
) -> Vec<String> {
    vec![
        "--docket".to_owned(),
        bins.docket.display().to_string(),
        "--docket-state".to_owned(),
        root.join("docket-state").display().to_string(),
        "--docket-trust".to_owned(),
        trust.display().to_string(),
        "--docket-standing-resolver".to_owned(),
        docket_standing.display().to_string(),
        "--executor".to_owned(),
        bins.effectd.display().to_string(),
        "--executor-config".to_owned(),
        plan.display().to_string(),
        "--issuer-principal".to_owned(),
        ISSUER_PRINCIPAL.to_owned(),
        "--issuer-key-id".to_owned(),
        ISSUER_KEY_ID.to_owned(),
        "--issuer-key".to_owned(),
        issuer_key.display().to_string(),
    ]
}

/// The program counter of one snapshot: the single key of the externally
/// tagged state sum.
fn program_counter(snapshot: &serde_json::Value) -> String {
    let state = snapshot["state"].as_object().unwrap();
    assert_eq!(state.len(), 1);
    state.keys().next().unwrap().clone()
}

fn replay(bins: &Bins, database: &Path) -> serde_json::Value {
    loopctl_ok(
        bins,
        &str_args(&["replay", "--database", &database.display().to_string()]),
    )
}

fn status(bins: &Bins, database: &Path) -> serde_json::Value {
    loopctl_ok(
        bins,
        &str_args(&["status", "--database", &database.display().to_string()]),
    )
}

struct DocketRig {
    trust: PathBuf,
    issuer_key: PathBuf,
    target: PathBuf,
    plan: PathBuf,
    plan_value: serde_json::Value,
    plan_identity: String,
}

/// The real Docket custody rig: trust config naming the embedded test issuer,
/// the issuer's pkcs8 credential, and the sealed executor plan whose identity
/// is the proposal's exact work digest.
fn docket_rig(root: &Path, scope: &str) -> DocketRig {
    let (uid, gid) = current_uid_gid();
    let artifact = root.join("artifact");
    let target = root.join("target");
    std::fs::write(&artifact, b"wo9-governed-effect\n").unwrap();
    let content = sha256_digest(b"wo9-governed-effect\n");
    let plan = serde_json::json!({
        "schema": PLAN_SCHEMA,
        "attempt_store": root.join("effect-attempts.sqlite").display().to_string(),
        "subject": SUBJECT_DIGEST,
        "scope": scope,
        "effect_index": 0,
        "effect": {
            "kind": "managed_file_put",
            "target": "wo9-integration",
            "path": target.display().to_string(),
            "expected_content": null,
            "content": content,
            "mode": 0o600,
            "uid": uid,
            "gid": gid
        },
        "artifacts": [{"digest": content, "path": artifact.display().to_string()}],
        "file_policy": {
            "max_content_bytes": 1024,
            "trusted_ancestor_uid": std::fs::metadata("/").unwrap().uid(),
            "trusted_parent_uid": uid,
            "require_private_parent_writes": true
        },
        "preparation_checkpoint": null
    });
    let plan_identity = ag_digest_value(PLAN_DIGEST_DOMAIN, &plan);
    let plan_path = root.join("effect-plan.json");
    write_jcs(&plan_path, &plan);

    let issuer_key = root.join("issuer.pk8");
    let key_bytes: Vec<u8> = (0..ISSUER_PKCS8_HEX.len())
        .step_by(2)
        .map(|offset| u8::from_str_radix(&ISSUER_PKCS8_HEX[offset..offset + 2], 16).unwrap())
        .collect();
    std::fs::write(&issuer_key, key_bytes).unwrap();
    std::fs::set_permissions(&issuer_key, std::fs::Permissions::from_mode(0o600)).unwrap();
    let trust = root.join("docket-trust.json");
    write_jcs(
        &trust,
        &serde_json::json!({"issuers":[{
            "issuer_principal": ISSUER_PRINCIPAL,
            "key_id": ISSUER_KEY_ID,
            "public_key": ISSUER_PUBLIC_KEY_B64URL
        }]}),
    );
    DocketRig {
        trust,
        issuer_key,
        target,
        plan: plan_path,
        plan_value: plan,
        plan_identity,
    }
}

/// The controlled Docket execution-standing fixture (unchanged from the
/// existing governed-Docket harness; Docket's own production standing
/// authority is out of scope for WO-9).
fn docket_standing_script(root: &Path, status: &str) -> PathBuf {
    let script = root.join(format!("docket-standing-{status}.py"));
    write_wrapper(
        &script,
        &format!(
            r#"#!/usr/bin/python3
import hashlib,json,sys
r=json.load(sys.stdin); i=r["issuance"]
def d(label): return "sha256:"+hashlib.sha256(label.encode()).hexdigest()
o={{"schema":"docket.governed-loop.execution-standing-resolution/v1","resolution":d("resolution"),"currentness":d("currentness"),"execution_standing":d("execution-standing"),"issuance":i["issuance"],"campaign":i["key"]["campaign"],"occurrence":i["key"]["occurrence"],"subject":i["subject"],"scope":i["scope"],"status":"{status}","resolved_at_unix_ms":r["now_unix_ms"],"expires_at_unix_ms":r["now_unix_ms"]+60000}}
sys.stdout.write(json.dumps(o,sort_keys=True,separators=(",",":")))
"#
        ),
    );
    script
}

/// A minimal exact executor-plan document for store-only fixture tests: any
/// exact object has a deterministic AG executable-work identity.
fn fixture_plan() -> serde_json::Value {
    serde_json::json!({
        "schema": "ag-effectd.docket-executor-plan/v1",
        "fixture": "ag-governed-integration"
    })
}

/// Builds a real Nightshift store containing the given cycles through the
/// real canonical runtime. Returns the opened database path.
fn build_store(
    root: &Path,
    condition_present: bool,
    cycles: &[(u64, char, bool)],
    plan: &serde_json::Value,
) -> (PathBuf, String) {
    let (policy, clean_inputs, clean_recurrence) = example_policy_inputs_recurrence();
    let (inputs, recurrence) = if condition_present {
        condition_present_inputs_recurrence()
    } else {
        (clean_inputs, clean_recurrence)
    };
    let scope = policy.subject.scope.digest.clone();
    let database = root.join("ns.sqlite");
    let mut store = CanonicalStore::open(&database).unwrap();
    for &(occurrence, observation, with_proposal) in cycles {
        run_cycle(
            &mut store,
            cycle_request(
                &policy,
                &inputs,
                &recurrence,
                occurrence,
                &digest(observation),
                with_proposal,
                plan,
            ),
        );
    }
    drop(store);
    (database, scope)
}

/// Initializes one AG campaign database and records the proposal through the
/// real observation-resolver subprocess. Returns the proposal-recorded
/// snapshot.
fn init_and_record_proposal(
    bins: &Bins,
    root: &Path,
    ag_database: &Path,
    observation_wrapper_path: &Path,
    scope: &str,
    work: &str,
) -> serde_json::Value {
    let genesis = root.join(format!(
        "genesis-{}.json",
        ag_database.file_name().unwrap().to_string_lossy()
    ));
    write_jcs(&genesis, &genesis_json(work));
    loopctl_ok(
        bins,
        &str_args(&[
            "init",
            "--database",
            &ag_database.display().to_string(),
            "--genesis",
            &genesis.display().to_string(),
        ]),
    );
    let proposal_input = root.join("proposal-input.json");
    write_jcs(
        &proposal_input,
        &proposal_input_json(&digest('d'), scope, work),
    );
    let recorded = loopctl_ok(
        bins,
        &str_args(&[
            "record-proposal",
            "--database",
            &ag_database.display().to_string(),
            "--input",
            &proposal_input.display().to_string(),
            "--observation-resolver",
            &observation_wrapper_path.display().to_string(),
            "--expected-observation-resolver-id",
            OBSERVATION_RESOLVER_ID,
        ]),
    );
    assert_eq!(program_counter(&recorded), "proposal_recorded");
    recorded
}

fn require_standing(bins: &Bins, ag_database: &Path) {
    let snapshot = loopctl_ok(
        bins,
        &str_args(&[
            "require-standing",
            "--database",
            &ag_database.display().to_string(),
        ]),
    );
    assert_eq!(program_counter(&snapshot), "standing_required");
}

// --- Tests ---

/// Always-on fixture pin: the reseal algorithm reproduces the checked-in
/// specimen identity, and the mutated specimen drives the real runtime to a
/// genuine `ConditionPresent` posture. No external binaries.
#[test]
fn condition_present_fixture_is_real_and_resealed() {
    let mut unmodified: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/operator/examples/diagnostic-posture-v1/inputs.json"
    ))
    .unwrap();
    let artifact = &mut unmodified["inputs"][0]["artifact"];
    let checked_in = artifact["artifact_id"].as_str().unwrap().to_owned();
    let mut preimage = artifact.clone();
    preimage.as_object_mut().unwrap().remove("artifact_id");
    assert_eq!(digest_value(&preimage), checked_in);

    let root = tempfile::tempdir().unwrap();
    let (database, _) = build_store(root.path(), true, &[(0, 'd', true)], &fixture_plan());
    let store = CanonicalStore::open(&database).unwrap();
    let family = store.find_cycles_by_observation_id(&digest('d')).unwrap();
    assert_eq!(family.len(), 1);
    let posture = &family[0].observation.as_ref().unwrap().posture;
    assert!(posture.current);
    assert_eq!(posture.condition, ConditionAxis::ConditionPresent);
    let basis = normalize_posture(posture);
    assert_eq!(
        basis.atoms,
        std::collections::BTreeSet::from([
            "condition.condition_present".to_owned(),
            "delivery.not_required".to_owned(),
        ])
    );
}

/// Scenario A: the full healthy chain, with the complete provenance
/// reconstruction of section 16/17.
#[test]
#[ignore = "requires adjacent AG and Docket binaries; see module documentation"]
fn healthy_chain_reaches_docket_and_executes_exactly_once() {
    let bins = bins();
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();

    // The plan identity is the proposal's exact work, so the rig comes first.
    let (policy, _, _) = example_policy_inputs_recurrence();
    let scope = policy.subject.scope.digest.clone();
    let rig = docket_rig(root.path(), &scope);
    let (ns_database, _) = build_store(root.path(), false, &[(0, 'd', true)], &rig.plan_value);

    let observation = observation_wrapper(root.path(), &ns_database);
    let mandate_store = root.path().join("mandates.json");
    let mandate = mandate_json(&scope, 1, "active", wall_now_ms() + 3_600_000);
    write_jcs(&mandate_store, &mandate_store_json(vec![mandate.clone()]));
    let standing = standing_wrapper(&bins, root.path(), &mandate_store);

    let ag_database = root.path().join("ag.sqlite");
    let recorded = init_and_record_proposal(
        &bins,
        root.path(),
        &ag_database,
        &observation,
        &scope,
        &rig.plan_identity,
    );
    // The recorded observation resolution is the real resolver's answer over
    // the real persisted record: current, clean, and digest-pinned.
    let recorded_observation = &recorded["state"]["proposal_recorded"]["observation"];
    assert_eq!(recorded_observation["status"], "current");
    assert_eq!(recorded_observation["resolver_id"], OBSERVATION_RESOLVER_ID);
    assert_eq!(recorded_observation["observation"], digest('d'));
    assert_eq!(
        recorded_observation["normalized_preconditions"],
        "sha256:d67f86277b1604cad1916d01bcd5e01fc3a9002d4630cb8fdf5b749febf4b2c7",
        "the clean specimen basis is the frozen WO-3 cross-repository vector"
    );
    let proposal_ref = recorded["state"]["proposal_recorded"]["proposal_ref"]
        .as_str()
        .unwrap()
        .to_owned();

    require_standing(&bins, &ag_database);
    let catalog = root.path().join("catalog.json");
    let rollout = catalog_json(&scope, &["condition.clean"]);
    write_jcs(&catalog, &rollout);
    let gate = gate_args(&catalog, &observation, &standing);
    let mut decide_args = str_args(&["decide", "--database", &ag_database.display().to_string()]);
    decide_args.extend(gate.clone());
    let decided = loopctl_ok(&bins, &decide_args);
    assert_eq!(
        program_counter(&decided),
        "admissible_pending_authorization"
    );
    // The recorded policy identity is recomputed here from the exact catalog
    // document, not taken on trust.
    let expected_policy_basis = ag_digest_value(CATALOG_DIGEST_DOMAIN, &rollout);
    assert_eq!(
        decided["state"]["admissible_pending_authorization"]["decision"]["policy_basis"],
        serde_json::Value::String(expected_policy_basis.clone())
    );

    let mut authorize_args = str_args(&[
        "authorize",
        "--database",
        &ag_database.display().to_string(),
    ]);
    authorize_args.extend(gate);
    let authorized = loopctl_ok(&bins, &authorize_args);
    assert_eq!(program_counter(&authorized), "authorization_consumed");

    // Section 16/17 provenance reconstruction from the persisted authority
    // state.
    let state = &authorized["state"]["authorization_consumed"];
    let admitted = &state["admitted"];
    let observation_resolution = &admitted["proposal"]["observation"];
    assert_eq!(observation_resolution["observation"], digest('d'));
    assert_eq!(
        observation_resolution["resolver_id"],
        OBSERVATION_RESOLVER_ID
    );
    assert_eq!(observation_resolution["status"], "current");
    assert_eq!(
        observation_resolution["basis"]["rule"]["id"],
        "nightshift.posture-normalization"
    );
    // The pinned basis digest is recomputed from the persisted Nightshift
    // record through Nightshift's own normalization.
    let store = CanonicalStore::open(&ns_database).unwrap();
    let cycles = store.find_cycles_by_observation_id(&digest('d')).unwrap();
    let posture = &cycles[0].observation.as_ref().unwrap().posture;
    let expected_basis = normalize_posture(posture);
    assert_eq!(
        observation_resolution["normalized_preconditions"]
            .as_str()
            .unwrap(),
        expected_basis.digest().unwrap()
    );
    // The sealed cross-domain work binding: the persisted intent carries the
    // Nightshift-domain compiled-payload identity and the AG-domain
    // executable-work identity derived from the actual executor plan, and the
    // persisted prepared request proposes exactly that AG work.
    let intent = cycles[0].intent.as_ref().unwrap();
    assert_eq!(intent.expected_ag_work, rig.plan_identity);
    assert_eq!(
        intent.compiled_work,
        digest_value(&serde_json::json!({
            "parameters": {"resource_id": "resource-1"},
            "schema": WORK_SCHEMA,
        }))
    );
    assert_ne!(intent.compiled_work, intent.expected_ag_work);
    let prepared = cycles[0].prepared_ag_request.as_ref().unwrap();
    assert_eq!(
        prepared.exact_request["proposal_input"]["proposal"]["work"]
            .as_str()
            .unwrap(),
        rig.plan_identity
    );
    drop(store);

    let standing_resolution = &admitted["standing"];
    assert_eq!(standing_resolution["resolver_id"], STANDING_RESOLVER_ID);
    assert_eq!(standing_resolution["status"], "current");
    let window = standing_resolution["expires_at_unix_ms"].as_u64().unwrap()
        - standing_resolution["resolved_at_unix_ms"].as_u64().unwrap();
    assert!(window <= STANDING_TTL_MS);
    // The mandate identity recorded at spend is the content-derived identity
    // of the exact mandate document, confirmed independently by a direct
    // probe of the production resolver binary.
    let expected_mandate = mandate_ref(&mandate);
    let probed_mandate = probe_standing_mandate_ref(&bins, &mandate_store, &scope, &proposal_ref);
    assert_eq!(expected_mandate, probed_mandate);
    assert_eq!(
        standing_resolution["mandate"],
        serde_json::Value::String(expected_mandate.clone())
    );

    let spend = &state["spend"];
    let issuance = &state["issuance"];
    assert_eq!(issuance["spend"], spend["spend"]);
    assert_eq!(issuance["observation"], digest('d'));
    assert_eq!(issuance["work"], rig.plan_identity);
    assert_eq!(issuance["work_schema"], WORK_SCHEMA);
    assert_eq!(issuance["subject"], SUBJECT_DIGEST);
    assert_eq!(issuance["scope"], scope);
    assert_eq!(
        issuance["mandate"],
        serde_json::Value::String(expected_mandate)
    );
    assert_eq!(
        issuance["standing_resolution"],
        standing_resolution["resolution"]
    );
    assert_eq!(
        spend["admission_decision"],
        admitted["decision"]["decision"]
    );
    assert_eq!(admitted["decision"]["disposition"], "admitted");
    assert_eq!(
        admitted["decision"]["policy_basis"],
        serde_json::Value::String(expected_policy_basis)
    );
    let report = replay(&bins, &ag_database);
    assert_eq!(report["ag_spends"], 1);

    // Docket custody and the safe executor: exactly one effect.
    let docket_standing = docket_standing_script(root.path(), "current");
    let docket = docket_args(
        root.path(),
        &rig.trust,
        &docket_standing,
        &rig.plan,
        &rig.issuer_key,
        &bins,
    );
    let mut dispatch_args =
        str_args(&["dispatch", "--database", &ag_database.display().to_string()]);
    dispatch_args.extend(docket.clone());
    let dispatched = loopctl_ok(&bins, &dispatch_args);
    assert_eq!(program_counter(&dispatched), "dispatched");
    let mut poll_args = str_args(&["poll", "--database", &ag_database.display().to_string()]);
    poll_args.extend(docket.clone());
    let settled = loopctl_ok(&bins, &poll_args);
    assert_eq!(program_counter(&settled), "settled_observation_required");
    assert_eq!(
        std::fs::read(&rig.target).unwrap(),
        b"wo9-governed-effect\n"
    );
    let report = replay(&bins, &ag_database);
    assert_eq!(report["ag_spends"], 1);
    assert_eq!(report["docket_attempts"], 1);
    assert_eq!(report["settlements"], 1);

    // The executor does not re-run on a repeated poll.
    std::fs::write(&rig.target, b"must-not-run-again\n").unwrap();
    let repolled = loopctl_ok(&bins, &poll_args);
    assert_eq!(program_counter(&repolled), "settled_observation_required");
    assert_eq!(std::fs::read(&rig.target).unwrap(), b"must-not-run-again\n");
}

/// Scenarios B and C over literally the same persisted Nightshift
/// observation: the rollout policy refuses the real condition-present basis
/// and the remediation policy admits it. Two independent AG campaign
/// databases read one Nightshift store.
#[test]
#[ignore = "requires adjacent AG and Docket binaries; see module documentation"]
fn rollout_refuses_and_remediation_admits_identical_condition_present_evidence() {
    let bins = bins();
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();

    let (policy, _, _) = example_policy_inputs_recurrence();
    let scope = policy.subject.scope.digest.clone();
    let rig = docket_rig(root.path(), &scope);
    let (ns_database, _) = build_store(root.path(), true, &[(0, 'd', true)], &rig.plan_value);
    let observation = observation_wrapper(root.path(), &ns_database);
    let mandate_store = root.path().join("mandates.json");
    write_jcs(
        &mandate_store,
        &mandate_store_json(vec![mandate_json(
            &scope,
            1,
            "active",
            wall_now_ms() + 3_600_000,
        )]),
    );
    let standing = standing_wrapper(&bins, root.path(), &mandate_store);

    // The honest basis of the persisted record, computed Nightshift-side.
    let store = CanonicalStore::open(&ns_database).unwrap();
    let cycles = store.find_cycles_by_observation_id(&digest('d')).unwrap();
    let posture = &cycles[0].observation.as_ref().unwrap().posture;
    let expected_basis = normalize_posture(posture);
    assert_eq!(posture.condition, ConditionAxis::ConditionPresent);
    let expected_basis_digest = expected_basis.digest().unwrap();
    drop(store);

    // B: the rollout policy (`required = {condition.clean}`) refuses this
    // evidence. The failure is catalog policy refusal, not evidence-health
    // failure: the observation resolved Current at record time.
    let rollout_database = root.path().join("ag-rollout.sqlite");
    let recorded = init_and_record_proposal(
        &bins,
        root.path(),
        &rollout_database,
        &observation,
        &scope,
        &rig.plan_identity,
    );
    let recorded_observation = &recorded["state"]["proposal_recorded"]["observation"];
    assert_eq!(recorded_observation["status"], "current");
    assert_eq!(
        recorded_observation["normalized_preconditions"]
            .as_str()
            .unwrap(),
        expected_basis_digest
    );
    require_standing(&bins, &rollout_database);
    let rollout_catalog = root.path().join("catalog-rollout.json");
    let rollout = catalog_json(&scope, &["condition.clean"]);
    write_jcs(&rollout_catalog, &rollout);
    let mut decide_args = str_args(&[
        "decide",
        "--database",
        &rollout_database.display().to_string(),
    ]);
    decide_args.extend(gate_args(&rollout_catalog, &observation, &standing));
    let refusal = loopctl_fail(&bins, &decide_args);
    assert!(
        String::from_utf8_lossy(&refusal.stderr).contains("not admissible"),
        "expected an inadmissibility refusal: {}",
        String::from_utf8_lossy(&refusal.stderr)
    );
    // State is preserved pre-decision; no spend, no issuance.
    let snapshot = status(&bins, &rollout_database);
    assert_eq!(program_counter(&snapshot), "standing_required");
    let report = replay(&bins, &rollout_database);
    assert_eq!(report["ag_spends"], 0);

    // C: the remediation policy (`required = {condition.condition_present}`)
    // admits exactly the same persisted evidence through a second independent
    // campaign. There is no universal Clean rule anywhere in the chain.
    let remediation_database = root.path().join("ag-remediation.sqlite");
    init_and_record_proposal(
        &bins,
        root.path(),
        &remediation_database,
        &observation,
        &scope,
        &rig.plan_identity,
    );
    require_standing(&bins, &remediation_database);
    let remediation_catalog = root.path().join("catalog-remediation.json");
    let remediation = catalog_json(&scope, &["condition.condition_present"]);
    write_jcs(&remediation_catalog, &remediation);
    let gate = gate_args(&remediation_catalog, &observation, &standing);
    let mut decide_args = str_args(&[
        "decide",
        "--database",
        &remediation_database.display().to_string(),
    ]);
    decide_args.extend(gate.clone());
    let decided = loopctl_ok(&bins, &decide_args);
    assert_eq!(
        program_counter(&decided),
        "admissible_pending_authorization"
    );
    // Distinct catalog policies are distinct content-derived identities.
    assert_ne!(
        ag_digest_value(CATALOG_DIGEST_DOMAIN, &rollout),
        ag_digest_value(CATALOG_DIGEST_DOMAIN, &remediation)
    );
    assert_eq!(
        decided["state"]["admissible_pending_authorization"]["decision"]["policy_basis"]
            .as_str()
            .unwrap(),
        ag_digest_value(CATALOG_DIGEST_DOMAIN, &remediation)
    );

    let mut authorize_args = str_args(&[
        "authorize",
        "--database",
        &remediation_database.display().to_string(),
    ]);
    authorize_args.extend(gate);
    let authorized = loopctl_ok(&bins, &authorize_args);
    assert_eq!(program_counter(&authorized), "authorization_consumed");
    let state = &authorized["state"]["authorization_consumed"];
    assert_eq!(
        state["admitted"]["proposal"]["observation"]["normalized_preconditions"]
            .as_str()
            .unwrap(),
        expected_basis_digest
    );
    let report = replay(&bins, &remediation_database);
    assert_eq!(report["ag_spends"], 1);

    // The admitted remediation work crosses Docket and executes once.
    let docket_standing = docket_standing_script(root.path(), "current");
    let docket = docket_args(
        root.path(),
        &rig.trust,
        &docket_standing,
        &rig.plan,
        &rig.issuer_key,
        &bins,
    );
    let mut dispatch_args = str_args(&[
        "dispatch",
        "--database",
        &remediation_database.display().to_string(),
    ]);
    dispatch_args.extend(docket.clone());
    loopctl_ok(&bins, &dispatch_args);
    let mut poll_args = str_args(&[
        "poll",
        "--database",
        &remediation_database.display().to_string(),
    ]);
    poll_args.extend(docket);
    let settled = loopctl_ok(&bins, &poll_args);
    assert_eq!(program_counter(&settled), "settled_observation_required");
    assert_eq!(
        std::fs::read(&rig.target).unwrap(),
        b"wo9-governed-effect\n"
    );
}

/// Scenario D: a strictly later qualified same-family Nightshift observation
/// supersedes the cited evidence between decide and authorize. The old
/// proposal is not refreshed onto the new evidence; no spend occurs.
#[test]
#[ignore = "requires adjacent AG and Docket binaries; see module documentation"]
fn newer_same_family_evidence_supersedes_before_spend() {
    let bins = bins();
    let root = tempfile::tempdir().unwrap();

    let (policy, _, _) = example_policy_inputs_recurrence();
    let scope = policy.subject.scope.digest.clone();
    let rig = docket_rig(root.path(), &scope);
    let (ns_database, _) = build_store(root.path(), false, &[(0, 'd', true)], &rig.plan_value);
    let observation = observation_wrapper(root.path(), &ns_database);
    let mandate_store = root.path().join("mandates.json");
    write_jcs(
        &mandate_store,
        &mandate_store_json(vec![mandate_json(
            &scope,
            1,
            "active",
            wall_now_ms() + 3_600_000,
        )]),
    );
    let standing = standing_wrapper(&bins, root.path(), &mandate_store);

    let ag_database = root.path().join("ag.sqlite");
    init_and_record_proposal(
        &bins,
        root.path(),
        &ag_database,
        &observation,
        &scope,
        &rig.plan_identity,
    );
    require_standing(&bins, &ag_database);
    let catalog_path = root.path().join("catalog.json");
    write_jcs(&catalog_path, &catalog_json(&scope, &["condition.clean"]));
    let gate = gate_args(&catalog_path, &observation, &standing);
    let mut decide_args = str_args(&["decide", "--database", &ag_database.display().to_string()]);
    decide_args.extend(gate.clone());
    let decided = loopctl_ok(&bins, &decide_args);
    assert_eq!(
        program_counter(&decided),
        "admissible_pending_authorization"
    );

    // A later logical slot in the same family observes successfully. The
    // pinned proposal still cites the older observation.
    let (_, inputs, recurrence) = example_policy_inputs_recurrence();
    let mut store = CanonicalStore::open(&ns_database).unwrap();
    let outcome = run_cycle(
        &mut store,
        cycle_request(
            &policy,
            &inputs,
            &recurrence,
            1,
            &digest('e'),
            false,
            &rig.plan_value,
        ),
    );
    assert!(matches!(outcome, CycleRunOutcomeV1::PostureOnly { .. }));
    drop(store);

    // The real resolver now classifies the cited observation as superseded
    // (direct probe of the production binary).
    let mut child = Command::new(env!("CARGO_BIN_EXE_nightshift-observation-resolver"))
        .arg("--store")
        .arg(&ns_database)
        .arg("--resolver-id")
        .arg(OBSERVATION_RESOLVER_ID)
        .arg("--default-ttl-ms")
        .arg(OBSERVATION_TTL_MS.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let probe = serde_json::json!({
        "schema": "ag.governed-loop.observation-request/v1",
        "key": {"campaign": campaign(), "occurrence": occurrence_uuid(0)},
        "observation": digest('d'),
        "subject": SUBJECT_DIGEST,
        "now_unix_ms": wall_now_ms()
    });
    child
        .stdin
        .take()
        .unwrap()
        .write_all(probe.to_string().as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let body: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["status"], "superseded");

    // Authorization refuses: the pinned basis is not refreshed onto O2.
    let mut authorize_args = str_args(&[
        "authorize",
        "--database",
        &ag_database.display().to_string(),
    ]);
    authorize_args.extend(gate);
    loopctl_fail(&bins, &authorize_args);
    let snapshot = status(&bins, &ag_database);
    assert_eq!(
        program_counter(&snapshot),
        "admissible_pending_authorization"
    );
    let report = replay(&bins, &ag_database);
    assert_eq!(report["ag_spends"], 0);
}

/// Scenarios E and F: standing is revoked between decide and authorize (no
/// spend), then recovers under a newer mandate generation; the same proposal
/// authorizes without new evidence, and the spend names the new mandate.
#[test]
#[ignore = "requires adjacent AG and Docket binaries; see module documentation"]
fn standing_revocation_and_recovery_across_authorize() {
    let bins = bins();
    let root = tempfile::tempdir().unwrap();

    let (policy, _, _) = example_policy_inputs_recurrence();
    let scope = policy.subject.scope.digest.clone();
    let rig = docket_rig(root.path(), &scope);
    let (ns_database, _) = build_store(root.path(), false, &[(0, 'd', true)], &rig.plan_value);
    let observation = observation_wrapper(root.path(), &ns_database);
    let mandate_store = root.path().join("mandates.json");
    let validity = wall_now_ms() + 3_600_000;
    let generation_one = mandate_json(&scope, 1, "active", validity);
    write_jcs(
        &mandate_store,
        &mandate_store_json(vec![generation_one.clone()]),
    );
    let standing = standing_wrapper(&bins, root.path(), &mandate_store);

    let ag_database = root.path().join("ag.sqlite");
    let recorded = init_and_record_proposal(
        &bins,
        root.path(),
        &ag_database,
        &observation,
        &scope,
        &rig.plan_identity,
    );
    let proposal_ref = recorded["state"]["proposal_recorded"]["proposal_ref"]
        .as_str()
        .unwrap()
        .to_owned();
    require_standing(&bins, &ag_database);
    let catalog_path = root.path().join("catalog.json");
    write_jcs(&catalog_path, &catalog_json(&scope, &["condition.clean"]));
    let gate = gate_args(&catalog_path, &observation, &standing);
    let mut decide_args = str_args(&["decide", "--database", &ag_database.display().to_string()]);
    decide_args.extend(gate.clone());
    let decided = loopctl_ok(&bins, &decide_args);
    assert_eq!(
        program_counter(&decided),
        "admissible_pending_authorization"
    );

    // E: governance revokes by superseding generation 1 with a revoked
    // generation 2. The read-only resolver loads the store fresh.
    let generation_two = mandate_json(&scope, 2, "revoked", validity);
    write_jcs(
        &mandate_store,
        &mandate_store_json(vec![generation_one.clone(), generation_two.clone()]),
    );
    let mut authorize_args = str_args(&[
        "authorize",
        "--database",
        &ag_database.display().to_string(),
    ]);
    authorize_args.extend(gate.clone());
    loopctl_fail(&bins, &authorize_args);
    let snapshot = status(&bins, &ag_database);
    assert_eq!(
        program_counter(&snapshot),
        "admissible_pending_authorization"
    );
    let report = replay(&bins, &ag_database);
    assert_eq!(report["ag_spends"], 0);

    // F: governance restores standing under generation 3. The same proposal
    // authorizes with no new observation, proposal, or occurrence.
    let generation_three = mandate_json(&scope, 3, "active", validity);
    write_jcs(
        &mandate_store,
        &mandate_store_json(vec![
            generation_one,
            generation_two,
            generation_three.clone(),
        ]),
    );
    let authorized = loopctl_ok(&bins, &authorize_args);
    assert_eq!(program_counter(&authorized), "authorization_consumed");
    let state = &authorized["state"]["authorization_consumed"];
    assert_eq!(
        state["issuance"]["proposal"].as_str().unwrap(),
        proposal_ref,
        "the spent proposal is the original one"
    );
    let expected_mandate = mandate_ref(&generation_three);
    let probed_mandate = probe_standing_mandate_ref(&bins, &mandate_store, &scope, &proposal_ref);
    assert_eq!(expected_mandate, probed_mandate);
    assert_eq!(
        state["issuance"]["mandate"],
        serde_json::Value::String(expected_mandate)
    );
    assert_eq!(
        state["admitted"]["standing"]["resolver_id"],
        STANDING_RESOLVER_ID
    );
    let report = replay(&bins, &ag_database);
    assert_eq!(report["ag_spends"], 1);
}

/// Scenario G: Docket execution standing refuses after the AG spend. The
/// spend remains historically real, no executor effect occurs, and a later
/// current answer lets the same issuance through exactly once.
#[test]
#[ignore = "requires adjacent AG and Docket binaries; see module documentation"]
fn docket_refusal_after_spend_prevents_effect() {
    let bins = bins();
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();

    let (policy, _, _) = example_policy_inputs_recurrence();
    let scope = policy.subject.scope.digest.clone();
    let rig = docket_rig(root.path(), &scope);
    let (ns_database, _) = build_store(root.path(), false, &[(0, 'd', true)], &rig.plan_value);
    let observation = observation_wrapper(root.path(), &ns_database);
    let mandate_store = root.path().join("mandates.json");
    write_jcs(
        &mandate_store,
        &mandate_store_json(vec![mandate_json(
            &scope,
            1,
            "active",
            wall_now_ms() + 3_600_000,
        )]),
    );
    let standing = standing_wrapper(&bins, root.path(), &mandate_store);

    let ag_database = root.path().join("ag.sqlite");
    init_and_record_proposal(
        &bins,
        root.path(),
        &ag_database,
        &observation,
        &scope,
        &rig.plan_identity,
    );
    require_standing(&bins, &ag_database);
    let catalog_path = root.path().join("catalog.json");
    write_jcs(&catalog_path, &catalog_json(&scope, &["condition.clean"]));
    let gate = gate_args(&catalog_path, &observation, &standing);
    let mut decide_args = str_args(&["decide", "--database", &ag_database.display().to_string()]);
    decide_args.extend(gate.clone());
    loopctl_ok(&bins, &decide_args);
    let mut authorize_args = str_args(&[
        "authorize",
        "--database",
        &ag_database.display().to_string(),
    ]);
    authorize_args.extend(gate);
    let authorized = loopctl_ok(&bins, &authorize_args);
    assert_eq!(program_counter(&authorized), "authorization_consumed");

    // Docket's execution-standing resolver says revoked: custody is refused
    // and the executor never runs, though the AG spend is durable history.
    let revoked = docket_standing_script(root.path(), "revoked");
    let docket = docket_args(
        root.path(),
        &rig.trust,
        &revoked,
        &rig.plan,
        &rig.issuer_key,
        &bins,
    );
    let mut dispatch_args =
        str_args(&["dispatch", "--database", &ag_database.display().to_string()]);
    dispatch_args.extend(docket.clone());
    loopctl_fail(&bins, &dispatch_args);
    assert!(!rig.target.exists(), "no executor effect on refusal");
    let report = replay(&bins, &ag_database);
    assert_eq!(
        report["ag_spends"], 1,
        "the spend remains historically real"
    );
    assert_eq!(report["docket_attempts"], 0);
    let snapshot = status(&bins, &ag_database);
    assert_eq!(program_counter(&snapshot), "authorization_consumed");

    // When execution standing is current again, the same issuance crosses
    // and executes exactly once.
    let current = docket_standing_script(root.path(), "current");
    let docket = docket_args(
        root.path(),
        &rig.trust,
        &current,
        &rig.plan,
        &rig.issuer_key,
        &bins,
    );
    let mut dispatch_args =
        str_args(&["dispatch", "--database", &ag_database.display().to_string()]);
    dispatch_args.extend(docket.clone());
    loopctl_ok(&bins, &dispatch_args);
    let mut poll_args = str_args(&["poll", "--database", &ag_database.display().to_string()]);
    poll_args.extend(docket);
    let settled = loopctl_ok(&bins, &poll_args);
    assert_eq!(program_counter(&settled), "settled_observation_required");
    assert_eq!(
        std::fs::read(&rig.target).unwrap(),
        b"wo9-governed-effect\n"
    );
    let report = replay(&bins, &ag_database);
    assert_eq!(report["ag_spends"], 1);
    assert_eq!(report["docket_attempts"], 1);
}

/// Scenario H: an AG refusal mints nothing, and Docket — even with
/// permissive execution standing — has no issuance to accept and produces no
/// effect. Docket is downstream custody, not an alternate authority source.
#[test]
#[ignore = "requires adjacent AG and Docket binaries; see module documentation"]
fn ag_refusal_cannot_be_resurrected_by_docket() {
    let bins = bins();
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();

    let (policy, _, _) = example_policy_inputs_recurrence();
    let scope = policy.subject.scope.digest.clone();
    let rig = docket_rig(root.path(), &scope);
    let (ns_database, _) = build_store(root.path(), true, &[(0, 'd', true)], &rig.plan_value);
    let observation = observation_wrapper(root.path(), &ns_database);
    let mandate_store = root.path().join("mandates.json");
    write_jcs(
        &mandate_store,
        &mandate_store_json(vec![mandate_json(
            &scope,
            1,
            "active",
            wall_now_ms() + 3_600_000,
        )]),
    );
    let standing = standing_wrapper(&bins, root.path(), &mandate_store);

    let ag_database = root.path().join("ag.sqlite");
    init_and_record_proposal(
        &bins,
        root.path(),
        &ag_database,
        &observation,
        &scope,
        &rig.plan_identity,
    );
    require_standing(&bins, &ag_database);
    let rollout_catalog = root.path().join("catalog-rollout.json");
    write_jcs(
        &rollout_catalog,
        &catalog_json(&scope, &["condition.clean"]),
    );
    let mut decide_args = str_args(&["decide", "--database", &ag_database.display().to_string()]);
    decide_args.extend(gate_args(&rollout_catalog, &observation, &standing));
    loopctl_fail(&bins, &decide_args);
    let report = replay(&bins, &ag_database);
    assert_eq!(report["ag_spends"], 0);

    // A permissive Docket cannot manufacture an execution: there is no
    // issuance to present, and dispatch from a pre-spend state fails.
    let current = docket_standing_script(root.path(), "current");
    let docket = docket_args(
        root.path(),
        &rig.trust,
        &current,
        &rig.plan,
        &rig.issuer_key,
        &bins,
    );
    let mut dispatch_args =
        str_args(&["dispatch", "--database", &ag_database.display().to_string()]);
    dispatch_args.extend(docket);
    loopctl_fail(&bins, &dispatch_args);
    assert!(!rig.target.exists());
    let report = replay(&bins, &ag_database);
    assert_eq!(report["ag_spends"], 0);
    assert_eq!(report["docket_attempts"], 0);
    assert_eq!(report["settlements"], 0);
}

/// The WO-9.1 attack test: Nightshift prepared plan P and AG's occurrence was
/// opened expecting P's identity. A caller submitting an otherwise valid
/// proposal naming foreign work is refused at record time; the bound work
/// still records informationally.
#[test]
#[ignore = "requires adjacent AG and Docket binaries; see module documentation"]
fn submitted_work_other_than_the_prepared_binding_is_rejected() {
    let bins = bins();
    let root = tempfile::tempdir().unwrap();

    let (policy, _, _) = example_policy_inputs_recurrence();
    let scope = policy.subject.scope.digest.clone();
    let rig = docket_rig(root.path(), &scope);
    let (ns_database, _) = build_store(root.path(), false, &[(0, 'd', true)], &rig.plan_value);
    let observation = observation_wrapper(root.path(), &ns_database);

    let ag_database = root.path().join("ag.sqlite");
    let genesis = root.path().join("genesis.json");
    write_jcs(&genesis, &genesis_json(&rig.plan_identity));
    loopctl_ok(
        &bins,
        &str_args(&[
            "init",
            "--database",
            &ag_database.display().to_string(),
            "--genesis",
            &genesis.display().to_string(),
        ]),
    );

    // Foreign work: a validly shaped proposal naming a digest the occurrence
    // was never opened to govern.
    let proposal_input = root.path().join("proposal-input.json");
    write_jcs(
        &proposal_input,
        &proposal_input_json(&digest('d'), &scope, &digest('9')),
    );
    let record_args = str_args(&[
        "record-proposal",
        "--database",
        &ag_database.display().to_string(),
        "--input",
        &proposal_input.display().to_string(),
        "--observation-resolver",
        &observation.display().to_string(),
        "--expected-observation-resolver-id",
        OBSERVATION_RESOLVER_ID,
    ]);
    let refusal = loopctl_fail(&bins, &record_args);
    assert!(
        String::from_utf8_lossy(&refusal.stderr).contains("binding mismatch"),
        "expected an exact-binding failure: {}",
        String::from_utf8_lossy(&refusal.stderr)
    );
    let snapshot = status(&bins, &ag_database);
    assert_eq!(program_counter(&snapshot), "observation_required");
    let report = replay(&bins, &ag_database);
    assert_eq!(report["ag_spends"], 0);

    // The exact prepared work records.
    write_jcs(
        &proposal_input,
        &proposal_input_json(&digest('d'), &scope, &rig.plan_identity),
    );
    let recorded = loopctl_ok(&bins, &record_args);
    assert_eq!(program_counter(&recorded), "proposal_recorded");
}
