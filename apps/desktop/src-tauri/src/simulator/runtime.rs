use super::{
    adapter::SimulatorAdapter,
    model::{self, Router, Snapshot},
    types::*,
    Authority,
};
use crate::{adapters::*, discovery, profiles};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

struct Pending {
    id: String,
    binding: SessionBinding,
    approval: HandshakeApproval,
}
#[derive(Default)]
struct Access {
    pending: Option<Pending>,
    session: Option<u64>,
    grant: Option<AuthorizationGrant>,
}
struct ScenarioReservation<'a>(&'a AtomicBool);
impl Drop for ScenarioReservation<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}
pub(super) struct Runtime {
    state: Arc<Mutex<Router>>,
    authority: Authority,
    profiles: profiles::Registry,
    registry: Registry,
    manager: SessionManager,
    access: Mutex<Access>,
    busy: AtomicBool,
    cancellation: Mutex<Option<Cancellation>>,
    pub(super) resources: Arc<AtomicUsize>,
}
struct ContextData {
    device: discovery::Device,
    report: profiles::CompatibilityReport,
    now: u64,
    epoch: u64,
}
impl ContextData {
    fn ctx<'a>(&'a self, profiles: &'a profiles::Registry) -> DeviceContext<'a> {
        DeviceContext {
            device: &self.device,
            profile: &self.report,
            profiles,
            revision: self.epoch,
            now: self.now,
        }
    }
}
fn lock<T>(m: &Mutex<T>) -> Result<std::sync::MutexGuard<'_, T>> {
    m.lock()
        .map_err(|_| fail(ErrorCode::Internal, "Simulation state lock unavailable."))
}
impl Runtime {
    fn new(owner: &str) -> Result<Self> {
        let authority = Authority {
            epoch: Arc::new(AtomicU64::new(1)),
            enabled: Arc::new(AtomicBool::new(true)),
            endpoint: format!("simulator://{owner}/{DEVICE_ID}"),
            pin: format!("SIMULATOR-ONLY:{owner}"),
        };
        let state = Arc::new(Mutex::new(Router::new(10_000_000)));
        let resources = Arc::new(AtomicUsize::new(0));
        let mut registry = Registry::simulator(authority.clone());
        registry.register(SimulatorAdapter {
            state: state.clone(),
            authority: authority.clone(),
            resources: resources.clone(),
        })?;
        // This registry never enters Repository/install_registry or physical matching. Local
        // fictional specifications are authoritative only within this sealed virtual runtime.
        let mut profiles = profiles::Registry::load(vec![(
            include_str!("../../../../../tests/fixtures/simulator/virtual-router.json"),
            "simulation-only",
            false,
        )]);
        if profiles.profiles.len() != 1 {
            return Err(fail(
                ErrorCode::Internal,
                &format!(
                    "Fictional simulator profile failed schema validation: {}",
                    profiles.diagnostics.join("; ")
                ),
            ));
        }
        profiles.profiles[0].authoritative = true;
        profiles.digest =
            profiles::digest(&serde_json::to_vec(&profiles.profiles).map_err(|_| {
                fail(
                    ErrorCode::Internal,
                    "Simulation profile serialization failed.",
                )
            })?);
        let manager = SessionManager::default();
        manager.observe_discovery(&discovery::DiscoverySnapshot {
            revision: 1,
            ..Default::default()
        });
        Ok(Self {
            state,
            authority,
            profiles,
            registry,
            manager,
            access: Mutex::new(Access::default()),
            busy: AtomicBool::new(false),
            cancellation: Mutex::new(None),
            resources,
        })
    }
    fn data(&self) -> Result<ContextData> {
        let s = lock(&self.state)?;
        let epoch = self.authority.epoch.load(Ordering::Acquire);
        let d = discovery::Device {
            id: DEVICE_ID.into(),
            display_name: "SIMULATION: Virtual Router Lab".into(),
            connected: s.available,
            transports: vec!["simulation".into()],
            vendor_id: None,
            product_id: None,
            container_id: None,
            interfaces: vec![],
            networks: vec![],
            assessment: "Fictional virtual device; never Windows discovery evidence.".into(),
            confidence: 0,
            evidence: vec![],
            capabilities: discovery::Capabilities {
                inventory: discovery::Capability::Supported,
                backup: discovery::Capability::Unsupported,
                flash: discovery::Capability::Unsupported,
                features: Default::default(),
            },
            last_seen_at: 0,
            identity: Default::default(),
            category: discovery::identity::Category::Unknown,
            lifecycle: "simulation".into(),
        };
        let report = profiles::evaluate(&self.profiles, &d, vec![], s.clock, epoch);
        Ok(ContextData {
            device: d,
            report,
            now: s.clock,
            epoch,
        })
    }
    fn binding(&self, d: &ContextData) -> SessionBinding {
        let p = &self.profiles.profiles[0];
        SessionBinding {
            device_id: DEVICE_ID.into(),
            adapter_id: "obtainer-simulator".into(),
            adapter_version: "1.0.0".into(),
            profile_id: PROFILE_ID.into(),
            profile_version: p.profile.profile_version.clone(),
            profile_hash: p.content_hash.clone(),
            evidence_digest: d.report.input_digest.clone(),
            discovery_revision: d.epoch,
            endpoint: self.authority.endpoint.clone(),
            interface_id: "sim-interface-001".into(),
            endpoint_identity: self.authority.pin.clone(),
            expires_at: d.now + 30_000,
            simulated: true,
        }
    }
    fn device(&self) -> VirtualDevice {
        let identity = model::identity_fixture();
        VirtualDevice {
            id: DEVICE_ID.into(),
            manufacturer: identity.manufacturer,
            model: identity.model,
            hardware_revision: identity.hardware_revision,
            firmware_family: identity.firmware_family,
            firmware_version: identity.firmware_version,
            operating_system: identity.operating_system,
            endpoint: self.authority.endpoint.clone(),
            interface_id: "sim-interface-001".into(),
            peer_pin: self.authority.pin.clone(),
            simulated: true,
        }
    }
    pub(super) fn status(&self) -> Result<Status> {
        let mut a = lock(&self.access)?;
        let s = lock(&self.state)?;
        let session = a
            .session
            .and_then(|id| self.manager.status(id, s.clock).ok());
        if session.is_none() {
            a.session = None;
            a.grant = None;
        }
        Ok(Status {
            active_connections: self.resources.load(Ordering::Acquire),
            available: true,
            active: self.authority.enabled.load(Ordering::Acquire),
            device: Some(self.device()),
            epoch: self.authority.epoch.load(Ordering::Acquire),
            revision: s.revision,
            virtual_time: s.clock,
            scenario: s.scenario,
            scenarios: SCENARIOS.to_vec(),
            faults: s.faults.clone(),
            connection: session.as_ref().map_or(
                if s.available {
                    ConnectionState::AuthenticationRequired
                } else {
                    ConnectionState::Disconnected
                },
                |v| v.state,
            ),
            authentication: session
                .as_ref()
                .map_or(AuthenticationState::Required, |v| v.authentication),
            approval_id: a.pending.as_ref().map(|p| p.id.clone()),
            read_authorized: a.grant.is_some(),
            features: s
                .features
                .clone()
                .filter(|f| f.observed_revision == s.revision),
            preview: s.preview.clone(),
            outcome: s.outcome.clone(),
            snapshots: s.snapshots.iter().map(|s| s.info.clone()).collect(),
            ussd: s.ussd.clone(),
            events: s.events.clone(),
            busy: self.busy.load(Ordering::Acquire),
        })
    }
    fn cancel(&self) -> Result<()> {
        if let Some(c) = lock(&self.cancellation)?.as_ref() {
            c.cancel();
        }
        Ok(())
    }
    fn invalidate(&self) -> Result<()> {
        self.cancel()?;
        let revision = self.authority.epoch.fetch_add(1, Ordering::AcqRel) + 1;
        {
            let mut state = lock(&self.state)?;
            state.features = None;
            state.preview = None;
            state.outcome = None;
        }
        self.manager
            .observe_discovery(&discovery::DiscoverySnapshot {
                revision,
                ..Default::default()
            });
        *lock(&self.access)? = Access::default();
        Ok(())
    }
    fn stop(&self) -> Result<()> {
        self.authority.enabled.store(false, Ordering::Release);
        self.cancel()?;
        self.manager.shutdown();
        Ok(())
    }
    fn control(&self, control: Control) -> Result<()> {
        match control {
            Control::Cancel => {
                self.cancel()?;
                let deadline = std::time::Instant::now() + Duration::from_secs(1);
                while self.busy.load(Ordering::Acquire) && std::time::Instant::now() < deadline {
                    std::thread::sleep(Duration::from_millis(1));
                }
                if self
                    .busy
                    .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                    .is_err()
                {
                    return Err(fail(
                        ErrorCode::Timeout,
                        "Virtual cancellation did not finish within its bound.",
                    ));
                }
            }
            Control::Reset => {
                self.cancel()?;
                self.authority.epoch.fetch_add(1, Ordering::AcqRel);
                let mut s = lock(&self.state)?;
                let clock = s.clock + 1;
                *s = Router::new(clock);
                drop(s);
                self.manager
                    .observe_discovery(&discovery::DiscoverySnapshot {
                        revision: self.authority.epoch.load(Ordering::Acquire),
                        ..Default::default()
                    });
                *lock(&self.access)? = Access::default();
            }
            Control::Scenario { scenario, repeat } => {
                if self.busy.load(Ordering::Acquire) {
                    return Err(fail(
                        ErrorCode::Busy,
                        "Cancel the running virtual operation before changing scenarios.",
                    ));
                }
                let _reservation = ScenarioReservation(&self.busy);
                let mut state = lock(&self.state)?;
                let changed_identity = matches!(
                    scenario,
                    Scenario::WrongIdentity | Scenario::EndpointSubstitution
                ) || matches!(
                    state.scenario,
                    Scenario::WrongIdentity | Scenario::EndpointSubstitution
                );
                state.scenario(scenario, repeat);
                drop(state);
                if scenario == Scenario::RouterDisconnected || changed_identity {
                    self.invalidate()?;
                }
            }
            Control::Advance { milliseconds } => {
                if milliseconds > 300_000 {
                    return Err(fail(
                        ErrorCode::InvalidInput,
                        "Advance virtual time by at most 300000 ms.",
                    ));
                }
                let mut s = lock(&self.state)?;
                s.clock = s.clock.saturating_add(milliseconds);
                if s.reconnect_at.is_some_and(|t| s.clock >= t) {
                    s.available = true;
                    s.configuration.wifi_service = "running".into();
                    s.reconnect_at = None;
                }
                if s.cellular_reconnect_at.is_some_and(|t| s.clock >= t) {
                    s.configuration.modem.service = "running".into();
                    s.configuration.modem.data_connected = true;
                    s.cellular_reconnect_at = None;
                    s.features = None;
                    s.event("Virtual cellular service restart completed.");
                }
                let now = s.clock;
                if let Some(u) = s.ussd.as_mut() {
                    if u.state == "active" && u.expires_at <= now {
                        u.state = "timed_out".into();
                        u.text = "SIMULATION: USSD session timed out.".into();
                    }
                }
            }
            _ => {
                return Err(fail(
                    ErrorCode::InvalidInput,
                    "Use the service start/stop boundary.",
                ))
            }
        };
        Ok(())
    }
    fn read_dns(
        &self,
        a: &Access,
        d: &ContextData,
        c: &OperationControl,
    ) -> Result<DnsConfiguration> {
        let id = a.session.ok_or_else(|| {
            fail(
                ErrorCode::Unauthorized,
                "Connect and authorize reads first.",
            )
        })?;
        let grant = a.grant.as_ref().ok_or_else(|| {
            fail(
                ErrorCode::Unauthorized,
                "Separate read approval is required.",
            )
        })?;
        Ok(self
            .manager
            .read_dns(id, &self.registry, &d.ctx(&self.profiles), grant, c)?)
    }
    pub(super) fn perform(&self, input: Request) -> Result<Status> {
        if input.endpoint != self.authority.endpoint {
            return Err(fail(
                ErrorCode::EndpointUntrusted,
                "Request endpoint belongs to another simulator runtime.",
            ));
        }
        if input.target != DEVICE_ID {
            return Err(fail(
                ErrorCode::NotFound,
                "Target is not owned by this simulator.",
            ));
        }
        if !self.authority.enabled.load(Ordering::Acquire) {
            return Err(fail(ErrorCode::Disabled, "Simulation runtime stopped."));
        }
        let mut token_slot = lock(&self.cancellation)?;
        if self
            .busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(fail(
                ErrorCode::Busy,
                "A virtual operation is already running.",
            ));
        }
        let cancellation = Cancellation::default();
        *token_slot = Some(cancellation.clone());
        drop(token_slot);
        let c = OperationControl::new(
            cancellation,
            Stage::Connection,
            AdapterOperation::AuthorizedIdentityProbe,
            Duration::from_millis(500),
        )?;
        let result = self.perform_inner(input, &c);
        if result
            .as_ref()
            .err()
            .is_some_and(|e| matches!(e.code, ErrorCode::Cancelled | ErrorCode::Timeout))
        {
            if let Ok(mut access) = self.access.lock() {
                if let Some(id) = access.session.take() {
                    let _ = self.manager.disconnect(id);
                }
                access.grant = None;
            }
        }
        if let Err(error) = &result {
            if let Ok(mut state) = self.state.lock() {
                state.event(&format!("Simulation operation rejected: {:?}", error.code));
            }
        }
        *lock(&self.cancellation)? = None;
        self.busy.store(false, Ordering::Release);
        result?;
        self.status()
    }
    fn perform_inner(&self, input: Request, c: &OperationControl) -> Result<()> {
        let d = self.data()?;
        if input.epoch != d.epoch {
            return Err(fail(
                ErrorCode::Stale,
                "Virtual device generation changed; refresh and reauthorize.",
            ));
        }
        if input.revision != lock(&self.state)?.revision {
            return Err(fail(
                ErrorCode::Stale,
                "Configuration revision changed; read and preview again.",
            ));
        }
        let mut a = lock(&self.access)?;
        c.checkpoint()?;
        match input.action {
            Action::ApproveHandshake => {
                let b = self.binding(&d);
                let approval = HandshakeApproval::confirm_selected_target(b.clone(), d.now)?;
                let id = lock(&self.state)?.id("approval");
                a.pending = Some(Pending {
                    id,
                    binding: b,
                    approval,
                });
            }
            Action::Connect {
                approval_id,
                credential,
            } => {
                let p=a.pending.take().filter(|p|p.id==approval_id).ok_or_else(||fail(ErrorCode::Unauthorized,"Approve this virtual handshake explicitly; approval cannot be replayed."))?;
                if let Some(id) = a.session.take() {
                    self.manager.disconnect(id)?;
                }
                a.grant = None;
                let status = self.manager.connect_initial(
                    &self.registry,
                    &d.ctx(&self.profiles),
                    p.binding,
                    &p.approval,
                    Secret::new(credential.into_bytes())?,
                    c,
                )?;
                a.session = Some(status.id.parse().map_err(|_| {
                    fail(ErrorCode::Internal, "Invalid internal session identifier.")
                })?);
                lock(&self.state)?.event("Simulated Connected: peer and fictional identity validated through Phase 5 bootstrap.");
            }
            Action::ApproveReads => {
                let id = a.session.ok_or_else(|| {
                    fail(ErrorCode::Unauthorized, "Connect before approving reads.")
                })?;
                a.grant = Some(self.manager.confirm_session_reads(
                    id,
                    &self.registry,
                    &d.ctx(&self.profiles),
                    vec![
                        AdapterOperation::AuthorizedIdentityProbe,
                        AdapterOperation::InspectCapabilities,
                        AdapterOperation::ReadDns,
                        AdapterOperation::ReadWifiLimit,
                        AdapterOperation::ReadApn,
                        AdapterOperation::ReadUssdCapability,
                    ],
                )?);
            }
            Action::Disconnect => {
                if let Some(id) = a.session.take() {
                    self.manager.disconnect(id)?;
                }
                *a = Access::default();
                lock(&self.state)?.features = None;
            }
            Action::ReadFeatures => {
                let dns = self.read_dns(&a, &d, c)?;
                let id = a
                    .session
                    .ok_or_else(|| fail(ErrorCode::Expired, "Virtual session closed."))?;
                let g = a
                    .grant
                    .as_ref()
                    .ok_or_else(|| fail(ErrorCode::Unauthorized, "Read approval absent."))?;
                let ctx = d.ctx(&self.profiles);
                self.manager.read_identity(id, &self.registry, &ctx, g, c)?;
                self.manager.probe(id, &self.registry, &ctx, g, c)?;
                let wifi = self
                    .manager
                    .read_wifi_all(id, &self.registry, &ctx, g, c)
                    .map_err(SimError::from);
                let ussd = self
                    .manager
                    .read_ussd(id, &self.registry, &ctx, g, c)
                    .map_err(SimError::from);
                let apn = self
                    .manager
                    .read_apn(id, &self.registry, &ctx, g, c)
                    .map_err(SimError::from);
                let mut s = lock(&self.state)?;
                let apn = if let Some(e) = s.modem_error() {
                    Err(e)
                } else {
                    apn
                };
                s.features = Some(Features {
                    dns_service: s.configuration.dns_service.clone(),
                    wifi_service: s.configuration.wifi_service.clone(),
                    dns: FeatureRead::from_result(Ok(dns)),
                    wifi: FeatureRead::from_result(wifi),
                    apn: FeatureRead::from_result(apn),
                    ussd: FeatureRead::from_result(ussd),
                    modem: s.configuration.modem.clone(),
                    observed_revision: s.revision,
                    observed_at: s.clock,
                });
            }
            Action::CancelDraft => {
                lock(&self.state)?.preview = None;
            }
            Action::PreviewDns { configuration } => {
                let previous = self.read_dns(&a, &d, c)?;
                let proposed = model::validate_dns(configuration)?;
                let mut s = lock(&self.state)?;
                let mut changes = vec![];
                for (field, before, after) in [
                    (
                        "Upstream IPv4",
                        &previous.upstream_ipv4,
                        &proposed.upstream_ipv4,
                    ),
                    (
                        "Upstream IPv6",
                        &previous.upstream_ipv6,
                        &proposed.upstream_ipv6,
                    ),
                    ("LAN IPv4", &previous.lan_ipv4, &proposed.lan_ipv4),
                    ("LAN IPv6", &previous.lan_ipv6, &proposed.lan_ipv6),
                ] {
                    if before != after {
                        changes.push(Difference {
                            field: field.into(),
                            before: before.clone(),
                            after: after.clone(),
                        });
                    }
                }
                for (field, before, after) in [
                    (
                        "Upstream IPv4 source",
                        &previous.upstream_source_ipv4,
                        &proposed.upstream_source_ipv4,
                    ),
                    (
                        "Upstream IPv6 source",
                        &previous.upstream_source_ipv6,
                        &proposed.upstream_source_ipv6,
                    ),
                    (
                        "LAN IPv4 source",
                        &previous.lan_source_ipv4,
                        &proposed.lan_source_ipv4,
                    ),
                    (
                        "LAN IPv6 source",
                        &previous.lan_source_ipv6,
                        &proposed.lan_source_ipv6,
                    ),
                ] {
                    if before != after {
                        changes.push(Difference {
                            field: field.into(),
                            before: vec![format!("{before:?}")],
                            after: vec![format!("{after:?}")],
                        });
                    }
                }
                s.preview = Some(DnsPreview {
                    id: s.id("preview"),
                    target: DEVICE_ID.into(),
                    epoch: d.epoch,
                    base_revision: s.revision,
                    previous,
                    proposed,
                    changes,
                    affected_services: vec!["virtual-dns-forwarder".into()],
                    snapshot_required: true,
                });
                s.outcome = None;
            }
            Action::ApplyDns { preview_id } => {
                self.read_dns(&a, &d, c)?;
                self.apply(&preview_id, &d, c)?;
            }
            Action::VerifyDns => {
                self.read_dns(&a, &d, c)?;
                self.verify()?;
            }
            Action::Restore { snapshot_id } => {
                self.read_dns(&a, &d, c)?;
                self.restore(&snapshot_id, &d, c)?;
            }
            Action::SetWifi { radio, limit } => {
                self.read_dns(&a, &d, c)?;
                let mut s = lock(&self.state)?;
                let wifi = s
                    .configuration
                    .wifi
                    .iter_mut()
                    .find(|w| w.radio == radio)
                    .ok_or_else(|| fail(ErrorCode::Unsupported, "Unknown virtual radio/BSS."))?;
                if limit < wifi.verified_minimum.unwrap_or(1)
                    || limit > wifi.verified_maximum.unwrap_or(0)
                    || limit < wifi.connected_clients.unwrap_or(0)
                {
                    return Err(fail(
                        ErrorCode::InvalidInput,
                        "Virtual limit exceeds hardware range or is below associated client count.",
                    ));
                }
                wifi.station_limit = Some(limit);
                s.revision += 1;
                s.features = None;
                if s.fault(FaultStage::Wifi) {
                    s.configuration.wifi_service = "failed".into();
                    s.available = false;
                    s.reconnect_at = Some(s.clock + 5000);
                    drop(s);
                    self.authority.epoch.fetch_add(1, Ordering::AcqRel);
                    self.manager
                        .observe_discovery(&discovery::DiscoverySnapshot {
                            revision: self.authority.epoch.load(Ordering::Acquire),
                            ..Default::default()
                        });
                    return Err(fail(ErrorCode::Offline,"Virtual Wi-Fi reload interrupted management. Advance 5000 ms, then approve a new handshake."));
                }
            }
            Action::SetApn {
                source,
                apn,
                authentication,
                ip_family,
            } => {
                self.read_dns(&a, &d, c)?;
                let mut s = lock(&self.state)?;
                if let Some(e) = s.modem_error() {
                    return Err(e);
                }
                if matches!(ip_family, IpFamily::Unknown)
                    || (!s.configuration.modem.ipv6_supported && ip_family != IpFamily::Ipv4)
                {
                    return Err(fail(
                        ErrorCode::Unsupported,
                        "Virtual modem does not support this IP family.",
                    ));
                }
                let value = match source {
                    ConfigurationSource::Automatic => {
                        match s.configuration.modem.operator.as_deref() {
                            Some("SIM-001") => "lab.example".into(),
                            _ => {
                                return Err(fail(
                                    ErrorCode::UnknownOperator,
                                    "No synthetic operator/APN mapping.",
                                ))
                            }
                        }
                    }
                    ConfigurationSource::Manual => apn,
                    ConfigurationSource::Unknown => {
                        return Err(fail(
                            ErrorCode::InvalidInput,
                            "Select automatic or manual APN.",
                        ))
                    }
                };
                if value.is_empty()
                    || value.len() > 100
                    || !value
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b".-".contains(&b))
                    || authentication == ApnAuthentication::Unknown
                {
                    return Err(fail(
                        ErrorCode::InvalidInput,
                        "Invalid virtual APN/authentication.",
                    ));
                }
                s.configuration.apn=ApnConfiguration {source,apn:Some(value),operator:s.configuration.modem.operator.clone(),authentication_required:authentication!=ApnAuthentication::None,
                    authentication_method:authentication,ip_family,observed_by:"SIMULATION: synthetic operator fixture or manual override; no SIM unlocking".into()};
                s.revision += 1;
                s.features = None;
            }
            action @ (Action::UssdStart { .. }
            | Action::UssdReply { .. }
            | Action::UssdCancel { .. }) => {
                self.read_dns(&a, &d, c)?;
                self.ussd(action)?;
            }
        }
        c.checkpoint()?;
        if d.epoch != self.authority.epoch.load(Ordering::Acquire) {
            return Err(fail(
                ErrorCode::Stale,
                "Virtual reset/disconnection invalidated this result.",
            ));
        }
        Ok(())
    }
    fn apply(&self, id: &str, d: &ContextData, c: &OperationControl) -> Result<()> {
        c.checkpoint()?;
        let mut s = lock(&self.state)?;
        if s.recovery_required {
            return Err(fail(
                ErrorCode::RecoveryRequired,
                "Virtual restore failed; reset the simulator before more changes.",
            ));
        }
        let p = s
            .preview
            .take()
            .filter(|p| p.id == id && p.epoch == d.epoch && p.base_revision == s.revision)
            .ok_or_else(|| {
                fail(
                    ErrorCode::Stale,
                    "DNS preview is absent, consumed or stale.",
                )
            })?;
        if s.snapshots.len() >= 16 {
            s.snapshots.remove(0);
        }
        let snapshot = Snapshot {
            info: SnapshotInfo {
                scenario: s.scenario,
                id: s.id("snapshot"),
                target: DEVICE_ID.into(),
                firmware_version: s.configuration.identity.firmware_version.clone(),
                revision: s.revision,
                virtual_time: s.clock,
            },
            configuration: s.configuration.clone(),
            epoch: d.epoch,
        };
        let snapshot_id = snapshot.info.id.clone();
        s.snapshots.push(snapshot);
        let apply_fault = s.fault(FaultStage::Apply);
        if apply_fault && s.scenario == Scenario::DnsApplyFailure {
            s.outcome=Some(DnsOutcome {operation:"apply".into(),accepted:true,applied:false,configuration_matches:false,
                service_healthy:s.configuration.dns_service=="running",resolution_successful:false,verified:false,rollback:Rollback::NotRequested,requires_reset:false,
                message:"SIMULATION: apply rejected; snapshot exists and previous configuration is intact.".into(),snapshot_id:Some(snapshot_id),revision:s.revision});
            return Ok(());
        }
        s.configuration.dns = p.proposed.clone();
        s.revision += 1;
        s.features = None;
        s.configuration.dns_service = if apply_fault { "failed" } else { "running" }.into();
        s.configuration.resolver_ok = !s.fault(FaultStage::Resolve);
        if s.fault(FaultStage::Verify) {
            s.configuration.dns.upstream_ipv4 = vec!["203.0.113.53".into()];
        }
        let matches = s.configuration.dns == p.proposed && s.revision == p.base_revision + 1;
        let healthy = s.configuration.dns_service == "running";
        let resolution = s.configuration.resolves();
        s.outcome=Some(DnsOutcome {operation:"apply".into(),accepted:true,applied:true,configuration_matches:matches,service_healthy:healthy,
            resolution_successful:resolution,verified:matches&&healthy&&resolution,rollback:Rollback::NotRequested,requires_reset:false,
            message:if matches&&healthy&&resolution {"SIMULATION: applied, service healthy and virtual resolver check succeeded."}else{"SIMULATION: change applied but verification failed; restore the captured snapshot or reset."}.into(),
            snapshot_id:Some(snapshot_id),revision:s.revision});
        // Retain expected values for independent explicit verification; preview is consumed.
        s.preview = Some(DnsPreview {
            id: format!("consumed-{}", p.id),
            ..p
        });
        s.event("Simulated DNS snapshot, apply and readback completed.");
        Ok(())
    }
    fn verify(&self) -> Result<()> {
        let mut s = lock(&self.state)?;
        let p = s
            .preview
            .as_ref()
            .filter(|p| p.id.starts_with("consumed-"))
            .ok_or_else(|| fail(ErrorCode::Stale, "No applied DNS change to verify."))?;
        let matches = s.configuration.dns == p.proposed && s.revision == p.base_revision + 1;
        let healthy = s.configuration.dns_service == "running";
        let resolution = s.configuration.resolves();
        let revision = s.revision;
        let o = s
            .outcome
            .as_mut()
            .ok_or_else(|| fail(ErrorCode::Stale, "No virtual apply result."))?;
        o.operation = "verify".into();
        o.configuration_matches = matches;
        o.service_healthy = healthy;
        o.resolution_successful = resolution;
        o.verified = matches && healthy && resolution;
        o.revision = revision;
        o.message = if o.verified {
            "SIMULATION: independent DNS readback verified."
        } else {
            "SIMULATION: DNS readback/service/resolution verification failed."
        }
        .into();
        Ok(())
    }
    fn restore(&self, id: &str, d: &ContextData, c: &OperationControl) -> Result<()> {
        c.checkpoint()?;
        let mut s = lock(&self.state)?;
        let snapshot = s
            .snapshots
            .iter()
            .find(|v| {
                v.info.id == id
                    && v.epoch == d.epoch
                    && v.info.target == DEVICE_ID
                    && v.info.firmware_version == "SIM-0.1"
            })
            .cloned()
            .ok_or_else(|| {
                fail(
                    ErrorCode::Stale,
                    "Snapshot belongs to another virtual device/firmware/generation.",
                )
            })?;
        if snapshot.configuration.identity != s.configuration.identity {
            return Err(fail(
                ErrorCode::IdentityConflict,
                "Virtual snapshot identity differs from current device identity.",
            ));
        }
        if s.fault(FaultStage::Restore) {
            s.recovery_required = true;
            s.outcome = Some(DnsOutcome {
                operation: "restore".into(),
                accepted: true,
                applied: false,
                configuration_matches: false,
                service_healthy: s.configuration.dns_service == "running",
                resolution_successful: s.configuration.resolves(),
                verified: false,
                rollback: Rollback::Failed,
                requires_reset: true,
                message: "SIMULATION: restore failed; manual simulator reset required.".into(),
                snapshot_id: Some(id.into()),
                revision: s.revision,
            });
            return Ok(());
        }
        s.configuration = snapshot.configuration;
        s.revision += 1;
        s.preview = None;
        s.features = None;
        s.recovery_required = false;
        s.outcome = Some(DnsOutcome {
            operation: "restore".into(),
            accepted: true,
            applied: true,
            configuration_matches: true,
            service_healthy: s.configuration.dns_service == "running",
            resolution_successful: s.configuration.resolves(),
            verified: s.configuration.resolves(),
            rollback: Rollback::Succeeded,
            requires_reset: false,
            message:
                "SIMULATION: previous configuration snapshot restored; firmware was not touched."
                    .into(),
            snapshot_id: Some(id.into()),
            revision: s.revision,
        });
        s.event("Simulation configuration snapshot restored.");
        Ok(())
    }
    fn ussd(&self, action: Action) -> Result<()> {
        let mut s = lock(&self.state)?;
        if let Some(e) = s.modem_error() {
            return Err(e);
        }
        if s.scenario == Scenario::UnsupportedModem {
            return Err(fail(
                ErrorCode::Unsupported,
                "Virtual modem does not support USSD.",
            ));
        }
        match action {
            Action::UssdStart { code } => {
                if s.ussd
                    .as_ref()
                    .is_some_and(|u| u.state == "active" && u.expires_at > s.clock)
                {
                    return Err(fail(ErrorCode::Busy, "A virtual USSD session is active."));
                }
                if code != "*123#" && code != "*100#" {
                    return Err(fail(
                        ErrorCode::InvalidInput,
                        "Only scripted *123# and *100# are supported; no raw commands.",
                    ));
                }
                let now = s.clock;
                s.ussd_requests.retain(|t| t.saturating_add(60_000) > now);
                if s.ussd_requests.len() >= 5 {
                    return Err(fail(ErrorCode::Busy, "Virtual USSD rate limit reached."));
                }
                s.ussd_requests.push(now);
                let fault = s.fault(FaultStage::Ussd);
                let (state, text) = if fault && s.scenario == Scenario::UssdNetworkFailure {
                    ("failed", "SIMULATION: virtual mobile network failure.")
                } else if code == "*100#" {
                    ("completed", "SIMULATION: fictional balance 42 LAB credits.")
                } else {
                    (
                        "active",
                        "SIMULATION MENU: 1. Balance  2. Bundles  3. Cancel",
                    )
                };
                s.ussd = Some(UssdSession {
                    id: s.id("ussd"),
                    state: state.into(),
                    text: text.into(),
                    expires_at: s.clock + if fault { 1000 } else { 30_000 },
                });
            }
            Action::UssdReply { session_id, text } => {
                let now = s.clock;
                let u = s
                    .ussd
                    .as_mut()
                    .filter(|u| u.id == session_id && u.state == "active" && u.expires_at > now)
                    .ok_or_else(|| {
                        fail(
                            ErrorCode::Expired,
                            "Virtual USSD session expired or closed.",
                        )
                    })?;
                u.text = match text.as_str() {
                    "1" => "SIMULATION: fictional balance 42 LAB credits.",
                    "2" => "SIMULATION: fictional 5 GB LAB bundle.",
                    "3" => "SIMULATION: session cancelled.",
                    _ => {
                        return Err(fail(
                            ErrorCode::InvalidInput,
                            "Select scripted menu option 1, 2 or 3.",
                        ))
                    }
                }
                .into();
                u.state = if text == "3" {
                    "cancelled"
                } else {
                    "completed"
                }
                .into();
            }
            Action::UssdCancel { session_id } => {
                let u = s
                    .ussd
                    .as_mut()
                    .filter(|u| u.id == session_id && u.state == "active")
                    .ok_or_else(|| fail(ErrorCode::Expired, "Virtual USSD session is closed."))?;
                u.state = "cancelled".into();
                u.text = "SIMULATION: session cancelled.".into();
            }
            _ => {
                return Err(fail(
                    ErrorCode::InvalidInput,
                    "Unsupported scripted USSD action.",
                ))
            }
        };
        Ok(())
    }
}
impl Drop for Runtime {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
#[cfg(test)]
#[path = "tests.rs"]
mod tests;

pub struct SimulatorService {
    available: bool,
    owner: String,
    runtime: Mutex<Option<Arc<Runtime>>>,
    sequence: AtomicU64,
}
impl SimulatorService {
    pub(crate) fn new(isolated_enabled: bool, owner: String) -> Self {
        Self {
            available: isolated_enabled && (cfg!(feature = "native-test-profile") || cfg!(test)),
            owner,
            runtime: Mutex::new(None),
            sequence: AtomicU64::new(0),
        }
    }
    fn enabled(&self) -> Result<()> {
        if self.available {
            Ok(())
        } else {
            Err(fail(
                ErrorCode::Disabled,
                "Simulator requires the dedicated isolated --simulator entry point.",
            ))
        }
    }
    fn runtime(&self) -> Result<Arc<Runtime>> {
        self.enabled()?;
        lock(&self.runtime)?
            .as_ref()
            .cloned()
            .ok_or_else(|| fail(ErrorCode::Disabled, "Start simulation explicitly."))
    }
    pub fn status(&self) -> Result<Status> {
        self.enabled()?;
        if let Some(r) = lock(&self.runtime)?.as_ref() {
            return r.status();
        }
        Ok(Status {
            active_connections: 0,
            available: true,
            active: false,
            device: None,
            epoch: 0,
            revision: 0,
            virtual_time: 0,
            scenario: Scenario::Healthy,
            scenarios: SCENARIOS.to_vec(),
            faults: vec![],
            connection: ConnectionState::Discovered,
            authentication: AuthenticationState::NotAttempted,
            approval_id: None,
            read_authorized: false,
            features: None,
            preview: None,
            outcome: None,
            snapshots: vec![],
            ussd: None,
            events: vec![],
            busy: false,
        })
    }
    pub fn control(&self, c: Control) -> Result<Status> {
        self.enabled()?;
        match c {
            Control::Start => {
                let mut runtime = lock(&self.runtime)?;
                if runtime.is_some() {
                    return Err(fail(ErrorCode::Busy, "Simulation is already active."));
                }
                let owner = format!(
                    "{}-{}",
                    self.owner,
                    self.sequence.fetch_add(1, Ordering::AcqRel)
                );
                *runtime = Some(Arc::new(Runtime::new(&owner)?));
            }
            Control::Stop => {
                if let Some(r) = lock(&self.runtime)?.take() {
                    r.stop()?;
                }
            }
            c => self.runtime()?.control(c)?,
        };
        self.status()
    }
    pub fn request(&self, r: Request) -> Result<Status> {
        self.runtime()?.perform(r)
    }
    pub fn shutdown(&self) {
        if let Ok(mut runtime) = self.runtime.lock() {
            if let Some(r) = runtime.take() {
                let _ = r.stop();
            }
        }
    }
}
