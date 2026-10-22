//! Backend receipts follow the Phase 5 bound, expiring, single-use grant pattern.
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
pub(super) struct Approvals(Mutex<BTreeMap<String, (Binding, Instant)>>);
impl Approvals {
    pub(super) fn issue(&self, binding: Binding) -> Result<String, DiscoveryError> {
        let id = crate::library::files::nonce()
            .map_err(|_| error("INTERNAL", "Approval unavailable."))?;
        let mut receipts = self
            .0
            .lock()
            .map_err(|_| error("INTERNAL", "Approval unavailable."))?;
        receipts.retain(|_, (_, deadline)| *deadline > Instant::now());
        if receipts.len() >= 64 {
            return Err(error("BUSY", "Too many pending approvals."));
        }
        receipts.insert(
            id.clone(),
            (binding, Instant::now() + Duration::from_secs(30)),
        );
        Ok(id)
    }
    pub(super) fn claim(&self, id: &str, binding: &Binding) -> Result<(), DiscoveryError> {
        let (expected, deadline) = self
            .0
            .lock()
            .map_err(|_| error("INTERNAL", "Approval unavailable."))?
            .remove(id)
            .ok_or_else(|| error("APPROVAL_REQUIRED", "Approve this check again."))?;
        if expected != *binding || Instant::now() >= deadline {
            return Err(error(
                "APPROVAL_EXPIRED",
                "Approval expired or target changed. Approve again.",
            ));
        }
        Ok(())
    }
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
        grants.0.lock().unwrap().get_mut(&id).unwrap().1 = Instant::now();
        assert!(grants.claim(&id, &b).is_err());
    }
}
