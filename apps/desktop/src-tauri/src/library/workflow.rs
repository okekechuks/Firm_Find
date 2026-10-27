//! Device workflow is a fresh projection of evidence plus durable Library jobs.
//! It grants no device transport, login, write, build or installation authority.
use super::*;
use crate::{
    acquisition::{self, CapabilityState, DeviceIdentityAssessment},
    discovery::{identity::Category, Device, DiscoverySnapshot, DiscoveryStatus},
    profiles::{self, EvidenceRecord},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowBinding {
    pub device_id: String,
    pub revision: u64,
    pub identity_digest: String,
    pub expires_at: u64,
}
pub(super) struct AcquisitionFence {
    pub binding: WorkflowBinding,
    pub live: Arc<Mutex<DiscoverySnapshot>>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stage {
    pub id: String,
    pub state: CapabilityState,
    pub reason_code: String,
    pub reason: String,
    pub evidence: Vec<String>,
    pub next_action: String,
    pub executable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job: Option<JobRecord>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowArtifact {
    pub artifact: FirmwareArtifact,
    pub source: official::OfficialSource,
    pub association: String,
    pub analysis: Option<analysis::AnalysisReport>,
    pub extracted: bool,
    pub can_analyze: bool,
    pub can_extract: bool,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowRecord {
    pub device: Device,
    pub binding: WorkflowBinding,
    pub identity: DeviceIdentityAssessment,
    pub selected_profile: Option<String>,
    pub adapter: Option<String>,
    pub stages: Vec<Stage>,
    pub strategies: Vec<acquisition::FirmwareAcquisitionCandidate>,
    pub official_sources: Vec<official::OfficialSource>,
    pub artifacts: Vec<WorkflowArtifact>,
    pub jobs: Vec<JobRecord>,
    pub configuration: Vec<Stage>,
}

fn blocked(code: &str, message: &str) -> LibraryError {
    LibraryError::new(code, message)
}
fn evidence(repo: &Repository, device: &Device) -> Result<Vec<EvidenceRecord>> {
    let mut all = profiles::gather_evidence(
        device,
        &repo
            .routers()
            .map_err(|_| blocked("EVIDENCE_UNAVAILABLE", "Saved device evidence unavailable."))?,
    );
    all.extend(repo.identity_observations(&device.id).map_err(|_| {
        blocked(
            "EVIDENCE_UNAVAILABLE",
            "Device identity evidence unavailable.",
        )
    })?);
    Ok(all)
}
fn device<'a>(snapshot: &'a DiscoverySnapshot, id: &str) -> Result<&'a Device> {
    snapshot.devices.iter().find(|d| d.id == id).ok_or_else(|| {
        blocked(
            "DEVICE_NOT_FOUND",
            "Select a current device. Previous Library artifacts remain available.",
        )
    })
}
fn identity_digest(identity: &DeviceIdentityAssessment) -> String {
    // Incidental Windows scan timestamps and user labels must not revoke an
    // unchanged approved public-image download. Authority still binds the
    // inventory revision and the complete trusted identity evidence.
    let trusted: Vec<_> = identity
        .evidence
        .iter()
        .filter(|e| {
            e.verification == profiles::Verification::Verified
                && e.reliability == profiles::Reliability::Strong
                && matches!(
                    e.source,
                    profiles::EvidenceOrigin::AuthenticatedManagement
                        | profiles::EvidenceOrigin::SupportedModem
                )
        })
        .collect();
    format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(&(identity.confidence.as_str(), trusted))
                .expect("serializable identity")
        )
    )
}
fn current(snapshot: &DiscoverySnapshot, device: &Device, time: u64) -> bool {
    device.connected
        && !snapshot.cancelled
        && matches!(
            snapshot.status,
            DiscoveryStatus::Ready | DiscoveryStatus::Degraded
        )
        && snapshot
            .scanned_at
            .is_some_and(|at| at <= time && time - at < profiles::EVIDENCE_TTL_MS)
}
fn eligible(device: &Device, identity: &DeviceIdentityAssessment) -> bool {
    if !matches!(
        device.category,
        Category::RouterMifiCandidate
            | Category::UsbModemCandidate
            | Category::NetworkGatewayCandidate
    ) {
        return false;
    }
    // An authenticated observation must still refer to an active local interface.
    identity.evidence.iter().any(|e| {
        e.verification == profiles::Verification::Verified
            && e.reliability == profiles::Reliability::Strong
            && matches!(
                e.source,
                profiles::EvidenceOrigin::AuthenticatedManagement
                    | profiles::EvidenceOrigin::SupportedModem
            )
            && e.interface_id.as_ref().is_some_and(|id| {
                device
                    .networks
                    .iter()
                    .any(|n| &n.id == id && n.link_up && !n.virtual_interface)
            })
            && e.endpoint.as_ref().is_some_and(|endpoint| {
                url::Url::parse(endpoint).is_ok_and(|url| {
                    device.networks.iter().any(|n| {
                        e.interface_id.as_ref() == Some(&n.id)
                            && url
                                .host_str()
                                .is_some_and(|host| n.gateways.iter().any(|g| g == host))
                    })
                })
            })
    })
}

