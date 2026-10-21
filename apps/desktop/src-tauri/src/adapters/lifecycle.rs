use super::registry::failure;
use super::*;
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

/// Secret material is non-serializable, non-cloneable and always redacted in Debug.
pub struct Secret(Vec<u8>);
impl Secret {
    pub fn new(value: Vec<u8>) -> Result<Self, AdapterError> {
        let secret = Self(value);
        if secret.0.is_empty() || secret.0.len() > 4096 {
            return Err(failure(
                AdapterErrorCode::AuthenticationFailed,
                "Credential material is missing or exceeds its bound.",
            ));
        }
        Ok(secret)
    }
    pub fn expose(&self) -> &[u8] {
        &self.0
    }
}
impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Secret([REDACTED])")
    }
}
impl Drop for Secret {
    fn drop(&mut self) {
        for b in &mut self.0 {
            // Volatile writes keep the clearing operation observable to the compiler.
            unsafe {
                std::ptr::write_volatile(b, 0);
            }
        }
        std::sync::atomic::compiler_fence(Ordering::SeqCst);
    }
}
#[derive(Clone, Default)]
pub struct Cancellation(Arc<AtomicBool>);
impl Cancellation {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
    pub(crate) fn same_instance(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
#[derive(Clone, Copy, Debug)]
pub enum Stage {
    Connection,
    Authentication,
    Identity,
    Capabilities,
    Disconnection,
    Read,
}
#[derive(Clone)]
pub struct OperationControl {
    token: Cancellation,
    deadline: Instant,
    operation: AdapterOperation,
    stage: Stage,
}
impl OperationControl {
    pub fn new(
        token: Cancellation,
        stage: Stage,
        operation: AdapterOperation,
        budget: Duration,
    ) -> Result<Self, AdapterError> {
        if budget.is_zero() || budget > Duration::from_secs(60) {
            return Err(failure(
                AdapterErrorCode::OperationTimeout,
                "Operation budget must be between 1 ns and 60 seconds.",
            ));
        }
        Ok(Self {
            token,
            deadline: Instant::now() + budget,
            operation,
            stage,
        })
    }
    pub fn stage(&self, stage: Stage) -> Self {
        let limit = match stage {
            Stage::Connection => 10,
            Stage::Authentication => 8,
            Stage::Identity => 4,
            Stage::Capabilities => 6,
            Stage::Disconnection => 2,
            Stage::Read => 5,
        };
        Self {
            token: self.token.clone(),
            deadline: self
                .deadline
                .min(Instant::now() + Duration::from_secs(limit)),
            operation: self.operation,
            stage,
        }
    }
    pub fn checkpoint(&self) -> Result<(), AdapterError> {
        let code = if self.token.is_cancelled() {
            Some(AdapterErrorCode::OperationCancelled)
        } else if Instant::now() >= self.deadline {
            Some(if matches!(self.stage, Stage::Connection) {
                AdapterErrorCode::ConnectionTimeout
            } else {
                AdapterErrorCode::OperationTimeout
            })
        } else {
            None
        };
        if let Some(code) = code {
            Err(AdapterError::new(
                code,
                "Adapter operation was cancelled or exceeded its deadline.",
                Some(self.operation),
                true,
                "Retry with fresh identity evidence and authorization.",
            ))
        } else {
            Ok(())
        }
    }
    pub fn cancellation(&self) -> Cancellation {
        self.token.clone()
    }
}
struct Slot(Arc<AtomicUsize>);
impl Drop for Slot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}
struct ManagedSession {
    connection: Box<dyn AdapterConnection>,
    binding: SessionBinding,
    _slot: Slot,
    authentication: AuthenticationState,
    bootstrap: Option<super::bootstrap::BootstrapEvidence>,
    identity: Option<AdapterIdentity>,
}
impl Drop for ManagedSession {
    fn drop(&mut self) {
        let c = OperationControl::new(
            Cancellation::default(),
            Stage::Disconnection,
            AdapterOperation::ReadConfiguration,
            Duration::from_secs(2),
        )
        .expect("bounded cleanup");
        let _ = self.connection.close(&c);
    }
}
#[derive(Default)]
pub struct SessionManager {
    sessions: Mutex<BTreeMap<u64, ManagedSession>>,
    in_use: Arc<AtomicUsize>,
    next: AtomicU64,
    stopped: AtomicBool,
    revision: AtomicU64,
}
impl SessionManager {
    /// Reconnect an already verified target. Previously unknown targets use `connect_initial`.
    #[allow(clippy::too_many_arguments)] // Explicit independent trust inputs; no IPC entry point.
    pub fn connect(
        &self,
        registry: &Registry,
        ctx: &DeviceContext<'_>,
        binding: SessionBinding,
        grant: &AuthorizationGrant,
        secret: Secret,
        control: &OperationControl,
    ) -> Result<SessionStatus, AdapterError> {
        if self.stopped.load(Ordering::Acquire) {
            return Err(failure(
                AdapterErrorCode::OperationCancelled,
                "Adapter service is shut down.",
            ));
        }
        self.reap_expired(ctx.now);
        self.check_revision(ctx.revision)?;
        control.checkpoint()?;
        authorize(
            registry,
            ctx,
            &binding,
            AdapterOperation::AuthorizedIdentityProbe,
            grant,
        )?;
        self.connect_checked(registry, ctx, binding, secret, control, false)
    }
    /// Only for a backend flow holding an explicit, single-use handshake approval.
    /// This does not grant configuration or capability reads and has no IPC entry point.
    #[allow(clippy::too_many_arguments)]
    pub fn connect_initial(
        &self,
        registry: &Registry,
        ctx: &DeviceContext<'_>,
        binding: SessionBinding,
        approval: &HandshakeApproval,
        secret: Secret,
        control: &OperationControl,
    ) -> Result<SessionStatus, AdapterError> {
        self.reap_expired(ctx.now);
        self.check_revision(ctx.revision)?;
        control.checkpoint()?;
        let deadline = approval.claim(registry, ctx, &binding)?;
        let bounded = OperationControl {
            deadline: control.deadline.min(deadline),
            ..control.clone()
        };
        self.connect_checked(registry, ctx, binding, secret, &bounded, true)
    }
    #[allow(clippy::too_many_arguments)]
    fn connect_checked(
        &self,
        registry: &Registry,
        ctx: &DeviceContext<'_>,
        mut binding: SessionBinding,
        secret: Secret,
        control: &OperationControl,
        initial: bool,
    ) -> Result<SessionStatus, AdapterError> {
        if self.stopped.load(Ordering::Acquire) {
            return Err(failure(
                AdapterErrorCode::OperationCancelled,
                "Adapter service is shut down.",
            ));
        }
        let previous = self
            .in_use
            .try_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < 2).then_some(n + 1)
            });
        if previous.is_err() {
            return Err(failure(
                AdapterErrorCode::TransportError,
                "Two adapter sessions or connection attempts are already active.",
            ));
        }
        let slot = Slot(Arc::clone(&self.in_use));
        let adapter = registry.adapter(&binding.adapter_id)?;
        let connector = adapter.connector().ok_or_else(|| {
            failure(
                AdapterErrorCode::OperationNotImplemented,
                "Read-only connector is unavailable.",
            )
        })?;
        let mut authentication = AuthenticationLifecycle::default();
        authentication.begin()?;
        let connection = connector
            .connect(&binding, &secret, &control.stage(Stage::Connection))
            .inspect_err(|_| {
                authentication.reject();
            })?;
        authentication.accept()?;
        drop(secret);
        let mut managed = ManagedSession {
            connection,
            binding: binding.clone(),
            _slot: slot,
            authentication: authentication.state(),
            bootstrap: None,
            identity: None,
        };
        if initial
            && managed.connection.endpoint_trust()
                != Some(EndpointTrust {
                    endpoint: binding.endpoint.clone(),
                    interface_id: binding.interface_id.clone(),
                    endpoint_identity: binding.endpoint_identity.clone(),
                    simulated: binding.simulated,
                })
        {
            return Err(failure(
                AdapterErrorCode::EndpointUntrusted,
                "Endpoint trust must be verified before requesting identity.",
            ));
        }
        let identity = managed
            .connection
            .read_identity(&control.stage(Stage::Identity))?;
        if initial {
            let verified = super::bootstrap::verify_handshake(
                ctx,
                &binding,
                managed.connection.as_ref(),
                &identity,
            )?;
            let verified_ctx = DeviceContext {
                profile: &verified.report,
                ..*ctx
            };
            if super::bootstrap::identity_only_binding(&binding) {
                if verified.report.confidence == crate::profiles::MatchConfidence::Verified {
                    binding.profile_id =
                        verified.report.selected_profile.clone().unwrap_or_default();
                    binding.profile_version =
                        verified.report.profile_version.clone().unwrap_or_default();
                    binding.profile_hash = verified
                        .report
                        .profile_content_hash
                        .clone()
                        .unwrap_or_default();
                    validate_identity(&verified_ctx, &binding, &identity)?;
                }
            } else {
                validate_identity(&verified_ctx, &binding, &identity)?;
            }
            binding.evidence_digest = verified.report.input_digest.clone();
            binding.expires_at = binding.expires_at.min(verified.report.expires_at);
            managed.binding = binding.clone();
            managed.bootstrap = Some(verified);
        } else {
            validate_identity(ctx, &binding, &identity)?;
        }
        managed.identity = Some(identity);
        // Cancellation after handshake still destroys the new resource instead of publishing it.
        control.checkpoint()?;
        let id = self.next.fetch_add(1, Ordering::AcqRel) + 1;
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| AdapterError::internal("session lock"))?;
        self.check_revision(ctx.revision)?;
        if self.stopped.load(Ordering::Acquire) {
            return Err(failure(
                AdapterErrorCode::OperationCancelled,
                "Adapter service stopped during connection.",
            ));
        }
        sessions.insert(id, managed);
        Ok(SessionStatus {
            id: id.to_string(),
            state: if binding.profile_id.is_empty() {
                ConnectionState::AuthorizationRequired
            } else {
                ConnectionState::ConnectedReadOnly
            },
            simulated: binding.simulated,
            expires_at: binding.expires_at,
            authentication: AuthenticationState::Authenticated,
        })
    }
    /// Return only the already authenticated identity and evaluation; performs no remote read.
    pub fn authenticated_snapshot(
        &self,
        id: u64,
    ) -> Result<(AdapterIdentity, crate::profiles::CompatibilityReport), AdapterError> {
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| AdapterError::internal("session lock"))?;
        let session = sessions
            .get(&id)
            .ok_or_else(|| failure(AdapterErrorCode::SessionExpired, "Session is absent."))?;
        let evidence = session.bootstrap.as_ref().ok_or_else(|| {
            failure(
                AdapterErrorCode::IdentityUnverified,
                "Bootstrap evidence is absent.",
            )
        })?;
        Ok((
            session
                .identity
                .clone()
                .ok_or_else(|| AdapterError::internal("identity"))?,
            evidence.report.clone(),
        ))
    }
    pub fn inspect_openwrt(
        &self,
        id: u64,
        registry: &Registry,
        ctx: &DeviceContext<'_>,
        grant: &AuthorizationGrant,
        control: &OperationControl,
    ) -> Result<crate::openwrt::adapter::Inspection, AdapterError> {
        self.run(
            id,
            registry,
            ctx,
            grant,
            AdapterOperation::ReadConfiguration,
            control,
            |connection, control, _| connection.inspect_openwrt(control),
        )
    }
    /// Separate explicit approval for established reads. A handshake receipt cannot be used here.
    pub fn confirm_session_reads(
        &self,
        id: u64,
        registry: &Registry,
        ctx: &DeviceContext<'_>,
        operations: Vec<AdapterOperation>,
    ) -> Result<AuthorizationGrant, AdapterError> {
        self.reap_expired(ctx.now);
        self.check_revision(ctx.revision)?;
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| AdapterError::internal("session lock"))?;
        let session = sessions.get(&id).ok_or_else(|| {
            failure(
                AdapterErrorCode::SessionExpired,
                "Session is absent or expired.",
            )
        })?;
        let effective = match &session.bootstrap {
            Some(e) => e.context(ctx)?,
            None => DeviceContext { ..*ctx },
        };
        let grant = AuthorizationGrant::confirm_selected_target(
            session.binding.clone(),
            operations.clone(),
            ctx.now,
        )?;
        for op in operations {
            authorize(registry, &effective, &session.binding, op, &grant)?;
        }
        Ok(grant)
    }
    #[allow(clippy::too_many_arguments)] // Shared session gate keeps every read under the same checks.
    fn run<T>(
        &self,
        id: u64,
        registry: &Registry,
        ctx: &DeviceContext<'_>,
        grant: &AuthorizationGrant,
        op: AdapterOperation,
        control: &OperationControl,
        action: impl FnOnce(
            &mut dyn AdapterConnection,
            &OperationControl,
            &DeviceContext<'_>,
        ) -> Result<T, AdapterError>,
    ) -> Result<T, AdapterError> {
        let mut managed = self
            .sessions
            .lock()
            .map_err(|_| AdapterError::internal("session lock"))?
            .remove(&id)
            .ok_or_else(|| {
                failure(
                    AdapterErrorCode::SessionExpired,
                    "Session is closed, busy or expired.",
                )
            })?;
        control.checkpoint()?;
        self.check_revision(ctx.revision)?;
        let effective = match &managed.bootstrap {
            Some(e) => e.context(ctx)?,
            None => DeviceContext { ..*ctx },
        };
        authorize(registry, &effective, &managed.binding, op, grant)?;
        let started = Instant::now();
        let expiry = managed
            .binding
            .expires_at
            .min(grant.expires_at())
            .min(effective.profile.expires_at);
        let bounded = bounded_read(control, expiry, ctx.now);
        bounded.checkpoint()?;
        let value = action(managed.connection.as_mut(), &bounded, &effective)?;
        bounded.checkpoint()?;
        let fresh = DeviceContext {
            now: ctx
                .now
                .saturating_add(started.elapsed().as_millis().try_into().unwrap_or(u64::MAX)),
            ..effective
        };
        authorize(registry, &fresh, &managed.binding, op, grant)?;
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| AdapterError::internal("session lock"))?;
        self.check_revision(ctx.revision)?;
        if self.stopped.load(Ordering::Acquire) {
            return Err(failure(
                AdapterErrorCode::OperationCancelled,
                "Adapter service stopped during read.",
            ));
        }
        sessions.insert(id, managed);
        Ok(value)
    }
    pub fn read_identity(
        &self,
        id: u64,
        registry: &Registry,
        ctx: &DeviceContext<'_>,
        grant: &AuthorizationGrant,
        control: &OperationControl,
    ) -> Result<AdapterIdentity, AdapterError> {
        self.run(
            id,
            registry,
            ctx,
            grant,
            AdapterOperation::AuthorizedIdentityProbe,
            &control.stage(Stage::Identity),
            |s, c, ctx| {
                let identity = s.read_identity(c)?;
                let expected = registry.evaluate(ctx);
                if identity.simulated
                    != expected
                        .matches
                        .iter()
                        .find(|m| Some(&m.adapter.id) == expected.selected_adapter.as_ref())
                        .is_some_and(|m| {
                            matches!(
                                m.adapter.implementation_status,
                                AdapterImplementationStatus::TestOnly
                                    | AdapterImplementationStatus::Simulator
                            )
                        })
                {
                    return Err(failure(
                        AdapterErrorCode::IdentityConflict,
                        "Session provenance changed.",
                    ));
                }
                validate_identity_fields(ctx, &identity)?;
                Ok(identity)
            },
        )
    }
    pub fn probe(
        &self,
        id: u64,
        registry: &Registry,
        ctx: &DeviceContext<'_>,
        grant: &AuthorizationGrant,
        control: &OperationControl,
    ) -> Result<Vec<AdapterCapability>, AdapterError> {
        self.run(
            id,
            registry,
            ctx,
            grant,
            AdapterOperation::InspectCapabilities,
            &control.stage(Stage::Capabilities),
            |s, c, _| {
                let result = s.probe_capabilities(c)?;
                if result.iter().any(|v| {
                    v.can_apply || v.can_plan || v.can_restore || v.operation.is_physical_write()
                }) {
                    return Err(failure(
                        AdapterErrorCode::OperationNotAuthorized,
                        "Adapter probe attempted to grant prohibited operations.",
                    ));
                }
                Ok(result)
            },
        )
    }
    pub fn read_dns(
        &self,
        id: u64,
        registry: &Registry,
        ctx: &DeviceContext<'_>,
        grant: &AuthorizationGrant,
        control: &OperationControl,
    ) -> Result<DnsConfiguration, AdapterError> {
        self.run(
            id,
            registry,
            ctx,
            grant,
            AdapterOperation::ReadDns,
            &control.stage(Stage::Read),
            |s, c, _| {
                s.dns()
                    .ok_or_else(|| unsupported(AdapterOperation::ReadDns))?
                    .read_dns(c)
            },
        )
    }
    pub fn read_apn(
        &self,
        id: u64,
        registry: &Registry,
        ctx: &DeviceContext<'_>,
        grant: &AuthorizationGrant,
        control: &OperationControl,
    ) -> Result<ApnConfiguration, AdapterError> {
        self.run(
            id,
            registry,
            ctx,
            grant,
            AdapterOperation::ReadApn,
            &control.stage(Stage::Read),
            |s, c, _| {
                s.apn()
                    .ok_or_else(|| unsupported(AdapterOperation::ReadApn))?
                    .read_apn(c)
            },
        )
    }
    pub fn read_wifi_all(
        &self,
        id: u64,
        registry: &Registry,
        ctx: &DeviceContext<'_>,
        grant: &AuthorizationGrant,
        control: &OperationControl,
    ) -> Result<Vec<WifiClientLimitConfiguration>, AdapterError> {
        self.run(
            id,
            registry,
            ctx,
            grant,
            AdapterOperation::ReadWifiLimit,
            &control.stage(Stage::Read),
            |s, c, _| {
                s.wifi()
                    .ok_or_else(|| unsupported(AdapterOperation::ReadWifiLimit))?
                    .read_wifi_configurations(c)
            },
        )
    }
    pub fn read_wifi(
        &self,
        id: u64,
        registry: &Registry,
        ctx: &DeviceContext<'_>,
        grant: &AuthorizationGrant,
        control: &OperationControl,
    ) -> Result<WifiClientLimitConfiguration, AdapterError> {
        self.run(
            id,
            registry,
            ctx,
            grant,
            AdapterOperation::ReadWifiLimit,
            &control.stage(Stage::Read),
            |s, c, _| {
                s.wifi()
                    .ok_or_else(|| unsupported(AdapterOperation::ReadWifiLimit))?
                    .read_wifi_client_limit(c)
            },
        )
    }
    pub fn read_ussd(
        &self,
        id: u64,
        registry: &Registry,
        ctx: &DeviceContext<'_>,
        grant: &AuthorizationGrant,
        control: &OperationControl,
    ) -> Result<UssdCapability, AdapterError> {
        self.run(
            id,
            registry,
            ctx,
            grant,
            AdapterOperation::ReadUssdCapability,
            &control.stage(Stage::Read),
            |s, c, _| {
                s.ussd()
                    .ok_or_else(|| unsupported(AdapterOperation::ReadUssdCapability))?
                    .read_ussd_capability(c)
            },
        )
    }
    pub fn disconnect(&self, id: u64) -> Result<(), AdapterError> {
        let managed = self
            .sessions
            .lock()
            .map_err(|_| AdapterError::internal("session lock"))?
            .remove(&id);
        drop(managed);
        Ok(())
    }
    pub fn active_count(&self) -> usize {
        self.in_use.load(Ordering::Acquire)
    }
    pub fn shutdown(&self) {
        self.stopped.store(true, Ordering::Release);
        if let Ok(mut s) = self.sessions.lock() {
            s.clear();
        }
    }
    pub fn reap_expired(&self, now: u64) {
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.retain(|_, session| session.binding.expires_at > now);
        }
    }
    pub fn status(&self, id: u64, now: u64) -> Result<SessionStatus, AdapterError> {
        self.reap_expired(now);
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| AdapterError::internal("session lock"))?;
        let session = sessions.get(&id).ok_or_else(|| {
            failure(
                AdapterErrorCode::SessionExpired,
                "Session is closed or expired.",
            )
        })?;
        Ok(SessionStatus {
            id: id.to_string(),
            state: ConnectionState::ConnectedReadOnly,
            simulated: session.binding.simulated,
            expires_at: session.binding.expires_at,
            authentication: session.authentication,
        })
    }
    pub fn observe_discovery(&self, snapshot: &crate::discovery::DiscoverySnapshot) {
        self.revision.store(snapshot.revision, Ordering::Release);
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.retain(|_, s| {
                s.binding.discovery_revision == snapshot.revision
                    && snapshot
                        .devices
                        .iter()
                        .any(|d| d.id == s.binding.device_id && d.connected)
            });
        }
    }
    fn check_revision(&self, revision: u64) -> Result<(), AdapterError> {
        let current = self.revision.load(Ordering::Acquire);
        if current != 0 && current != revision {
            Err(failure(
                AdapterErrorCode::OperationCancelled,
                "Discovery changed during adapter operation.",
            ))
        } else {
            Ok(())
        }
    }
}
fn bounded_read(control: &OperationControl, expiry: u64, now: u64) -> OperationControl {
    OperationControl {
        deadline: control
            .deadline
            .min(Instant::now() + Duration::from_millis(expiry.saturating_sub(now))),
        ..control.clone()
    }
}
fn validate_identity(
    ctx: &DeviceContext<'_>,
    binding: &SessionBinding,
    identity: &AdapterIdentity,
) -> Result<(), AdapterError> {
    if identity.simulated != binding.simulated {
        return Err(failure(
            AdapterErrorCode::IdentityConflict,
            "Session and identity provenance differ.",
        ));
    }
    validate_identity_fields(ctx, identity)
}
fn validate_identity_fields(
    ctx: &DeviceContext<'_>,
    identity: &AdapterIdentity,
) -> Result<(), AdapterError> {
    let profile = ctx
        .profile
        .selected_profile
        .as_deref()
        .and_then(|id| {
            ctx.profiles
                .by_id(id, ctx.profile.profile_version.as_deref())
        })
        .ok_or_else(|| {
            failure(
                AdapterErrorCode::IdentityUnverified,
                "Exact profile is unavailable.",
            )
        })?;
    let p = &profile.profile;
    if (!p.inspection_only && !p.manufacturer.eq_ignore_ascii_case(&identity.manufacturer))
        || !p
            .firmware_families
            .iter()
            .any(|s| s.eq_ignore_ascii_case(&identity.firmware_family))
        || !(p.model.eq_ignore_ascii_case(&identity.model)
            || p.model_aliases
                .iter()
                .any(|s| s.eq_ignore_ascii_case(&identity.model)))
        || (!p.inspection_only
            && !p
                .hardware_revisions
                .iter()
                .any(|s| s.eq_ignore_ascii_case(&identity.hardware_revision)))
        || crate::profiles::version_allowed(
            &identity.firmware_version,
            &p.firmware_version_constraints,
        ) != Some(true)
        || p.match_rules.iter().any(|rule| {
            rule.purpose == crate::profiles::Purpose::Identity
                && rule.attribute == crate::profiles::Attribute::ManagementIdentity
                && !rule
                    .values
                    .iter()
                    .any(|v| v.eq_ignore_ascii_case(&identity.management_identity))
        })
    {
        return Err(failure(
            AdapterErrorCode::IdentityConflict,
            "Read-only handshake identity differs from the authorized hardware or firmware.",
        ));
    }
    Ok(())
}
pub fn unsupported(operation: AdapterOperation) -> AdapterError {
    AdapterError::new(
        AdapterErrorCode::OperationUnsupported,
        "This adapter does not implement the requested operation.",
        Some(operation),
        false,
        "Review adapter capabilities.",
    )
}
#[cfg(test)]
mod read_deadline_tests {
    use super::*;
    #[test]
    fn established_reads_use_earliest_authority_expiry_and_never_extend_caller_budget() {
        let control = OperationControl::new(
            Cancellation::default(),
            Stage::Read,
            AdapterOperation::ReadConfiguration,
            Duration::from_secs(60),
        )
        .unwrap();
        let start = Instant::now();
        let deadline = bounded_read(&control, 1200, 1000).deadline;
        assert!(deadline <= start + Duration::from_millis(201));
        let expired = bounded_read(&control, 1000, 1000);
        assert!(expired.checkpoint().is_err());
        let shorter = OperationControl::new(
            Cancellation::default(),
            Stage::Read,
            AdapterOperation::ReadConfiguration,
            Duration::from_millis(1),
        )
        .unwrap();
        assert_eq!(
            bounded_read(&shorter, 10_000, 1000).deadline,
            shorter.deadline
        );
    }
}
