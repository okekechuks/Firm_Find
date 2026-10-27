//! Explicit synthetic identity, reachable only in tests/the isolated native binary.
use super::*;
use crate::discovery::{Inventory, NetworkAdapter, RawNode};
use profiles::{Attribute, EvidenceOrigin, Reliability, Verification};
pub const ID: &str = "1111111111111111111111111111111111111111111111111111111111111111";
pub fn seed(repo: &mut Repository, time: u64) -> Result<DiscoverySnapshot> {
    let mut snapshot = DiscoverySnapshot::default();
    snapshot.reconcile(
        Inventory {
            nodes: vec![RawNode {
                instance_id: "USB\\SYNTHETIC_WORKFLOW_ONLY".into(),
                display_name: "SYNTHETIC workflow target".into(),
                ..Default::default()
            }],
            adapters: vec![NetworkAdapter {
                id: "synthetic-workflow-interface".into(),
                display_name: "SYNTHETIC USB interface".into(),
                link_up: true,
                addresses: vec!["192.0.2.2".into()],
                gateways: vec!["192.0.2.1".into()],
                ..Default::default()
            }],
        },
        time,
    );
    let mut target = snapshot.devices.remove(0);
    target.id = ID.into();
    target.display_name = "SOFTWARE FIXTURE - TP-Link Archer C7 v5".into();
    target.category = Category::RouterMifiCandidate;
    target.connected = true;
    target.networks = vec![NetworkAdapter {
        id: "synthetic-workflow-interface".into(),
        display_name: "SYNTHETIC USB interface".into(),
        link_up: true,
        addresses: vec!["192.0.2.2".into()],
        gateways: vec!["192.0.2.1".into()],
        ..Default::default()
    }];
    snapshot.devices = vec![target];
    snapshot.revision = 17;
    snapshot.scanned_at = Some(time);
    snapshot.status = DiscoveryStatus::Ready;
    snapshot.warnings=vec!["Synthetic device identity for software integration only. No physical router was identified or contacted.".into()];
    repo.save_discovery(&snapshot).map_err(|_| {
        blocked(
            "DATABASE_UNAVAILABLE",
            "Cannot seed isolated workflow fixture.",
        )
    })?;
    for (attribute, value) in [
        (Attribute::Manufacturer, "TP-Link"),
        (Attribute::Model, "Archer C7"),
        (Attribute::HardwareRevision, "v5"),
        (Attribute::FirmwareFamily, "OpenWrt"),
        (Attribute::FirmwareVersion, "25.12.6"),
    ] {
        let e = EvidenceRecord {
            id: format!(
                "{:x}",
                Sha256::digest(format!("synthetic:{attribute:?}:{time}").as_bytes())
            ),
            subject_id: ID.into(),
            attribute,
            value: value.into(),
            source: EvidenceOrigin::SupportedModem,
            collection_method: "SYNTHETIC software fixture; not physical identity evidence".into(),
            collected_at: time,
            reliability: Reliability::Strong,
            verification: Verification::Verified,
            interface_id: Some("synthetic-workflow-interface".into()),
            endpoint: Some("http://192.0.2.1/".into()),
            reference: Some("synthetic fixture".into()),
            hardware_applicability: None,
            firmware_applicability: None,
            conflicts: vec![],
            stale: false,
        };
        repo.save_identity_observation(&e).map_err(|_| {
            blocked(
                "DATABASE_UNAVAILABLE",
                "Cannot seed isolated fixture evidence.",
            )
        })?;
    }
    Ok(snapshot)
}
