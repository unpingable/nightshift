use chrono::{DateTime, Duration, Utc};
use nightshift_provider_capacity::{
    decide_capacity, normalize_codex_response, unknown_observation, AdmissionDisposition,
    CapacityObservationV1, CapacityPolicyV1, CapacityState, CapacityWindow, CodexProbeOptions,
    Confidence, ObservationDisposition, ObservationEvidence, SourceClass, WindowType,
    CAPACITY_OBSERVATION_SCHEMA_V1,
};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::time::Duration as StdDuration;

fn at(seconds: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(seconds, 0).unwrap()
}

fn options() -> CodexProbeOptions {
    CodexProbeOptions {
        codex_executable: PathBuf::from("/fixture/codex-native"),
        expected_executable_digest: format!("sha256:{}", "0".repeat(64)),
        expected_protocol_version: "0.147.0".into(),
        account_profile_locator: "local-codex-profile".into(),
        observed_at: at(1_800_000_000),
        expires_after: Duration::minutes(15),
        timeout: StdDuration::from_millis(20),
        maximum_response_bytes: 64 * 1024,
    }
}

fn response(primary: f64, secondary: f64) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "id": 2,
        "result": {
            "rateLimits": {
                "primary": {
                    "usedPercent": primary,
                    "windowDurationMins": 300,
                    "resetsAt": 1_800_003_600_i64
                },
                "secondary": {
                    "usedPercent": secondary,
                    "windowDurationMins": 10_080,
                    "resetsAt": 1_800_604_800_i64
                },
                "rateLimitReachedType": null
            },
            "rateLimitsByLimitId": null,
            "rateLimitResetCredits": null
        }
    }))
    .unwrap()
}

fn synthetic(source: SourceClass, confidence: Confidence, remaining: f64) -> CapacityObservationV1 {
    let mut observation = CapacityObservationV1 {
        schema: CAPACITY_OBSERVATION_SCHEMA_V1.into(),
        provider_id: "fixture-provider".into(),
        account_profile_locator: "fixture-profile".into(),
        model_family: Some("large".into()),
        observed_at: at(1_800_000_000),
        expires_at: at(1_800_003_600),
        source_class: source,
        confidence,
        disposition: ObservationDisposition::Usable,
        unknown_reasons: vec![],
        windows: vec![
            CapacityWindow {
                window_id: "five-hour".into(),
                window_type: WindowType::FiveHour,
                remaining_fraction: Some(remaining),
                remaining_units: None,
                resets_at: Some(at(1_800_003_600)),
            },
            CapacityWindow {
                window_id: "weekly".into(),
                window_type: WindowType::Weekly,
                remaining_fraction: Some(remaining),
                remaining_units: None,
                resets_at: Some(at(1_800_604_800)),
            },
        ],
        evidence: ObservationEvidence {
            probe_id: "fixture".into(),
            protocol_method: "fixture/read".into(),
            protocol_version: Some("fixture/v1".into()),
            executable_path: Some("/fixture/codex-native".into()),
            executable_digest: Some(format!("sha256:{}", "0".repeat(64))),
            raw_source_digest: format!("sha256:{}", "1".repeat(64)),
        },
        observation_digest: String::new(),
    };
    observation.observation_digest = observation.compute_digest().unwrap();
    observation
}

#[test]
fn supported_response_normalizes_and_binds_raw_digest() {
    let raw = response(25.0, 4.0);
    let observation = normalize_codex_response(&raw, &options());
    observation.validate().unwrap();
    assert_eq!(observation.source_class, SourceClass::Observed);
    assert_eq!(observation.confidence, Confidence::High);
    assert_eq!(observation.windows[0].window_type, WindowType::FiveHour);
    assert!(observation
        .evidence
        .raw_source_digest
        .starts_with("sha256:"));
}

#[test]
fn impossible_layout_mutation_and_malformed_input_become_unknown() {
    for raw in [
        response(101.0, 4.0),
        br#"{"id":2,"result":{"rate_limits":{"primary":{}}}}"#.to_vec(),
        br#"{"id":7,"result":{"rateLimits":{}}}"#.to_vec(),
        b"not-json".to_vec(),
    ] {
        let observation = normalize_codex_response(&raw, &options());
        assert_eq!(observation.disposition, ObservationDisposition::Unknown);
        assert_eq!(observation.confidence, Confidence::Unknown);
        observation.validate().unwrap();
    }
}

