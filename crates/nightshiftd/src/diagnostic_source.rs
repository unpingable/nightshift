//! Reproducible, read-only import of exact NQ diagnostic artifacts from an
//! extracted NQ release payload.
//!
//! The source manifest binds an unattested repository/commit declaration, the
//! immutable NQ contract package manifests, and each exact diagnostic
//! artifact. The NQ package remains authoritative for its contract assets;
//! Nightshift's Rust DTO is only a strict consumer and evaluator input.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::diagnostic_execution_v2::{DiagnosticExecution, NQ_DIAGNOSTIC_EXECUTION_V2_SCHEMA};
use crate::diagnostic_posture::{
    DiagnosticInput, DiagnosticInputStatus, DiagnosticInputs, DiagnosticKey, INPUTS_SCHEMA,
    INPUTS_SCHEMA_V1, NQ_DIAGNOSTIC_EXECUTION_SCHEMA,
};

pub const NQ_SOURCE_MANIFEST_SCHEMA_V1: &str = "nightshift.nq_diagnostic_sources.v1";
pub const NQ_SOURCE_MANIFEST_SCHEMA_V2: &str = "nightshift.nq_diagnostic_sources.v2";
pub const NQ_SOURCE_MANIFEST_SCHEMA: &str = NQ_SOURCE_MANIFEST_SCHEMA_V2;
pub const NQ_SOURCE_IMPORT_RECEIPT_SCHEMA_V1: &str = "nightshift.nq_source_import_receipt.v1";
pub const NQ_SOURCE_IMPORT_RECEIPT_SCHEMA_V2: &str = "nightshift.nq_source_import_receipt.v2";
pub const NQ_SOURCE_IMPORT_RECEIPT_SCHEMA: &str = NQ_SOURCE_IMPORT_RECEIPT_SCHEMA_V2;
pub const NQ_CONTRACT_ASSET_SCHEMA_V1: &str = "nq.diagnostic_contract_assets.v1";
pub const NQ_CONTRACT_ASSET_SCHEMA_V2: &str = "nq.diagnostic_contract_assets.v2";
const MAX_JSON_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NqPackagePin {
    /// Consumer-declared source identity. The current package format does not
    /// attest this field; the declared payload-manifest and exact contract
    /// assets are bound by the digests below.
    pub repository_identity: String,
    /// Consumer-declared source revision, syntax-checked but not attested by
    /// the current NQ package.
    pub commit: String,
    /// Consumer-declared release name, syntax-checked but not attested by the
    /// current NQ package.
    pub release_identity: String,
    pub contract_schema: String,
    pub asset_root: String,
    pub asset_manifest_path: String,
    pub asset_manifest_sha256: String,
    pub payload_manifest_path: String,
    pub payload_manifest_sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContractAssetManifest {
    schema: String,
    digest_basis: String,
    documentation: ContractAssetFile,
    contract: ContractAssetDescriptor,
    fixtures: Vec<ContractAssetFixture>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContractAssetFile {
    path: String,
    sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContractAssetDescriptor {
    schema: String,
    canonicalization: ContractCanonicalization,
    schema_path: String,
    schema_sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContractCanonicalization {
    id: String,
    version: String,
    digest: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContractAssetFixture {
    id: String,
    class: String,
    path: String,
    sha256: String,
    artifact_id: String,
    expected_disposition: String,
    #[serde(default)]
    expected_error: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum NqSourceStatus {
    Delivered {
        artifact_path: String,
        artifact_sha256: String,
        artifact_id: String,
    },
    NoResponse,
    AcquisitionFailed {
        reason: String,
    },
    NotConfigured,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NqSourceEntry {
    pub key: DiagnosticKey,
    #[serde(flatten)]
    pub status: NqSourceStatus,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NqSourceManifest {
    pub schema: String,
    pub source_manifest_id: String,
    pub package: NqPackagePin,
    pub inputs: Vec<NqSourceEntry>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NqSourceImportReceipt {
    pub schema: String,
    pub receipt_id: String,
    pub source_manifest: NqSourceManifest,
    pub imported_inputs_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportedDiagnosticSources {
    pub inputs: DiagnosticInputs,
    pub receipt: NqSourceImportReceipt,
}

impl NqSourceImportReceipt {
    pub fn validate(&self) -> Result<(), String> {
        if !matches!(
            self.schema.as_str(),
            NQ_SOURCE_IMPORT_RECEIPT_SCHEMA_V1 | NQ_SOURCE_IMPORT_RECEIPT_SCHEMA_V2
        ) {
            return Err(format!(
                "NQ source import receipt schema must be {NQ_SOURCE_IMPORT_RECEIPT_SCHEMA_V1} or {NQ_SOURCE_IMPORT_RECEIPT_SCHEMA_V2}"
            ));
        }
        if self.schema == NQ_SOURCE_IMPORT_RECEIPT_SCHEMA_V1
            && self.source_manifest.schema != NQ_SOURCE_MANIFEST_SCHEMA_V1
        {
            return Err("source import receipt v1 cannot wrap a v2 source manifest".into());
        }
        self.source_manifest.validate()?;
        validate_digest(&self.imported_inputs_id, "imported_inputs_id")?;
        validate_digest(&self.receipt_id, "source import receipt_id")?;
        if self.receipt_id != computed_object_id(self, "receipt_id")? {
            return Err("source import receipt_id does not match its canonical preimage".into());
        }
        Ok(())
    }

    pub fn validate_inputs(&self, inputs: &DiagnosticInputs) -> Result<(), String> {
        self.validate()?;
        inputs.validate()?;
        if self.schema == NQ_SOURCE_IMPORT_RECEIPT_SCHEMA_V1 && inputs.schema != INPUTS_SCHEMA_V1 {
            return Err(
                "source import receipt v1 cannot bind a v2 diagnostic-input carrier".into(),
            );
        }
        if self.imported_inputs_id != inputs.inputs_id {
            return Err("source import receipt does not bind the diagnostic inputs id".into());
        }
        if self.source_manifest.inputs.len() != inputs.inputs.len() {
            return Err(
                "source import receipt and diagnostic inputs have different cardinality".into(),
            );
        }
        for (source, input) in self.source_manifest.inputs.iter().zip(&inputs.inputs) {
            if source.key != input.key {
                return Err("source import receipt substitutes a diagnostic key".into());
            }
            match (&source.status, &input.status) {
                (
                    NqSourceStatus::Delivered {
                        artifact_sha256,
                        artifact_id,
                        ..
                    },
                    DiagnosticInputStatus::Delivered { artifact },
                ) => {
                    if artifact.artifact_id() != artifact_id {
                        return Err("source import receipt substitutes an NQ artifact id".into());
                    }
                    let canonical =
                        serde_jcs::to_vec(artifact).map_err(|error| error.to_string())?;
                    verify_exact_digest(
                        &canonical,
                        artifact_sha256,
                        "source import exact NQ artifact",
                    )?;
                }
                (NqSourceStatus::NoResponse, DiagnosticInputStatus::NoResponse)
                | (NqSourceStatus::NotConfigured, DiagnosticInputStatus::NotConfigured) => {}
                (
                    NqSourceStatus::AcquisitionFailed {
                        reason: source_reason,
                    },
                    DiagnosticInputStatus::AcquisitionFailed { reason },
                ) if source_reason == reason => {}
                _ => {
                    return Err(
                        "source import receipt status differs from the diagnostic inputs".into(),
                    )
                }
            }
        }
        Ok(())
    }
}

impl NqSourceManifest {
    pub fn computed_source_manifest_id(&self) -> Result<String, String> {
        computed_object_id(self, "source_manifest_id")
    }

    pub fn validate(&self) -> Result<(), String> {
        if !matches!(
            self.schema.as_str(),
            NQ_SOURCE_MANIFEST_SCHEMA_V1 | NQ_SOURCE_MANIFEST_SCHEMA_V2
        ) {
            return Err(format!(
                "NQ source manifest schema must be {NQ_SOURCE_MANIFEST_SCHEMA_V1} or {NQ_SOURCE_MANIFEST_SCHEMA_V2}, got {}",
                self.schema
            ));
        }
        validate_package_pin(&self.package)?;
        if self.schema == NQ_SOURCE_MANIFEST_SCHEMA_V1
            && self.package.contract_schema != NQ_DIAGNOSTIC_EXECUTION_SCHEMA
        {
            return Err("NQ source manifest v1 cannot pin diagnostic_execution.v2".into());
        }
        validate_digest(&self.source_manifest_id, "source_manifest_id")?;
        let mut previous: Option<&DiagnosticKey> = None;
        for input in &self.inputs {
            validate_key(&input.key, "source.key")?;
            if previous.is_some_and(|value| value >= &input.key) {
                return Err(
                    "NQ sources must be strictly ordered by diagnostic key and unique".into(),
                );
            }
            previous = Some(&input.key);
            match &input.status {
                NqSourceStatus::Delivered {
                    artifact_path,
                    artifact_sha256,
                    artifact_id,
                } => {
                    validate_relative_path(artifact_path, "artifact_path")?;
                    validate_digest(artifact_sha256, "artifact_sha256")?;
                    validate_digest(artifact_id, "artifact_id")?;
                }
                NqSourceStatus::AcquisitionFailed { reason } => {
                    require_token("acquisition_failed.reason", reason)?;
                }
                NqSourceStatus::NoResponse | NqSourceStatus::NotConfigured => {}
            }
        }
        if self.source_manifest_id != self.computed_source_manifest_id()? {
            return Err("source_manifest_id does not match the canonical source preimage".into());
        }
        Ok(())
    }
}

/// Verify one extracted NQ package and import exact diagnostic artifacts.
///
/// This function performs no network access, acquisition, persistence, retry,
/// proposal, authorization, or action. `package_root` is the root whose
/// `share/nq` directory is covered by the pinned payload manifest.
pub fn load_diagnostic_sources(
    source_manifest_path: &Path,
    package_root: &Path,
) -> Result<ImportedDiagnosticSources, String> {
    let source_bytes = read_regular_absolute(source_manifest_path, "source manifest")?;
    let manifest: NqSourceManifest = serde_json::from_slice(&source_bytes).map_err(|error| {
        format!(
            "parse NQ source manifest {}: {error}",
            source_manifest_path.display()
        )
    })?;
    manifest.validate()?;
    if serde_jcs::to_vec(&manifest).map_err(|error| error.to_string())? != source_bytes {
        return Err("NQ source manifest is not its exact RFC 8785 canonical representation".into());
    }
    verify_package(package_root, &manifest.package)?;

    let source_root = source_manifest_path
        .parent()
        .ok_or_else(|| "source manifest has no parent directory".to_string())?;
    let mut inputs = Vec::with_capacity(manifest.inputs.len());
    for source in manifest.inputs.iter().cloned() {
        let status = match source.status {
            NqSourceStatus::Delivered {
                artifact_path,
                artifact_sha256,
                artifact_id,
            } => {
                let bytes = read_bounded_relative(source_root, &artifact_path, "NQ artifact")?;
                verify_exact_digest(&bytes, &artifact_sha256, "NQ artifact")?;
                let artifact: DiagnosticExecution = serde_json::from_slice(&bytes)
                    .map_err(|error| format!("parse exact NQ artifact {artifact_path}: {error}"))?;
                artifact.validate().map_err(|error| {
                    format!("NQ artifact {artifact_path} is inadmissible: {error}")
                })?;
                if serde_jcs::to_vec(&artifact).map_err(|error| error.to_string())? != bytes {
                    return Err(format!(
                        "NQ artifact {artifact_path} is not its exact canonical byte representation"
                    ));
                }
                if artifact.schema_name() != manifest.package.contract_schema {
                    return Err(format!(
                        "NQ artifact {artifact_path} schema does not match its package pin"
                    ));
                }
                if artifact.artifact_id() != artifact_id {
                    return Err(format!(
                        "NQ artifact {artifact_path} does not match its source-manifest identity"
                    ));
                }
                let artifact_key = artifact.key();
                if artifact_key != source.key {
                    return Err(format!(
                        "NQ artifact {artifact_path} does not bind its declared source key"
                    ));
                }
                DiagnosticInputStatus::Delivered {
                    artifact: Box::new(artifact),
                }
            }
            NqSourceStatus::NoResponse => DiagnosticInputStatus::NoResponse,
            NqSourceStatus::AcquisitionFailed { reason } => {
                DiagnosticInputStatus::AcquisitionFailed { reason }
            }
            NqSourceStatus::NotConfigured => DiagnosticInputStatus::NotConfigured,
        };
        inputs.push(DiagnosticInput {
            key: source.key,
            status,
        });
    }
    let mut result = DiagnosticInputs {
        schema: INPUTS_SCHEMA.into(),
        inputs_id: String::new(),
        inputs,
    };
    result.inputs_id = result.computed_inputs_id()?;
    result.validate()?;
    let mut receipt = NqSourceImportReceipt {
        schema: NQ_SOURCE_IMPORT_RECEIPT_SCHEMA.into(),
        receipt_id: String::new(),
        source_manifest: manifest,
        imported_inputs_id: result.inputs_id.clone(),
    };
    receipt.receipt_id = computed_object_id(&receipt, "receipt_id")?;
    Ok(ImportedDiagnosticSources {
        inputs: result,
        receipt,
    })
}

fn validate_package_pin(pin: &NqPackagePin) -> Result<(), String> {
    require_token("package.repository_identity", &pin.repository_identity)?;
    if pin.commit.len() != 40
        || !pin
            .commit
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("package.commit must be an exact 40-character lowercase Git commit".into());
    }
    require_token("package.release_identity", &pin.release_identity)?;
    if !matches!(
        pin.contract_schema.as_str(),
        NQ_DIAGNOSTIC_EXECUTION_SCHEMA | NQ_DIAGNOSTIC_EXECUTION_V2_SCHEMA
    ) {
        return Err(format!(
            "package contract_schema must be {NQ_DIAGNOSTIC_EXECUTION_SCHEMA} or {NQ_DIAGNOSTIC_EXECUTION_V2_SCHEMA}"
        ));
    }
    for (field, path) in [
        ("package.asset_root", &pin.asset_root),
        ("package.asset_manifest_path", &pin.asset_manifest_path),
        ("package.payload_manifest_path", &pin.payload_manifest_path),
    ] {
        validate_relative_path(path, field)?;
    }
    let (expected_asset_root, expected_asset_manifest) = match pin.contract_schema.as_str() {
        NQ_DIAGNOSTIC_EXECUTION_SCHEMA => (
            "share/nq/diagnostic-contract",
            "share/nq/diagnostic-contract/manifest.json",
        ),
        NQ_DIAGNOSTIC_EXECUTION_V2_SCHEMA => (
            "share/nq/diagnostic-contract-v2",
            "share/nq/diagnostic-contract-v2/manifest.json",
        ),
        _ => unreachable!("contract schema was validated above"),
    };
    if pin.asset_root != expected_asset_root
        || pin.asset_manifest_path != expected_asset_manifest
        || pin.payload_manifest_path != "share/nq/MANIFEST.sha256"
    {
        return Err("package pin does not use the qualified NQ diagnostic-contract layout".into());
    }
    validate_digest(&pin.asset_manifest_sha256, "package.asset_manifest_sha256")?;
    validate_digest(
        &pin.payload_manifest_sha256,
        "package.payload_manifest_sha256",
    )
}

fn verify_package(root: &Path, pin: &NqPackagePin) -> Result<(), String> {
    let root_metadata =
        fs::symlink_metadata(root).map_err(|error| format!("inspect package root: {error}"))?;
    if root_metadata.file_type().is_symlink() || !root_metadata.is_dir() {
        return Err("package root must be a regular non-symlink directory".into());
    }
    let payload_bytes =
        read_bounded_relative(root, &pin.payload_manifest_path, "NQ payload manifest")?;
    verify_exact_digest(
        &payload_bytes,
        &pin.payload_manifest_sha256,
        "NQ payload manifest",
    )?;
    let entries = parse_payload_manifest(&payload_bytes)?;

    let asset_manifest =
        read_bounded_relative(root, &pin.asset_manifest_path, "NQ contract asset manifest")?;
    verify_exact_digest(
        &asset_manifest,
        &pin.asset_manifest_sha256,
        "NQ contract asset manifest",
    )?;
    let asset_value: ContractAssetManifest = serde_json::from_slice(&asset_manifest)
        .map_err(|error| format!("parse NQ contract asset manifest: {error}"))?;
    let expected_asset_schema = match pin.contract_schema.as_str() {
        NQ_DIAGNOSTIC_EXECUTION_SCHEMA => NQ_CONTRACT_ASSET_SCHEMA_V1,
        NQ_DIAGNOSTIC_EXECUTION_V2_SCHEMA => NQ_CONTRACT_ASSET_SCHEMA_V2,
        _ => unreachable!("contract schema was validated above"),
    };
    if asset_value.schema != expected_asset_schema {
        return Err(format!(
            "NQ contract asset manifest schema must be {expected_asset_schema}"
        ));
    }
    require_token("asset manifest digest_basis", &asset_value.digest_basis)?;
    if asset_value.contract.schema != pin.contract_schema {
        return Err(
            "NQ contract asset manifest does not bind the pinned diagnostic contract".into(),
        );
    }
    if asset_value.contract.canonicalization.id != "rfc8785-jcs"
        || asset_value.contract.canonicalization.version != "1"
        || asset_value.contract.canonicalization.digest
            != "sha256:e49d92d4e86052e66ed2a481b9386d3b214ce3d2df5fd109a6491ccb9ffb24f3"
    {
        return Err("NQ contract asset manifest binds an unknown canonicalization".into());
    }
    validate_relative_path(&asset_value.contract.schema_path, "contract schema path")?;
    validate_digest(
        &asset_value.contract.schema_sha256,
        "contract schema sha256",
    )?;
    let mut declared_assets = BTreeMap::new();
    validate_relative_path(
        &asset_value.documentation.path,
        "contract documentation path",
    )?;
    validate_digest(
        &asset_value.documentation.sha256,
        "contract documentation sha256",
    )?;
    declared_assets.insert(
        format!("{}/{}", pin.asset_root, asset_value.documentation.path),
        asset_value.documentation.sha256,
    );
    declared_assets.insert(
        format!("{}/{}", pin.asset_root, asset_value.contract.schema_path),
        asset_value.contract.schema_sha256,
    );
    let mut fixture_ids = BTreeSet::new();
    let mut qualified_fixtures = Vec::new();
    for fixture in asset_value.fixtures {
        require_token("contract fixture id", &fixture.id)?;
        if !fixture_ids.insert(fixture.id.clone()) {
            return Err("NQ contract asset manifest repeats a fixture id".into());
        }
        if !matches!(fixture.class.as_str(), "valid" | "hostile") {
            return Err("NQ contract asset manifest has an unknown fixture class".into());
        }
        validate_relative_path(&fixture.path, "contract fixture path")?;
        validate_digest(&fixture.sha256, "contract fixture sha256")?;
        validate_digest(&fixture.artifact_id, "contract fixture artifact_id")?;
        require_token(
            "contract fixture expected_disposition",
            &fixture.expected_disposition,
        )?;
        if fixture.class == "hostile"
            && fixture
                .expected_error
                .as_ref()
                .is_none_or(|value| value.trim().is_empty())
        {
            return Err("hostile NQ contract fixture lacks an expected_error".into());
        }
        let path = format!("{}/{}", pin.asset_root, fixture.path);
        if declared_assets
            .insert(path.clone(), fixture.sha256.clone())
            .is_some()
        {
            return Err(format!("NQ contract asset manifest repeats {path}"));
        }
        qualified_fixtures.push(fixture);
    }

    let asset_files = regular_files_below(root, &pin.asset_root)?;
    if asset_files.is_empty() {
        return Err("NQ diagnostic-contract asset root is empty".into());
    }
    for relative in asset_files {
        let package_expected = entries
            .get(relative.as_str())
            .ok_or_else(|| format!("NQ payload manifest omits {relative}"))?;
        let bytes = read_bounded_relative(root, &relative, "NQ package asset")?;
        verify_exact_digest(
            &bytes,
            package_expected,
            &format!("NQ package asset {relative}"),
        )?;
        if relative == pin.asset_manifest_path {
            continue;
        }
        let static_expected = declared_assets
            .remove(relative.as_str())
            .ok_or_else(|| format!("NQ static contract manifest omits package asset {relative}"))?;
        verify_exact_digest(
            &bytes,
            &static_expected,
            &format!("NQ statically declared contract asset {relative}"),
        )?;
    }
    if !declared_assets.is_empty() {
        return Err(format!(
            "NQ package omits statically declared assets: {}",
            declared_assets
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    for fixture in &qualified_fixtures {
        verify_contract_fixture(root, pin, fixture)?;
    }
    Ok(())
}

fn verify_contract_fixture(
    root: &Path,
    pin: &NqPackagePin,
    fixture: &ContractAssetFixture,
) -> Result<(), String> {
    let path = format!("{}/{}", pin.asset_root, fixture.path);
    let bytes = read_bounded_relative(root, &path, "NQ contract fixture")?;
    let evaluation: Result<(), String> = (|| {
        let artifact: DiagnosticExecution =
            serde_json::from_slice(&bytes).map_err(|error| format!("decode fixture: {error}"))?;
        artifact.validate()?;
        if serde_jcs::to_vec(&artifact).map_err(|error| error.to_string())? != bytes {
            return Err("fixture is not exact RFC 8785 canonical JSON".into());
        }
        if artifact.schema_name() != pin.contract_schema {
            return Err("fixture schema does not match its package contract".into());
        }
        if artifact.artifact_id() != fixture.artifact_id {
            return Err("fixture artifact_id differs from its asset manifest".into());
        }
        Ok(())
    })();

    match fixture.class.as_str() {
        "valid" => {
            if fixture.expected_disposition != "accepted" {
                return Err(format!(
                    "valid NQ contract fixture {} has a non-accepted expected disposition",
                    fixture.id
                ));
            }
            evaluation.map_err(|error| {
                format!(
                    "Nightshift rejected valid NQ contract fixture {}: {error}",
                    fixture.id
                )
            })
        }
        "hostile" => {
            if !fixture.expected_disposition.starts_with("rejected_") {
                return Err(format!(
                    "hostile NQ contract fixture {} lacks a rejected expected disposition",
                    fixture.id
                ));
            }
            match evaluation {
                Ok(()) => Err(format!(
                    "Nightshift accepted hostile NQ contract fixture {}",
                    fixture.id
                )),
                Err(_) => Ok(()),
            }
        }
        _ => unreachable!("fixture class was validated above"),
    }
}

fn parse_payload_manifest(bytes: &[u8]) -> Result<BTreeMap<String, String>, String> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| format!("NQ payload manifest is not UTF-8: {error}"))?;
    let mut entries = BTreeMap::new();
    for (index, line) in text.lines().enumerate() {
        let Some((hex, path)) = line.split_once("  ") else {
            return Err(format!(
                "NQ payload manifest line {} is not sha256sum format",
                index + 1
            ));
        };
        let path = path.strip_prefix("./").unwrap_or(path);
        validate_relative_path(path, "payload manifest path")?;
        let digest = format!("sha256:{hex}");
        validate_digest(&digest, "payload manifest digest")?;
        if entries.insert(path.to_string(), digest).is_some() {
            return Err(format!("NQ payload manifest repeats {path}"));
        }
    }
    if entries.is_empty() {
        return Err("NQ payload manifest is empty".into());
    }
    Ok(entries)
}

fn regular_files_below(root: &Path, relative_root: &str) -> Result<Vec<String>, String> {
    let start = bounded_path(root, relative_root, "asset root")?;
    let metadata =
        fs::symlink_metadata(&start).map_err(|error| format!("inspect asset root: {error}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err("asset root must be a regular non-symlink directory".into());
    }
    let mut pending = vec![start];
    let mut files = Vec::new();
    while let Some(directory) = pending.pop() {
        let entries =
            fs::read_dir(&directory).map_err(|error| format!("read asset directory: {error}"))?;
        for entry in entries {
            let entry = entry.map_err(|error| format!("read asset directory entry: {error}"))?;
            let metadata = entry
                .file_type()
                .map_err(|error| format!("inspect asset entry: {error}"))?;
            if metadata.is_symlink() {
                return Err(format!(
                    "NQ contract asset tree contains a symlink: {}",
                    entry.path().display()
                ));
            }
            if metadata.is_dir() {
                pending.push(entry.path());
            } else if metadata.is_file() {
                let relative = entry
                    .path()
                    .strip_prefix(root)
                    .map_err(|_| "asset escaped package root".to_string())?
                    .to_string_lossy()
                    .replace('\\', "/");
                validate_relative_path(&relative, "asset path")?;
                files.push(relative);
            } else {
                return Err(format!(
                    "NQ contract asset tree contains a non-regular entry: {}",
                    entry.path().display()
                ));
            }
        }
    }
    files.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    Ok(files)
}

fn read_regular_absolute(path: &Path, kind: &str) -> Result<Vec<u8>, String> {
    let metadata =
        fs::symlink_metadata(path).map_err(|error| format!("inspect {kind}: {error}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!("{kind} must be a regular non-symlink file"));
    }
    if metadata.len() > MAX_JSON_BYTES {
        return Err(format!("{kind} exceeds {MAX_JSON_BYTES} bytes"));
    }
    fs::read(path).map_err(|error| format!("read {kind}: {error}"))
}

fn read_bounded_relative(root: &Path, relative: &str, kind: &str) -> Result<Vec<u8>, String> {
    let path = bounded_path(root, relative, kind)?;
    read_regular_absolute(&path, kind)
}

fn bounded_path(root: &Path, relative: &str, kind: &str) -> Result<PathBuf, String> {
    validate_relative_path(relative, kind)?;
    let mut path = root.to_path_buf();
    for component in Path::new(relative).components() {
        let Component::Normal(value) = component else {
            return Err(format!("{kind} path is not normalized"));
        };
        path.push(value);
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| format!("inspect {kind} path component: {error}"))?;
        if metadata.file_type().is_symlink() {
            return Err(format!("{kind} path traverses a symlink"));
        }
    }
    Ok(path)
}

fn validate_relative_path(value: &str, field: &str) -> Result<(), String> {
    let path = Path::new(value);
    let components: Option<Vec<_>> = path
        .components()
        .map(|component| match component {
            Component::Normal(value) => value.to_str(),
            _ => None,
        })
        .collect();
    let normalized = components
        .as_ref()
        .map(|values| values.join("/"))
        .unwrap_or_default();
    if value.is_empty()
        || value.contains('\\')
        || path.is_absolute()
        || components.is_none()
        || normalized != value
    {
        return Err(format!("{field} must be a normalized relative POSIX path"));
    }
    Ok(())
}

fn verify_exact_digest(bytes: &[u8], expected: &str, field: &str) -> Result<(), String> {
    let actual = format!("sha256:{:x}", Sha256::digest(bytes));
    if actual != expected {
        Err(format!(
            "{field} digest mismatch: expected {expected}, got {actual}"
        ))
    } else {
        Ok(())
    }
}

fn computed_object_id<T: Serialize>(value: &T, identity_field: &str) -> Result<String, String> {
    let mut value = serde_json::to_value(value).map_err(|error| error.to_string())?;
    value
        .as_object_mut()
        .ok_or_else(|| "self-identified contract must serialize as an object".to_string())?
        .remove(identity_field);
    let canonical = serde_jcs::to_vec(&value).map_err(|error| error.to_string())?;
    Ok(format!("sha256:{:x}", Sha256::digest(canonical)))
}

fn validate_key(key: &DiagnosticKey, field: &str) -> Result<(), String> {
    require_token(&format!("{field}.question_id"), &key.question_id)?;
    require_token(&format!("{field}.subject_id"), &key.subject_id)?;
    require_token(&format!("{field}.profile_id"), &key.profile_id)?;
    require_token(&format!("{field}.vantage_id"), &key.vantage_id)
}

fn validate_digest(value: &str, field: &str) -> Result<(), String> {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return Err(format!("{field} must use sha256:<64 lowercase hex>"));
    };
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!("{field} must use sha256:<64 lowercase hex>"));
    }
    Ok(())
}

fn require_token(field: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("{field} is empty"))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic_posture::DiagnosticExecutionV1;

    const POSITIVE: &[u8] =
        include_bytes!("../tests/fixtures/nq_diagnostic_execution/positive.json");
    const HOSTILE: &[u8] = include_bytes!(
        "../tests/fixtures/nq_diagnostic_execution/hostile_projection_collision_match.json"
    );

    fn sha(bytes: &[u8]) -> String {
        format!("sha256:{:x}", Sha256::digest(bytes))
    }

    fn write(path: &Path, bytes: &[u8]) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    fn package_and_source() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        let package = temp.path().join("package");
        let source = temp.path().join("source");
        let asset_root = package.join("share/nq/diagnostic-contract");
        let readme = b"synthetic closed package used only by importer unit tests\n";
        let schema = b"{}\n";
        write(&asset_root.join("README.md"), readme);
        write(
            &asset_root.join("schemas/nq.diagnostic_execution.v1.schema.json"),
            schema,
        );
        write(&asset_root.join("fixtures/valid/positive.json"), POSITIVE);
        let artifact: DiagnosticExecutionV1 = serde_json::from_slice(POSITIVE).unwrap();
        let static_manifest = serde_json::to_vec_pretty(&serde_json::json!({
            "schema": NQ_CONTRACT_ASSET_SCHEMA_V1,
            "digest_basis": "SHA-256 of exact file bytes",
            "documentation": {
                "path": "README.md",
                "sha256": sha(readme),
            },
            "contract": {
                "schema": NQ_DIAGNOSTIC_EXECUTION_SCHEMA,
                "canonicalization": {
                    "id": "rfc8785-jcs",
                    "version": "1",
                    "digest": "sha256:e49d92d4e86052e66ed2a481b9386d3b214ce3d2df5fd109a6491ccb9ffb24f3"
                },
                "schema_path": "schemas/nq.diagnostic_execution.v1.schema.json",
                "schema_sha256": sha(schema),
            },
            "fixtures": [{
                "id": "positive",
                "class": "valid",
                "path": "fixtures/valid/positive.json",
                "sha256": sha(POSITIVE),
                "artifact_id": artifact.artifact_id,
                "expected_disposition": "accepted",
            }],
        }))
        .unwrap();
        write(&asset_root.join("manifest.json"), &static_manifest);

        let package_files = [
            ("share/nq/diagnostic-contract/README.md", readme.as_slice()),
            (
                "share/nq/diagnostic-contract/fixtures/valid/positive.json",
                POSITIVE,
            ),
            (
                "share/nq/diagnostic-contract/manifest.json",
                static_manifest.as_slice(),
            ),
            (
                "share/nq/diagnostic-contract/schemas/nq.diagnostic_execution.v1.schema.json",
                schema.as_slice(),
            ),
        ];
        let payload = package_files
            .iter()
            .map(|(path, bytes)| {
                format!("{}  ./{path}\n", sha(bytes).trim_start_matches("sha256:"))
            })
            .collect::<String>();
        write(
            &package.join("share/nq/MANIFEST.sha256"),
            payload.as_bytes(),
        );

        write(&source.join("positive.json"), POSITIVE);
        let key = DiagnosticKey {
            question_id: artifact.question.id,
            subject_id: artifact.subject.id,
            profile_id: artifact.profile.id,
            vantage_id: artifact.vantage.id,
        };
        let mut manifest = NqSourceManifest {
            schema: NQ_SOURCE_MANIFEST_SCHEMA.into(),
            source_manifest_id: String::new(),
            package: NqPackagePin {
                repository_identity: "nq-ng".into(),
                commit: "a".repeat(40),
                release_identity: "nq-ng:test-package".into(),
                contract_schema: NQ_DIAGNOSTIC_EXECUTION_SCHEMA.into(),
                asset_root: "share/nq/diagnostic-contract".into(),
                asset_manifest_path: "share/nq/diagnostic-contract/manifest.json".into(),
                asset_manifest_sha256: sha(&static_manifest),
                payload_manifest_path: "share/nq/MANIFEST.sha256".into(),
                payload_manifest_sha256: sha(payload.as_bytes()),
            },
            inputs: vec![NqSourceEntry {
                key,
                status: NqSourceStatus::Delivered {
                    artifact_path: "positive.json".into(),
                    artifact_sha256: sha(POSITIVE),
                    artifact_id: artifact.artifact_id,
                },
            }],
        };
        manifest.source_manifest_id = manifest.computed_source_manifest_id().unwrap();
        let source_manifest = source.join("sources.json");
        write(&source_manifest, &serde_jcs::to_vec(&manifest).unwrap());
        (temp, package, source_manifest)
    }

    #[test]
    fn exact_package_and_artifact_import_produces_attributed_receipt() {
        let (_temp, package, source) = package_and_source();
        let imported = load_diagnostic_sources(&source, &package).unwrap();
        assert_eq!(imported.inputs.inputs.len(), 1);
        assert_eq!(
            imported.receipt.source_manifest.package.repository_identity,
            "nq-ng"
        );
        assert_eq!(
            imported.receipt.imported_inputs_id,
            imported.inputs.inputs_id
        );
        imported.receipt.validate().unwrap();
    }

    #[test]
    fn package_asset_tamper_and_path_traversal_fail_closed() {
        let (_temp, package, source) = package_and_source();
        write(
            &package.join("share/nq/diagnostic-contract/README.md"),
            b"tampered\n",
        );
        assert!(load_diagnostic_sources(&source, &package)
            .unwrap_err()
            .contains("digest mismatch"));
        assert!(validate_relative_path("../artifact.json", "test").is_err());
        assert!(validate_relative_path("a//b", "test").is_err());
        assert!(validate_relative_path("a/./b", "test").is_err());
    }

    #[test]
    fn source_entries_reject_unknown_fields_despite_flattened_status() {
        let (_temp, _package, source) = package_and_source();
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(source).unwrap()).unwrap();
        value["inputs"][0]["invented_authority"] = serde_json::json!(true);
        assert!(serde_json::from_value::<NqSourceManifest>(value).is_err());
    }

    #[test]
    fn contract_version_selects_its_exact_qualified_asset_namespace() {
        let digest = sha(b"manifest");
        let mut pin = NqPackagePin {
            repository_identity: "nq-ng".into(),
            commit: "a".repeat(40),
            release_identity: "nq-ng:test-package".into(),
            contract_schema: NQ_DIAGNOSTIC_EXECUTION_V2_SCHEMA.into(),
            asset_root: "share/nq/diagnostic-contract-v2".into(),
            asset_manifest_path: "share/nq/diagnostic-contract-v2/manifest.json".into(),
            asset_manifest_sha256: digest.clone(),
            payload_manifest_path: "share/nq/MANIFEST.sha256".into(),
            payload_manifest_sha256: digest,
        };
        validate_package_pin(&pin).unwrap();

        pin.asset_root = "share/nq/diagnostic-contract".into();
        pin.asset_manifest_path = "share/nq/diagnostic-contract/manifest.json".into();
        assert!(validate_package_pin(&pin)
            .unwrap_err()
            .contains("qualified NQ diagnostic-contract layout"));
    }

    #[test]
    fn package_fixture_gate_requires_valid_acceptance_and_hostile_rejection() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("package");
        write(
            &root.join("share/nq/diagnostic-contract/fixtures/positive.json"),
            POSITIVE,
        );
        write(
            &root.join("share/nq/diagnostic-contract/fixtures/hostile.json"),
            HOSTILE,
        );
        let positive: DiagnosticExecutionV1 = serde_json::from_slice(POSITIVE).unwrap();
        let hostile: DiagnosticExecutionV1 = serde_json::from_slice(HOSTILE).unwrap();
        let pin = NqPackagePin {
            repository_identity: "nq-ng".into(),
            commit: "a".repeat(40),
            release_identity: "nq-ng:test-package".into(),
            contract_schema: NQ_DIAGNOSTIC_EXECUTION_SCHEMA.into(),
            asset_root: "share/nq/diagnostic-contract".into(),
            asset_manifest_path: "share/nq/diagnostic-contract/manifest.json".into(),
            asset_manifest_sha256: sha(b"manifest"),
            payload_manifest_path: "share/nq/MANIFEST.sha256".into(),
            payload_manifest_sha256: sha(b"payload"),
        };
        let mut fixture = ContractAssetFixture {
            id: "positive".into(),
            class: "valid".into(),
            path: "fixtures/positive.json".into(),
            sha256: sha(POSITIVE),
            artifact_id: positive.artifact_id,
            expected_disposition: "accepted".into(),
            expected_error: None,
        };
        verify_contract_fixture(&root, &pin, &fixture).unwrap();

        fixture.class = "hostile".into();
        fixture.expected_disposition = "rejected_semantic_invariant".into();
        fixture.expected_error = Some("must reject".into());
        assert!(verify_contract_fixture(&root, &pin, &fixture)
            .unwrap_err()
            .contains("accepted hostile"));

        fixture.id = "hostile".into();
        fixture.path = "fixtures/hostile.json".into();
        fixture.sha256 = sha(HOSTILE);
        fixture.artifact_id = hostile.artifact_id;
        verify_contract_fixture(&root, &pin, &fixture).unwrap();

        fixture.class = "valid".into();
        fixture.expected_disposition = "accepted".into();
        fixture.expected_error = None;
        assert!(verify_contract_fixture(&root, &pin, &fixture)
            .unwrap_err()
            .contains("rejected valid"));
    }
}