/// Exact tuples only. Labels, USB IDs, HTTP success and partial revisions never match.
pub fn matching_sources(
    identity: &DeviceIdentityAssessment,
) -> Result<Vec<official::OfficialSource>> {
    if identity.confidence != "verified" {
        return Ok(vec![]);
    }
    let (Some(vendor), Some(model), Some(revision)) = (
        &identity.verified_manufacturer,
        &identity.verified_model,
        &identity.verified_hardware_revision,
    ) else {
        return Ok(vec![]);
    };
    let matches: Vec<_> = official::catalogue()?
        .into_iter()
        .filter(|s| {
            s.vendor.eq_ignore_ascii_case(vendor)
                && s.model.eq_ignore_ascii_case(model)
                && s.revision.eq_ignore_ascii_case(revision)
        })
        .collect();
    // Multiple applicable releases need an independently qualified selection policy.
    if matches.len() == 1 {
        Ok(matches)
    } else {
        Ok(vec![])
    }
}
fn assess(
    repo: &Repository,
    snapshot: &DiscoverySnapshot,
    id: &str,
    time: u64,
) -> Result<(
    DeviceIdentityAssessment,
    WorkflowBinding,
    Vec<official::OfficialSource>,
)> {
    let device = device(snapshot, id)?;
    let identity = acquisition::identity(evidence(repo, device)?, time, None, None);
    let expires_at = identity
        .evidence
        .iter()
        .filter(|e| {
            !e.stale
                && e.collected_at <= time
                && time - e.collected_at < profiles::EVIDENCE_TTL_MS
                && e.verification == profiles::Verification::Verified
        })
        .map(|e| e.collected_at.saturating_add(profiles::EVIDENCE_TTL_MS))
        .min()
        .unwrap_or(time)
        .min(
            snapshot
                .scanned_at
                .unwrap_or(0)
                .saturating_add(profiles::EVIDENCE_TTL_MS),
        );
    let binding = WorkflowBinding {
        device_id: id.into(),
        revision: snapshot.revision,
        identity_digest: identity_digest(&identity),
        expires_at,
    };
    let sources = if current(snapshot, device, time) && eligible(device, &identity) {
        matching_sources(&identity)?
    } else {
        vec![]
    };
    Ok((identity, binding, sources))
}
fn stage(
    id: &str,
    state: CapabilityState,
    code: &str,
    reason: &str,
    next: &str,
    executable: bool,
    evidence: &[String],
) -> Stage {
    Stage {
        id: id.into(),
        state,
        reason_code: code.into(),
        reason: reason.into(),
        evidence: evidence.to_vec(),
        next_action: next.into(),
        executable,
        job: None,
    }
}
pub(super) fn validate_acquisition(
    repo: &Repository,
    snapshot: &DiscoverySnapshot,
    expected: &WorkflowBinding,
    source_id: &str,
    time: u64,
) -> Result<()> {
    let (_, actual, sources) = assess(repo, snapshot, &expected.device_id, time)?;
    if actual != *expected
        || time >= expected.expires_at
        || !sources.iter().any(|s| s.id == source_id)
    {
        return Err(blocked("WORKFLOW_BINDING_CHANGED", "Device identity, revision, interface or evidence changed/expired. No successful firmware acquisition was published. Check the device again."));
    }
    Ok(())
}
impl LibraryService {
    pub fn device_workflow(
        &self,
        snapshot: &DiscoverySnapshot,
        id: &str,
        registry: &profiles::Registry,
    ) -> Result<WorkflowRecord> {
        let time = now();
        let repo = lock(&self.inner.repo)?;
        let device = device(snapshot, id)?.clone();
        let (identity, binding, sources) = assess(&repo, snapshot, id, time)?;
        let compatibility = profiles::evaluate(
            registry,
            &device,
            identity.evidence.clone(),
            time,
            snapshot.revision,
        );
        let ids: Vec<_> = identity.evidence.iter().map(|e| e.id.clone()).collect();
        let jobs = repo.workflow_jobs(id)?;
        drop(repo);
        let mut artifacts = vec![];
        let catalogue = official::catalogue()?;
        for job in jobs.iter().filter(|j| {
            j.kind == "official_image_download"
                && j.status == JobStatus::Completed
                && j.device_id.as_deref() == Some(id)
        }) {
            let Some(hash) = &job.related_artifact else {
                continue;
            };
            if artifacts
                .iter()
                .any(|a: &WorkflowArtifact| &a.artifact.sha256 == hash)
            {
                continue;
            }
            let Some(source) = catalogue.iter().find(|s| &s.sha256 == hash) else {
                continue;
            };
            let details = match self.details(hash) {
                Ok(value) => value,
                Err(e) if e.code == "ARTIFACT_NOT_FOUND" => continue,
                Err(e) => return Err(e),
            };
            // A content-address match alone never substitutes for official provenance.
            if !self.official_provenance(hash)?.contains(source) {
                continue;
            }
            let analysis = self.analysis_report(hash)?;
            let extracted = self.extraction_report(hash)?.is_some();
            let can_analyze = details.artifact.storage_status == "present"
                && details.artifact.size_bytes <= 512 * 1024 * 1024;
            let can_extract = can_analyze
                && analysis.as_ref().is_some_and(|a| {
                    a.regions
                        .iter()
                        .any(|r| r.format == "SquashFS little endian")
                        || a.format == "gzip wrapper"
                        || a.format == "TRX container"
                });
            artifacts.push(WorkflowArtifact {
                artifact: details.artifact,
                source: source.clone(),
                association: "historical_verified_target_official_image_not_device_dump".into(),
                analysis,
                extracted,
                can_analyze,
                can_extract,
            });
        }
        let mut strategies =
            acquisition::AcquisitionStrategyRegistry::default().evaluate(&identity);
        if let Some(candidate) = strategies.iter_mut().find(|c| {
            matches!(
                c.strategy,
                acquisition::FirmwareAcquisitionStrategy::OfficialImage
            )
        }) {
            candidate.capability.state = if sources.is_empty() {
                CapabilityState::Unknown
            } else {
                CapabilityState::RequiresApproval
            };
            candidate.capability.executable = !sources.is_empty();
            candidate.capability.reason = if sources.is_empty() { "No unique qualified official image matches fresh, trusted manufacturer/model/revision evidence on the selected active interface." } else { "Exact catalogue target matched. This downloads a compatible-target official image; it does not read installed firmware or authorize installation." }.into();
            candidate.capability.evidence = ids.clone();
            if !sources.is_empty() {
                candidate.capability.missing_requirements =
                    vec!["Explicit approval of this exact public image source".into()];
            }
        }
        let a = artifacts.first();
        let mut stages = vec![
            stage("discovery", if current(snapshot, &device, time) {CapabilityState::Observed} else {CapabilityState::Unknown}, "WINDOWS_OBSERVATION", "Windows inventory observes connection hardware; this is not verified router identity.", "Connect/power on device and refresh.", false, &ids),
            stage("identification", if identity.confidence == "verified" {CapabilityState::AvailableVerified} else {CapabilityState::Unknown}, "EXACT_IDENTITY_REQUIRED", "Observed and user-reported fields remain separate from trusted identity. Conflicting or missing revisions block acquisition.", "Qualify an approved exact identity reader for this firmware family.", false, &ids),
            stage("acquisition", if !artifacts.is_empty() {CapabilityState::CompletedVerified} else if !sources.is_empty() {CapabilityState::RequiresApproval} else {CapabilityState::Unknown}, if sources.is_empty() {"NO_QUALIFIED_SOURCE"} else {"OFFICIAL_TARGET_IMAGE"}, "Official images, installed-device dumps, settings snapshots and imported files retain distinct provenance. No physical firmware-read route is enabled.", "Review exact source and explicitly approve the public HTTPS download.", !sources.is_empty(), &ids),
            stage("analysis", if a.is_some_and(|a| a.analysis.is_some()) {CapabilityState::CompletedVerified} else {CapabilityState::NotChecked}, "BOUNDED_FORMAT_ANALYSIS", "Inspect the preserved image; signatures alone do not verify partitions or hardware support.", "Analyze acquired firmware.", a.is_some_and(|a| a.can_analyze), &ids),
            stage("extraction", if a.is_some_and(|a| a.extracted) {CapabilityState::CompletedVerified} else if a.is_some_and(|a| a.can_extract) {CapabilityState::RequiresExternalTool} else {CapabilityState::Unknown}, "QUALIFIED_SANDBOX_REQUIRED", "Supported filesystem candidates require the pinned WSL sandbox. Only successful validated outputs are published; extraction is not a complete flash backup.", "Analyze first; install qualified tools if the worker reports them unavailable.", a.is_some_and(|a| a.can_extract), &ids),
            stage("customization", CapabilityState::NotImplemented, "EXACT_BUILD_ADAPTER_REQUIRED", "Extraction does not authorize firmware changes. The separate OpenWrt DNS helper requires independently qualified build inputs.", "Qualify exact target build inputs, layout, preserved partitions and recovery constraints.", false, &ids),
            stage("rebuild", CapabilityState::NotImplemented, "NO_INTEGRATED_QUALIFIED_BUILDER", "No integrated builder has qualified this device, revision and original image. Signatures and size limits remain unverified.", "Qualify a target-specific builder and output validation.", false, &ids),
            stage("installation", CapabilityState::Unsupported, "INSTALLATION_DISABLED", "No qualified installation or signature-bypass operation is exposed.", "Independently qualify installation, validation and recovery before enabling.", false, &ids),
            stage("recovery", CapabilityState::Unknown, "RECOVERY_UNQUALIFIED", "No device-specific recovery route is qualified by this workflow.", "Document and independently qualify exact-hardware recovery.", false, &ids),
        ];
        let configuration = ["router_dns", "client_dns", "apn", "wifi_client_limit", "ussd", "modem_parameters", "overlay", "configuration_replacement", "packages", "filesystem_inputs", "preserved_partitions", "layout_constraints"].into_iter().map(|id| stage(id, CapabilityState::NotImplemented, "NO_WORKFLOW_EXECUTOR", "No qualified executor for this device/image. Live configuration and firmware customization require separate authorization.", "Qualify the exact platform capability and its verification/recovery contract.", false, &ids)).collect();
        for stage in &mut stages {
            let kind = match stage.id.as_str() {
                "acquisition" => "official_image_download",
                "analysis" => "firmware_analysis",
                "extraction" => "firmware_extraction",
                _ => continue,
            };
            stage.job = jobs
                .iter()
                .find(|job| {
                    job.kind == kind
                        && if kind == "official_image_download" {
                            job.device_id.as_deref() == Some(id)
                        } else {
                            a.is_some_and(|a| {
                                job.related_artifact.as_deref() == Some(&a.artifact.sha256)
                            })
                        }
                })
                .cloned();
            if let Some(job) = &stage.job {
                if matches!(job.status, JobStatus::Failed | JobStatus::Cancelled)
                    && stage.state != CapabilityState::CompletedVerified
                {
                    stage.state = CapabilityState::Failed;
                    stage.reason_code = job
                        .error_code
                        .clone()
                        .unwrap_or_else(|| job.status.text().to_uppercase());
                    stage.reason.clone_from(&job.message);
                    stage.next_action = "Resolve the reported cause, refresh current evidence and explicitly retry. Prior verified outputs remain preserved.".into();
                }
            }
        }
        Ok(WorkflowRecord {
            device,
            binding,
            identity,
            selected_profile: compatibility.selected_profile,
            adapter: None,
            stages,
            strategies,
            official_sources: sources,
            artifacts,
            jobs,
            configuration,
        })
    }
    pub fn acquire_for_device(
        &self,
        live: Arc<Mutex<DiscoverySnapshot>>,
        expected: WorkflowBinding,
        source_id: &str,
        confirmed: bool,
    ) -> Result<JobRecord> {
        if !confirmed {
            return Err(blocked("SOURCE_APPROVAL_REQUIRED", "Approve this exact official source and hardware revision first. No request was sent."));
        }
        let repo = lock(&self.inner.repo)?;
        let snapshot = lock(&live)?;
        validate_acquisition(&repo, &snapshot, &expected, source_id, now())?;
        drop(snapshot);
        drop(repo);
        self.queue_official(
            source_id,
            true,
            Some(AcquisitionFence {
                binding: expected,
                live,
            }),
        )
    }
}

