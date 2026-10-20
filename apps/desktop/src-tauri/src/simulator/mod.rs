//! Explicit, isolated virtual router. No sockets, OS networking calls or physical write dispatch.
mod adapter;
pub mod ipc;
mod model;
mod runtime;
pub mod types;
use crate::adapters::{DeviceContext, SessionBinding};
pub use runtime::SimulatorService;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};

/// Constructible only inside this module tree, by an explicitly activated isolated runtime.
/// There is no serde representation and no frontend-provided simulation authority.
#[derive(Clone)]
pub(crate) struct Authority {
    epoch: Arc<AtomicU64>,
    enabled: Arc<AtomicBool>,
    endpoint: String,
    pin: String,
}
impl Authority {
    pub(crate) fn allows(&self, ctx: &DeviceContext<'_>, b: &SessionBinding) -> bool {
        self.enabled.load(Ordering::Acquire)
            && b.simulated
            && b.adapter_id == "obtainer-simulator"
            && b.adapter_version == "1.0.0"
            && b.device_id == types::DEVICE_ID
            && ctx.device.id == types::DEVICE_ID
            && ctx.device.networks.is_empty()
            && ctx.device.interfaces.is_empty()
            && ctx.device.transports == ["simulation"]
            && b.endpoint == self.endpoint
            && b.endpoint_identity == self.pin
            && b.interface_id == "sim-interface-001"
            && b.discovery_revision == self.epoch.load(Ordering::Acquire)
            && ctx.revision == b.discovery_revision
            && b.profile_id == types::PROFILE_ID
    }
}
