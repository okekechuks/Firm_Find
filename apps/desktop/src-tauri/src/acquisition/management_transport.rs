//! Selected-interface facts for a future, explicitly approved management request.
//! Capturing these facts performs local Windows queries only, never network I/O.
use crate::{
    connection,
    discovery::{DiscoverySnapshot, DiscoveryStatus, NetworkAdapter},
};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    net::{IpAddr, Ipv4Addr},
    time::{Duration, Instant},
};
#[cfg(windows)]
#[path = "management_route_windows.rs"]
mod system;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingFailure {
    FreshInventoryRequired,
    DeviceUnavailable,
    ProtectedDevice,
    InvalidEndpoint,
    InvalidInterface,
    AmbiguousSource,
    OverlappingSubnet,
    RouteUnproven,
    DeviceChanged,
    UnsupportedPlatform,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RouteFacts {
    source: Ipv4Addr,
    index: u32,
    luid: u64,
    next_hop: Ipv4Addr,
}

/// Not cryptographic device identity. This object cannot grant request permission.
pub struct SourceBinding {
    device_id: String,
    interface_id: String,
    endpoint: String,
    source: Ipv4Addr,
    network: NetworkAdapter,
    route: RouteFacts,
    device_stamp: [u8; 32],
    deadline: Instant,
}

impl SourceBinding {
    /// Display-only local routing observations; never an approval or identity proof.
    pub(crate) fn display_metadata(&self) -> (String, String) {
        (self.source.to_string(), self.interface_id.clone())
    }
    pub fn capture(
        snapshot: &DiscoverySnapshot,
        device: &str,
        interface: &str,
        endpoint: &str,
    ) -> Result<Self, BindingFailure> {
        Self::capture_with(snapshot, device, interface, endpoint, platform_route)
    }

    fn capture_with(
        snapshot: &DiscoverySnapshot,
        device: &str,
        interface: &str,
        endpoint: &str,
        route_provider: impl Fn(
            &NetworkAdapter,
            Ipv4Addr,
            Ipv4Addr,
        ) -> Result<RouteFacts, BindingFailure>,
    ) -> Result<Self, BindingFailure> {
        let now = connection::now();
        let scanned = snapshot
            .scanned_at
            .ok_or(BindingFailure::FreshInventoryRequired)?;
        if snapshot.cancelled
            || !matches!(
                snapshot.status,
                DiscoveryStatus::Ready | DiscoveryStatus::Degraded
            )
            || scanned > now
            || now - scanned >= 60_000
        {
            return Err(BindingFailure::FreshInventoryRequired);
        }
        let device = snapshot
            .devices
            .iter()
            .find(|d| d.id == device && d.connected)
            .ok_or(BindingFailure::DeviceUnavailable)?;
        if ["conflicting", "ambiguous"].contains(&device.identity.confidence.as_str()) {
            return Err(BindingFailure::DeviceChanged);
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
            .contains(&i.class_name.to_ascii_lowercase().as_str())
        }) {
            return Err(BindingFailure::ProtectedDevice);
        }
        let network = device
            .networks
            .iter()
            .find(|n| {
                n.id == interface && n.link_up && !n.virtual_interface && n.interface_index != 0
            })
            .ok_or(BindingFailure::InvalidInterface)?;
        let target =
            connection::parse_address(endpoint).map_err(|_| BindingFailure::InvalidEndpoint)?;
        let destination = target
            .host
            .parse::<Ipv4Addr>()
            .map_err(|_| BindingFailure::InvalidEndpoint)?;
        let all: Vec<_> = snapshot
            .devices
            .iter()
            .flat_map(|d| d.networks.clone())
            .collect();
        if connection::subnet_conflict(IpAddr::V4(destination), network, &all).is_some() {
            return Err(BindingFailure::OverlappingSubnet);
        }
        if all.iter().any(|n| {
            n.addresses
                .iter()
                .any(|a| a.parse::<Ipv4Addr>() == Ok(destination))
        }) {
            return Err(BindingFailure::InvalidEndpoint);
        }
        let sources: BTreeSet<_> = network
            .prefixes
            .iter()
            .filter_map(|p| {
                let source = p.address.parse::<Ipv4Addr>().ok()?;
                if p.length == 0 || p.length > 32 || !network.addresses.contains(&p.address) {
                    return None;
                }
                let mask = u32::MAX.checked_shl(32 - u32::from(p.length)).unwrap_or(0);
                ((u32::from(source) & mask) == (u32::from(destination) & mask)).then_some(source)
            })
            .collect();
        if sources.len() != 1 {
            return Err(BindingFailure::AmbiguousSource);
        }
        let source = *sources.first().ok_or(BindingFailure::AmbiguousSource)?;
        let route = route_provider(network, source, destination)?;
        if route.source != source
            || route.index != network.interface_index
            || route.luid == 0
            || route.luid.to_string() != network.luid
        {
            return Err(BindingFailure::RouteUnproven);
        }
        let mut interfaces = device.interfaces.clone();
        interfaces.sort_by(|a, b| a.instance_id.cmp(&b.instance_id));
        let material = serde_json::to_vec(&(
            &device.id,
            &device.container_id,
            &device.vendor_id,
            &device.product_id,
            &interfaces,
            &device.identity.confidence,
            &device.identity.manufacturer,
            &device.identity.model,
            &device.identity.firmware,
        ))
        .map_err(|_| BindingFailure::DeviceChanged)?;
        Ok(Self {
            device_id: device.id.clone(),
            interface_id: interface.to_string(),
            endpoint: target.url,
            source,
            network: network.clone(),
            route,
            device_stamp: Sha256::digest(material).into(),
            deadline: Instant::now() + Duration::from_millis(60_000 - (now - scanned)),
        })
    }

