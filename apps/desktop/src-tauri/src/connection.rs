//! User-initiated, unauthenticated endpoint checks. No crawling or device writes.
#[cfg(windows)]
mod windows;
use crate::{
    discovery::{DiscoverySnapshot, NetworkAdapter},
    storage::Repository,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    net::IpAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EndpointResult {
    pub code: String,
    pub message: String,
    pub state: String,
    pub status: Option<u16>,
    pub server: Option<String>,
    pub checked_at: u64,
    pub elapsed_ms: u64,
    pub resolved_address: Option<String>,
    pub tls: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_media_type: Option<String>,
}
impl EndpointResult {
    pub fn failure(code: &str, message: &str) -> Box<Self> {
        Box::new(Self {
            code: code.into(),
            message: message.into(),
            state: "error".into(),
            status: None,
            server: None,
            checked_at: now(),
            elapsed_ms: 0,
            resolved_address: None,
            tls: "not_checked".into(),
            observed_media_type: None,
        })
    }
}
/// Retain only a bounded media-type token, never arbitrary response header text.
pub(crate) fn menu_media_type(value: Option<&str>) -> String {
    value
        .filter(|v| {
            !v.is_empty()
                && v.len() <= 64
                && v.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b".+-/".contains(&b))
        })
        .unwrap_or("unavailable")
        .into()
}
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// One owner-approved root GET. Response bytes remain transient and are never logged.
pub(crate) fn root_metadata(
    approval: &crate::acquisition::ClaimedApproval,
    target: &Target,
    interface: &str,
    networks: &[NetworkAdapter],
    token: &AtomicBool,
) -> (EndpointResult, Option<Vec<u8>>) {
    if !approval.metadata_target(target, interface) {
        return (
            *EndpointResult::failure("approval_required", "Approve the selected metadata check."),
            None,
        );
    }
    #[cfg(windows)]
    return windows::probe_root(target, Some(interface), networks, token);
    #[cfg(not(windows))]
    {
        let _ = (target, interface, networks, token);
        (
            *EndpointResult::failure(
                "unsupported_platform",
                "Endpoint metadata requires Windows.",
            ),
            None,
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManualInput {
    pub address: String,
    pub label: String,
    pub interface_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedRouter {
    pub id: String,
    pub address: String,
    pub label: String,
    pub interface_id: Option<String>,
    pub device_id: Option<String>,
    pub created_at: u64,
    pub last_result: Option<EndpointResult>,
    pub state: String,
}

#[derive(Debug, Clone)]
pub struct Target {
    pub url: String,
    pub host: String,
    pub port: u16,
    pub secure: bool,
}

pub fn parse_address(input: &str) -> Result<Target, Box<EndpointResult>> {
    let invalid = || {
        EndpointResult::failure("invalid_address", "Enter a router hostname or IP, optionally with http://, https:// and a port. Paths, credentials, queries and fragments are not supported.")
    };
    let input = input.trim();
    if input.is_empty()
        || input.len() > 2048
        || input.chars().any(|c| c.is_control() || c.is_whitespace())
        || input.contains(['\\', '@', '%', '?', '#'])
    {
        return Err(invalid());
    }
    let address = if input.contains("://") {
        input.into()
    } else if input.parse::<std::net::Ipv6Addr>().is_ok() {
        format!("http://[{input}]")
    } else {
        format!("http://{input}")
    };
    let url = url::Url::parse(&address).map_err(|_| invalid())?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(EndpointResult::failure(
            "unsupported_scheme",
            "Only read-only HTTP and HTTPS checks are available in Phase 2.",
        ));
    }
    if url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(invalid());
    }
    let host = url
        .host_str()
        .ok_or_else(invalid)?
        .trim_matches(['[', ']'])
        .to_owned();
    if host.len() > 253 || host.is_empty() || host.split('.').any(|part| part.len() > 63) {
        return Err(invalid());
    }
    let port = url
        .port_or_known_default()
        .filter(|p| *p != 0)
        .ok_or_else(invalid)?;
    if let Ok(ip) = host.parse::<IpAddr>() {
        if !local_address(ip) {
            return Err(EndpointResult::failure("target_out_of_scope", "Only private or link-local router addresses on an active local interface are allowed."));
        }
    }
    Ok(Target {
        url: url.to_string(),
        host,
        port,
        secure: url.scheme() == "https",
    })
}

pub fn local_address(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => ip.is_private() || (ip.is_link_local() && ip.octets()[2] != 169),
        IpAddr::V6(ip) => {
            ip.to_ipv4_mapped().is_none()
                && ((ip.segments()[0] & 0xfe00 == 0xfc00) || ip.is_unicast_link_local())
        }
    }
}
fn bare_ip(value: &str) -> Option<IpAddr> {
    value.split('%').next()?.parse().ok()
}
pub fn on_link(ip: IpAddr, network: &NetworkAdapter) -> bool {
    network
        .prefixes
        .iter()
        .any(|p| match (ip, bare_ip(&p.address)) {
            (IpAddr::V4(ip), Some(IpAddr::V4(local))) if p.length > 0 && p.length <= 32 => {
                let mask = u32::MAX << (32 - p.length);
                u32::from(ip) & mask == u32::from(local) & mask
            }
            (IpAddr::V6(ip), Some(IpAddr::V6(local))) if p.length > 0 && p.length <= 128 => {
                let mask = u128::MAX << (128 - p.length);
                u128::from(ip) & mask == u128::from(local) & mask
            }
            _ => false,
        })
        || network.gateways.iter().any(|g| bare_ip(g) == Some(ip))
}
pub fn select_interface<'a>(
    ip: IpAddr,
    scope: Option<&str>,
    networks: &'a [NetworkAdapter],
) -> Result<&'a NetworkAdapter, Box<EndpointResult>> {
    if !local_address(ip) {
        return Err(EndpointResult::failure(
            "target_out_of_scope",
            "Resolved address is outside the permitted local network scope.",
        ));
    }
    let matches: Vec<_> = networks
        .iter()
        .filter(|n| {
            n.link_up && !n.virtual_interface && scope.is_none_or(|id| id == n.id) && on_link(ip, n)
        })
        .collect();
    match matches.as_slice() {
        [one] => Ok(one),
        [] => Err(EndpointResult::failure("no_route", "No active local physical interface reaches this address. Connect the router, refresh discovery and select its interface.")),
        _ => Err(EndpointResult::failure("ambiguous_interface", "Multiple local interfaces match. Select the router's interface before checking.")),
    }
}

