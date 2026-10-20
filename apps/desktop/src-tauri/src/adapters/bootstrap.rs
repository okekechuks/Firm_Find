//! Initial approval authorizes one identity/authentication handshake, never an established read.
use super::registry::failure;
use super::*;
use crate::profiles::{
    self, Attribute, CompatibilityReport, EvidenceOrigin, EvidenceRecord, MatchConfidence,
    Reliability, Verification,
};
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

/// Backend-owned receipt of an explicit user approval for this exact target and peer pin.
/// No serde/Clone, no caller-controlled authenticated flag, and no arbitrary operation list.
/// Phase 7 must obtain this approval through a reviewed user flow before calling this constructor.
pub struct HandshakeApproval {
    target: SessionBinding,
    used: AtomicBool,
    deadline: Instant,
}
impl HandshakeApproval {
    pub fn confirm_selected_target(target: SessionBinding, now: u64) -> Result<Self, AdapterError> {
        if target.expires_at <= now
            || [
                &target.endpoint,
                &target.interface_id,
                &target.endpoint_identity,
            ]
            .iter()
            .any(|s| s.trim().is_empty() || s.len() > 1024 || s.chars().any(char::is_control))
        {
            return Err(failure(
                AdapterErrorCode::OperationNotAuthorized,
                "Approve one fresh endpoint, interface and independently selected peer pin.",
            ));
        }
        let lifetime = target.expires_at.saturating_sub(now).min(30_000);
        Ok(Self {
            target,
            used: AtomicBool::new(false),
            deadline: Instant::now() + Duration::from_millis(lifetime),
        })
    }
    pub(super) fn claim(
        &self,
        registry: &Registry,
        ctx: &DeviceContext<'_>,
        target: &SessionBinding,
    ) -> Result<Instant, AdapterError> {
        if &self.target != target {
            return Err(failure(
                AdapterErrorCode::OperationNotAuthorized,
                "Handshake approval belongs to another endpoint, interface or evidence snapshot.",
            ));
        }
        if Instant::now() >= self.deadline || ctx.now >= target.expires_at {
            return Err(failure(
                AdapterErrorCode::SessionExpired,
                "Handshake approval expired.",
            ));
        }
        authorize_initial(registry, ctx, target)?;
        self.used
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| {
                failure(
                    AdapterErrorCode::OperationNotAuthorized,
                    "Handshake approval is single-use; approve a retry explicitly.",
                )
            })?;
        Ok(self.deadline)
    }
}

fn current_snapshot(ctx: &DeviceContext<'_>) -> Result<(), AdapterError> {
    if protected(ctx.device) {
        return Err(failure(
            AdapterErrorCode::ProtectedDevice,
            "Protected host hardware is excluded.",
        ));
    }
    if !ctx.device.connected {
        return Err(failure(
            AdapterErrorCode::DeviceDisconnected,
            "Device is disconnected.",
        ));
    }
    let p = ctx.profile;
    let fresh = profiles::evaluate(
        ctx.profiles,
        ctx.device,
        p.evidence.clone(),
        ctx.now,
        ctx.revision,
    );
    if p.device_id != ctx.device.id
        || p.discovery_revision != ctx.revision
        || p.engine_version != profiles::ENGINE_VERSION
        || p.registry_digest != ctx.profiles.digest
        || p.evaluated_at > ctx.now
        || p.expires_at <= ctx.now
        || p.input_digest != fresh.input_digest
    {
        return Err(failure(
            AdapterErrorCode::IdentityUnverified,
            "Discovery or available evidence changed; reauthorize.",
        ));
    }
    if matches!(
        fresh.confidence,
        MatchConfidence::Ambiguous | MatchConfidence::Conflicting
    ) || !fresh.conflicts.is_empty()
    {
        return Err(failure(
            AdapterErrorCode::IdentityConflict,
            "Ambiguous or conflicting device identity.",
        ));
    }
    Ok(())
}

fn authorize_initial(
    registry: &Registry,
    ctx: &DeviceContext<'_>,
    b: &SessionBinding,
) -> Result<(), AdapterError> {
    current_snapshot(ctx)?;
    if b.device_id != ctx.device.id
        || b.discovery_revision != ctx.revision
        || b.evidence_digest != ctx.profile.input_digest
    {
        return Err(failure(
            AdapterErrorCode::IdentityUnverified,
            "Handshake target or evidence changed.",
        ));
    }
    // A current, linked Windows interface is required even when router identity is unknown.
    if !ctx
        .device
        .networks
        .iter()
        .any(|n| n.id == b.interface_id && n.link_up && !n.virtual_interface)
    {
        return Err(failure(
            AdapterErrorCode::EndpointUntrusted,
            "Selected interface is absent, down or virtual.",
        ));
    }
    let profile = ctx
        .profiles
        .by_id(&b.profile_id, Some(&b.profile_version))
        .filter(|p| p.authoritative && p.content_hash == b.profile_hash)
        .ok_or_else(|| {
            failure(
                AdapterErrorCode::IdentityUnverified,
                "Expected trusted profile is unavailable.",
            )
        })?;
    // These are expected constraints, not a claim that discovery verified the profile.
    if ctx
        .profile
        .selected_profile
        .as_deref()
        .is_some_and(|id| id != b.profile_id)
    {
        return Err(failure(
            AdapterErrorCode::IdentityConflict,
            "Expected profile conflicts with verified identity.",
        ));
    }
    let evaluation = registry.evaluate(ctx);
    if evaluation.selection == AdapterSelectionStatus::Ambiguous {
        return Err(failure(
            AdapterErrorCode::IdentityConflict,
            "Adapter selection is ambiguous.",
        ));
    }
    let compatible = |d: &AdapterDescriptor| {
        (d.profile_ids.is_empty() || d.profile_ids.contains(&profile.profile.profile_id))
            && (d.firmware_families.is_empty()
                || d.firmware_families.iter().any(|f| {
                    profile
                        .profile
                        .firmware_families
                        .iter()
                        .any(|v| v.eq_ignore_ascii_case(f))
                }))
            && d.transports.contains(&Transport::Network)
            && ctx.device.transports.iter().any(|t| t == "network")
    };
    let descriptors = registry.descriptors();
    let viable: Vec<_> = descriptors.iter().filter(|d| compatible(d)).collect();
    if viable.len() != 1 {
        return Err(failure(
            AdapterErrorCode::IdentityConflict,
            "No unique handshake adapter exists.",
        ));
    }
    let d = viable[0];
    if d.id != b.adapter_id || d.version != b.adapter_version {
        return Err(failure(
            AdapterErrorCode::AdapterNotCompatible,
            "Selected adapter or version changed.",
        ));
    }
    if !matches!(
        d.implementation_status,
        AdapterImplementationStatus::BuiltIn | AdapterImplementationStatus::TestOnly
    ) {
        return Err(failure(
            AdapterErrorCode::OperationNotImplemented,
            "Handshake connector is not implemented.",
        ));
    }
    if d.authentication == AuthenticationMethod::None
        || !d
            .operations
            .contains(&AdapterOperation::AuthorizedIdentityProbe)
    {
        return Err(failure(
            AdapterErrorCode::OperationUnsupported,
            "Authenticated identity handshake is unavailable.",
        ));
    }
    if b.simulated != (d.implementation_status == AdapterImplementationStatus::TestOnly) {
        return Err(failure(
            AdapterErrorCode::EndpointUntrusted,
            "Simulated and physical provenance differ.",
        ));
    }
    Ok(())
}

