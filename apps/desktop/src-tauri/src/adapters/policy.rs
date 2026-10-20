use super::registry::failure;
use super::*;
use crate::profiles::{EvidenceOrigin, Reliability, Verification};

/// Created only by a backend authorization flow. No serde, no frontend-controlled boolean.
pub struct AuthorizationGrant {
    binding: SessionBinding,
    operations: Vec<AdapterOperation>,
    expires_at: u64,
}
impl AuthorizationGrant {
    pub fn confirm_selected_target(
        binding: SessionBinding,
        operations: Vec<AdapterOperation>,
        now: u64,
    ) -> Result<Self, AdapterError> {
        if binding.expires_at <= now
            || operations.is_empty()
            || operations.len() > 16
            || operations.iter().any(|op| op.is_physical_write())
        {
            return Err(failure(
                AdapterErrorCode::OperationNotAuthorized,
                "Authorization must cover fresh read-only operations on one explicit target.",
            ));
        }
        Ok(Self {
            expires_at: binding.expires_at.min(now.saturating_add(60_000)),
            binding,
            operations,
        })
    }
}
/// Established-session policy. Never weaken this gate to bootstrap authentication;
/// `HandshakeApproval` authorizes only the separate initial identity handshake.
pub fn authorize(
    registry: &Registry,
    ctx: &DeviceContext<'_>,
    binding: &SessionBinding,
    op: AdapterOperation,
    grant: &AuthorizationGrant,
) -> Result<(), AdapterError> {
    if op.is_physical_write() || matches!(op, AdapterOperation::FirmwareBuild) {
        return Err(failure(AdapterErrorCode::OperationNotAuthorized,"Physical changes, extraction, USSD requests and firmware execution are disabled in Phase 5."));
    }
    if protected(ctx.device) {
        return Err(failure(
            AdapterErrorCode::ProtectedDevice,
            "Protected host hardware cannot be managed as a router.",
        ));
    }
    if !ctx.device.connected {
        return Err(failure(
            AdapterErrorCode::DeviceDisconnected,
            "Device is disconnected.",
        ));
    }
    if ctx.now >= binding.expires_at || ctx.now >= grant.expires_at {
        return Err(failure(
            AdapterErrorCode::SessionExpired,
            "Authorization or identity evidence expired.",
        ));
    }
    if &grant.binding != binding || !grant.operations.contains(&op) {
        return Err(failure(
            AdapterErrorCode::OperationNotAuthorized,
            "Authorization is bound to a different endpoint or operation.",
        ));
    }
    let p = ctx.profile;
    if binding.device_id != ctx.device.id
        || binding.discovery_revision != ctx.revision
        || p.discovery_revision != ctx.revision
        || p.device_id != ctx.device.id
        || p.input_digest != binding.evidence_digest
        || p.expires_at <= ctx.now
        || p.selected_profile.as_deref() != Some(&binding.profile_id)
        || p.profile_version.as_deref() != Some(&binding.profile_version)
        || p.profile_content_hash.as_deref() != Some(&binding.profile_hash)
    {
        return Err(failure(
            AdapterErrorCode::IdentityUnverified,
            "Target, profile, firmware or discovery evidence changed; reauthorize.",
        ));
    }
    let e = registry.evaluate(ctx);
    if e.selection == AdapterSelectionStatus::Ambiguous {
        return Err(failure(
            AdapterErrorCode::IdentityConflict,
            "Adapter match is ambiguous.",
        ));
    }
    if e.selected_adapter.as_deref() != Some(&binding.adapter_id) {
        return Err(failure(
            AdapterErrorCode::AdapterNotCompatible,
            "No unique exact adapter match exists.",
        ));
    }
    let m = e
        .matches
        .iter()
        .find(|m| m.adapter.id == binding.adapter_id)
        .ok_or_else(|| failure(AdapterErrorCode::AdapterNotFound, "Adapter is absent."))?;
    if m.adapter.version != binding.adapter_version {
        return Err(failure(
            AdapterErrorCode::AdapterNotCompatible,
            "Adapter version changed.",
        ));
    }
    if m.adapter.implementation_status == AdapterImplementationStatus::Unavailable {
        return Err(failure(
            AdapterErrorCode::DependencyMissing,
            "Adapter dependency is unavailable.",
        ));
    }
    if m.adapter.implementation_status == AdapterImplementationStatus::Declared {
        return Err(failure(
            AdapterErrorCode::OperationNotImplemented,
            "Physical adapter is declared but not implemented.",
        ));
    }
    if !m.adapter.operations.contains(&op) {
        return Err(failure(
            AdapterErrorCode::OperationUnsupported,
            "Adapter does not implement this operation.",
        ));
    }
    if binding.endpoint_identity.is_empty()
        || binding.interface_id.is_empty()
        || !p.evidence.iter().any(|v| {
            !v.stale
                && v.verification == Verification::Verified
                && v.reliability == Reliability::Strong
                && matches!(
                    v.source,
                    EvidenceOrigin::AuthenticatedManagement | EvidenceOrigin::SupportedModem
                )
                && v.endpoint.as_deref() == Some(&binding.endpoint)
                && v.interface_id.as_deref() == Some(&binding.interface_id)
        })
    {
        return Err(failure(
            AdapterErrorCode::EndpointUntrusted,
            "Endpoint trust and interface must match verified identity evidence.",
        ));
    }
    if binding.simulated
        != matches!(
            m.adapter.implementation_status,
            AdapterImplementationStatus::TestOnly | AdapterImplementationStatus::Simulator
        )
        || (m.adapter.implementation_status == AdapterImplementationStatus::Simulator
            && !registry.simulation_target(ctx, binding))
    {
        return Err(failure(
            AdapterErrorCode::EndpointUntrusted,
            "Simulated and physical endpoint provenance differ.",
        ));
    }
    Ok(())
}