    fn recheck_with(
        &self,
        snapshot: &DiscoverySnapshot,
        route: impl Fn(&NetworkAdapter, Ipv4Addr, Ipv4Addr) -> Result<RouteFacts, BindingFailure>,
    ) -> Result<(), BindingFailure> {
        if Instant::now() >= self.deadline {
            return Err(BindingFailure::FreshInventoryRequired);
        }
        let next = Self::capture_with(
            snapshot,
            &self.device_id,
            &self.interface_id,
            &self.endpoint,
            route,
        )
        .map_err(|_| BindingFailure::DeviceChanged)?;
        if next.source != self.source
            || next.network != self.network
            || next.route != self.route
            || next.device_stamp != self.device_stamp
        {
            return Err(BindingFailure::DeviceChanged);
        }
        Ok(())
    }

    pub fn recheck(&self, snapshot: &DiscoverySnapshot) -> Result<(), BindingFailure> {
        self.recheck_with(snapshot, platform_route)
    }

    /// Only a qualified route can create a client for authenticated reads. The
    /// production route registry is empty; imported evidence cannot reach this.
    pub fn identity_client(
        &self,
        _route: &super::management_auth::VerifiedReadRoute,
    ) -> Result<BoundIdentityClient, BindingFailure> {
        if Instant::now() >= self.deadline {
            return Err(BindingFailure::FreshInventoryRequired);
        }
        let client = reqwest::blocking::Client::builder()
            .local_address(IpAddr::V4(self.source))
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(2))
            .timeout(Duration::from_secs(5))
            .pool_max_idle_per_host(0)
            .build()
            .map_err(|_| BindingFailure::RouteUnproven)?;
        Ok(BoundIdentityClient {
            client,
            origin: self.endpoint.clone(),
        })
    }

    pub fn source_ipv4(&self) -> Ipv4Addr {
        self.source
    }
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }
}

/// Does not expose the raw reqwest client or accept caller-supplied URLs.
pub struct BoundIdentityClient {
    client: reqwest::blocking::Client,
    origin: String,
}
impl BoundIdentityClient {
    pub fn qualified_request(
        &self,
        route: &super::management_auth::VerifiedReadRoute,
    ) -> Result<reqwest::blocking::Request, BindingFailure> {
        let url = url::Url::parse(&self.origin).map_err(|_| BindingFailure::InvalidEndpoint)?;
        let requested = url
            .join(route.uri())
            .map_err(|_| BindingFailure::InvalidEndpoint)?;
        if requested.origin() != url.origin() {
            return Err(BindingFailure::InvalidEndpoint);
        }
        self.client
            .get(requested)
            .build()
            .map_err(|_| BindingFailure::RouteUnproven)
    }
}

fn platform_route(
    network: &NetworkAdapter,
    source: Ipv4Addr,
    destination: Ipv4Addr,
) -> Result<RouteFacts, BindingFailure> {
    #[cfg(windows)]
    {
        system::selected_route(network, source, destination)
    }
    #[cfg(not(windows))]
    {
        let _ = (network, source, destination);
        Err(BindingFailure::UnsupportedPlatform)
    }
}

#[cfg(test)]
#[path = "management_transport_tests.rs"]
mod tests;