pub(super) struct BootstrapEvidence {
    device: crate::discovery::Device,
    initial_digest: String,
    pub report: CompatibilityReport,
}
impl BootstrapEvidence {
    pub fn context<'a>(
        &'a self,
        ctx: &'a DeviceContext<'_>,
    ) -> Result<DeviceContext<'a>, AdapterError> {
        current_snapshot(ctx)?;
        if self.device != *ctx.device || self.initial_digest != ctx.profile.input_digest {
            return Err(failure(
                AdapterErrorCode::IdentityUnverified,
                "Initial device or evidence changed; reconnect.",
            ));
        }
        Ok(DeviceContext {
            device: ctx.device,
            profile: &self.report,
            profiles: ctx.profiles,
            revision: ctx.revision,
            now: ctx.now,
        })
    }
}

pub(super) fn verify_handshake(
    ctx: &DeviceContext<'_>,
    b: &SessionBinding,
    connection: &dyn AdapterConnection,
    identity: &AdapterIdentity,
) -> Result<BootstrapEvidence, AdapterError> {
    let expected = EndpointTrust {
        endpoint: b.endpoint.clone(),
        interface_id: b.interface_id.clone(),
        endpoint_identity: b.endpoint_identity.clone(),
        simulated: b.simulated,
    };
    if connection.endpoint_trust().as_ref() != Some(&expected) || identity.simulated != b.simulated
    {
        return Err(failure(
            AdapterErrorCode::EndpointUntrusted,
            "Connector did not attest the approved endpoint and interface.",
        ));
    }
    let fields = [
        (Attribute::Manufacturer, &identity.manufacturer),
        (Attribute::Model, &identity.model),
        (Attribute::HardwareRevision, &identity.hardware_revision),
        (Attribute::FirmwareFamily, &identity.firmware_family),
        (Attribute::FirmwareVersion, &identity.firmware_version),
        (Attribute::ManagementIdentity, &identity.management_identity),
    ];
    if fields
        .iter()
        .any(|(_, v)| v.trim().is_empty() || v.len() > 256 || v.chars().any(char::is_control))
        || ctx.profile.evidence.len() > 250
    {
        return Err(failure(
            AdapterErrorCode::IdentityUnverified,
            "Handshake identity is incomplete or exceeds bounds.",
        ));
    }
    let mut evidence = ctx.profile.evidence.clone();
    for (attribute, value) in fields {
        evidence.push(EvidenceRecord {
            id: format!("handshake-{}-{attribute:?}", b.adapter_id),
            subject_id: b.device_id.clone(),
            attribute,
            value: value.clone(),
            source: EvidenceOrigin::AuthenticatedManagement,
            collection_method: if b.simulated {
                "SIMULATED test-only authenticated handshake"
            } else {
                "Pinned authenticated adapter identity handshake"
            }
            .into(),
            collected_at: ctx.now,
            reliability: Reliability::Strong,
            verification: Verification::Verified,
            interface_id: Some(b.interface_id.clone()),
            endpoint: Some(b.endpoint.clone()),
            reference: None,
            hardware_applicability: Some(identity.hardware_revision.clone()),
            firmware_applicability: Some(identity.firmware_version.clone()),
            conflicts: vec![],
            stale: false,
        });
    }
    let report = profiles::evaluate(ctx.profiles, ctx.device, evidence, ctx.now, ctx.revision);
    if report.confidence != MatchConfidence::Verified
        || report.selected_profile.as_deref() != Some(&b.profile_id)
        || report.profile_version.as_deref() != Some(&b.profile_version)
        || report.profile_content_hash.as_deref() != Some(&b.profile_hash)
    {
        return Err(failure(
            AdapterErrorCode::IdentityConflict,
            "Authenticated identity does not uniquely verify the expected profile.",
        ));
    }
    Ok(BootstrapEvidence {
        device: ctx.device.clone(),
        initial_digest: ctx.profile.input_digest.clone(),
        report,
    })
}