#[test]
fn contradictory_windows_become_unknown() {
    let raw = serde_json::to_vec(&json!({
        "id": 2,
        "result": {"rateLimits": {
            "primary": {
                "usedPercent": 10,
                "windowDurationMins": 300,
                "resetsAt": 1_800_003_600_i64
            },
            "secondary": {
                "usedPercent": 20,
                "windowDurationMins": 300,
                "resetsAt": 1_800_003_600_i64
            }
        }}
    }))
    .unwrap();
    let observation = normalize_codex_response(&raw, &options());
    assert_eq!(observation.unknown_reasons, ["CONTRADICTORY_WINDOWS"]);
}

#[test]
fn refusal_timeout_and_no_output_are_explicit_unknown() {
    let refusal = normalize_codex_response(
        br#"{"id":2,"error":{"code":-32600,"message":"authentication required"}}"#,
        &options(),
    );
    assert_eq!(refusal.unknown_reasons, ["PROVIDER_READ_REFUSED"]);

    for reason in [
        "PROBE_TIMEOUT",
        "PROBE_NO_OUTPUT",
        "PROBE_RESPONSE_OVERSIZED",
    ] {
        let observation = unknown_observation(&[], &options(), reason);
        assert_eq!(observation.unknown_reasons, [reason]);
        assert_eq!(observation.disposition, ObservationDisposition::Unknown);
        assert!(observation
            .evidence
            .raw_source_digest
            .starts_with("sha256:"));
    }
}

#[test]
fn source_and_confidence_are_distinct_policy_inputs() {
    let policy = CapacityPolicyV1::default();
    for source in [
        SourceClass::Authoritative,
        SourceClass::Observed,
        SourceClass::Inferred,
    ] {
        let observation = synthetic(source, Confidence::Medium, 0.60);
        assert_eq!(
            decide_capacity(&observation, &policy, at(1_800_000_001))
                .unwrap()
                .state,
            CapacityState::Abundant
        );
    }
    let unknown_source = synthetic(SourceClass::Unknown, Confidence::High, 0.90);
    assert_eq!(
        decide_capacity(&unknown_source, &policy, at(1_800_000_001))
            .unwrap()
            .state,
        CapacityState::Unknown
    );
    let low_confidence = synthetic(SourceClass::Authoritative, Confidence::Low, 0.90);
    assert_eq!(
        decide_capacity(&low_confidence, &policy, at(1_800_000_001))
            .unwrap()
            .state,
        CapacityState::Unknown
    );
}

#[test]
fn all_policy_states_and_digest_bound_decisions_reproduce() {
    let policy = CapacityPolicyV1::default();
    policy.validate().unwrap();
    for (remaining, expected) in [
        (0.80, CapacityState::Abundant),
        (0.40, CapacityState::Normal),
        (0.20, CapacityState::Conserve),
        (0.05, CapacityState::Critical),
    ] {
        let observation = synthetic(SourceClass::Observed, Confidence::High, remaining);
        let first = decide_capacity(&observation, &policy, at(1_800_000_001)).unwrap();
        let second = decide_capacity(&observation, &policy, at(1_800_000_001)).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.state, expected);
        assert_eq!(first.observation_digest, observation.observation_digest);
        assert_eq!(first.policy_digest, policy.policy_digest);
        first.validate().unwrap();
    }
}

#[test]
fn weekly_and_short_window_minimum_controls_while_context_is_distinct() {
    let mut critical_short: Value = serde_json::from_slice(&response(96.0, 2.0)).unwrap();
    critical_short["result"]["contextWindow"] = json!({"usedPercent": 1});
    let observation =
        normalize_codex_response(&serde_json::to_vec(&critical_short).unwrap(), &options());
    assert_eq!(
        decide_capacity(
            &observation,
            &CapacityPolicyV1::default(),
            at(1_800_000_001)
        )
        .unwrap()
        .state,
        CapacityState::Critical
    );

    let mut high_quota: Value = serde_json::from_slice(&response(5.0, 5.0)).unwrap();
    high_quota["result"]["contextWindow"] = json!({"usedPercent": 99});
    let observation =
        normalize_codex_response(&serde_json::to_vec(&high_quota).unwrap(), &options());
    assert_eq!(
        decide_capacity(
            &observation,
            &CapacityPolicyV1::default(),
            at(1_800_000_001)
        )
        .unwrap()
        .state,
        CapacityState::Abundant
    );
}

