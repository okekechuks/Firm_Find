//! Backend receipts follow the Phase 5 bound, expiring, single-use grant pattern.
use super::strategy_api::{StrategyError, StrategyId};
use super::*;
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Binding {
    device: String,
    revision: u64,
    interface: String,
    endpoint: String,
    advanced: bool,
}
pub(super) fn binding(
    device: &Device,
    revision: u64,
    input: &DiscoveryRequest,
) -> Result<Binding, DiscoveryError> {
    if !device.connected || device.id != input.device_id {
        return Err(error("TARGET_MISMATCH", "Select a connected device."));
    }
    let target = connection::parse_address(input.endpoint.as_deref().unwrap_or(""))
        .map_err(|_| error("INVALID_ENDPOINT", "Select a private root address."))?;
    let network = device
        .networks
        .iter()
        .find(|n| {
            Some(n.id.as_str()) == input.interface_id.as_deref()
                && n.link_up
                && !n.virtual_interface
        })
        .ok_or_else(|| error("INTERFACE_REQUIRED", "Select an active physical interface."))?;
    let ip = target.host.parse::<std::net::IpAddr>().map_err(|_| {
        error(
            "INVALID_ENDPOINT",
            "Use the numeric management address for an approved check.",
        )
    })?;
    if !connection::on_link(ip, network)
        || (!input.advanced_endpoint
            && !network
                .gateways
                .iter()
                .any(|g| g.parse::<std::net::IpAddr>().ok() == Some(ip)))
    {
        return Err(error(
            "ENDPOINT_NOT_GATEWAY",
            "Select this interface's gateway, or explicitly approve an advanced local endpoint.",
        ));
    }
    if device.interfaces.iter().any(|i| {
        [
            "diskdrive",
            "hdc",
            "scsiadapter",
            "hidclass",
            "keyboard",
            "mouse",
            "display",
            "processor",
            "system",
            "computer",
            "battery",
        ]
        .iter()
        .any(|c| i.class_name.eq_ignore_ascii_case(c))
    }) {
        return Err(error("PROTECTED_DEVICE", "Protected hardware is excluded."));
    }
    Ok(Binding {
        device: device.id.clone(),
        revision,
        interface: network.id.clone(),
        endpoint: target.url,
        advanced: input.advanced_endpoint,
    })
}
#[derive(Default)]
pub(super) struct Approvals(Mutex<BTreeMap<String, (Binding, ApprovalScope, Instant)>>);
impl Approvals {
    pub(super) fn issue(&self, binding: Binding) -> Result<String, DiscoveryError> {
        let id = crate::library::files::nonce()
            .map_err(|_| error("INTERNAL", "Approval unavailable."))?;
        let mut receipts = self
            .0
            .lock()
            .map_err(|_| error("INTERNAL", "Approval unavailable."))?;
        receipts.retain(|_, (_, _, deadline)| *deadline > Instant::now());
        if receipts.len() >= 64 {
            return Err(error("BUSY", "Too many pending approvals."));
        }
        receipts.insert(
            id.clone(),
            (
                binding,
                ApprovalScope::EndpointMetadata,
                Instant::now() + Duration::from_secs(30),
            ),
        );
        Ok(id)
    }
    pub(super) fn claim(
        &self,
        id: &str,
        binding: &Binding,
    ) -> Result<ClaimedApproval, DiscoveryError> {
        let (expected, scope, deadline) = self
            .0
            .lock()
            .map_err(|_| error("INTERNAL", "Approval unavailable."))?
            .remove(id)
            .ok_or_else(|| error("APPROVAL_REQUIRED", "Approve this check again."))?;
        if expected != *binding
            || scope != ApprovalScope::EndpointMetadata
            || Instant::now() >= deadline
        {
            return Err(error(
                "APPROVAL_EXPIRED",
                "Approval expired or target changed. Approve again.",
            ));
        }
        Ok(ClaimedApproval {
            binding: expected,
            scope,
            deadline,
        })
    }
}

/// Authority stays explicit and operation-specific. This phase only issues EndpointMetadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalScope {
    EndpointMetadata,
    StrategyProbe {
        strategy: StrategyId,
        requests: Vec<super::request_registry::RequestId>,
    },
}

