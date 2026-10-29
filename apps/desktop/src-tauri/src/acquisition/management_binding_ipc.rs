//! User-visible local Windows route assessment; this command performs no HTTP.
use super::management_transport::{BindingFailure, SourceBinding};
use crate::discovery::DiscoveryService;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BindingAssessment {
    state: &'static str,
    source_ip: Option<String>,
    interface_id: Option<String>,
    reason: &'static str,
    cryptographically_verified: bool,
    can_authenticate: bool,
}

fn assessment(result: Result<(String, String), BindingFailure>) -> BindingAssessment {
    match result {
        Ok((source, interface)) => BindingAssessment {
            state: "observed", source_ip: Some(source), interface_id: Some(interface),
            reason: "Windows confirmed the selected adapter, source IPv4 and route. This is a local routing observation, not cryptographic device identity or permission to authenticate.",
            cryptographically_verified: false, can_authenticate: false,
        },
        Err(failure) => BindingAssessment {
            state: "blocked", source_ip: None, interface_id: None,
            reason: match failure {
                BindingFailure::OverlappingSubnet => "Overlapping local subnets make device selection ambiguous. No router request was sent.",
                BindingFailure::FreshInventoryRequired => "Refresh device discovery before checking the selected route.",
                BindingFailure::DeviceUnavailable | BindingFailure::DeviceChanged => "The selected device, interface or route changed. Select it again.",
                BindingFailure::ProtectedDevice => "This Windows device is protected; router operations are not permitted.",
                BindingFailure::InvalidEndpoint => "Select a numeric IPv4 HTTP/HTTPS management origin without credentials or paths.",
                BindingFailure::InvalidInterface | BindingFailure::AmbiguousSource => "Select one connected physical interface with one provable local IPv4 source.",
                BindingFailure::RouteUnproven => "Windows could not prove the selected adapter, source address and route. Authentication remains blocked.",
                BindingFailure::UnsupportedPlatform => "Selected-device source binding requires Windows qualification.",
            },
            cryptographically_verified: false, can_authenticate: false,
        },
    }
}

#[tauri::command]
pub async fn assess_web_management_binding(
    device_id: String,
    endpoint: String,
    interface_id: String,
    discovery: tauri::State<'_, DiscoveryService>,
) -> Result<BindingAssessment, super::DiscoveryError> {
    let snapshot = discovery
        .snapshot()
        .map_err(|_| super::error("DISCOVERY_UNAVAILABLE", "Refresh discovery first."))?;
    let current = discovery.shared_snapshot();
    tauri::async_runtime::spawn_blocking(move || {
        let binding = SourceBinding::capture(&snapshot, &device_id, &interface_id, &endpoint);
        let result = binding.and_then(|binding| {
            let snapshot = current
                .lock()
                .map_err(|_| BindingFailure::DeviceChanged)?
                .clone();
            binding.recheck(&snapshot)?;
            Ok(binding.display_metadata())
        });
        assessment(result)
    })
    .await
    .map_err(|_| super::error("WORKER_FAILED", "Local route assessment could not finish."))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_route_observation_never_grants_authentication() {
        let result = assessment(Ok(("192.0.2.2".into(), "synthetic-interface".into())));
        assert_eq!(result.state, "observed");
        assert_eq!(result.source_ip.as_deref(), Some("192.0.2.2"));
        assert!(!result.cryptographically_verified);
        assert!(!result.can_authenticate);
    }
    #[test]
    fn failed_local_route_hides_source_and_remains_blocked() {
        let result = assessment(Err(BindingFailure::OverlappingSubnet));
        assert_eq!(result.state, "blocked");
        assert!(result.source_ip.is_none());
        assert!(result.interface_id.is_none());
        assert!(!result.can_authenticate);
    }
}
