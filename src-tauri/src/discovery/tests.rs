use super::*;

const FIXTURE: &str = include_str!("../../fixtures/discovery-lifecycle.json");

#[test]
fn composite_correlation_deduplication_and_capabilities() {
    let snapshots = replay_fixture(FIXTURE).unwrap();
    let arrival = &snapshots[1];
    assert_eq!(
        arrival.devices.len(),
        1,
        "One physical USB device, not four interfaces"
    );
    let device = &arrival.devices[0];
    assert_eq!(device.interfaces.len(), 3);
    assert_eq!(
        device.networks.len(),
        1,
        "Adapter GUID matches despite case and braces"
    );
    assert_eq!(device.vendor_id.as_deref(), Some("12D1"));
    assert_eq!(device.product_id.as_deref(), Some("14DC"));
    assert_eq!(device.confidence, 70);
    assert_eq!(device.capabilities.backup, Capability::Unknown);
    assert_eq!(device.capabilities.flash, Capability::Unsupported);
    assert_eq!(device.capabilities.features.apn.state, Capability::Unknown);
    assert!(!device.capabilities.features.apn.can_write);
    assert!(!device.capabilities.features.apn.can_verify);
    assert!(!device.capabilities.features.apn.can_rollback);
    assert_eq!(device.capabilities.features.ussd.confidence, 0);
    assert_eq!(snapshots[2].devices[0].interfaces.len(), 2);
    assert_eq!(snapshots[2].devices[0].confidence, 45);
}

#[test]
fn arrival_removal_and_reconnection_keep_stable_identity() {
    let snapshots = replay_fixture(FIXTURE).unwrap();
    assert!(snapshots[0].devices.is_empty());
    let original = &snapshots[1].devices[0];
    let removed = &snapshots[3].devices[0];
    assert!(!removed.connected);
    assert!(removed
        .interfaces
        .iter()
        .all(|interface| !interface.present));
    assert!(removed.networks.iter().all(|network| !network.link_up));
    assert_eq!(removed.last_seen_at, 3);
    let reconnected = &snapshots[4].devices[0];
    assert_eq!(reconnected.id, original.id);
    assert!(reconnected.connected);
    assert_eq!(snapshots[4].devices.len(), 1);
}

#[test]
fn similar_names_and_shared_pc_container_do_not_merge_unrelated_devices() {
    let nodes = ["PCI\\DEVICE_A", "PCI\\DEVICE_B"]
        .into_iter()
        .map(|instance| RawNode {
            instance_id: instance.into(),
            display_name: "Same name".into(),
            port_name: Some("COM1".into()),
            container_id: Some("PC-CONTAINER".into()),
            ..Default::default()
        })
        .collect();
    assert_eq!(
        assemble(
            Inventory {
                nodes,
                adapters: vec![]
            },
            1
        )
        .len(),
        2
    );
}

#[test]
fn downstream_usb_device_is_not_merged_into_upstream_hub() {
    let nodes = [
        RawNode {
            instance_id: "USB\\VID_1234&PID_0001\\HUB".into(),
            ..Default::default()
        },
        RawNode {
            instance_id: "USB\\VID_5678&PID_0002\\CHILD".into(),
            parent_instance_id: Some("USB\\VID_1234&PID_0001\\HUB".into()),
            ..Default::default()
        },
    ]
    .to_vec();
    assert_eq!(
        assemble(
            Inventory {
                nodes,
                adapters: vec![]
            },
            1
        )
        .len(),
        2
    );
}

#[test]
fn gateway_never_establishes_router_support_or_ownership() {
    let adapter = NetworkAdapter {
        id: "TEST-ADAPTER".into(),
        gateways: vec!["192.0.2.1".into()],
        ..Default::default()
    };
    let devices = assemble(
        Inventory {
            nodes: vec![],
            adapters: vec![adapter],
        },
        1,
    );
    assert_eq!(devices[0].confidence, 0);
    assert_eq!(devices[0].capabilities.backup, Capability::Unknown);
    assert!(devices[0].assessment.contains("unknown"));
}

#[test]
fn cyclic_parent_records_are_bounded_and_invalid_fixture_is_rejected() {
    let node = RawNode {
        instance_id: "USB\\ROOT_HUB\\TEST".into(),
        parent_instance_id: Some("USB\\ROOT_HUB\\TEST".into()),
        ..Default::default()
    };
    assert_eq!(
        assemble(
            Inventory {
                nodes: vec![node],
                adapters: vec![]
            },
            1
        )
        .len(),
        1
    );
    assert!(replay_fixture(
        r#"[{"name":"bad","inventory":{"nodes":[],"adapters":[],"shell":"cmd"}}]"#
    )
    .is_err());
}

#[cfg(windows)]
#[test]
fn native_inventory_is_available_without_opening_devices() {
    let devices = assemble(
        super::windows::capture().expect("Windows inventory APIs failed"),
        1,
    );
    let unique: BTreeSet<_> = devices.iter().map(|device| &device.id).collect();
    assert_eq!(devices.len(), unique.len());
    assert!(devices
        .iter()
        .all(|device| device.capabilities.flash == Capability::Unsupported));
}
