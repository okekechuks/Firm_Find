use super::super::*;

pub struct ExternalLaboratory;

impl AcquisitionStrategy for ExternalLaboratory {
    fn id(&self) -> StrategyId {
        StrategyId::ExternalLaboratory
    }
    fn kind(&self) -> StrategyKind {
        StrategyKind::ExternalLaboratory
    }
    fn qualified_requirements(&self) -> &'static [&'static str] {
        &[
            "External tools and separate owner approval",
            "Hardware-qualified read and coverage procedure",
        ]
    }
}
