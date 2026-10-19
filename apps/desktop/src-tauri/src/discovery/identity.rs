//! Evidence describes observations, never implied model or feature support.
use super::Device;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    RouterMifiCandidate,
    UsbModemCandidate,
    NetworkGatewayCandidate,
    GenericNetworkInterface,
    UnrelatedDevice,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Evidence {
    pub kind: String,
    pub value: String,
    pub source: String,
    pub reliability: String,
    pub observed_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Identity {
    pub confidence: String,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub firmware: Option<String>,
    pub observations: Vec<Evidence>,
    pub explanation: String,
}
impl Default for Identity {
    fn default() -> Self {
        Self {
            confidence: "unknown".into(),
            manufacturer: None,
            model: None,
            firmware: None,
            observations: vec![],
            explanation: "No supported management identity response has been collected.".into(),
        }
    }
}

/// Compare observations of the same identity claim. Windows driver/provider strings
/// are separate from endpoint headers and never treated as verified manufacturers.
pub fn conflicting_claims(observations: &[Evidence], kind: &str) -> bool {
    observations
        .iter()
        .filter(|e| e.kind == kind && e.reliability != "hint")
        .map(|e| e.value.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect::<BTreeSet<_>>()
        .len()
        > 1
}

pub fn resolve(device: &mut Device, now: u64) {
    let usb = device.transports.iter().any(|s| s == "usb");
    let serial = device.transports.iter().any(|s| s == "serial");
    let modem = device
        .interfaces
        .iter()
        .any(|i| i.class_name.eq_ignore_ascii_case("Modem"));
    let physical_network = device.networks.iter().any(|n| !n.virtual_interface);
    // An adapter observed only through IP Helper belongs to this PC. Its gateway can be a
    // router candidate, but the PC's Wi-Fi/Ethernet interface is never that router.
    let host_network_adapter = is_host_hardware(device);
    let gateway = device
        .networks
        .iter()
        .any(|n| !n.virtual_interface && n.link_up && !n.gateways.is_empty());
    let phone_hint = device
        .interfaces
        .iter()
        .flat_map(|i| &i.compatible_ids)
        .any(|v| v.to_ascii_lowercase().contains("android"));
    let bluetooth = device.interfaces.iter().any(|i| {
        i.class_name.eq_ignore_ascii_case("Bluetooth")
            || i.instance_id.to_ascii_uppercase().starts_with("BTH")
    });
    device.category = if host_network_adapter && !device.networks.is_empty() {
        Category::GenericNetworkInterface
    } else if bluetooth || host_network_adapter {
        Category::UnrelatedDevice
    } else if usb && physical_network && !phone_hint {
        Category::RouterMifiCandidate
    } else if usb && (modem || serial) && !phone_hint {
        Category::UsbModemCandidate
    } else if gateway && !host_network_adapter && !phone_hint {
        Category::NetworkGatewayCandidate
    } else if !device.networks.is_empty() {
        Category::GenericNetworkInterface
    } else {
        Category::UnrelatedDevice
    };
    let mut identity = Identity::default();
    for interface in &device.interfaces {
        if let Some(value) = &interface.manufacturer {
            identity.observations.push(Evidence {
                kind: "windows_manufacturer".into(),
                value: value.clone(),
                source: format!("Windows {} property", interface.class_name),
                reliability: "observed".into(),
                observed_at: now,
            });
        }
    }
    for network in &device.networks {
        for value in &network.gateways {
            identity.observations.push(Evidence {
                kind: "gateway".into(),
                value: value.clone(),
                source: format!("Windows interface {}", network.interface_index),
                reliability: "hint".into(),
                observed_at: now,
            });
        }
    }
    let relevant = matches!(
        device.category,
        Category::RouterMifiCandidate
            | Category::UsbModemCandidate
            | Category::NetworkGatewayCandidate
    );
    identity.confidence = if conflicting_claims(&identity.observations, "manufacturer") {
        "conflicting"
    } else if relevant {
        "candidate"
    } else {
        "unknown"
    }
    .into();
    if relevant {
        identity.explanation = "Windows topology supports a candidate. A phone, modem or other tethering device can expose the same interfaces. Manufacturer, model and firmware remain unverified.".into();
    }
    device.lifecycle = if !device.connected {
        "disconnected"
    } else if device.networks.is_empty() || device.networks.iter().any(|n| n.link_up) {
        "identity_unknown"
    } else {
        "disconnected"
    }
    .into();
    device.identity = identity;
}

/// Host PCI/ACPI/software adapters and storage/HID peripherals are not the gateway they reach.
pub fn is_host_hardware(device: &Device) -> bool {
    let usb = device.transports.iter().any(|t| t == "usb");
    (!usb && !device.networks.is_empty())
        || device.interfaces.iter().any(|i| {
            let instance = i.instance_id.to_ascii_uppercase();
            instance.starts_with("USBSTOR\\")
                || (!usb
                    && ["PCI\\", "ACPI\\", "ROOT\\", "SWD\\"]
                        .iter()
                        .any(|p| instance.starts_with(p)))
                || [
                    "diskdrive",
                    "hdc",
                    "scsiadapter",
                    "hidclass",
                    "keyboard",
                    "mouse",
                    "display",
                    "processor",
                    "system",
                    "computer",
                    "battery",
                ]
                .iter()
                .any(|c| i.class_name.eq_ignore_ascii_case(c))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn contradictory_claims_are_preserved_and_detected() {
        let entries: Vec<_> = ["Maker A", "Maker B"]
            .into_iter()
            .map(|value| Evidence {
                kind: "manufacturer".into(),
                value: value.into(),
                source: "test protocol".into(),
                reliability: "observed".into(),
                observed_at: 1,
            })
            .collect();
        assert!(conflicting_claims(&entries, "manufacturer"));
        assert_eq!(entries.len(), 2);
        assert!(!conflicting_claims(&entries[..1], "manufacturer"));
    }
}