/// Non-Clone, non-serializable proof, constructed only by a successful claim.
pub(crate) struct ClaimedApproval {
    binding: Binding,
    scope: ApprovalScope,
    deadline: Instant,
}
impl ClaimedApproval {
    pub(super) fn validate(&self, binding: &Binding) -> Result<(), DiscoveryError> {
        if self.scope != ApprovalScope::EndpointMetadata
            || self.binding != *binding
            || Instant::now() >= self.deadline
        {
            return Err(error(
                "APPROVAL_EXPIRED",
                "Approval expired or target changed.",
            ));
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn scope(&self) -> ApprovalScope {
        self.scope.clone()
    }

    #[allow(dead_code)] // Strategy-probe receipts are intentionally not issued in this phase.
    pub(crate) fn strategy_probe(
        &self,
        strategy: StrategyId,
    ) -> Result<StrategyProbeApproval, StrategyError> {
        match &self.scope {
            ApprovalScope::StrategyProbe {
                strategy: bound,
                requests,
            } if *bound == strategy && !requests.is_empty() && Instant::now() < self.deadline => {
                Ok(StrategyProbeApproval {
                    strategy,
                    deadline: self.deadline,
                })
            }
            _ => Err(StrategyError::ApprovalScopeMismatch),
        }
    }
}

/// Proof can only be minted by a claimed receipt inside this module.
/// ```compile_fail,E0451
/// use obtainer_lib::acquisition::{StrategyId, StrategyProbeApproval};
/// let _ = StrategyProbeApproval { strategy: StrategyId::OfficialImage, deadline: std::time::Instant::now() };
/// ```
pub struct StrategyProbeApproval {
    strategy: StrategyId,
    deadline: Instant,
}
impl StrategyProbeApproval {
    pub(crate) fn authorizes(&self, strategy: StrategyId) -> bool {
        self.strategy == strategy && Instant::now() < self.deadline
    }
    #[cfg(test)]
    pub(crate) fn for_test(strategy: StrategyId) -> Self {
        let claimed = ClaimedApproval {
            binding: Binding {
                device: "test".into(),
                revision: 1,
                interface: "test".into(),
                endpoint: "http://127.0.0.1/".into(),
                advanced: false,
            },
            scope: ApprovalScope::StrategyProbe {
                strategy,
                requests: vec![super::RequestId::Root],
            },
            deadline: Instant::now() + Duration::from_secs(30),
        };
        claimed.strategy_probe(strategy).unwrap()
    }
}

pub(super) fn confirmation_value(value: &str) -> String {
    value
        .chars()
        .filter(|c| {
            !c.is_control() && !matches!(*c,'\u{202a}'..='\u{202e}'|'\u{2066}'..='\u{2069}')
        })
        .take(80)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a1_gateway_scope_replay_interface_revision_endpoint_expiry() {
        let mut d = super::super::tests::device();
        d.networks[0].gateways = vec!["192.168.100.1".into()];
        let n = &d.networks[0];
        let mut request = DiscoveryRequest {
            device_id: d.id.clone(),
            endpoint: Some("http://192.168.100.77/".into()),
            interface_id: Some(n.id.clone()),
            approval_token: None,
            advanced_endpoint: false,
        };
        assert!(binding(&d, 1, &request).is_err());
        request.endpoint = Some(format!("http://{}/", n.gateways[0]));
        let b = binding(&d, 1, &request).unwrap();
        let grants = Approvals::default();
        let id = grants.issue(b.clone()).unwrap();
        grants.claim(&id, &b).unwrap();
        assert!(grants.claim(&id, &b).is_err());
        for field in 0..3 {
            let id = grants.issue(b.clone()).unwrap();
            let mut other = b.clone();
            match field {
                0 => other.interface = "other".into(),
                1 => other.revision += 1,
                _ => other.endpoint = "http://192.168.100.2/".into(),
            };
            assert!(grants.claim(&id, &other).is_err());
            assert!(grants.claim(&id, &b).is_err());
        }
        let id = grants.issue(b.clone()).unwrap();
        grants.0.lock().unwrap().get_mut(&id).unwrap().2 = Instant::now();
        assert!(grants.claim(&id, &b).is_err());
    }
}
