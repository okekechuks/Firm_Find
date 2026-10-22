//! Backend-owned target snapshots, single-use approvals and scoped operation ownership.
use super::{
    adapter::{denied, Inspection, OpenWrtAdapter},
    transport::{self, HostObservation, SshTarget},
};
use crate::{
    adapters::*,
    connection::{self, SavedRouter},
    discovery::{self, Device, DiscoverySnapshot, NetworkAdapter},
    profiles,
};
use serde::{Deserialize, Serialize};
use std::{
    net::IpAddr,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Condvar, Mutex,
    },
    time::{Duration, Instant},
};

pub const LAB_TARGET: &str = "8888888888888888888888888888888888888888888888888888888888888888";
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetChoice {
    pub id: String,
    pub label: String,
    pub address: String,
    pub interface_id: Option<String>,
    pub lab: bool,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Identity {
    pub manufacturer: Option<String>,
    pub model: String,
    pub hardware_revision: Option<String>,
    pub firmware_family: String,
    pub firmware_version: String,
    pub board_name: String,
    pub build_revision: String,
    pub target: String,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub enabled: bool,
    pub busy: bool,
    pub generation: u64,
    pub state: String,
    pub target_id: Option<String>,
    pub endpoint: Option<String>,
    pub interface_id: Option<String>,
    pub source_address: Option<String>,
    pub host_fingerprint: Option<String>,
    pub expires_at: Option<u64>,
    pub identity: Option<Identity>,
    pub profile_qualified: bool,
    pub read_approved: bool,
    pub inspection: Option<Inspection>,
    pub error: Option<AdapterError>,
    pub targets: Vec<TargetChoice>,
}
impl Default for Status {
    fn default() -> Self {
        Self {
            enabled: true,
            busy: false,
            generation: 0,
            state: "idle".into(),
            target_id: None,
            endpoint: None,
            interface_id: None,
            source_address: None,
            host_fingerprint: None,
            expires_at: None,
            identity: None,
            profile_qualified: false,
            read_approved: false,
            inspection: None,
            error: None,
            targets: vec![],
        }
    }
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Observe {
    pub target_id: String,
    pub port: u16,
    pub approved: bool,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Connect {
    pub fingerprint: String,
    pub username: String,
    pub approved: bool,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Approval {
    pub approved: bool,
}

#[derive(Clone)]
pub struct Selected {
    pub id: String,
    pub address: IpAddr,
    pub source: IpAddr,
    pub interface_id: String,
    pub device: Device,
    pub profiles: profiles::Registry,
    pub report: profiles::CompatibilityReport,
    pub revision: u64,
    pub lab: bool,
    pub network: Option<NetworkAdapter>,
}
impl Selected {
    fn context(&self) -> DeviceContext<'_> {
        DeviceContext {
            device: &self.device,
            profile: &self.report,
            profiles: &self.profiles,
            revision: self.revision,
            now: connection::now(),
        }
    }
    fn same(&self, other: &Self) -> bool {
        self.id == other.id
            && self.address == other.address
            && self.source == other.source
            && self.interface_id == other.interface_id
            && self.device == other.device
            && self.report.input_digest == other.report.input_digest
            && self.profiles.digest == other.profiles.digest
            && self.revision == other.revision
            && self.lab == other.lab
            && self.network == other.network
    }
}
#[derive(Default)]
struct State {
    status: Status,
    selected: Option<Selected>,
    observation: Option<HostObservation>,
    registry: Option<Registry>,
    session: Option<u64>,
    grant: Option<Arc<AuthorizationGrant>>,
}
#[derive(Default)]
pub struct Service {
    state: Mutex<State>,
    busy: AtomicBool,
    generation: AtomicU64,
    active: Mutex<Option<Cancellation>>,
    idle: Condvar,
    sessions: SessionManager,
    lab: bool,
}
struct Guard<'a> {
    service: &'a Service,
    token: Cancellation,
    generation: u64,
    completed: std::cell::Cell<bool>,
}
impl Drop for Guard<'_> {
    fn drop(&mut self) {
        let mut active = self
            .service
            .active
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if active
            .as_ref()
            .is_some_and(|token| token.same_instance(&self.token))
        {
            if !self.completed.get() {
                if let Ok(mut state) = self.service.state.lock() {
                    if let Some(id) = state.session.take() {
                        let _ = self.service.sessions.disconnect(id);
                    }
                    state.observation = None;
                    state.registry = None;
                    state.grant = None;
                    state.status.state = "failed".into();
                    state.status.read_approved = false;
                    state.status.profile_qualified = false;
                    state.status.inspection = None;
                }
            }
            *active = None;
            self.service.busy.store(false, Ordering::Release);
            self.service.idle.notify_all();
        }
    }
}
impl Guard<'_> {
    fn control(&self) -> Result<OperationControl, AdapterError> {
        OperationControl::new(
            self.token.clone(),
            Stage::Connection,
            AdapterOperation::AuthorizedIdentityProbe,
            Duration::from_secs(30),
        )
    }
    fn check(&self) -> Result<(), AdapterError> {
        if self.token.is_cancelled()
            || self.generation != self.service.generation.load(Ordering::Acquire)
        {
            Err(denied(
                "Operation was cancelled or its target generation changed.",
            ))
        } else {
            Ok(())
        }
    }
    fn publication(&self) -> Result<std::sync::MutexGuard<'_, Option<Cancellation>>, AdapterError> {
        let ownership = self
            .service
            .active
            .lock()
            .map_err(|_| AdapterError::internal("publication lock"))?;
        self.check()?;
        if ownership
            .as_ref()
            .is_none_or(|token| !token.same_instance(&self.token))
        {
            return Err(denied("Operation no longer owns its authorization scope."));
        }
        Ok(ownership)
    }
}
impl Service {
    #[cfg(feature = "native-test-profile")]
    pub(crate) fn native_lab() -> Self {
        Self {
            lab: true,
            ..Self::default()
        }
    }
    fn begin(&self) -> Result<Guard<'_>, AdapterError> {
        let mut active = self
            .active
            .lock()
            .map_err(|_| AdapterError::internal("operation lock"))?;
        self.busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| denied("Finish or cancel the active OpenWrt operation first."))?;
        let token = Cancellation::default();
        *active = Some(token.clone());
        Ok(Guard {
            service: self,
            token,
            generation: self.generation.fetch_add(1, Ordering::AcqRel) + 1,
            completed: std::cell::Cell::new(false),
        })
    }
    pub fn status(&self) -> Result<Status, AdapterError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| AdapterError::internal("inspection state"))?;
        if state
            .status
            .expires_at
            .is_some_and(|expires| connection::now() >= expires)
            && !self.busy.load(Ordering::Acquire)
        {
            if let Some(id) = state.session.take() {
                self.sessions.disconnect(id)?;
            }
            state.observation = None;
            state.grant = None;
            state.status.read_approved = false;
            state.status.profile_qualified = false;
            state.status.inspection = None;
            state.status.state = "disconnected".into();
        }
        let mut status = state.status.clone();
        status.busy = self.busy.load(Ordering::Acquire);
        status.generation = self.generation.load(Ordering::Acquire);
        Ok(status)
    }
    pub fn target_id(&self) -> Result<String, AdapterError> {
        self.status()?
            .target_id
            .ok_or_else(|| denied("Select a saved router first."))
    }
    pub fn lab_enabled(&self) -> bool {
        self.lab
    }
    pub fn choices(&self, saved: &[SavedRouter]) -> Vec<TargetChoice> {
        let mut values: Vec<_> = saved
            .iter()
            .map(|r| TargetChoice {
                id: r.id.clone(),
                label: r.label.clone(),
                address: r.address.clone(),
                interface_id: r.interface_id.clone(),
                lab: false,
            })
            .collect();
        if self.lab {
            values.push(TargetChoice {
                id: LAB_TARGET.into(),
                label: "Owned OpenWrt OS lab — QEMU, hardware unqualified".into(),
                address: "127.0.0.1".into(),
                interface_id: Some("owned-openwrt-qemu-loopback".into()),
                lab: true,
            });
        }
        values
    }
    fn fresh(&self, selected: &Selected) -> Result<(), AdapterError> {
        let state = self
            .state
            .lock()
            .map_err(|_| AdapterError::internal("inspection state"))?;
        if state
            .selected
            .as_ref()
            .is_none_or(|old| !old.same(selected))
            || state
                .status
                .expires_at
                .is_none_or(|expires| connection::now() >= expires)
        {
            return Err(denied(
                "Endpoint, interface, source address, evidence or approval changed; approve again.",
            ));
        }
        Ok(())
    }
    pub fn observe(&self, selected: Selected, request: Observe) -> Result<Status, AdapterError> {
        if !request.approved || request.target_id != selected.id || request.port == 0 {
            return Err(denied(
                "Explicit approval for one selected endpoint and port is required.",
            ));
        }
        if selected.lab
            && (!self.lab
                || selected.address != "127.0.0.1".parse::<IpAddr>().unwrap()
                || request.port != 22270)
        {
            return Err(denied(
                "Only the sealed owned OpenWrt OS lab endpoint is permitted.",
            ));
        }
        let guard = self.begin()?;
        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| AdapterError::internal("inspection state"))?;
            if let Some(id) = state.session.take() {
                self.sessions.disconnect(id)?;
            }
            *state = State::default();
            state.selected = Some(selected.clone());
        }
        let observed = transport::observe(selected.address, request.port, &guard.control()?)?;
        let publication = guard.publication()?;
        let endpoint = match selected.address {
            IpAddr::V4(ip) => format!("ssh://{ip}:{}", request.port),
            IpAddr::V6(ip) => format!("ssh://[{ip}]:{}", request.port),
        };
        let mut state = self
            .state
            .lock()
            .map_err(|_| AdapterError::internal("inspection state"))?;
        state.status = Status {
            state: "host_observed".into(),
            target_id: Some(selected.id.clone()),
            endpoint: Some(endpoint),
            interface_id: Some(selected.interface_id.clone()),
            source_address: Some(selected.source.to_string()),
            host_fingerprint: Some(observed.fingerprint().into()),
            expires_at: Some(connection::now() + 30_000),
            ..Status::default()
        };
        state.selected = Some(selected);
        state.observation = Some(observed);
        drop(state);
        drop(publication);
        guard.completed.set(true);
        drop(guard);
        self.status()
    }
    pub fn connect(
        &self,
        selected: Selected,
        request: Connect,
        key: &Path,
    ) -> Result<Status, AdapterError> {
        if !request.approved {
            return Err(denied(
                "Explicit pinned identity-handshake approval is required.",
            ));
        }
        let guard = self.begin()?;
        self.fresh(&selected)?;
        self.connect_owned(guard, selected, request, key)
    }
    pub fn connect_select_key(
        &self,
        selected: Selected,
        request: Connect,
        owner: isize,
        refresh: impl FnOnce() -> Result<Selected, AdapterError>,
    ) -> Result<Status, AdapterError> {
        if !request.approved {
            return Err(denied(
                "Explicit pinned identity-handshake approval is required.",
            ));
        }
        let guard = self.begin()?;
        self.fresh(&selected)?;
        {
            let state = self
                .state
                .lock()
                .map_err(|_| AdapterError::internal("inspection state"))?;
            if state
                .observation
                .as_ref()
                .is_none_or(|observed| observed.fingerprint() != request.fingerprint)
            {
                return Err(denied(
                    "Independently verified fingerprint does not match the single-use observation.",
                ));
            }
        }
        let key = pick_key(owner, guard.token.clone())?.ok_or_else(|| {
            denied("SSH key selection was cancelled; no authentication was attempted.")
        })?;
        guard.check()?;
        let current = refresh()?;
        self.fresh(&current)?;
        self.connect_owned(guard, current, request, &key)
    }
    fn connect_owned(
        &self,
        guard: Guard<'_>,
        selected: Selected,
        request: Connect,
        key: &Path,
    ) -> Result<Status, AdapterError> {
        let (observed, endpoint, expires_at) = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| AdapterError::internal("inspection state"))?;
            (
                state
                    .observation
                    .take()
                    .ok_or_else(|| denied("Host observation is absent or already consumed."))?,
                state.status.endpoint.clone().unwrap_or_default(),
                state.status.expires_at.unwrap_or(0),
            )
        };
        let target = SshTarget::confirm(
            observed,
            &request.fingerprint,
            selected.source,
            &request.username,
            key,
        )?;
        let peer_scope = format!("{}|source={}", request.fingerprint, selected.source);
        let binding = SessionBinding {
            device_id: selected.device.id.clone(),
            adapter_id: "openwrt-readonly-contract".into(),
            adapter_version: "0.2.0".into(),
            profile_id: String::new(),
            profile_version: String::new(),
            profile_hash: String::new(),
            evidence_digest: selected.report.input_digest.clone(),
            discovery_revision: selected.revision,
            endpoint,
            interface_id: selected.interface_id.clone(),
            endpoint_identity: peer_scope,
            expires_at,
            simulated: false,
        };
        let approval =
            HandshakeApproval::confirm_selected_target(binding.clone(), connection::now())?;
        let remote =
            (!selected.lab).then(|| (selected.device.id.clone(), selected.interface_id.clone()));
        let registry = Registry::for_openwrt(
            OpenWrtAdapter::prepared(binding.clone(), target),
            selected.lab,
            remote,
        )?;
        let session = self.sessions.connect_initial(
            &registry,
            &selected.context(),
            binding,
            &approval,
            Secret::new(vec![0])?,
            &guard.control()?,
        )?;
        let id = session
            .id
            .parse()
            .map_err(|_| AdapterError::internal("session id"))?;
        let publication = match guard.publication() {
            Ok(value) => value,
            Err(error) => {
                self.sessions.disconnect(id)?;
                return Err(error);
            }
        };
        let (identity, report) = self.sessions.authenticated_snapshot(id)?;
        let qualified = report.confidence == profiles::MatchConfidence::Verified;
        let mut parts = identity.management_identity.split('|');
        let public_identity = Identity {
            manufacturer: (!identity.manufacturer.is_empty()).then_some(identity.manufacturer),
            hardware_revision: (!identity.hardware_revision.is_empty())
                .then_some(identity.hardware_revision),
            model: identity.model,
            firmware_family: identity.firmware_family,
            firmware_version: identity.firmware_version,
            board_name: parts.next().unwrap_or_default().into(),
            target: parts.next().unwrap_or_default().into(),
            build_revision: parts.next().unwrap_or_default().into(),
        };
        let mut state = self
            .state
            .lock()
            .map_err(|_| AdapterError::internal("inspection state"))?;
        state.session = Some(id);
        state.registry = Some(registry);
        state.status.state = "identity_only".into();
        state.status.identity = Some(public_identity);
        state.status.profile_qualified = qualified;
        state.status.expires_at = Some(session.expires_at);
        drop(state);
        drop(publication);
        guard.completed.set(true);
        drop(guard);
        self.status()
    }
    pub fn approve_reads(
        &self,
        selected: Selected,
        request: Approval,
    ) -> Result<Status, AdapterError> {
        if !request.approved {
            return Err(denied("Separate configuration-read approval is required."));
        }
        let guard = self.begin()?;
        self.fresh(&selected)?;
        let publication = guard.publication()?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| AdapterError::internal("inspection state"))?;
        let id = state.session.ok_or_else(|| denied("Authenticate first."))?;
        let registry = state
            .registry
            .as_ref()
            .ok_or_else(|| denied("Authenticate first."))?;
        let grant = self.sessions.confirm_session_reads(
            id,
            registry,
            &selected.context(),
            vec![AdapterOperation::ReadConfiguration],
        )?;
        guard.check()?;
        state.grant = Some(Arc::new(grant));
        state.status.read_approved = true;
        state.status.state = "read_authorized".into();
        drop(state);
        drop(publication);
        guard.completed.set(true);
        drop(guard);
        self.status()
    }
    pub fn inspect(&self, selected: Selected) -> Result<Status, AdapterError> {
        if self
            .state
            .lock()
            .map_err(|_| AdapterError::internal("inspection state"))?
            .grant
            .is_none()
        {
            return Err(denied("Approve configuration reads separately."));
        }
        let guard = self.begin()?;
        self.fresh(&selected)?;
        let (id, registry, grant) = {
            let state = self
                .state
                .lock()
                .map_err(|_| AdapterError::internal("inspection state"))?;
            (
                state.session.ok_or_else(|| denied("Authenticate first."))?,
                state
                    .registry
                    .clone()
                    .ok_or_else(|| denied("Authenticate first."))?,
                state
                    .grant
                    .clone()
                    .ok_or_else(|| denied("Approve configuration reads separately."))?,
            )
        };
        let inspection = self.sessions.inspect_openwrt(
            id,
            &registry,
            &selected.context(),
            &grant,
            &guard.control()?,
        )?;
        let publication = guard.publication()?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| AdapterError::internal("inspection state"))?;
        state.status.inspection = Some(inspection);
        state.status.state = "inspected".into();
        // The approved inspection is single-use. A repeat requires another explicit approval.
        state.grant = None;
        drop(state);
        drop(publication);
        guard.completed.set(true);
        drop(guard);
        self.status()
    }
    pub fn cancel(&self) -> Result<Status, AdapterError> {
        let mut active = self
            .active
            .lock()
            .map_err(|_| AdapterError::internal("operation lock"))?;
        let cancelled = active.clone();
        if let Some(token) = &cancelled {
            token.cancel();
        }
        self.generation.fetch_add(1, Ordering::AcqRel);
        let deadline = Instant::now() + Duration::from_secs(1);
        while self.busy.load(Ordering::Acquire) {
            if active
                .as_ref()
                .zip(cancelled.as_ref())
                .is_none_or(|(current, old)| !current.same_instance(old))
            {
                return Err(denied(
                    "A later operation owns the busy state; retry cancellation explicitly.",
                ));
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(denied(
                    "Cancellation requested; operation cleanup is still in progress.",
                ));
            }
            active = self
                .idle
                .wait_timeout(active, remaining)
                .map_err(|_| AdapterError::internal("cancel wait"))?
                .0;
        }
        self.busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| denied("Another operation is active."))?;
        let token = Cancellation::default();
        *active = Some(token.clone());
        let guard = Guard {
            service: self,
            token,
            generation: self.generation.load(Ordering::Acquire),
            completed: std::cell::Cell::new(false),
        };
        drop(active);
        let mut state = self
            .state
            .lock()
            .map_err(|_| AdapterError::internal("inspection state"))?;
        if let Some(id) = state.session.take() {
            self.sessions.disconnect(id)?;
        }
        *state = State {
            status: Status {
                state: "disconnected".into(),
                ..Status::default()
            },
            ..State::default()
        };
        drop(state);
        guard.completed.set(true);
        drop(guard);
        self.status()
    }
    pub fn fail_generation(&self, generation: u64, error: AdapterError) {
        let Ok(active) = self.active.lock() else {
            return;
        };
        if self.generation.load(Ordering::Acquire) != generation || active.is_some() {
            return;
        }
        if let Ok(mut state) = self.state.lock() {
            if let Some(id) = state.session.take() {
                let _ = self.sessions.disconnect(id);
            }
            state.status.error = Some(error);
            state.status.state = "failed".into();
            state.observation = None;
            state.registry = None;
            state.grant = None;
            state.status.read_approved = false;
            state.status.profile_qualified = false;
            state.status.inspection = None;
        }
    }
    pub fn validate_current(&self, selected: &Selected) -> Result<(), AdapterError> {
        self.fresh(selected)
    }
    pub fn observe_discovery(&self, snapshot: &DiscoverySnapshot) {
        let Ok(active) = self.active.lock() else {
            return;
        };
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        if state
            .selected
            .as_ref()
            .is_some_and(|selected| !selected.lab && selected.revision != snapshot.revision)
        {
            if let Some(token) = active.as_ref() {
                token.cancel();
            }
            self.generation.fetch_add(1, Ordering::AcqRel);
            if let Some(id) = state.session.take() {
                let _ = self.sessions.disconnect(id);
            }
            state.observation = None;
            state.registry = None;
            state.grant = None;
            state.status.state = "disconnected".into();
            state.status.read_approved = false;
            state.status.profile_qualified = false;
            state.status.inspection = None;
        }
    }
    pub fn shutdown(&self) {
        if let Ok(active) = self.active.lock() {
            if let Some(token) = active.as_ref() {
                token.cancel();
            }
        }
        self.generation.fetch_add(1, Ordering::AcqRel);
        self.sessions.shutdown();
    }
}