#[test]
fn missing_required_window_is_unknown_and_admits_no_new_work() {
    let mut observation = synthetic(SourceClass::Observed, Confidence::High, 0.95);
    observation
        .windows
        .retain(|window| window.window_type == WindowType::Weekly);
    observation.observation_digest = observation.compute_digest().unwrap();

    let decision = decide_capacity(
        &observation,
        &CapacityPolicyV1::default(),
        at(1_800_000_001),
    )
    .unwrap();
    assert_eq!(decision.state, CapacityState::Unknown);
    assert_eq!(decision.admission, AdmissionDisposition::NoNewWork);
    assert!(!decision.allow_new_expensive_work);
    assert!(!decision.allow_new_speculative_work);
    assert!(decision.allow_active_work_to_reach_custody);
    assert_eq!(decision.reason_codes, ["REQUIRED_WINDOW_MISSING_FIVE_HOUR"]);
}

#[test]
fn stale_reset_rollover_and_unknown_refuse_new_work_but_preserve_custody() {
    let policy = CapacityPolicyV1::default();
    let observation = synthetic(SourceClass::Observed, Confidence::High, 0.9);
    for decision_at in [at(1_800_003_600), at(1_800_604_800)] {
        let decision = decide_capacity(&observation, &policy, decision_at).unwrap();
        assert_eq!(decision.state, CapacityState::Unknown);
        assert!(!decision.allow_new_expensive_work);
        assert!(!decision.allow_new_speculative_work);
        assert!(decision.allow_active_work_to_reach_custody);
    }

    let critical = synthetic(SourceClass::Observed, Confidence::High, 0.01);
    let decision = decide_capacity(&critical, &policy, at(1_800_000_001)).unwrap();
    assert_eq!(decision.admission, AdmissionDisposition::NoNewWork);
    assert!(decision.allow_active_work_to_reach_custody);

    let unknown = unknown_observation(&[], &options(), "PROBE_TIMEOUT");
    let decision = decide_capacity(&unknown, &policy, at(1_800_000_001)).unwrap();
    assert_eq!(decision.state, CapacityState::Unknown);
    assert_eq!(decision.admission, AdmissionDisposition::NoNewWork);
    assert!(decision.allow_active_work_to_reach_custody);
}

#[test]
fn reset_rollover_does_not_invent_new_capacity() {
    let raw = response(99.0, 99.0);
    let mut observation = normalize_codex_response(&raw, &options());
    observation.expires_at = at(1_800_700_000);
    observation.observation_digest = observation.compute_digest().unwrap();
    let decision = decide_capacity(
        &observation,
        &CapacityPolicyV1::default(),
        at(1_800_003_600),
    )
    .unwrap();
    assert_eq!(decision.state, CapacityState::Unknown);
    assert_eq!(
        decision.reason_codes,
        ["RESET_ROLLOVER_REQUIRES_NEW_OBSERVATION"]
    );
}

#[test]
fn digest_domains_and_content_mutation_are_detected() {
    let policy = CapacityPolicyV1::default();
    let mut observation = synthetic(SourceClass::Observed, Confidence::High, 0.4);
    assert_ne!(observation.observation_digest, policy.policy_digest);
    observation.windows[0].remaining_fraction = Some(0.9);
    assert!(observation
        .validate()
        .unwrap_err()
        .to_string()
        .contains("digest mismatch"));
}

#[test]
fn closed_records_refuse_unknown_semantic_fields() {
    let observation = synthetic(SourceClass::Observed, Confidence::High, 0.4);
    let mut value = serde_json::to_value(observation).unwrap();
    value["aggregateResult"] = json!("fine");
    assert!(serde_json::from_value::<CapacityObservationV1>(value).is_err());
}

#[test]
fn digest_consistent_decision_state_substitution_is_refused() {
    let observation = synthetic(SourceClass::Observed, Confidence::High, 0.01);
    let mut decision = decide_capacity(
        &observation,
        &CapacityPolicyV1::default(),
        at(1_800_000_001),
    )
    .unwrap();
    assert_eq!(decision.state, CapacityState::Critical);

    decision.admission = AdmissionDisposition::OrdinaryBounded;
    decision.allow_new_expensive_work = true;
    decision.allow_new_speculative_work = true;
    decision.decision_digest = decision.compute_digest().unwrap();
    assert!(decision
        .validate()
        .unwrap_err()
        .to_string()
        .contains("contradict capacity state"));
}
