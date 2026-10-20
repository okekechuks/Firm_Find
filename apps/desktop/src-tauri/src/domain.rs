use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Light,
    Dark,
    #[default]
    System,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    pub theme: Theme,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Counts {
    pub devices: i64,
    pub artifacts: i64,
    pub running_jobs: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEvent {
    pub id: i64,
    pub kind: String,
    pub message: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub version: String,
    pub database_version: i64,
    pub data_directory: String,
    pub settings: Settings,
    pub counts: Counts,
    pub recent_events: Vec<AuditEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_profile: Option<NativeTestInfo>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeTestInfo {
    pub run_id: String,
    pub discovery_source: String,
    pub simulated_adapters: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub simulation_available: Option<bool>,
}

#[derive(Debug, Clone, Copy)]
pub enum OperationRisk {
    Passive,
    ReadOnly,
    DerivedFileWrite,
    DeviceWrite,
    Destructive,
}

impl OperationRisk {
    pub fn permitted_in_foundation(self) -> bool {
        matches!(
            self,
            Self::Passive | Self::ReadOnly | Self::DerivedFileWrite
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hardware_writes_are_forbidden() {
        assert!(!OperationRisk::DeviceWrite.permitted_in_foundation());
        assert!(!OperationRisk::Destructive.permitted_in_foundation());
        assert!(OperationRisk::Passive.permitted_in_foundation());
        assert!(OperationRisk::ReadOnly.permitted_in_foundation());
        assert!(OperationRisk::DerivedFileWrite.permitted_in_foundation());
    }

    #[test]
    fn settings_reject_unknown_fields_and_invalid_themes() {
        assert!(serde_json::from_str::<Settings>(r#"{"theme":"dark","command":"cmd"}"#).is_err());
        assert!(serde_json::from_str::<Settings>(r#"{"theme":"invalid"}"#).is_err());
    }
}
