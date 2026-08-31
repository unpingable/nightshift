use nightshift_foreman::{
    WorkerStartRequestV2, WorkerStartRequestV3, WORKER_START_REQUEST_SCHEMA_V2,
    WORKER_TERMINAL_RECEIPT_SCHEMA_V1,
};
use serde_json::{json, Value};
use sha2::{Digest as _, Sha256};

type V3Substitution = Box<dyn Fn(&mut WorkerStartRequestV3)>;

fn digest(fill: char) -> String {
    format!("sha256:{}", fill.to_string().repeat(64))
}

fn v2() -> WorkerStartRequestV2 {
    let mut request = WorkerStartRequestV2 {
        schema: WORKER_START_REQUEST_SCHEMA_V2.to_owned(),
        request_digest: digest('0'),
        adapter_id: "switchyard-codex".to_owned(),
        adapter_version: "2.0.0".to_owned(),
        adapter_protocol: "switchyard.codex-app-server/v2".to_owned(),
        packet_digest: digest('1'),
        run_id: "run-holding".to_owned(),
        work_item_id: "WORK-A".to_owned(),
        attempt_id: "attempt-holding-1".to_owned(),
        worker_brief_digest: digest('2'),
        workspace_identity: "workspace-holding".to_owned(),
        provider_model_class: "large".to_owned(),
        timeout_seconds: 600,
        maximum_output_bytes: 1024 * 1024,
        recursive_worker_swarms_forbidden: true,
        approval_policy: "SURFACE_ONLY_NO_RESPONSE".to_owned(),
        expected_receipt_schema: WORKER_TERMINAL_RECEIPT_SCHEMA_V1.to_owned(),
    };
    request.seal().unwrap();
    request
}

fn canonical<T: serde::Serialize>(value: &T) -> Vec<u8> {
    serde_jcs::to_vec(value).unwrap()
}

fn v3() -> WorkerStartRequestV3 {
    WorkerStartRequestV3::from_v2(
        &canonical(&v2()),
        digest('3'),
        "dispatch-holding-1",
        "openai",
        "gpt-5.6-sol",
        "large",
        0,
    )
    .unwrap()
}

#[test]
fn v3_retains_exact_v2_and_has_stable_independent_digest() {
    let request = v3();
    request.validate().unwrap();
    assert_eq!(request.predecessor_v2().unwrap(), v2());
    assert_eq!(request.work_attempt_id, request.attempt_id);
    assert_ne!(request.request_digest, request.predecessor_request_digest);
    assert_eq!(
        request.predecessor_sha256,
        format!("sha256:{:x}", Sha256::digest(canonical(&v2())))
    );
    let bytes = canonical(&request);
    assert_eq!(WorkerStartRequestV3::from_slice(&bytes).unwrap(), request);
    assert_eq!(
        request.request_digest,
        "sha256:3dbc6fb1190f2d8edc494750456b505db42c7460a2ab09a0232e7fb840afd57d"
    );
}

#[test]
fn v3_refuses_outer_predecessor_and_owner_pin_substitutions() {
    let base = v3();
    let substitutions: Vec<V3Substitution> = vec![
        Box::new(|value| value.packet_digest = digest('4')),
        Box::new(|value| value.run_id = "run-other".to_owned()),
        Box::new(|value| value.work_item_id = "WORK-B".to_owned()),
        Box::new(|value| value.attempt_id = "attempt-other".to_owned()),
        Box::new(|value| value.work_attempt_id = "attempt-other".to_owned()),
        Box::new(|value| value.adapter_id = "adapter-other".to_owned()),
        Box::new(|value| value.adapter_version = "9.0.0".to_owned()),
        Box::new(|value| value.adapter_protocol = "switchyard.other/v1".to_owned()),
        Box::new(|value| value.worker_brief_digest = digest('5')),
        Box::new(|value| value.workspace_identity = "workspace-other".to_owned()),
        Box::new(|value| value.provider_model_class = "medium".to_owned()),
        Box::new(|value| value.model_class = "medium".to_owned()),
        Box::new(|value| value.timeout_seconds += 1),
        Box::new(|value| value.maximum_output_bytes += 1),
        Box::new(|value| value.codex_owner_head = "0".repeat(40)),
        Box::new(|value| value.switchyard_owner_head = "0".repeat(40)),
        Box::new(|value| value.switchyard_schema_sha256 = digest('6')),
        Box::new(|value| value.switchyard_deterministic_fixture_sha256 = digest('7')),
        Box::new(|value| value.provider_execution_id = Some("execution-too-early".to_owned())),
        Box::new(|value| value.internal_provider_retry_count = 1),
        Box::new(|value| value.semantic_retry = true),
        Box::new(|value| value.approval_response_authorized = true),
    ];
    for substitute in substitutions {
        let mut changed = base.clone();
        substitute(&mut changed);
        assert!(changed.seal().is_err());
    }
}

#[test]
fn v3_refuses_coherently_resealed_or_noncanonical_predecessor() {
    let mut changed_v2 = v2();
    changed_v2.workspace_identity = "workspace-substituted".to_owned();
    changed_v2.seal().unwrap();
    let changed_bytes = canonical(&changed_v2);
    let mut changed = v3();
    changed.predecessor_request_digest = changed_v2.request_digest;
    changed.predecessor_sha256 = format!("sha256:{:x}", Sha256::digest(&changed_bytes));
    changed.predecessor_bytes_hex = hex::encode(&changed_bytes);
    assert!(changed.seal().is_err());

    let pretty = serde_json::to_vec_pretty(&v2()).unwrap();
    let mut noncanonical = v3();
    noncanonical.predecessor_sha256 = format!("sha256:{:x}", Sha256::digest(&pretty));
    noncanonical.predecessor_bytes_hex = hex::encode(pretty);
    assert!(noncanonical.seal().is_err());
}

#[test]
fn v2_remains_valid_and_v3_is_recursively_closed() {
    let predecessor = v2();
    predecessor.validate().unwrap();

    let request = v3();
    let mut value: Value = serde_json::from_slice(&canonical(&request)).unwrap();
    value["invented_authority"] = json!(true);
    assert!(serde_json::from_value::<WorkerStartRequestV3>(value).is_err());

    let mut noncanonical = canonical(&request);
    noncanonical.push(b' ');
    assert!(WorkerStartRequestV3::from_slice(&noncanonical).is_err());
}
