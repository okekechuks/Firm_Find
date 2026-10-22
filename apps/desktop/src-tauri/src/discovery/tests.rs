use super::*;

const FIXTURE: &str = include_str!("../../fixtures/discovery-lifecycle.json");

#[test]
fn review_unchanged_scan_preserves_authorization_revision() {
    let frames: Vec<ReplayFrame> = serde_json::from_str(FIXTURE).unwrap();
    let inventory = frames[1].inventory.clone();
    let mut snapshot = DiscoverySnapshot::default();
    snapshot.reconcile(inventory.clone(), 1);
    let revision = snapshot.revision;
    snapshot.reconcile(inventory, 2);
    assert_eq!(snapshot.revision, revision);
    assert_eq!(snapshot.scanned_at, Some(2));
    snapshot.reconcile(Inventory::default(), 3);
    assert!(
        snapshot.revision > revision,
        "removal must revoke authority"
    );
}

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
    assert_eq!(device.confidence, 0);
    assert_eq!(device.identity.confidence, "candidate");
    assert_eq!(
        device.category,
        super::identity::Category::RouterMifiCandidate
    );
    assert_eq!(device.capabilities.backup, Capability::Unknown);
    assert_eq!(device.capabilities.flash, Capability::Unsupported);
    assert_eq!(device.capabilities.features.apn.state, Capability::Unknown);
    assert!(!device.capabilities.features.apn.can_write);
    assert!(!device.capabilities.features.apn.can_verify);
    assert!(!device.capabilities.features.apn.can_rollback);
    assert_eq!(device.capabilities.features.ussd.confidence, 0);
    assert_eq!(snapshots[2].devices[0].interfaces.len(), 2);
    assert_eq!(snapshots[2].devices[0].identity.confidence, "candidate");
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

#[test]
fn identical_usb_vendor_ids_do_not_merge_and_missing_ids_remain_unknown() {
    let nodes = [
        "USB\\VID_1234&PID_5678\\ONE",
        "USB\\VID_1234&PID_5678\\TWO",
        "USB\\MISSING\\THREE",
    ]
    .into_iter()
    .map(|id| RawNode {
        instance_id: id.into(),
        ..Default::default()
    })
    .collect();
    let devices = assemble(
        Inventory {
            nodes,
            adapters: vec![],
        },
        1,
    );
    assert_eq!(devices.len(), 3);
    assert_eq!(devices.iter().filter(|d| d.vendor_id.is_none()).count(), 1);
    assert_eq!(usb_ids("USB\\VID_ZZZZ&PID_123"), (None, None));
}

#[test]
fn bluetooth_usb_serial_and_pan_functions_are_not_router_candidates() {
    let nodes = vec![RawNode {
        instance_id: "USB\\VID_1234&PID_5678\\BT".into(),
        class_name: "Bluetooth".into(),
        adapter_id: Some("pan".into()),
        port_name: Some("COM7".into()),
        ..Default::default()
    }];
    let adapters = vec![NetworkAdapter {
        id: "pan".into(),
        virtual_interface: true,
        ..Default::default()
    }];
    let devices = assemble(Inventory { nodes, adapters }, 1);
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].category, identity::Category::UnrelatedDevice);
    assert_eq!(devices[0].identity.confidence, "unknown");
}

#[test]
fn virtual_gateway_phone_hint_and_host_network_adapters_stay_unverified() {
    let nodes = vec![RawNode {
        instance_id: "USB\\VID_1234&PID_5678\\PHONE".into(),
        adapter_id: Some("phone".into()),
        compatible_ids: vec!["android".into()],
        ..Default::default()
    }];
    let adapters = ["phone", "vpn", "wifi", "ethernet"]
        .into_iter()
        .map(|id| NetworkAdapter {
            id: id.into(),
            link_up: true,
            virtual_interface: id == "vpn",
            gateways: vec!["192.168.1.1".into()],
            ..Default::default()
        })
        .collect();
    let devices = assemble(Inventory { nodes, adapters }, 1);
    assert_eq!(devices.len(), 4);
    assert!(devices
        .iter()
        .all(|d| d.identity.confidence != "verified" && d.identity.model.is_none()));
    assert!(devices
        .iter()
        .filter(|d| d.networks[0].id == "vpn")
        .all(|d| d.category == identity::Category::GenericNetworkInterface));
    assert!(devices
        .iter()
        .filter(|d| d.networks[0].id == "phone")
        .all(|d| d.category != identity::Category::RouterMifiCandidate));
    assert!(devices
        .iter()
        .filter(|d| d.interfaces.is_empty())
        .all(|d| d.category == identity::Category::GenericNetworkInterface));
    assert!(devices
        .iter()
        .filter(|d| d.networks[0].id == "phone")
        .all(|d| d.category == identity::Category::GenericNetworkInterface));
}

#[test]
fn external_usb_network_device_remains_a_router_candidate_while_host_wifi_is_excluded() {
    let host = NetworkAdapter {
        id: "host-wifi".into(),
        display_name: "MediaTek MT7921 Wi-Fi 6 802.11ax PCIe Adapter".into(),
        link_up: true,
        gateways: vec!["192.168.1.1".into()],
        ..Default::default()
    };
    let external_node = RawNode {
        instance_id: "USB\\VID_12D1&PID_14DC\\MIFI".into(),
        adapter_id: Some("external-rndis".into()),
        ..Default::default()
    };
    let external = NetworkAdapter {
        id: "external-rndis".into(),
        link_up: true,
        gateways: vec!["192.168.8.1".into()],
        ..Default::default()
    };
    let devices = assemble(
        Inventory {
            nodes: vec![
                external_node,
                RawNode {
                    instance_id: "PCI\\VEN_14C3&DEV_7961\\HOST".into(),
                    adapter_id: Some("host-wifi".into()),
                    class_name: "Net".into(),
                    ..Default::default()
                },
            ],
            adapters: vec![host, external],
        },
        1,
    );
    assert!(devices.iter().any(|device| {
        device
            .networks
            .iter()
            .any(|network| network.id == "host-wifi")
            && device.category == identity::Category::GenericNetworkInterface
    }));
    assert!(devices.iter().any(|device| {
        device
            .networks
            .iter()
            .any(|network| network.id == "external-rndis")
            && device.category == identity::Category::RouterMifiCandidate
    }));
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
