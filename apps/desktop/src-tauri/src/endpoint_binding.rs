//! Ephemeral endpoint consistency, not authentication. No sockets or persistent identity.
use crate::{
    connection,
    discovery::{DiscoverySnapshot, DiscoveryStatus, NetworkAdapter},
};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    net::IpAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

const LIFETIME_MS: u64 = 60_000;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BindingError;
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Selection {
    pub device: String,
    pub interface: String,
    pub endpoint: String,
}
#[derive(Debug, Clone)]
pub(crate) struct Route {
    pub source: String,
    pub gateway: Option<String>,
    pub luid: u64,
}
pub(crate) trait RouteProvider {
    fn route(
        &self,
        target: &connection::Target,
        network: &NetworkAdapter,
    ) -> Result<Route, BindingError>;
}
pub(crate) struct SystemRoute;
impl RouteProvider for SystemRoute {
    fn route(
        &self,
        target: &connection::Target,
        network: &NetworkAdapter,
    ) -> Result<Route, BindingError> {
        connection::binding_route(target, network)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Address {
    ip: IpAddr,
    scope: u32,
}
impl Address {
    fn parse(value: &str) -> Result<Self, BindingError> {
        let (ip, scope) = match value.split_once('%') {
            Some((a, s)) => (a, s.parse::<u32>().map_err(|_| BindingError)?),
            None => (value, 0),
        };
        let ip: IpAddr = ip.parse().map_err(|_| BindingError)?;
        if ip.is_unspecified()
            || ip.is_multicast()
            || ip.is_loopback()
            || matches!(ip,IpAddr::V4(v) if v.is_broadcast())
            || (ip.is_ipv4() && scope != 0)
        {
            return Err(BindingError);
        }
        Ok(Self { ip, scope })
    }
    fn bytes(&self) -> Vec<u8> {
        let mut bytes = match self.ip {
            IpAddr::V4(v) => {
                let mut b = vec![4];
                b.extend(v.octets());
                b
            }
            IpAddr::V6(v) => {
                let mut b = vec![6];
                b.extend(v.octets());
                b
            }
        };
        bytes.extend(self.scope.to_be_bytes());
        bytes
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct Facts {
    device: String,
    guid: [u8; 16],
    luid: u64,
    endpoint: String,
    source: Address,
    prefix: u8,
    gateway: Option<Address>,
    gateways: Vec<Address>,
    routes: Vec<Vec<u8>>,
    dhcp: Option<Address>,
    usb: Vec<Vec<u8>>,
}
fn field(out: &mut Vec<u8>, value: &[u8]) {
    out.extend((value.len() as u32).to_be_bytes());
    out.extend(value)
}
fn optional(out: &mut Vec<u8>, value: Option<Vec<u8>>) {
    out.push(u8::from(value.is_some()));
    if let Some(v) = value {
        field(out, &v)
    }
}
fn set(out: &mut Vec<u8>, values: impl IntoIterator<Item = Vec<u8>>) {
    let values: BTreeSet<_> = values.into_iter().collect();
    out.extend((values.len() as u32).to_be_bytes());
    for value in values {
        field(out, &value)
    }
}
impl Facts {
    fn canonical(&self, epoch: &[u8], generation: u64) -> Vec<u8> {
        let mut out = vec![];
        field(&mut out, b"OBTAINER/plain-http-binding/v1");
        field(&mut out, self.device.as_bytes());
        field(&mut out, epoch);
        out.extend(generation.to_be_bytes());
        field(&mut out, &self.guid);
        out.extend(self.luid.to_be_bytes());
        field(&mut out, self.endpoint.as_bytes());
        field(&mut out, &self.source.bytes());
        out.push(self.prefix);
        optional(&mut out, self.gateway.as_ref().map(Address::bytes));
        set(&mut out, self.gateways.iter().map(Address::bytes));
        set(&mut out, self.routes.clone());
        optional(&mut out, self.dhcp.as_ref().map(Address::bytes));
        set(&mut out, self.usb.clone());
        out
    }
}
fn guid(value: &str) -> Result<[u8; 16], BindingError> {
    let v = value.trim_matches(['{', '}']);
    if v.len() != 36 || [8, 13, 18, 23].into_iter().any(|i| v.as_bytes()[i] != b'-') {
        return Err(BindingError);
    }
    let hex: String = v.chars().filter(|c| *c != '-').collect();
    let mut out = [0; 16];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(hex.get(i * 2..i * 2 + 2).ok_or(BindingError)?, 16)
            .map_err(|_| BindingError)?;
    }
    Ok(out)
}
fn capture(
    snapshot: &DiscoverySnapshot,
    selection: &Selection,
    provider: &impl RouteProvider,
) -> Result<Facts, BindingError> {
    let d = snapshot
        .devices
        .iter()
        .find(|d| d.id == selection.device && d.connected)
        .ok_or(BindingError)?;
    if ["conflicting", "ambiguous"].contains(&d.identity.confidence.as_str()) {
        return Err(BindingError);
    }
    if d.interfaces.iter().any(|i| {
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
        return Err(BindingError);
    }
    let n = d
        .networks
        .iter()
        .find(|n| n.id == selection.interface && n.link_up && !n.virtual_interface)
        .ok_or(BindingError)?;
    let target = connection::parse_address(&selection.endpoint).map_err(|_| BindingError)?;
    let ip: IpAddr = target.host.parse().map_err(|_| BindingError)?;
    let networks: Vec<_> = snapshot
        .devices
        .iter()
        .flat_map(|d| d.networks.clone())
        .collect();
    if !connection::on_link(ip, n)
        || connection::subnet_conflict(ip, n, &networks).is_some()
        || n.addresses
            .iter()
            .any(|a| Address::parse(a).is_ok_and(|a| a.ip == ip))
    {
        return Err(BindingError);
    }
    let route = provider.route(&target, n)?;
    let luid = n.luid.parse::<u64>().map_err(|_| BindingError)?;
    if luid == 0 || route.luid != luid {
        return Err(BindingError);
    }
    let source = Address::parse(&route.source)?;
    if matches!(source.ip,IpAddr::V6(v) if v.is_unicast_link_local() && (source.scope==0 || source.scope!=n.ipv6_index))
    {
        return Err(BindingError);
    }
    let gateway = route.gateway.as_deref().map(Address::parse).transpose()?;
    if gateway
        .as_ref()
        .is_some_and(|g| g.ip.is_ipv4() != ip.is_ipv4())
    {
        return Err(BindingError);
    }
    if source.ip.is_ipv4() != ip.is_ipv4()
        || !n
            .addresses
            .iter()
            .any(|a| Address::parse(a).is_ok_and(|v| v == source))
    {
        return Err(BindingError);
    }
    let lengths: BTreeSet<_> = n
        .prefixes
        .iter()
        .filter(|p| Address::parse(&p.address).is_ok_and(|v| v == source))
        .map(|p| p.length)
        .collect();
    if lengths.len() != 1 {
        return Err(BindingError);
    }
    let prefix = *lengths.first().ok_or(BindingError)?;
    if prefix > if source.ip.is_ipv4() { 32 } else { 128 } {
        return Err(BindingError);
    }
    let gateways = n
        .gateways
        .iter()
        .map(|g| Address::parse(g))
        .collect::<Result<BTreeSet<_>, _>>()?
        .into_iter()
        .collect();
    let mut routes = BTreeSet::new();
    for r in &n.default_routes {
        let a = Address::parse(&r.gateway)?;
        if (a.ip.is_ipv4() && r.family != "ipv4") || (a.ip.is_ipv6() && r.family != "ipv6") {
            return Err(BindingError);
        }
        let mut b = a.bytes();
        field(&mut b, r.family.as_bytes());
        b.extend(r.metric.to_be_bytes());
        routes.insert(b);
    }
    let dhcp = n.dhcp_server.as_deref().map(Address::parse).transpose()?;
    if dhcp.as_ref().is_some_and(|d| !d.ip.is_ipv4()) {
        return Err(BindingError);
    }
    let mut usb = vec![];
    let mut container = vec![];
    optional(
        &mut container,
        d.container_id.as_ref().map(|v| v.as_bytes().to_vec()),
    );
    usb.push(container);
    for i in &d.interfaces {
        let mut entry = vec![];
        field(&mut entry, i.instance_id.as_bytes());
        optional(
            &mut entry,
            i.parent_instance_id.as_ref().map(|v| v.as_bytes().to_vec()),
        );
        entry.push(u8::from(i.present));
        usb.push(entry);
    }
    usb.sort();
    usb.dedup();
    Ok(Facts {
        device: d.id.clone(),
        guid: guid(&n.id)?,
        luid,
        endpoint: target.url,
        source,
        prefix,
        gateway,
        gateways,
        routes: routes.into_iter().collect(),
        dhcp,
        usb,
    })
}

/// Not serialized or exported. Consistency evidence never verifies endpoint identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stamp {
    digest: [u8; 32],
    generation: u64,
}
#[derive(Debug, Clone)]
pub(crate) struct BoundEndpoint {
    stamp: Stamp,
    live: Arc<AtomicBool>,
    expires_at: u64,
    deadline: Instant,
}
impl PartialEq for BoundEndpoint {
    fn eq(&self, other: &Self) -> bool {
        self.stamp == other.stamp
    }
}
impl Eq for BoundEndpoint {}
impl BoundEndpoint {
    #[cfg(test)]
    pub(crate) fn synthetic() -> Self {
        static GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        Self {
            stamp: Stamp {
                digest: [0; 32],
                generation: GENERATION.fetch_add(1, Ordering::Relaxed),
            },
            live: Arc::new(AtomicBool::new(true)),
            expires_at: connection::now() + 60_000,
            deadline: Instant::now() + Duration::from_secs(60),
        }
    }
    pub(crate) fn stamp(&self) -> Stamp {
        self.stamp.clone()
    }
    pub(crate) fn current(&self) -> bool {
        self.current_at(connection::now())
    }
    pub(crate) fn remaining(&self) -> Duration {
        if !self.current() {
            return Duration::ZERO;
        }
        self.deadline
            .saturating_duration_since(Instant::now())
            .min(Duration::from_millis(
                self.expires_at.saturating_sub(connection::now()),
            ))
    }
    fn current_at(&self, now: u64) -> bool {
        if now >= self.expires_at || Instant::now() >= self.deadline {
            // Expiry is irreversible even if the system wall clock subsequently rolls back.
            self.live.store(false, Ordering::Release);
        }
        self.live.load(Ordering::Acquire)
    }
}
struct Entry {
    selection: Selection,
    facts: Facts,
    bound: BoundEndpoint,
}
impl Drop for Entry {
    fn drop(&mut self) {
        self.bound.live.store(false, Ordering::Release)
    }
}
#[derive(Default)]
struct State {
    epoch: Option<String>,
    generation: u64,
    entries: BTreeMap<String, Entry>,
}
#[derive(Default)]
pub(crate) struct Registry(Mutex<State>);
impl Registry {
    pub(crate) fn bind(
        &self,
        snapshot: &DiscoverySnapshot,
        selection: &Selection,
        provider: &impl RouteProvider,
        now: u64,
    ) -> Result<BoundEndpoint, BindingError> {
        let mut state = self.0.lock().map_err(|_| BindingError)?;
        let facts = match capture(snapshot, selection, provider) {
            Ok(f) => f,
            Err(e) => {
                state.entries.remove(&selection.device);
                return Err(e);
            }
        };
        if let Some(entry) = state.entries.get(&selection.device) {
            if entry.facts == facts && entry.bound.current_at(now) {
                return Ok(entry.bound.clone());
            }
        }
        state.entries.remove(&selection.device);
        if snapshot.cancelled
            || !matches!(
                snapshot.status,
                DiscoveryStatus::Ready | DiscoveryStatus::Degraded
            )
            || snapshot
                .scanned_at
                .is_none_or(|t| t > now || now - t >= LIFETIME_MS)
        {
            return Err(BindingError);
        }
        state.entries.retain(|_, e| e.bound.current_at(now));
        if state.entries.len() >= 64 {
            return Err(BindingError);
        }
        if state.epoch.is_none() {
            state.epoch = Some(crate::library::files::nonce().map_err(|_| BindingError)?)
        }
        state.generation = state.generation.checked_add(1).ok_or(BindingError)?;
        let generation = state.generation;
        let digest = Sha256::digest(facts.canonical(
            state.epoch.as_ref().ok_or(BindingError)?.as_bytes(),
            generation,
        ))
        .into();
        let expires_at = snapshot
            .scanned_at
            .ok_or(BindingError)?
            .saturating_add(LIFETIME_MS);
        let bound = BoundEndpoint {
            stamp: Stamp { digest, generation },
            live: Arc::new(AtomicBool::new(true)),
            expires_at,
            deadline: Instant::now() + Duration::from_millis(expires_at - now),
        };
        state.entries.insert(
            selection.device.clone(),
            Entry {
                selection: selection.clone(),
                facts,
                bound: bound.clone(),
            },
        );
        Ok(bound)
    }
    pub(crate) fn observe(
        &self,
        snapshot: &DiscoverySnapshot,
        provider: &impl RouteProvider,
        now: u64,
    ) {
        if snapshot.cancelled
            || !matches!(
                snapshot.status,
                DiscoveryStatus::Ready | DiscoveryStatus::Degraded
            )
        {
            return;
        }
        if let Ok(mut state) = self.0.lock() {
            state.entries.retain(|_, e| {
                e.bound.current_at(now)
                    && capture(snapshot, &e.selection, provider).is_ok_and(|f| f == e.facts)
            });
        }
    }
    pub(crate) fn invalidate(&self, device: &str) {
        if let Ok(mut state) = self.0.lock() {
            state.entries.remove(device);
        }
    }
}

#[cfg(test)]
#[path = "endpoint_binding_tests.rs"]
mod tests;
