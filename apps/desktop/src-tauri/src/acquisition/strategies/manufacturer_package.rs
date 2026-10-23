use super::super::*;

pub struct ManufacturerPackage;

impl AcquisitionStrategy for ManufacturerPackage {
    fn id(&self) -> StrategyId {
        StrategyId::ManufacturerPackage
    }
    fn kind(&self) -> StrategyKind {
        StrategyKind::ManufacturerPackage
    }
    fn qualified_requirements(&self) -> &'static [&'static str] {
        &[
            "Authentic applicable OEM package source",
            "Exact region/carrier variant",
            "Non-executing package parser",
            "Integrity/authenticity validation",
        ]
    }
}