#[derive(Default)]
struct Control {
    active: BTreeMap<String, Arc<AtomicBool>>,
    running: usize,
    completed: BTreeMap<String, String>,
}
#[derive(Clone, Default)]
pub struct ConnectionService {
    control: Arc<Mutex<Control>>,
}
pub struct ProbeLease {
    service: ConnectionService,
    id: String,
    pub cancelled: Arc<AtomicBool>,
}
impl ProbeLease {
    /// Serialize publication with cancellation/supersession without releasing another lease.
    pub(crate) fn if_current<T>(&self, publish: impl FnOnce() -> T) -> Option<T> {
        let state = self.service.control.lock().ok()?;
        if !self.cancelled.load(Ordering::Acquire)
            && state
                .active
                .get(&self.id)
                .is_some_and(|token| Arc::ptr_eq(token, &self.cancelled))
        {
            Some(publish())
        } else {
            None
        }
    }
}
impl Drop for ProbeLease {
    fn drop(&mut self) {
        if let Ok(mut state) = self.service.control.lock() {
            state.running = state.running.saturating_sub(1);
            if state
                .active
                .get(&self.id)
                .is_some_and(|token| Arc::ptr_eq(token, &self.cancelled))
            {
                state.active.remove(&self.id);
            }
        }
    }
}
impl ConnectionService {
    pub fn begin(&self, id: &str) -> Result<ProbeLease, Box<EndpointResult>> {
        let mut state = self.control.lock().map_err(|_| {
            EndpointResult::failure(
                "unavailable",
                "Connection state unavailable. Restart OBTAINER.",
            )
        })?;
        if state.running >= 2 {
            return Err(EndpointResult::failure(
                "busy",
                "Two checks are already running. Cancel or wait before retrying.",
            ));
        }
        let token = Arc::new(AtomicBool::new(false));
        if let Some(old) = state.active.insert(id.into(), Arc::clone(&token)) {
            old.store(true, Ordering::Release);
        }
        state.running += 1;
        Ok(ProbeLease {
            service: self.clone(),
            id: id.into(),
            cancelled: token,
        })
    }
    pub fn annotate(&self, routers: &mut [SavedRouter], discovery: &DiscoverySnapshot) {
        if let Ok(state) = self.control.lock() {
            for router in routers {
                router.state = if state.active.contains_key(&router.id) {
                    "identifying".into()
                } else if router.interface_id.as_ref().is_some_and(|id| {
                    !discovery
                        .devices
                        .iter()
                        .filter(|d| d.connected)
                        .flat_map(|d| &d.networks)
                        .any(|n| &n.id == id && n.link_up)
                }) {
                    "disconnected".into()
                } else {
                    state
                        .completed
                        .get(&router.id)
                        .cloned()
                        .unwrap_or_else(|| "discovered".into())
                };
            }
        }
    }
    pub fn cancel(&self, id: &str) {
        if let Ok(mut state) = self.control.lock() {
            if let Some(token) = state.active.remove(id) {
                token.store(true, Ordering::Release);
            }
        }
    }
    pub fn remove(
        &self,
        id: &str,
        repository: &Mutex<Repository>,
    ) -> Result<(), crate::storage::StorageError> {
        // Same lock order as publish prevents completion from resurrecting a removed row.
        let mut state = self
            .control
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        if let Some(token) = state.active.remove(id) {
            token.store(true, Ordering::Release);
        }
        state.completed.remove(id);
        repository
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?
            .remove_router(id)
    }
    pub fn check(
        &self,
        saved: &SavedRouter,
        snapshot: &DiscoverySnapshot,
        lease: &ProbeLease,
    ) -> EndpointResult {
        let target = match parse_address(&saved.address) {
            Ok(t) => t,
            Err(e) => return *e,
        };
        let networks: Vec<_> = snapshot
            .devices
            .iter()
            .filter(|d| d.connected)
            .flat_map(|d| d.networks.clone())
            .collect();
        #[cfg(windows)]
        return windows::probe(
            &target,
            saved.interface_id.as_deref(),
            &networks,
            &lease.cancelled,
        );
        #[cfg(not(windows))]
        {
            let _ = (target, networks, lease);
            *EndpointResult::failure("unsupported_platform", "Endpoint checks require Windows.")
        }
    }
    pub fn publish(
        &self,
        lease: &ProbeLease,
        result: &EndpointResult,
        repository: &Mutex<Repository>,
    ) -> Result<bool, crate::storage::StorageError> {
        let mut state = self
            .control
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let current = state
            .active
            .get(&lease.id)
            .is_some_and(|t| Arc::ptr_eq(t, &lease.cancelled))
            && !lease.cancelled.load(Ordering::Acquire);
        if current {
            let saved = repository
                .lock()
                .map_err(|_| rusqlite::Error::InvalidQuery)?
                .save_router_result(&lease.id, result)?;
            if saved {
                state
                    .completed
                    .insert(lease.id.clone(), result.state.clone());
            }
            return Ok(saved);
        }
        Ok(false)
    }
}
pub fn prepare_saved(
    input: ManualInput,
    snapshot: &DiscoverySnapshot,
) -> Result<SavedRouter, Box<EndpointResult>> {
    let target = parse_address(&input.address)?;
    let label = input.label.trim();
    if label.len() > 120 || label.chars().any(char::is_control) {
        return Err(EndpointResult::failure(
            "invalid_label",
            "Name must be at most 120 bytes without control characters.",
        ));
    }
    let matching_device = match &input.interface_id {
        Some(id) => Some(
            snapshot
                .devices
                .iter()
                .find(|d| d.networks.iter().any(|n| &n.id == id))
                .ok_or_else(|| {
                    EndpointResult::failure(
                        "invalid_interface",
                        "Selected interface is unavailable. Refresh discovery.",
                    )
                })?
                .id
                .clone(),
        ),
        None => None,
    };
    let id = format!(
        "{:x}",
        Sha256::digest(
            format!(
                "{}|{}",
                target.url,
                input.interface_id.as_deref().unwrap_or("")
            )
            .as_bytes()
        )
    );
    Ok(SavedRouter {
        id,
        address: target.url,
        label: label.into(),
        interface_id: input.interface_id,
        device_id: matching_device,
        created_at: now(),
        last_result: None,
        state: "discovered".into(),
    })
}

