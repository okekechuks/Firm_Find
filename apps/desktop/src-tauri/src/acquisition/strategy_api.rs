use super::{CapabilityState, DeviceIdentityAssessment, FirmwareAcquisitionStrategy};
use serde::{Deserialize, Serialize};

/// Stable strategy identifiers. Serde names match the existing schema-1 wire IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StrategyId {
    OfficialImage,
    DocumentedExport,
    AuthorizedStorageRead,
    ManufacturerPackage,
    ExternalLaboratory,
}

impl StrategyId {
    pub const ALL: [Self; 5] = [
        Self::OfficialImage,
        Self::DocumentedExport,
        Self::AuthorizedStorageRead,
        Self::ManufacturerPackage,
        Self::ExternalLaboratory,
    ];

    pub fn wire_id(self) -> &'static str {
        match self {
            Self::OfficialImage => "official_image",
            Self::DocumentedExport => "documented_export",
            Self::AuthorizedStorageRead => "authorized_storage_read",
            Self::ManufacturerPackage => "manufacturer_package",
            Self::ExternalLaboratory => "external_laboratory",
        }
    }

    pub(crate) fn report_id(self) -> FirmwareAcquisitionStrategy {
        match self {
            Self::OfficialImage => FirmwareAcquisitionStrategy::OfficialImage,
            Self::DocumentedExport => FirmwareAcquisitionStrategy::DocumentedExport,
            Self::AuthorizedStorageRead => FirmwareAcquisitionStrategy::AuthorizedStorageRead,
            Self::ManufacturerPackage => FirmwareAcquisitionStrategy::ManufacturerPackage,
            Self::ExternalLaboratory => FirmwareAcquisitionStrategy::ExternalLaboratory,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StrategyKind {
    OfficialImage,
    DeviceExport,
    StorageRead,
    ManufacturerPackage,
    ExternalLaboratory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Requirement {
    VerifiedManufacturer,
    VerifiedFirmwareFamily,
    VerifiedModel,
    VerifiedHardwareRevision,
    VerifiedFirmware,
    CurrentDiscoveryGeneration,
    NoIdentityConflict,
    Qualified(&'static str),
}

impl Requirement {
    pub fn description(&self) -> &'static str {
        match self {
            Self::VerifiedManufacturer => "Trusted manufacturer identity",
            Self::VerifiedFirmwareFamily => "Verified firmware family",
            Self::VerifiedModel => "Trusted exact manufacturer/model",
            Self::VerifiedHardwareRevision => "Exact hardware revision",
            Self::VerifiedFirmware => "Exact firmware family and version",
            Self::CurrentDiscoveryGeneration => "Current discovery generation",
            Self::NoIdentityConflict => "No conflicting identity evidence",
            Self::Qualified(description) => description,
        }
    }

    pub fn is_satisfied(&self, context: &EvaluationContext<'_>) -> bool {
        match self {
            Self::VerifiedManufacturer => context.identity.verified_manufacturer.is_some(),
            Self::VerifiedFirmwareFamily => context.identity.verified_firmware_family.is_some(),
            Self::VerifiedModel => context.identity.verified_model.is_some(),
            Self::VerifiedHardwareRevision => context.identity.verified_hardware_revision.is_some(),
            Self::VerifiedFirmware => context.identity.verified_firmware.is_some(),
            Self::CurrentDiscoveryGeneration => context.generation_is_current(),
            Self::NoIdentityConflict => context.identity.confidence != "conflicting",
            Self::Qualified(_) => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Applicability {
    pub state: CapabilityState,
    pub evidence_ids: Vec<String>,
    pub missing: Vec<Requirement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeReport {
    pub strategy: StrategyId,
    pub state: CapabilityState,
    pub evidence_ids: Vec<String>,
    pub missing: Vec<Requirement>,
    pub limitations: Vec<LimitationCode>,
    pub executable: bool,
}

impl ProbeReport {
    pub fn not_implemented(strategy: StrategyId) -> Self {
        Self {
            strategy,
            state: CapabilityState::NotImplemented,
            evidence_ids: vec![],
            missing: vec![],
            limitations: vec![LimitationCode::NotImplemented],
            executable: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcquisitionPlan {
    pub strategy: StrategyId,
    pub evidence_ids: Vec<String>,
    pub missing: Vec<Requirement>,
    pub limitations: Vec<LimitationCode>,
    pub executable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LimitationCode {
    NotImplemented,
    RequirementsMissing,
    StaleEvidence,
    IdentityConflict,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum StrategyError {
    #[error("Approval scope does not authorize this strategy probe")]
    ApprovalScopeMismatch,
    #[error("Probe report belongs to another strategy")]
    StrategyMismatch,
}

/// Strategy probes receive no client, transport, filesystem or command interface.
#[derive(Debug, Default)]
pub struct ProbeContext {
    _sealed: (),
}

impl ProbeContext {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Pure inputs only. The user label is deliberately excluded from all decisions.
#[derive(Debug, Clone, Copy)]
pub struct EvaluationContext<'a> {
    pub identity: &'a DeviceIdentityAssessment,
    user_label: Option<&'a str>,
    generation: Option<(u64, u64)>,
}

impl<'a> EvaluationContext<'a> {
    pub fn new(identity: &'a DeviceIdentityAssessment) -> Self {
        Self {
            identity,
            user_label: None,
            generation: None,
        }
    }

    pub fn with_user_label(mut self, label: &'a str) -> Self {
        self.user_label = Some(label);
        self
    }

    pub fn with_generation(mut self, current: u64, evidence: u64) -> Self {
        self.generation = Some((current, evidence));
        self
    }

    pub fn user_label(&self) -> Option<&'a str> {
        self.user_label
    }

    fn generation_is_current(&self) -> bool {
        self.generation
            .is_none_or(|(current, evidence)| current == evidence)
    }
}

pub use super::approval::StrategyProbeApproval;

pub trait AcquisitionStrategy: Send + Sync {
    fn id(&self) -> StrategyId;
    fn kind(&self) -> StrategyKind;
    /// Current stubs share this state with their probe; no dispatch is performed.
    fn implementation_state(&self) -> CapabilityState {
        CapabilityState::NotImplemented
    }
    fn qualified_requirements(&self) -> &'static [&'static str];

    fn applicability(&self, context: &EvaluationContext<'_>) -> Applicability {
        let mut missing = [
            Requirement::VerifiedManufacturer,
            Requirement::VerifiedFirmwareFamily,
            Requirement::VerifiedModel,
            Requirement::VerifiedHardwareRevision,
            Requirement::VerifiedFirmware,
        ]
        .into_iter()
        .filter(|requirement| !requirement.is_satisfied(context))
        .collect::<Vec<_>>();
        missing.extend(
            self.qualified_requirements()
                .iter()
                .map(|requirement| Requirement::Qualified(requirement)),
        );
        Applicability {
            // Identity alone cannot qualify any acquisition implementation.
            state: CapabilityState::Unknown,
            evidence_ids: vec![],
            missing,
        }
    }

    fn probe(
        &self,
        _context: &mut ProbeContext,
        approval: &StrategyProbeApproval,
    ) -> Result<ProbeReport, StrategyError> {
        if !approval.authorizes(self.id()) {
            return Err(StrategyError::ApprovalScopeMismatch);
        }
        Ok(ProbeReport::not_implemented(self.id()))
    }

    fn plan(
        &self,
        context: &EvaluationContext<'_>,
        probe: &ProbeReport,
    ) -> Result<AcquisitionPlan, StrategyError> {
        if probe.strategy != self.id() {
            return Err(StrategyError::StrategyMismatch);
        }
        let applicability = self.applicability(context);
        let mut missing = applicability.missing;
        for requirement in &probe.missing {
            if !missing.contains(requirement) {
                missing.push(requirement.clone());
            }
        }
        let mut limitations = probe.limitations.clone();
        if !context.generation_is_current() {
            missing.push(Requirement::CurrentDiscoveryGeneration);
            limitations.push(LimitationCode::StaleEvidence);
        }
        if context.identity.confidence == "conflicting" {
            missing.push(Requirement::NoIdentityConflict);
            limitations.push(LimitationCode::IdentityConflict);
        }
        if !missing.is_empty() && !limitations.contains(&LimitationCode::RequirementsMissing) {
            limitations.push(LimitationCode::RequirementsMissing);
        }
        Ok(AcquisitionPlan {
            strategy: self.id(),
            evidence_ids: vec![],
            missing,
            limitations,
            // There is deliberately no executor or acquisition authority.
            executable: false,
        })
    }
}
