use thiserror::Error;

/// A closed classification of NQ interface-contract violations. NS
/// speaks a specific NQ export contract; these are the ways an inbound
/// export can fail to honor it. Deliberately a *category* taxonomy, not
/// a pin to today's wire schema: nq-ng (`~/git/skunkworks/nq-ng`) is
/// being rebuilt correctness-first, and these categories survive a wire
/// redesign even as the concrete schema id / field set changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NqContractViolationKind {
    /// The export declared a `schema` id NS does not speak.
    SchemaMismatch,
    /// The export's `contract_version` is not the one NS speaks.
    ContractVersionMismatch,
    /// A field NS requires was present but unparseable (e.g. a
    /// timestamp that is not RFC3339).
    MalformedField,
}

#[derive(Debug, Error)]
pub enum NightShiftError {
    #[error("agenda not found: {0}")]
    AgendaNotFound(String),

    #[error("invalid agenda: {0}")]
    InvalidAgenda(String),

    /// A violation of the NQ export/interface contract. Distinct from
    /// `InvalidAgenda`: an NQ refusal arrives in its own stratum rather
    /// than wearing agenda vocabulary (no-vocabulary-laundering). The
    /// `detail` carries the specific mismatch for operator display.
    #[error("NQ contract violation ({kind:?}): {detail}")]
    NqContractViolation {
        kind: NqContractViolationKind,
        detail: String,
    },

    #[error("evidence source not allowed: {0}")]
    EvidenceSourceNotAllowed(String),

    #[error("authority ceiling violated: requested {requested:?} exceeds ceiling {ceiling:?}")]
    AuthorityCeilingExceeded {
        requested: String,
        ceiling: String,
    },

    #[error("run aborted by coordination preflight: {0}")]
    PreflightBlocked(String),

    #[error("run not found: {0}")]
    RunNotFound(String),

    #[error("run already completed: {0} — reconcile is one-shot, start a new capture instead")]
    RunAlreadyCompleted(String),

    #[error("run has no persisted bundle: {0}")]
    RunBundleMissing(String),

    #[error("NQ finding inadmissible: finding_key={finding_key}, state={state}, reason={reason}")]
    NqInadmissible {
        finding_key: String,
        state: String,
        reason: String,
    },

    #[error("store error: {0}")]
    Store(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("yaml error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T, E = NightShiftError> = std::result::Result<T, E>;
