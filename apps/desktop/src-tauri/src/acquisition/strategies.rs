//! Built-in strategy catalog. Qualification is independent for each acquisition route.
use super::*;

pub struct AcquisitionStrategyRegistry {
    entries: Vec<StrategyDefinition>,
}
struct StrategyDefinition {
    strategy: FirmwareAcquisitionStrategy,
    name: &'static str,
    provenance: &'static str,
    reason: &'static str,
    prerequisites: &'static [&'static str],
}
impl Default for AcquisitionStrategyRegistry {
    fn default() -> Self {
        Self { entries: vec![
            StrategyDefinition {strategy:FirmwareAcquisitionStrategy::OfficialImage,name:"Official firmware image",provenance:"official_image",reason:"No exact applicable publisher artifact is registered. This does not establish device incompatibility.",prerequisites:&["Publisher-qualified source", "Exact region/carrier and image type", "Publisher integrity/authenticity evidence", "Reviewed acquisition worker"]},
            StrategyDefinition {strategy:FirmwareAcquisitionStrategy::DocumentedExport,name:"Read-only device export",provenance:"device_export",reason:"No documented or exact-hardware-qualified firmware export is registered. A configuration backup is not firmware.",prerequisites:&["Documented or exact-hardware-qualified firmware export", "Endpoint trust and authentication", "Explicit acquisition approval", "Reviewed export worker and binary validation"]},
            StrategyDefinition {strategy:FirmwareAcquisitionStrategy::AuthorizedStorageRead,name:"Authorized firmware storage read",provenance:"partition_dump",reason:"No qualified bounded storage-read implementation is registered.",prerequisites:&["Model-qualified read-only storage adapter", "Exact permitted ranges and exclusions", "Coverage and recovery requirements", "Authentication and explicit read approval"]},
            StrategyDefinition {strategy:FirmwareAcquisitionStrategy::ManufacturerPackage,name:"Manufacturer update package",provenance:"manufacturer_package",reason:"No applicable authenticated OEM package source is registered. Updaters are never executed.",prerequisites:&["Authentic applicable OEM package source", "Exact region/carrier variant", "Non-executing package parser", "Integrity/authenticity validation"]},
            StrategyDefinition {strategy:FirmwareAcquisitionStrategy::ExternalLaboratory,name:"External laboratory acquisition",provenance:"external_physical_backup",reason:"Future expert-assisted workflow; no invasive operation is implemented.",prerequisites:&["External tools and separate owner approval", "Hardware-qualified read and coverage procedure"]},
        ] }
    }
}
impl AcquisitionStrategyRegistry {
    pub fn evaluate(
        &self,
        identity: &DeviceIdentityAssessment,
    ) -> Vec<FirmwareAcquisitionCandidate> {
        self.entries
            .iter()
            .map(|entry| {
                let mut missing = Vec::new();
                if identity.verified_manufacturer.is_none() {
                    missing.push("Trusted manufacturer identity".into());
                }
                if identity.verified_firmware_family.is_none() {
                    missing.push("Verified firmware family".into());
                }
                if identity.verified_model.is_none() {
                    missing.push("Trusted exact manufacturer/model".into());
                }
                if identity.verified_hardware_revision.is_none() {
                    missing.push("Exact hardware revision".into());
                }
                if identity.verified_firmware.is_none() {
                    missing.push("Exact firmware family and version".into());
                }
                missing.extend(entry.prerequisites.iter().map(|p| (*p).into()));
                let mut decision = capability(CapabilityState::Unknown, entry.reason, &[]);
                decision.missing_requirements = missing;
                // No executable source/export/storage provider is registered in Sprint A.
                // Identity recognition alone cannot promote a route to available_verified.
                FirmwareAcquisitionCandidate {
                    strategy: entry.strategy,
                    name: entry.name.into(),
                    provenance_kind: entry.provenance.into(),
                    capability: decision,
                }
            })
            .collect()
    }
}