#[cfg(test)]
mod tests;

pub(crate) fn subnet_conflict(
    ip: IpAddr,
    selected: &NetworkAdapter,
    networks: &[NetworkAdapter],
) -> Option<Box<EndpointResult>> {
    networks.iter().find(|n| n.id!=selected.id && n.link_up && !n.virtual_interface && on_link(ip,n)).map(|other| EndpointResult::failure("subnet_conflict",&format!("Another network uses the same address range ({}). Disconnect it or change the MiFi's LAN range.",other.display_name.chars().filter(|c| !c.is_control()).take(120).collect::<String>())))
}
/// Fixed static metadata path, never supplied by the caller.
pub(crate) fn version_metadata(
    approval: &crate::acquisition::ClaimedApproval,
    target: &Target,
    interface: &str,
    networks: &[NetworkAdapter],
    token: &AtomicBool,
) -> (EndpointResult, Option<Vec<u8>>) {
    if !approval.metadata_target(target, interface) {
        return (
            *EndpointResult::failure("approval_required", "Approve the selected metadata check."),
            None,
        );
    }
    #[cfg(windows)]
    return windows::probe_version(target, Some(interface), networks, token);
    #[cfg(not(windows))]
    {
        let _ = (target, interface, networks, token);
        (
            *EndpointResult::failure("unsupported_platform", "Windows required."),
            None,
        )
    }
}

/// Only a claimed StaticMenu scope may reach this fixed, separately bounded request.
pub(crate) fn menu_metadata(
    proof: &crate::acquisition::StaticMenuApproval,
    target: &Target,
    interface: &str,
    networks: &[NetworkAdapter],
    token: &AtomicBool,
) -> (EndpointResult, Option<Vec<u8>>) {
    if !proof.take_target(target, interface) {
        return (
            *EndpointResult::failure(
                "approval_required",
                "Approve the selected static menu check.",
            ),
            None,
        );
    }
    #[cfg(windows)]
    return windows::probe_menu(target, Some(interface), networks, token, proof.remaining());
    #[cfg(not(windows))]
    {
        let _ = (target, interface, networks, token);
        (
            *EndpointResult::failure("unsupported_platform", "Windows required."),
            None,
        )
    }
}
