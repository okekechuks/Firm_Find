//! Closed compiled registry. Strategy implementations are pure stubs until separately reviewed.
use super::*;

pub mod authorized_storage_read;
pub mod documented_export;
pub mod external_laboratory;
pub mod manufacturer_package;
pub mod official_image;

struct StrategyDefinition {
    name: &'static str,
    provenance: &'static str,
    reason: &'static str,
}

fn definition(id: StrategyId) -> StrategyDefinition {
    match id {
        StrategyId::OfficialImage => StrategyDefinition {
            name: "Official firmware image",
            provenance: "official_image",
            reason: "No exact applicable publisher artifact is registered. This does not establish device incompatibility.",
        },
        StrategyId::DocumentedExport => StrategyDefinition {
            name: "Read-only device export",
            provenance: "device_export",
            reason: "No documented or exact-hardware-qualified firmware export is registered. A configuration backup is not firmware.",
        },
        StrategyId::AuthorizedStorageRead => StrategyDefinition {
            name: "Authorized firmware storage read",
            provenance: "partition_dump",
            reason: "No qualified bounded storage-read implementation is registered.",
        },
        StrategyId::ManufacturerPackage => StrategyDefinition {
            name: "Manufacturer update package",
            provenance: "manufacturer_package",
            reason: "No applicable authenticated OEM package source is registered. Updaters are never executed.",
        },
        StrategyId::ExternalLaboratory => StrategyDefinition {
            name: "External laboratory acquisition",
            provenance: "external_physical_backup",
            reason: "Future expert-assisted workflow; no invasive operation is implemented.",
        },
    }
}

pub struct AcquisitionStrategyRegistry {
    entries: Vec<Box<dyn AcquisitionStrategy>>,
}

impl Default for AcquisitionStrategyRegistry {
    fn default() -> Self {
        Self {
            entries: vec![
                Box::new(official_image::OfficialImage),
                Box::new(documented_export::DocumentedExport),
                Box::new(authorized_storage_read::AuthorizedStorageRead),
                Box::new(manufacturer_package::ManufacturerPackage),
                Box::new(external_laboratory::ExternalLaboratory),
            ],
        }
    }
}

impl AcquisitionStrategyRegistry {
    pub fn strategies(&self) -> impl Iterator<Item = &dyn AcquisitionStrategy> {
        self.entries.iter().map(Box::as_ref)
    }

    pub fn get(&self, id: StrategyId) -> Option<&dyn AcquisitionStrategy> {
        self.strategies().find(|strategy| strategy.id() == id)
    }

    pub fn can_obtain(&self) -> bool {
        false
    }

    pub fn evaluate(
        &self,
        identity: &DeviceIdentityAssessment,
    ) -> Vec<FirmwareAcquisitionCandidate> {
        let context = EvaluationContext::new(identity);
        self.strategies()
            .map(|strategy| {
                let definition = definition(strategy.id());
                let applicability = strategy.applicability(&context);
                let mut decision =
                    capability(strategy.implementation_state(), definition.reason, &[]);
                decision.missing_requirements = applicability
                    .missing
                    .iter()
                    .map(|requirement| requirement.description().to_owned())
                    .collect();
                FirmwareAcquisitionCandidate {
                    strategy: strategy.id().report_id(),
                    name: definition.name.into(),
                    provenance_kind: definition.provenance.into(),
                    capability: decision,
                }
            })
            .collect()
    }
}
