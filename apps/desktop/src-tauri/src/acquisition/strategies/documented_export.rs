use super::super::*;

pub struct DocumentedExport;

impl AcquisitionStrategy for DocumentedExport {
    fn id(&self) -> StrategyId {
        StrategyId::DocumentedExport
    }
    fn kind(&self) -> StrategyKind {
        StrategyKind::DeviceExport
    }
    fn qualified_requirements(&self) -> &'static [&'static str] {
        &[
            "Documented or exact-hardware-qualified firmware export",
            "Endpoint trust and authentication",
            "Explicit acquisition approval",
            "Reviewed export worker and binary validation",
        ]
    }
}