pub fn select(
    saved: &SavedRouter,
    snapshot: &DiscoverySnapshot,
    registry: profiles::Registry,
    observations: Vec<profiles::EvidenceRecord>,
) -> Result<Selected, AdapterError> {
    let parsed = connection::parse_address(&saved.address)
        .map_err(|_| denied("Saved address is invalid or outside the local router scope."))?;
    let address: IpAddr = parsed.host.parse().map_err(|_| denied("Select a numeric local management IP; DNS name resolution is not authorized by this flow."))?;
    if matches!(address, IpAddr::V6(ip) if ip.is_unicast_link_local()) {
        return Err(denied("Scoped IPv6 link-local SSH is not qualified; select a local IPv4 or unique-local IPv6 management address."));
    }
    let networks: Vec<_> = snapshot
        .devices
        .iter()
        .filter(|d| d.connected)
        .flat_map(|d| d.networks.clone())
        .collect();
    let network = connection::select_interface(address, saved.interface_id.as_deref(), &networks)
        .map_err(|_| {
        denied("A unique active physical interface must reach the selected management address.")
    })?;
    if networks
        .iter()
        .flat_map(|network| &network.addresses)
        .filter_map(|value| value.split('%').next()?.parse::<IpAddr>().ok())
        .any(|value| value == address)
    {
        return Err(denied(
            "Host PC addresses cannot be selected as router endpoints.",
        ));
    }
    let sources: Vec<IpAddr> = network
        .addresses
        .iter()
        .filter_map(|value| value.split('%').next()?.parse::<IpAddr>().ok())
        .filter(|value| {
            value.is_ipv4() == address.is_ipv4() && !value.is_unspecified() && !value.is_loopback()
        })
        .collect();
    let source = *sources
        .first()
        .ok_or_else(|| denied("Selected interface has no source address for the target family."))?;
    // A saved remote endpoint has a separate identity from the PC NIC used to reach it.
    let mut device = discovery::history_device(
        saved.id.clone(),
        "Selected router endpoint".into(),
        saved.created_at,
        discovery::identity::Category::NetworkGatewayCandidate,
    );
    device.connected = true;
    device.transports = vec!["network".into()];
    // is_host_hardware deliberately excludes host NICs. This endpoint has no host PnP identity;
    // mark remote provenance by retaining no PC interfaces and a backend-only target identifier.
    let mut evidence = profiles::gather_evidence(&device, &[]);
    evidence.extend(observations);
    let report = profiles::evaluate(
        &registry,
        &device,
        evidence,
        connection::now(),
        snapshot.revision,
    );
    Ok(Selected {
        id: saved.id.clone(),
        address,
        source,
        interface_id: network.id.clone(),
        device,
        profiles: registry,
        report,
        revision: snapshot.revision,
        lab: false,
        network: Some(network.clone()),
    })
}