pub mod ipc {
    use super::*;
    use crate::discovery::DiscoveryService;
    #[tauri::command]
    pub async fn get_device_workflow(
        device_id: String,
        discovery: tauri::State<'_, DiscoveryService>,
        service: tauri::State<'_, LibraryService>,
        profiles: tauri::State<'_, profiles::Registry>,
    ) -> Result<WorkflowRecord> {
        let snapshot = discovery
            .snapshot()
            .map_err(|_| blocked("DISCOVERY_UNAVAILABLE", "Refresh discovery first."))?;
        let service = service.inner().clone();
        let profiles = profiles.inner().clone();
        tauri::async_runtime::spawn_blocking(move || {
            service.device_workflow(&snapshot, &device_id, &profiles)
        })
        .await
        .map_err(|_| blocked("WORKER_FAILED", "Device workflow unavailable."))?
    }
    #[tauri::command]
    pub async fn start_device_firmware_acquisition(
        binding: WorkflowBinding,
        source_id: String,
        confirmed: bool,
        discovery: tauri::State<'_, DiscoveryService>,
        service: tauri::State<'_, LibraryService>,
    ) -> Result<JobRecord> {
        let live = discovery.shared_snapshot();
        let service = service.inner().clone();
        tauri::async_runtime::spawn_blocking(move || {
            service.acquire_for_device(live, binding, &source_id, confirmed)
        })
        .await
        .map_err(|_| blocked("WORKER_FAILED", "Device workflow unavailable."))?
    }
}

#[cfg(any(test, feature = "native-test-profile"))]
pub(crate) mod lab;
#[cfg(test)]
mod tests;
