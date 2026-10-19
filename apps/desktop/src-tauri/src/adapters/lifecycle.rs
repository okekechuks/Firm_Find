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
        };
        let identity = managed
            .connection
            .read_identity(&control.stage(Stage::Identity))?;
        validate_identity(ctx, &binding, &identity)?;
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
            state: ConnectionState::ConnectedReadOnly,
            simulated: binding.simulated,
            expires_at: binding.expires_at,
            authentication: AuthenticationState::Authenticated,
        })
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
        action: impl FnOnce(&mut dyn AdapterConnection, &OperationControl) -> Result<T, AdapterError>,
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
        authorize(registry, ctx, &managed.binding, op, grant)?;
        let value = action(managed.connection.as_mut(), control)?;
        control.checkpoint()?;
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
            |s, c| {
                let identity = s.read_identity(c)?;
                let expected = registry.evaluate(ctx);
                if identity.simulated
                    != expected
                        .matches
                        .iter()
                        .find(|m| Some(&m.adapter.id) == expected.selected_adapter.as_ref())
                        .is_some_and(|m| {
                            m.adapter.implementation_status == AdapterImplementationStatus::TestOnly
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
            |s, c| {
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
            |s, c| {
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
            |s, c| {
                s.apn()
                    .ok_or_else(|| unsupported(AdapterOperation::ReadApn))?
                    .read_apn(c)
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
            |s, c| {
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
            |s, c| {
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
    if !p.manufacturer.eq_ignore_ascii_case(&identity.manufacturer)
        || !p
            .firmware_families
            .iter()
            .any(|s| s.eq_ignore_ascii_case(&identity.firmware_family))
        || !(p.model.eq_ignore_ascii_case(&identity.model)
            || p.model_aliases
                .iter()
                .any(|s| s.eq_ignore_ascii_case(&identity.model)))
        || !p
            .hardware_revisions
            .iter()
            .any(|s| s.eq_ignore_ascii_case(&identity.hardware_revision))
        || crate::profiles::version_allowed(
            &identity.firmware_version,
            &p.firmware_version_constraints,
        ) != Some(true)
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