#[cfg(feature = "native-test-profile")]
pub(crate) fn lab_selected(registry: profiles::Registry) -> Result<Selected, AdapterError> {
    let mut device = discovery::history_device(
        LAB_TARGET.into(),
        "Owned OpenWrt QEMU OS lab".into(),
        1,
        discovery::identity::Category::NetworkGatewayCandidate,
    );
    device.connected = true;
    device.transports = vec!["network".into()];
    let report = profiles::evaluate(&registry, &device, vec![], connection::now(), 1);
    Ok(Selected {
        id: LAB_TARGET.into(),
        address: "127.0.0.1".parse().unwrap(),
        source: "127.0.0.1".parse().unwrap(),
        interface_id: "owned-openwrt-qemu-loopback".into(),
        device,
        profiles: registry,
        report,
        revision: 1,
        lab: true,
        network: None,
    })
}

#[cfg(feature = "native-test-profile")]
pub(crate) fn lab_profiles(
    mut registry: profiles::Registry,
) -> Result<profiles::Registry, AdapterError> {
    // These exact values came from the independently controlled QEMU serial console. This
    // profile exists only in the marked native-test process, never in bundled physical support.
    let mut document: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../device-profiles/tplink-archer-c7-v5-openwrt.json"
    ))
    .map_err(|_| AdapterError::internal("lab profile"))?;
    document["schemaVersion"] = 2.into();
    document["inspectionOnly"] = true.into();
    document["profileId"] = "owned.openwrt-qemu-25-12-6.inspection".into();
    document["profileVersion"] = "1.0.0".into();
    document["manufacturer"] = "".into();
    document["model"] = "QEMU Ubuntu 26.04 PC (i440FX + PIIX, 1996)".into();
    document["modelAliases"] = serde_json::json!([]);
    document["hardwareRevisions"] = serde_json::json!([]);
    document["deviceCategory"] = serde_json::json!(["openwrt_router"]);
    document["firmwareVersionConstraints"] =
        serde_json::json!({"exact":["25.12.6"],"minInclusive":null,"maxExclusive":null});
    document["matchRules"][0]["values"] =
        serde_json::json!(["qemu-ubuntu-26-04-pc-i440fx-piix-1996|x86/64|r33311-981c978eff"]);
    document["matchRules"][0]["explanation"] = "Exact owned virtual OS board, target and build; no physical manufacturer or revision assertion.".into();
    if let Some(features) = document["capabilities"].as_object_mut() {
        for feature in features.values_mut() {
            feature["assessment"] = "UNKNOWN".into();
            feature["evidenceSources"] = serde_json::json!([]);
            feature["reason"] =
                "Virtual OS inspection only; physical feature support is unqualified.".into();
            if let Some(sub) = feature["subCapabilities"].as_object_mut() {
                for field in sub.values_mut() {
                    field["assessment"] = "UNKNOWN".into();
                    field["evidenceSources"] = serde_json::json!([]);
                    field["reason"] = "No physical feature qualification.".into();
                }
            }
        }
    }
    document["knownLimitations"] = serde_json::json!(["Owned OpenWrt 25.12.6 x86/64 OS interoperability only; no hardware feature or write support."]);
    document["evidenceSources"] = serde_json::json!([{"id":"serial","category":"upstream_documentation","reference":"https://downloads.openwrt.org/releases/25.12.6/targets/x86/64/","summary":"Exact board, target and release were observed through the independently owned VM console using this official image. Physical hardware was not tested."}]);
    document["provenance"] = serde_json::json!({"author":"OBTAINER controlled OpenWrt OS lab","category":"upstream_documentation","notes":"Official release plus owned VM OS interoperability qualification; no physical device qualification."});
    // Remove inherited feature references to the unrelated Archer reference documentation.
    document["managementInterfaces"][0]["evidenceSources"] = serde_json::json!(["serial"]);
    document["managementInterfaces"][0]["security"] = "Pinned SSH key authentication plus exact inspection-only OS profile and separate single-use read approval.".into();
    document["managementInterfaces"][0]["limitations"] =
        "No physical hardware support, writes, extraction or live USSD.".into();
    document["modemInformation"]["notes"] =
        "Owned QEMU VM has no emulated modem; no physical modem qualification.".into();
    let text =
        serde_json::to_string(&document).map_err(|_| AdapterError::internal("lab profile"))?;
    profiles::validate(&text)
        .map_err(|error| denied(&format!("Owned OS lab profile: {}", error.message)))?;
    let mut loaded = profiles::Registry::load(vec![(&text, "bundled", true)]);
    if loaded.profiles.len() != 1 {
        return Err(denied(
            "Owned OpenWrt OS lab profile failed strict validation.",
        ));
    }
    loaded.profiles[0].origin = "owned_openwrt_os_lab".into();
    registry.profiles.extend(loaded.profiles);
    registry
        .profiles
        .sort_by(|a, b| a.profile.profile_id.cmp(&b.profile.profile_id));
    registry.digest = profiles::digest(
        &serde_json::to_vec(&registry.profiles)
            .map_err(|_| AdapterError::internal("lab registry"))?,
    );
    Ok(registry)
}

