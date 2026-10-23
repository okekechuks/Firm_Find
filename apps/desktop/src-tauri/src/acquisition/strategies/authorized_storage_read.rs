use super::super::*;

pub struct AuthorizedStorageRead;

impl AcquisitionStrategy for AuthorizedStorageRead {
    fn id(&self) -> StrategyId {
        StrategyId::AuthorizedStorageRead
    }
    fn kind(&self) -> StrategyKind {
        StrategyKind::StorageRead
    }
    fn qualified_requirements(&self) -> &'static [&'static str] {
        &[
            "Model-qualified read-only storage adapter",
            "Exact permitted ranges and exclusions",
            "Coverage and recovery requirements",
            "Authentication and explicit read approval",
        ]
    }
}
