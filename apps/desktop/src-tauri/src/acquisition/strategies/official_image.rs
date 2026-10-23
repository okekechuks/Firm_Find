use super::super::*;

pub struct OfficialImage;

impl AcquisitionStrategy for OfficialImage {
    fn id(&self) -> StrategyId {
        StrategyId::OfficialImage
    }
    fn kind(&self) -> StrategyKind {
        StrategyKind::OfficialImage
    }
    fn qualified_requirements(&self) -> &'static [&'static str] {
        &[
            "Publisher-qualified source",
            "Exact region/carrier and image type",
            "Publisher integrity/authenticity evidence",
            "Reviewed acquisition worker",
        ]
    }
}