pub(crate) fn pick_key(
    owner: isize,
    cancellation: Cancellation,
) -> Result<Option<PathBuf>, AdapterError> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::Controls::Dialogs::*;
        use windows_sys::Win32::UI::WindowsAndMessaging::*;
        struct Picker {
            cancellation: Cancellation,
            deadline: Instant,
        }
        unsafe extern "system" fn hook(
            window: windows_sys::Win32::Foundation::HWND,
            message: u32,
            _: usize,
            parameter: isize,
        ) -> usize {
            if message == WM_INITDIALOG {
                let info = unsafe { &*(parameter as *const OPENFILENAMEW) };
                unsafe {
                    SetWindowLongPtrW(window, GWLP_USERDATA, info.lCustData);
                    SetTimer(window, 7, 100, None);
                }
            } else if message == WM_TIMER {
                let pointer = unsafe { GetWindowLongPtrW(window, GWLP_USERDATA) } as *const Picker;
                if !pointer.is_null() {
                    let picker = unsafe { &*pointer };
                    if picker.cancellation.is_cancelled() || Instant::now() >= picker.deadline {
                        unsafe {
                            PostMessageW(GetParent(window), WM_COMMAND, 2, 0);
                        }
                    }
                }
            } else if message == WM_DESTROY {
                unsafe {
                    KillTimer(window, 7);
                }
            }
            0
        }
        let picker = Picker {
            cancellation,
            deadline: Instant::now() + Duration::from_secs(25),
        };
        let mut buffer = vec![0u16; 32768];
        let title: Vec<u16> = "Select existing SSH private key — contents stay in backend\0"
            .encode_utf16()
            .collect();
        let filter: Vec<u16> = "Existing SSH key\0*.*\0\0".encode_utf16().collect();
        let mut info: OPENFILENAMEW = unsafe { std::mem::zeroed() };
        info.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        info.hwndOwner = owner as _;
        info.lpstrFile = buffer.as_mut_ptr();
        info.nMaxFile = buffer.len() as u32;
        info.lpstrTitle = title.as_ptr();
        info.lpstrFilter = filter.as_ptr();
        info.Flags = OFN_EXPLORER
            | OFN_PATHMUSTEXIST
            | OFN_FILEMUSTEXIST
            | OFN_NOCHANGEDIR
            | OFN_DONTADDTORECENT
            | OFN_ENABLEHOOK;
        info.lpfnHook = Some(hook);
        info.lCustData = (&picker as *const Picker) as isize;
        if unsafe { GetOpenFileNameW(&mut info) } == 0 {
            if unsafe { CommDlgExtendedError() } == 0 {
                return Ok(None);
            }
            return Err(denied("Windows key selection failed."));
        }
        let end = buffer.iter().position(|v| *v == 0).unwrap_or(buffer.len());
        Ok(Some(PathBuf::from(
            String::from_utf16(&buffer[..end]).map_err(|_| denied("Invalid key filename."))?,
        )))
    }
    #[cfg(not(windows))]
    {
        let _ = (owner, cancellation);
        Err(denied(
            "Key selection requires the Windows native application.",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn operation_guard_owns_busy_and_idle_cancel_is_reusable() {
        let service = Service::default();
        for _ in 0..2 {
            assert!(!service.cancel().unwrap().busy);
        }
        let owner = service.begin().unwrap();
        assert!(service.begin().is_err());
        assert!(service.status().unwrap().busy);
        drop(owner);
        assert!(!service.status().unwrap().busy);
        let owner = service.begin().unwrap();
        drop(owner);
        assert!(!service.status().unwrap().busy);
    }
    #[test]
    fn cancellation_invalidates_publication_without_releasing_another_owner() {
        let service = Service::default();
        let guard = service.begin().unwrap();
        guard.token.cancel();
        assert!(guard.publication().is_err());
        assert!(service.status().unwrap().busy);
        drop(guard);
        assert!(!service.status().unwrap().busy);
    }
    #[test]
    fn discovery_change_cancels_inflight_remote_request_and_rejects_stale_publication() {
        let service = Service::default();
        let selected = lab_like();
        {
            let mut state = service.state.lock().unwrap();
            state.selected = Some(selected);
            state.status.expires_at = Some(connection::now() + 30_000);
        }
        let owner = service.begin().unwrap();
        service.observe_discovery(&DiscoverySnapshot {
            revision: 2,
            ..DiscoverySnapshot::default()
        });
        assert!(owner.token.is_cancelled());
        assert!(owner.publication().is_err());
        assert!(
            service.status().unwrap().busy,
            "only the active owner releases busy"
        );
        drop(owner);
        assert!(!service.status().unwrap().busy);
        assert_eq!(service.sessions.active_count(), 0);
    }
    #[test]
    fn late_failure_cannot_clear_newer_operation_state() {
        let service = Service::default();
        let previous = service.begin().unwrap();
        let generation = previous.generation;
        previous.completed.set(true);
        drop(previous);
        let newer = service.begin().unwrap();
        service.fail_generation(generation, denied("old operation failed"));
        assert!(service.status().unwrap().busy);
        assert!(service.status().unwrap().error.is_none());
        newer.completed.set(true);
        drop(newer);
    }
    #[test]
    fn cold_unapproved_actions_never_create_sessions_or_touch_network() {
        let service = Service::default();
        assert!(service
            .approve_reads(lab_like(), Approval { approved: true })
            .is_err());
        assert!(service.inspect(lab_like()).is_err());
        assert!(service
            .observe(
                lab_like(),
                Observe {
                    target_id: LAB_TARGET.into(),
                    port: 22,
                    approved: false
                }
            )
            .is_err());
        assert_eq!(service.sessions.active_count(), 0);
        assert!(!service.status().unwrap().busy);
    }
    fn lab_like() -> Selected {
        let profiles = profiles::Registry::bundled();
        let mut device = discovery::history_device(
            LAB_TARGET.into(),
            "test-only".into(),
            1,
            discovery::identity::Category::Unknown,
        );
        device.connected = true;
        device.transports = vec!["network".into()];
        let report = profiles::evaluate(&profiles, &device, vec![], connection::now(), 1);
        Selected {
            id: LAB_TARGET.into(),
            address: "127.0.0.1".parse().unwrap(),
            source: "127.0.0.1".parse().unwrap(),
            interface_id: "not-authority".into(),
            device,
            profiles,
            report,
            revision: 1,
            lab: false,
            network: None,
        }
    }
    #[test]
    fn stale_target_source_interface_evidence_and_endpoint_are_rejected() {
        let service = Service::default();
        let selected = lab_like();
        {
            let mut state = service.state.lock().unwrap();
            state.selected = Some(selected.clone());
            state.status.expires_at = Some(connection::now() + 30_000);
        }
        assert!(service.fresh(&selected).is_ok());
        for variant in 0..5 {
            let mut changed = selected.clone();
            match variant {
                0 => changed.source = "127.0.0.2".parse().unwrap(),
                1 => changed.address = "127.0.0.2".parse().unwrap(),
                2 => changed.interface_id = "other".into(),
                3 => changed.revision += 1,
                _ => changed.report.input_digest = "changed".into(),
            }
            assert!(service.fresh(&changed).is_err());
        }
    }
    #[test]
    fn normal_ipc_target_resolution_rejects_loopback_and_pc_address() {
        let saved = SavedRouter {
            id: LAB_TARGET.into(),
            address: "http://127.0.0.1/".into(),
            label: "cannot confer scope".into(),
            interface_id: None,
            device_id: None,
            created_at: 1,
            last_result: None,
            state: "discovered".into(),
        };
        assert!(select(
            &saved,
            &DiscoverySnapshot::default(),
            profiles::Registry::bundled(),
            vec![]
        )
        .is_err());
        let first = NetworkAdapter {
            id: "first-physical-interface".into(),
            link_up: true,
            addresses: vec!["192.168.20.10".into()],
            prefixes: vec![discovery::AddressPrefix {
                address: "192.168.20.10".into(),
                length: 24,
            }],
            ..NetworkAdapter::default()
        };
        let second = NetworkAdapter {
            id: "other-physical-interface".into(),
            addresses: vec!["192.168.20.2".into()],
            ..first.clone()
        };
        let mut host = discovery::history_device(
            "host-record".into(),
            "PC network interfaces".into(),
            1,
            discovery::identity::Category::GenericNetworkInterface,
        );
        host.connected = true;
        host.networks = vec![first.clone(), second];
        let snapshot = DiscoverySnapshot {
            revision: 1,
            devices: vec![host],
            ..DiscoverySnapshot::default()
        };
        let saved = SavedRouter {
            address: "http://192.168.20.2/".into(),
            interface_id: Some(first.id.clone()),
            ..saved
        };
        assert!(
            select(&saved, &snapshot, profiles::Registry::bundled(), vec![]).is_err(),
            "PC IP on another NIC must be rejected"
        );
        for interface_type in [6, 71] {
            // Ethernet and Wi-Fi reach the remote router, never become it.
            let mut snapshot = snapshot.clone();
            snapshot.devices[0].networks[0].interface_type = interface_type;
            let remote = SavedRouter {
                address: "http://192.168.20.1/".into(),
                ..saved.clone()
            };
            let selected =
                select(&remote, &snapshot, profiles::Registry::bundled(), vec![]).unwrap();
            assert!(!crate::adapters::protected(&selected.device));
            assert!(selected.device.interfaces.is_empty());
            assert_eq!(
                selected.device.category,
                discovery::identity::Category::NetworkGatewayCandidate
            );
            snapshot.devices[0].last_seen_at += 30_000;
            snapshot.scanned_at = Some(connection::now());
            let rescanned =
                select(&remote, &snapshot, profiles::Registry::bundled(), vec![]).unwrap();
            assert!(selected.same(&rescanned));
            let service = Service::default();
            service.state.lock().unwrap().selected = Some(selected);
            service.observe_discovery(&snapshot);
            assert_eq!(
                service.generation.load(Ordering::Acquire),
                0,
                "unchanged scan must not invalidate remote scope"
            );
            snapshot.revision += 1;
            service.observe_discovery(&snapshot);
            assert_eq!(service.status().unwrap().state, "disconnected");
        }
    }
    #[test]
    #[cfg(feature = "native-test-profile")]
    fn sealed_os_profile_has_no_invented_hardware_or_feature_authority() {
        let registry = lab_profiles(profiles::Registry::bundled()).unwrap();
        let profile = registry
            .profiles
            .iter()
            .find(|p| p.profile.profile_id == "owned.openwrt-qemu-25-12-6.inspection")
            .unwrap();
        assert!(profile.profile.inspection_only);
        assert!(profile.authoritative);
        assert!(profile.profile.manufacturer.is_empty());
        assert!(profile.profile.hardware_revisions.is_empty());
        let features = &profile.profile.capabilities;
        assert!([
            &features.dns,
            &features.wifi_user_limit,
            &features.apn,
            &features.ussd
        ]
        .iter()
        .all(|f| f.assessment == profiles::Assessment::Unknown));
        assert!(!Service::default().lab_enabled());
        assert!(Service::default().choices(&[]).is_empty());
    }
    #[test]
    #[cfg(feature = "native-test-profile")]
    #[ignore = "Requires the explicitly started owned genuine OpenWrt QEMU OS lab"]
    fn genuine_openwrt_full_bootstrap_approval_and_inspection() {
        let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../.tools/phase7-vm");
        let console = std::fs::read_to_string(directory.join("console-identity.log")).unwrap();
        let independent = console
            .split_whitespace()
            .find(|s| s.starts_with("SHA256:"))
            .unwrap();
        let profiles = lab_profiles(profiles::Registry::bundled()).unwrap();
        let service = Service::native_lab();
        let selected = lab_selected(profiles).unwrap();
        let before = service.status().unwrap();
        assert!(before.identity.is_none());
        let observed = service
            .observe(
                selected.clone(),
                Observe {
                    target_id: LAB_TARGET.into(),
                    port: 22270,
                    approved: true,
                },
            )
            .unwrap();
        assert_eq!(observed.host_fingerprint.as_deref(), Some(independent));
        let connected = service
            .connect(
                selected.clone(),
                Connect {
                    fingerprint: independent.into(),
                    username: "root".into(),
                    approved: true,
                },
                &directory.join("test-key"),
            )
            .unwrap();
        assert_eq!(connected.state, "identity_only");
        assert!(connected.profile_qualified);
        assert!(connected.identity.as_ref().unwrap().manufacturer.is_none());
        assert!(connected
            .identity
            .as_ref()
            .unwrap()
            .hardware_revision
            .is_none());
        assert!(!connected.read_approved);
        assert!(service.inspect(selected.clone()).is_err());
        service
            .approve_reads(selected.clone(), Approval { approved: true })
            .unwrap();
        let inspected = service.inspect(selected.clone()).unwrap();
        assert_eq!(inspected.state, "inspected");
        assert!(!inspected.busy);
        assert!(!inspected.inspection.as_ref().unwrap().networks.is_empty());
        assert!(
            service.inspect(selected).is_err(),
            "one explicit approval grants one inspection"
        );
        let stopped = service.cancel().unwrap();
        assert!(!stopped.busy);
        assert_eq!(service.sessions.active_count(), 0);
        std::fs::write(
            directory.join("rust-runtime-inspection.json"),
            serde_json::to_vec_pretty(&inspected).unwrap(),
        )
        .unwrap();
    }
}
