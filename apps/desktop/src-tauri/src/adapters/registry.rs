use super::*;
use crate::{
    discovery::{identity::Category, Device},
    profiles::{Assessment, MatchConfidence, Readiness},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Clone, Default)]
pub struct Registry {
    adapters: BTreeMap<String, (AdapterDescriptor, Arc<dyn DeviceAdapter>)>,
    allow_test: bool,
    simulation: Option<crate::simulator::Authority>,
    openwrt_lab: bool,
    selected_remote: Option<(String, String)>,
}
fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 96
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b".-_".contains(&b))
}
impl Registry {
    pub(crate) fn simulator(authority: crate::simulator::Authority) -> Self {
        Self {
            simulation: Some(authority),
            ..Self::default()
        }
    }
    pub(crate) fn simulation_target(
        &self,
        ctx: &DeviceContext<'_>,
        binding: &SessionBinding,
    ) -> bool {
        self.simulation
            .as_ref()
            .is_some_and(|s| s.allows(ctx, binding))
    }
    pub fn production() -> Result<Self, AdapterError> {
        let mut r = Self::default();
        r.register(crate::openwrt::adapter::OpenWrtAdapter::default())?;
        Ok(r)
    }
    pub(crate) fn for_openwrt(
        adapter: crate::openwrt::adapter::OpenWrtAdapter,
        lab: bool,
        remote: Option<(String, String)>,
    ) -> Result<Self, AdapterError> {
        let mut registry = Self {
            openwrt_lab: lab,
            selected_remote: remote,
            ..Self::default()
        };
        registry.register(adapter)?;
        Ok(registry)
    }
    pub(crate) fn selected_remote_target(
        &self,
        ctx: &DeviceContext<'_>,
        binding: &SessionBinding,
    ) -> bool {
        self.selected_remote
            .as_ref()
            .is_some_and(|(id, interface)| {
                id == &ctx.device.id
                    && id == &binding.device_id
                    && interface == &binding.interface_id
            })
            && ctx.device.networks.is_empty()
            && ctx.device.interfaces.is_empty()
            && binding.adapter_id == "openwrt-readonly-contract"
            && !binding.simulated
    }
    pub(crate) fn openwrt_lab_target(
        &self,
        ctx: &DeviceContext<'_>,
        binding: &SessionBinding,
    ) -> bool {
        self.openwrt_lab
            && cfg!(feature = "native-test-profile")
            && !binding.simulated
            && binding.device_id == crate::openwrt::runtime::LAB_TARGET
            && ctx.device.id == binding.device_id
            && binding.interface_id == "owned-openwrt-qemu-loopback"
            && binding.endpoint == "ssh://127.0.0.1:22270"
    }
    #[cfg(test)]
    pub(crate) fn test_registry() -> Self {
        Self {
            allow_test: true,
            ..Self::default()
        }
    }
    pub fn register<A: DeviceAdapter + 'static>(&mut self, a: A) -> Result<(), AdapterError> {
        let d = a.descriptor();
        let text =
            |s: &str| !s.trim().is_empty() && s.len() <= 1024 && !s.chars().any(char::is_control);
        let unique = |v: Vec<String>| v.iter().collect::<BTreeSet<_>>().len() == v.len();
        if !valid_id(&d.id)
            || !text(&d.name)
            || crate::profiles::version(&d.version).is_none()
            || d.transports.is_empty()
            || d.transports.len() > 4
            || d.operations.len() > 20
            || !unique(d.transports.iter().map(|v| format!("{v:?}")).collect())
            || !unique(d.operations.iter().map(|v| format!("{v:?}")).collect())
            || (d.profile_ids.is_empty() && d.firmware_families.is_empty())
            || d.profile_ids.iter().any(|v| !valid_id(v))
            || d.firmware_families
                .iter()
                .chain(&d.dependencies)
                .chain(&d.limitations)
                .any(|v| !text(v))
            || d.profile_ids.len() > 64
            || d.firmware_families.len() > 64
            || d.dependencies.len() > 32
            || d.limitations.len() > 32
            || (d.implementation_status == AdapterImplementationStatus::TestOnly
                && !self.allow_test)
            || (d.implementation_status == AdapterImplementationStatus::Simulator
                && self.simulation.is_none())
            || (self.simulation.is_some()
                && d.implementation_status != AdapterImplementationStatus::Simulator)
            || (d.transports.contains(&Transport::Simulation)
                != (d.implementation_status == AdapterImplementationStatus::Simulator))
            || (d.implementation_status == AdapterImplementationStatus::Simulator
                && (d.transports != [Transport::Simulation] || a.connector().is_none()))
            || (d.implementation_status == AdapterImplementationStatus::Declared
                && a.connector().is_some())
            || (d.implementation_status == AdapterImplementationStatus::BuiltIn
                && a.connector().is_none())
        {
            return Err(failure(
                AdapterErrorCode::AdapterNotCompatible,
                "Adapter descriptor or implementation status is invalid.",
            ));
        }
        if self.adapters.contains_key(&d.id) {
            return Err(failure(
                AdapterErrorCode::InternalAdapterError,
                "Duplicate adapter ID rejected.",
            ));
        }
        self.adapters.insert(d.id.clone(), (d, Arc::new(a)));
        Ok(())
    }
    pub fn descriptors(&self) -> Vec<AdapterDescriptor> {
        self.adapters.values().map(|(d, _)| d.clone()).collect()
    }
    pub(crate) fn adapter(&self, id: &str) -> Result<Arc<dyn DeviceAdapter>, AdapterError> {
        self.adapters
            .get(id)
            .map(|(_, a)| Arc::clone(a))
            .ok_or_else(|| {
                failure(
                    AdapterErrorCode::AdapterNotFound,
                    "Adapter is not registered.",
                )
            })
    }
    pub fn evaluate(&self, ctx: &DeviceContext<'_>) -> AdapterEvaluation {
        let mut matches: Vec<_> = self
            .adapters
            .values()
            .map(|(d, _)| match_descriptor(d, ctx))
            .collect();
        matches.sort_by(|a, b| {
            rank(b.status)
                .cmp(&rank(a.status))
                .then(a.adapter.id.cmp(&b.adapter.id))
        });
        let exact: Vec<_> = matches
            .iter()
            .filter(|m| m.status == AdapterMatchStatus::Exact)
            .collect();
        let weak = matches
            .iter()
            .filter(|m| m.status == AdapterMatchStatus::RequiresVerification)
            .count();
        let (selection, selected_adapter, mut state, reason) = if protected(ctx.device) {
            (
                AdapterSelectionStatus::Protected,
                None,
                ConnectionState::Unsupported,
                "Protected host hardware cannot be managed as a router.",
            )
        } else if exact.len() == 1 {
            (AdapterSelectionStatus::Candidate,Some(exact[0].adapter.id.clone()),ConnectionState::AuthorizationRequired,"One exact match; authorization, trusted endpoint and authentication are separate requirements.")
        } else if exact.len() > 1 || weak > 1 {
            (
                AdapterSelectionStatus::Ambiguous,
                None,
                ConnectionState::IdentityVerificationRequired,
                "Multiple viable adapters; no adapter selected.",
            )
        } else if weak == 1 {
            (
                AdapterSelectionStatus::Candidate,
                None,
                ConnectionState::IdentityVerificationRequired,
                "Partial adapter evidence requires verified hardware and firmware.",
            )
        } else {
            (
                AdapterSelectionStatus::NoMatch,
                None,
                ConnectionState::Unsupported,
                "No compatible adapter is available from current evidence.",
            )
        };
        if !ctx.device.connected {
            state = ConnectionState::Disconnected;
        } else if ctx.profile.expires_at <= ctx.now
            || ctx.profile.discovery_revision != ctx.revision
        {
            state = ConnectionState::Stale;
        } else if selected_adapter.as_ref().is_some_and(|id| {
            matches!(
                self.adapters[id].0.implementation_status,
                AdapterImplementationStatus::Declared | AdapterImplementationStatus::Unavailable
            )
        }) {
            state = ConnectionState::Unsupported;
        }
        AdapterEvaluation {
            device_id: ctx.device.id.clone(),
            discovery_revision: ctx.revision,
            protected_device: protected(ctx.device),
            selection,
            selected_adapter,
            connection_state: state,
            reason: reason.into(),
            matches,
            evaluated_at: ctx.now,
            expires_at: ctx
                .profile
                .expires_at
                .min(ctx.now.saturating_add(crate::profiles::EVIDENCE_TTL_MS)),
        }
    }
    pub fn evaluate_saved_endpoint(&self, id: &str, revision: u64, now: u64) -> AdapterEvaluation {
        AdapterEvaluation{device_id:id.into(),discovery_revision:revision,protected_device:false,selection:AdapterSelectionStatus::NoMatch,
            selected_adapter:None,connection_state:ConnectionState::IdentityVerificationRequired,
            reason:"Saved address and label do not verify router identity or authenticated management access.".into(),
            matches:vec![],evaluated_at:now,expires_at:now.saturating_add(crate::profiles::EVIDENCE_TTL_MS)}
    }
}
fn rank(s: AdapterMatchStatus) -> u8 {
    match s {
        AdapterMatchStatus::Exact => 3,
        AdapterMatchStatus::RequiresVerification => 2,
        AdapterMatchStatus::NotApplicable => 1,
        AdapterMatchStatus::Rejected => 0,
    }
}
pub fn protected(d: &Device) -> bool {
    crate::discovery::identity::is_host_hardware(d)
        || matches!(
            d.category,
            Category::GenericNetworkInterface | Category::UnrelatedDevice
        )
}
pub(crate) fn failure(code: AdapterErrorCode, message: &str) -> AdapterError {
    AdapterError::new(
        code,
        message,
        None,
        false,
        "Review current identity, endpoint and adapter requirements.",
    )
}
fn match_descriptor(d: &AdapterDescriptor, ctx: &DeviceContext<'_>) -> AdapterMatch {
    let p = ctx.profile;
    let mut status = AdapterMatchStatus::NotApplicable;
    let mut reason = "No compatible profile or verified firmware-family evidence.";
    let mut missing = vec!["verified compatible hardware and firmware".into()];
    if protected(ctx.device) {
        status = AdapterMatchStatus::Rejected;
        reason = "Protected host hardware is excluded.";
        missing.clear();
    } else if p.engine_version != crate::profiles::ENGINE_VERSION
        || p.evaluated_at > ctx.now
        || p.registry_digest != ctx.profiles.digest
        || p.device_id != ctx.device.id
        || p.discovery_revision != ctx.revision
        || p.expires_at <= ctx.now
        || !ctx.device.connected
    {
        status = AdapterMatchStatus::Rejected;
        reason =
            "Discovery or profile evidence is disconnected, expired or belongs to another target.";
    } else if matches!(
        p.confidence,
        MatchConfidence::Conflicting | MatchConfidence::Ambiguous
    ) || !p.conflicts.is_empty()
    {
        status = AdapterMatchStatus::Rejected;
        reason = "Conflicting or ambiguous identity prevents selection.";
    } else if !d.transports.iter().any(|t| {
        ctx.device.transports.iter().any(|v| {
            v == match t {
                Transport::Usb => "usb",
                Transport::Serial => "serial",
                Transport::Network => "network",
                Transport::Simulation => "simulation",
            }
        })
    }) {
        reason = "Device transport does not match.";
    } else {
        let compatible = |entry: &crate::profiles::RegistryEntry| {
            (d.profile_ids.is_empty() || d.profile_ids.contains(&entry.profile.profile_id))
                && (d.firmware_families.is_empty()
                    || entry.profile.firmware_families.iter().any(|f| {
                        d.firmware_families
                            .iter()
                            .any(|v| v.eq_ignore_ascii_case(f))
                    }))
        };
        let exact = p
            .selected_profile
            .as_deref()
            .and_then(|id| ctx.profiles.by_id(id, p.profile_version.as_deref()))
            .is_some_and(|entry| {
                compatible(entry)
                    && entry.authoritative
                    && p.profile_content_hash.as_deref() == Some(&entry.content_hash)
            });
        if exact && p.confidence == MatchConfidence::Verified {
            status = AdapterMatchStatus::Exact;
            reason="Fresh authoritative profile verifies exact hardware revision and constrained firmware.";
            missing = vec![
                "ownership authorization".into(),
                "trusted authenticated endpoint handshake".into(),
            ];
            if d.implementation_status == AdapterImplementationStatus::Declared {
                missing.push("implemented read-only transport (Phase 7)".into());
            }
        } else if p.candidates.iter().any(|c| {
            !c.matched_fields.is_empty()
                && c.conflicting_fields.is_empty()
                && !matches!(
                    c.confidence,
                    MatchConfidence::NoMatch | MatchConfidence::Conflicting
                )
                && ctx
                    .profiles
                    .by_id(&c.profile_id, Some(&c.profile_version))
                    .is_some_and(compatible)
        }) {
            status = AdapterMatchStatus::RequiresVerification;
            reason = "Partial profile evidence is a hint; no communication is authorized.";
        }
    }
    let capabilities = d
        .operations
        .iter()
        .map(|op| AdapterCapability {
            operation: *op,
            exists: false,
            can_read: false,
            can_plan: false,
            can_apply: false,
            can_verify: false,
            can_restore: false,
            authentication_required: d.authentication != AuthenticationMethod::None,
            reason: "No implemented authenticated session has probed this operation.".into(),
        })
        .collect();
    let features = ["dns", "wifiUserLimit", "apn", "ussd"]
        .into_iter()
        .map(|key| {
            let f = p.features.get(key);
            let assessment = if status == AdapterMatchStatus::Exact {
                f.map_or(Assessment::Unknown, |f| f.assessment)
            } else {
                Assessment::Unknown
            };
            let mut readiness = f.map_or_else(
                || vec![Readiness::RequiresManualVerification],
                |f| f.readiness.clone(),
            );
            if !readiness.contains(&Readiness::NotImplemented) {
                readiness.push(Readiness::NotImplemented);
            }
            (
                key.into(),
                AdapterFeatureCapability {
                    assessment,
                    readiness,
                    can_read: false,
                    can_write: false,
                    can_verify: false,
                    can_rollback: false,
                    evidence_sources: if status == AdapterMatchStatus::Exact {
                        f.map_or_else(Vec::new, |f| f.evidence_sources.clone())
                    } else {
                        vec![]
                    },
                    reason: f.map_or_else(
                        || "No verified feature evidence or authenticated probe.".into(),
                        |f| f.reason.clone(),
                    ),
                },
            )
        })
        .collect();
    AdapterMatch {
        adapter: d.clone(),
        status,
        confidence: match status {
            AdapterMatchStatus::Exact => 100,
            AdapterMatchStatus::RequiresVerification => 30,
            _ => 0,
        },
        reason: reason.into(),
        matched_evidence: if status == AdapterMatchStatus::Exact {
            vec![p.selected_profile.clone().unwrap_or_default()]
        } else {
            vec![]
        },
        missing_evidence: missing,
        capabilities,
        features,
    }
}
