pub mod identity;
mod service;
#[cfg(windows)]
mod windows;

use serde::{Deserialize, Serialize};
pub use service::DiscoveryService;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RawNode {
    pub instance_id: String,
    pub parent_instance_id: Option<String>,
    pub container_id: Option<String>,
    pub display_name: String,
    pub class_name: String,
    pub class_guid: String,
    #[serde(default)]
    pub hardware_ids: Vec<String>,
    pub port_name: Option<String>,
    pub adapter_id: Option<String>,
    #[serde(default)]
    pub interface_classes: Vec<String>,
    #[serde(default)]
    pub manufacturer: Option<String>,
    #[serde(default)]
    pub compatible_ids: Vec<String>,
    #[serde(default)]
    pub interface_paths: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NetworkAdapter {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub interface_index: u32,
    pub interface_type: u32,
    pub link_up: bool,
    pub addresses: Vec<String>,
    pub gateways: Vec<String>,
    #[serde(default)]
    pub prefixes: Vec<AddressPrefix>,
    #[serde(default)]
    pub dns_servers: Vec<String>,
    /// Windows DHCPv4 observation only. None means unknown, never a DNS/gateway fallback.
    #[serde(default)]
    pub dhcp_server: Option<String>,
    #[serde(default)]
    pub luid: String,
    #[serde(default)]
    pub ipv6_index: u32,
    #[serde(default)]
    pub virtual_interface: bool,
    #[serde(default)]
    pub default_routes: Vec<DefaultRoute>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AddressPrefix {
    pub address: String,
    pub length: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DefaultRoute {
    pub gateway: String,
    pub metric: u32,
    pub family: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Inventory {
    pub nodes: Vec<RawNode>,
    pub adapters: Vec<NetworkAdapter>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInterface {
    pub instance_id: String,
    pub parent_instance_id: Option<String>,
    pub class_name: String,
    pub class_guid: String,
    pub interface_classes: Vec<String>,
    pub hardware_ids: Vec<String>,
    pub port_name: Option<String>,
    pub present: bool,
    pub manufacturer: Option<String>,
    pub compatible_ids: Vec<String>,
    pub interface_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Capability {
    Supported,
    Unsupported,
    Unknown,
    RequiresAuth,
    RequiresTool,
    Experimental,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FeatureCapability {
    pub state: Capability,
    pub control_plane: Option<String>,
    pub can_read: bool,
    pub can_write: bool,
    pub can_verify: bool,
    pub can_rollback: bool,
    pub confidence: u8,
    pub evidence: Vec<String>,
}

impl Default for FeatureCapability {
    fn default() -> Self {
        Self {
            state: Capability::Unknown,
            control_plane: None,
            can_read: false,
            can_write: false,
            can_verify: false,
            can_rollback: false,
            confidence: 0,
            evidence: vec![],
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FeatureCapabilities {
    pub apn: FeatureCapability,
    pub wifi_user_limit: FeatureCapability,
    pub custom_dns: FeatureCapability,
    pub ussd: FeatureCapability,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub inventory: Capability,
    pub backup: Capability,
    pub flash: Capability,
    pub features: FeatureCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub id: String,
    pub display_name: String,
    pub connected: bool,
    pub transports: Vec<String>,
    pub vendor_id: Option<String>,
    pub product_id: Option<String>,
    pub container_id: Option<String>,
    pub interfaces: Vec<DeviceInterface>,
    pub networks: Vec<NetworkAdapter>,
    pub assessment: String,
    pub confidence: u8,
    pub evidence: Vec<String>,
    pub capabilities: Capabilities,
    pub last_seen_at: u64,
    pub identity: identity::Identity,
    pub category: identity::Category,
    pub lifecycle: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DiscoveryStatus {
    #[default]
    Starting,
    Ready,
    Degraded,
    Failed,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverySnapshot {
    pub revision: u64,
    pub scanned_at: Option<u64>,
    pub status: DiscoveryStatus,
    pub watching: bool,
    pub warnings: Vec<String>,
    pub devices: Vec<Device>,
    pub cancelled: bool,
    pub duration_ms: u64,
}

fn canonical(value: &str) -> String {
    value.trim().trim_matches(['{', '}']).to_ascii_uppercase()
}

fn identity(key: &str) -> String {
    format!("{:x}", Sha256::digest(key.as_bytes()))
}

fn usb_ids(value: &str) -> (Option<String>, Option<String>) {
    let value = value.to_ascii_uppercase();
    let read = |prefix: &str| {
        value.find(prefix).and_then(|index| {
            let id = value.get(index + prefix.len()..index + prefix.len() + 4)?;
            id.bytes()
                .all(|byte| byte.is_ascii_hexdigit())
                .then(|| id.to_owned())
        })
    };
    (read("VID_"), read("PID_"))
}

// Stop at this device's first non-composite USB node, not an upstream USB hub.
// Parent and container evidence are identity evidence; names are never join keys.
fn usb_root<'a>(node: &'a RawNode, nodes: &BTreeMap<String, &'a RawNode>) -> Option<&'a RawNode> {
    let mut current = node;
    let mut fallback = None;
    let mut visited = BTreeSet::new();
    for _ in 0..32 {
        let instance = canonical(&current.instance_id);
        if !visited.insert(instance.clone()) {
            break;
        }
        if instance.starts_with("USB\\") {
            fallback = Some(current);
            if instance.starts_with("USB\\VID_") && !instance.contains("&MI_") {
                return Some(current);
            }
        }
        match current
            .parent_instance_id
            .as_ref()
            .and_then(|id| nodes.get(&canonical(id)))
        {
            Some(parent) => current = parent,
            None => break,
        }
    }
    fallback
}

fn physical_key(node: &RawNode, root: Option<&RawNode>) -> String {
    let anchor = root.unwrap_or(node);
    if let Some(container) = anchor.container_id.as_ref().filter(|id| {
        let id = canonical(id);
        !id.is_empty() && id != "00000000-0000-0000-0000-000000000000"
    }) {
        // Container IDs for internal non-USB hardware can identify the whole PC.
        if root.is_some() {
            return format!("usb-container:{}", canonical(container));
        }
    }
    format!("pnp:{}", canonical(&anchor.instance_id))
}

fn new_device(key: &str, name: &str, now: u64) -> Device {
    Device {
        id: identity(key),
        display_name: name.to_owned(),
        connected: true,
        transports: vec![],
        vendor_id: None,
        product_id: None,
        container_id: None,
        interfaces: vec![],
        networks: vec![],
        assessment: "Unknown device".into(),
        confidence: 0,
        evidence: vec![],
        capabilities: Capabilities {
            inventory: Capability::Supported,
            backup: Capability::Unknown,
            flash: Capability::Unsupported,
            features: FeatureCapabilities::default(),
        },
        last_seen_at: now,
        identity: identity::Identity::default(),
        category: identity::Category::Unknown,
        lifecycle: "discovered".into(),
    }
}

pub(crate) fn history_device(
    id: String,
    name: String,
    last_seen_at: u64,
    category: identity::Category,
) -> Device {
    let mut device = new_device("history", &name, last_seen_at);
    device.id = id;
    device.connected = false;
    device.lifecycle = "disconnected".into();
    device.category = category;
    device.evidence =
        vec!["Persisted summary only. Refresh required to observe current interfaces.".into()];
    device
}

pub fn assemble(inventory: Inventory, now: u64) -> Vec<Device> {
    let nodes: BTreeMap<_, _> = inventory
        .nodes
        .iter()
        .map(|node| (canonical(&node.instance_id), node))
        .collect();
    let adapters: BTreeMap<_, _> = inventory
        .adapters
        .into_iter()
        .map(|adapter| (canonical(&adapter.id), adapter))
        .collect();
    let mut claimed_adapters = BTreeSet::new();
    let mut devices: BTreeMap<String, Device> = BTreeMap::new();
    for node in nodes.values() {
        let root = usb_root(node, &nodes);
        let serial = node
            .port_name
            .as_ref()
            .is_some_and(|port| port.to_ascii_uppercase().starts_with("COM"));
        let adapter = node
            .adapter_id
            .as_ref()
            .and_then(|id| adapters.get(&canonical(id)));
        if root.is_none() && !serial && adapter.is_none() {
            continue;
        }
        let key = physical_key(node, root);
        let anchor = root.unwrap_or(node);
        let device = devices
            .entry(key.clone())
            .or_insert_with(|| new_device(&key, &anchor.display_name, now));
        if root.is_some() {
            device.transports.push("usb".into());
            (device.vendor_id, device.product_id) = usb_ids(&anchor.instance_id);
            device.container_id = anchor.container_id.clone();
        }
        if serial {
            device.transports.push("serial".into());
        }
        device.interfaces.push(DeviceInterface {
            instance_id: node.instance_id.clone(),
            parent_instance_id: node.parent_instance_id.clone(),
            class_name: node.class_name.clone(),
            class_guid: node.class_guid.clone(),
            interface_classes: node.interface_classes.clone(),
            port_name: node.port_name.clone(),
            present: true,
            hardware_ids: node.hardware_ids.clone(),
            manufacturer: node.manufacturer.clone(),
            compatible_ids: node.compatible_ids.clone(),
            interface_paths: node.interface_paths.clone(),
        });
        if let Some(adapter) = adapter {
            if claimed_adapters.insert(canonical(&adapter.id)) {
                device.transports.push("network".into());
                device.networks.push(adapter.clone());
            }
        }
    }
    for (id, adapter) in adapters {
        if claimed_adapters.contains(&id) {
            continue;
        }
        let key = format!("adapter:{id}");
        let mut device = new_device(&key, &adapter.display_name, now);
        device.transports.push("network".into());
        device.networks.push(adapter);
        devices.insert(key, device);
    }
    for device in devices.values_mut() {
        device.transports.sort();
        device.transports.dedup();
        device
            .interfaces
            .sort_by(|a, b| a.instance_id.cmp(&b.instance_id));
        device.networks.sort_by(|a, b| a.id.cmp(&b.id));
        let usb = device.transports.iter().any(|value| value == "usb");
        let serial = device.transports.iter().any(|value| value == "serial");
        if usb && !device.networks.is_empty() {
            // Legacy wire field retained for existing consumers; no unvalidated score.
            device.confidence = 0;
            device.assessment = if serial {
                "Possible router or MiFi"
            } else {
                "USB networking device"
            }
            .into();
            device
                .evidence
                .push("Network adapter matched to its Windows device record".into());
            if serial {
                device
                    .evidence
                    .push("Serial and network functions share a USB identity".into());
            }
        } else if !device.networks.is_empty() {
            device.assessment = "Network adapter · router identity unknown".into();
        }
        if device
            .networks
            .iter()
            .any(|network| !network.gateways.is_empty())
        {
            device.evidence.push(
                "Windows reports a gateway; its identity and ownership are unverified".into(),
            );
        }
        if device.evidence.is_empty() {
            device
                .evidence
                .push("Windows inventory only; no verified device profile".into());
        }
        identity::resolve(device, now);
    }
    devices.into_values().collect()
}

impl DiscoverySnapshot {
    pub fn reconcile(&mut self, inventory: Inventory, now: u64) {
        let before = authority_inventory(&self.devices);
        let mut next: BTreeMap<_, _> = assemble(inventory, now)
            .into_iter()
            .map(|device| (device.id.clone(), device))
            .collect();
        for mut old in self.devices.drain(..) {
            if next.contains_key(&old.id) {
                continue;
            }
            old.connected = false;
            old.lifecycle = "disconnected".into();
            for interface in &mut old.interfaces {
                interface.present = false;
            }
            for network in &mut old.networks {
                network.link_up = false;
            }
            next.insert(old.id.clone(), old);
        }
        self.devices = next.into_values().collect();
        // Keep at most 512 disconnected records per session; present devices are never pruned.
        self.devices.sort_by(|a, b| {
            b.connected
                .cmp(&a.connected)
                .then(b.last_seen_at.cmp(&a.last_seen_at))
                .then(a.id.cmp(&b.id))
        });
        let connected = self
            .devices
            .iter()
            .filter(|device| device.connected)
            .count();
        self.devices.truncate(connected + 512);
        // Revision binds authorization to topology/evidence, not the scan clock.
        if self.revision == 0 || before != authority_inventory(&self.devices) {
            self.revision += 1;
        }
        self.scanned_at = Some(now);
        self.status = if self.warnings.is_empty() {
            DiscoveryStatus::Ready
        } else {
            DiscoveryStatus::Degraded
        };
    }
}

fn authority_inventory(devices: &[Device]) -> BTreeMap<String, Device> {
    devices
        .iter()
        .cloned()
        .map(|mut device| {
            device.last_seen_at = 0;
            for evidence in &mut device.identity.observations {
                evidence.observed_at = 0;
            }
            (device.id.clone(), device)
        })
        .collect()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReplayFrame {
    pub name: String,
    pub inventory: Inventory,
}

// Development-only fixture replay uses the same reducer as the native service.
// Fixtures are never selectable through desktop IPC or mixed with live devices.
pub fn replay_fixture(json: &str) -> Result<Vec<DiscoverySnapshot>, serde_json::Error> {
    let frames: Vec<ReplayFrame> = serde_json::from_str(json)?;
    let mut snapshot = DiscoverySnapshot::default();
    Ok(frames
        .into_iter()
        .enumerate()
        .map(|(index, frame)| {
            snapshot.reconcile(frame.inventory, index as u64 + 1);
            snapshot.clone()
        })
        .collect())
}

#[cfg(test)]
mod tests;
