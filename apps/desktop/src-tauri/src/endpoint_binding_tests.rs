//! Synthetic inventory and route observations only; never query host routes or owner storage.
use super::*;
use crate::discovery::{assemble, AddressPrefix, Inventory, NetworkAdapter};

fn fixture() -> (DiscoverySnapshot, Selection, Route) {
    let network = NetworkAdapter {
        id: "{00112233-4455-6677-8899-aabbccddeeff}".into(),
        luid: "1234".into(),
        interface_index: 7,
        link_up: true,
        addresses: vec!["192.168.100.2".into()],
        prefixes: vec![AddressPrefix {
            address: "192.168.100.2".into(),
            length: 24,
        }],
        gateways: vec!["192.168.100.1".into()],
        ..Default::default()
    };
    let mut devices = assemble(
        Inventory {
            nodes: vec![],
            adapters: vec![network],
        },
        1_000,
    );
    devices[0].id = "synthetic-device".into();
    let selection = Selection {
        device: devices[0].id.clone(),
        interface: devices[0].networks[0].id.clone(),
        endpoint: "http://192.168.100.1/".into(),
    };
    (
        DiscoverySnapshot {
            devices,
            scanned_at: Some(1_000),
            status: DiscoveryStatus::Ready,
            ..Default::default()
        },
        selection,
        Route {
            source: "192.168.100.2".into(),
            gateway: None,
            luid: 1234,
        },
    )
}
impl RouteProvider for Route {
    fn route(&self, _: &connection::Target, _: &NetworkAdapter) -> Result<Route, BindingError> {
        Ok(self.clone())
    }
}

#[test]
fn stage4_material_and_freshness() {
    let (mut snapshot, selection, route) = fixture();
    let registry = Registry::default();
    let first = registry.bind(&snapshot, &selection, &route, 1_000).unwrap();
    snapshot.revision += 1;
    snapshot.devices[0].display_name = "new label 🌍".into();
    snapshot.devices[0].networks[0]
        .dns_servers
        .push("192.168.100.99".into());
    snapshot.scanned_at = Some(2_000);
    let same = registry.bind(&snapshot, &selection, &route, 2_000).unwrap();
    assert_eq!(first.stamp(), same.stamp());
    assert_eq!(first.expires_at, same.expires_at);
    registry.observe(&snapshot, &route, 2_000);
    assert!(first.current_at(2_001));
    snapshot.status = DiscoveryStatus::Failed;
    snapshot.devices.clear();
    registry.observe(&snapshot, &route, 2_001);
    assert!(first.current_at(2_002));
    snapshot.cancelled = true;
    snapshot.status = DiscoveryStatus::Ready;
    registry.observe(&snapshot, &route, 2_002);
    assert_eq!(first.expires_at, same.expires_at);
    assert!(!first.current_at(first.expires_at));
}

#[test]
fn stage4_every_material_change_invalidates_without_stale_publication() {
    for change in 0..12 {
        let (mut snapshot, mut selection, mut route) = fixture();
        let registry = Registry::default();
        let first = registry.bind(&snapshot, &selection, &route, 1_000).unwrap();
        match change {
            0 => snapshot.devices[0].connected = false,
            1 => snapshot.devices[0].identity.confidence = "conflicting".into(),
            2 => {
                snapshot.devices[0].networks[0].id = "{11112233-4455-6677-8899-aabbccddeeff}".into()
            }
            3 => snapshot.devices[0].networks[0].luid = "4567".into(),
            4 => route.source = "192.168.100.3".into(),
            5 => snapshot.devices[0].networks[0].prefixes[0].length = 25,
            6 => route.gateway = Some("192.168.100.254".into()),
            7 => snapshot.devices[0].networks[0].dhcp_server = Some("192.168.100.1".into()),
            8 => snapshot.devices[0].container_id = Some("different-correlation".into()),
            9 => selection.endpoint = "http://192.168.100.3/".into(),
            10 => snapshot.devices[0].networks[0]
                .gateways
                .push("192.168.100.254".into()),
            _ => route.luid = 4567,
        }
        if change == 9 {
            let _ = registry.bind(&snapshot, &selection, &route, 1_001);
        } else {
            registry.observe(&snapshot, &route, 1_001);
        }
        assert!(!first.current_at(1_001), "change {change}");
    }
}

#[test]
fn stage4_unknown_dhcp_both_transitions_and_replug() {
    let (mut snapshot, selection, route) = fixture();
    let registry = Registry::default();
    let unknown = registry.bind(&snapshot, &selection, &route, 1_000).unwrap();
    snapshot.devices[0].networks[0].dhcp_server = Some("192.168.100.1".into());
    let known = registry.bind(&snapshot, &selection, &route, 1_001).unwrap();
    assert!(!unknown.current_at(1_001));
    assert_ne!(unknown.stamp(), known.stamp());
    snapshot.devices[0].networks[0].dhcp_server = None;
    registry.observe(&snapshot, &route, 1_002);
    assert!(!known.current_at(1_002));
    let old = registry.bind(&snapshot, &selection, &route, 1_003).unwrap();
    snapshot.devices[0].connected = false;
    registry.observe(&snapshot, &route, 1_004);
    snapshot.devices[0].connected = true;
    let replug = registry.bind(&snapshot, &selection, &route, 1_005).unwrap();
    assert!(!old.current_at(1_005));
    assert_ne!(old.stamp(), replug.stamp());
}

#[test]
fn stage4_cancel_reset_restart_and_timeout_discard_in_memory_authority() {
    let (snapshot, selection, route) = fixture();
    let registry = Registry::default();
    let old = registry.bind(&snapshot, &selection, &route, 1_000).unwrap();
    registry.invalidate(&selection.device);
    assert!(!old.current_at(1_001));
    let new = registry.bind(&snapshot, &selection, &route, 1_001).unwrap();
    assert_ne!(old.stamp(), new.stamp());
    drop(registry);
    assert!(!new.current_at(1_002));
    let restart = Registry::default()
        .bind(&snapshot, &selection, &route, 1_002)
        .unwrap();
    assert_ne!(new.stamp(), restart.stamp());
    // Temporary registry drops immediately; proofs cannot outlive its lifecycle.
    assert!(!restart.current_at(1_002));
    let registry = Registry::default();
    assert!(registry
        .bind(&snapshot, &selection, &route, 61_001)
        .is_err());
}

#[test]
fn stage4_collision_unknown_route_protected_and_bad_sources_fail_closed() {
    let (mut snapshot, selection, mut route) = fixture();
    let mut competing = snapshot.devices[0].networks[0].clone();
    competing.id = "other".into();
    snapshot.devices[0].networks.push(competing);
    assert!(capture(&snapshot, &selection, &route).is_err());
    snapshot.devices[0].networks.pop();
    route.source = "192.168.100.1".into();
    assert!(capture(&snapshot, &selection, &route).is_err());
    route.source = "192.168.100.2".into();
    snapshot.devices[0].networks[0].prefixes.clear();
    assert!(capture(&snapshot, &selection, &route).is_err());
    let (mut snapshot, selection, route) = fixture();
    snapshot.devices[0].identity.confidence = "ambiguous".into();
    assert!(capture(&snapshot, &selection, &route).is_err());
}

#[test]
fn stage4_canonical_fixed_vector_and_set_order() {
    let (mut snapshot, selection, route) = fixture();
    let original = capture(&snapshot, &selection, &route).unwrap();
    let bytes = original.canonical(b"epoch", 7);
    // Independent Python hashlib/struct vector, documented in the verification report.
    assert_eq!(bytes.len(), 170);
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        "95db4defe08ddb98ca3ddeb5b2a0595bf659e3a61cfbd824ebb7ffc0f86815a9"
    );
    snapshot.devices[0].networks[0]
        .gateways
        .push("192.168.100.1".into());
    snapshot.devices[0].networks[0].addresses.reverse();
    assert_eq!(original, capture(&snapshot, &selection, &route).unwrap());
    assert_ne!(
        original.canonical(b"epoch", 7),
        original.canonical(b"epoch", 8)
    );
    assert_ne!(
        original.canonical(b"epoch", 7),
        original.canonical(b"other", 7)
    );
    let mut known = original.clone();
    known.dhcp = Some(Address::parse("192.168.100.1").unwrap());
    assert_eq!(
        format!("{:x}", Sha256::digest(known.canonical(b"epoch", 7))),
        "68ceebd77b14779d93c969fe898d56c6be149bfeb7688d42fe0a3c8bfec32619"
    );
    let mut ipv6 = original.clone();
    ipv6.endpoint = "http://[fe80::1]/".into();
    ipv6.source = Address::parse("fe80::2%7").unwrap();
    ipv6.prefix = 64;
    ipv6.gateways = vec![Address::parse("fe80::1%7").unwrap()];
    assert_eq!(
        format!("{:x}", Sha256::digest(ipv6.canonical(b"epoch", 7))),
        "0957db8ce132de35f22e5ed5dcbcb459b5d688a71759c35d24393445d4087e35"
    );
    let mut other_scope = ipv6.clone();
    other_scope.source.scope = 8;
    assert_ne!(
        ipv6.canonical(b"epoch", 7),
        other_scope.canonical(b"epoch", 7)
    );
    let mut absent = vec![];
    optional(&mut absent, None);
    let mut empty = vec![];
    optional(&mut empty, Some(vec![]));
    assert_ne!(absent, empty);
    // Nontrivial reorder: the selected source is deliberately not the first address.
    let (mut snapshot, selection, route) = fixture();
    let n = &mut snapshot.devices[0].networks[0];
    n.addresses.insert(0, "192.168.100.3".into());
    n.prefixes.push(AddressPrefix {
        address: "192.168.100.3".into(),
        length: 24,
    });
    n.default_routes = vec![
        crate::discovery::DefaultRoute {
            gateway: "192.168.100.1".into(),
            metric: 0,
            family: "ipv4".into(),
        },
        crate::discovery::DefaultRoute {
            gateway: "192.168.100.254".into(),
            metric: 10,
            family: "ipv4".into(),
        },
    ];
    let other = NetworkAdapter {
        id: "other-network".into(),
        addresses: vec!["10.20.30.2".into()],
        prefixes: vec![AddressPrefix {
            address: "10.20.30.2".into(),
            length: 24,
        }],
        link_up: true,
        ..Default::default()
    };
    snapshot.devices[0].networks.push(other);
    for name in ["A", "B"] {
        snapshot.devices[0].interfaces.push(serde_json::from_value(serde_json::json!({"instanceId":format!("USB\\SYNTHETIC_{name}"),"parentInstanceId":"USB\\SYNTHETIC_PARENT","className":"Net","classGuid":"synthetic","interfaceClasses":[],"hardwareIds":[],"portName":null,"present":true,"manufacturer":null,"compatibleIds":[],"interfacePaths":[]})).unwrap());
    }
    let before = capture(&snapshot, &selection, &route).unwrap();
    assert_eq!(before.source.ip, "192.168.100.2".parse::<IpAddr>().unwrap());
    let registry = Registry::default();
    let binding = registry.bind(&snapshot, &selection, &route, 1_000).unwrap();
    snapshot.devices[0].networks.reverse();
    snapshot.devices[0].interfaces.reverse();
    for n in &mut snapshot.devices[0].networks {
        n.addresses.reverse();
        n.prefixes.reverse();
        n.default_routes.reverse();
    }
    assert_eq!(before, capture(&snapshot, &selection, &route).unwrap());
    registry.observe(&snapshot, &route, 1_001);
    assert!(binding.current_at(1_001));
    snapshot.devices[0].interfaces[0].parent_instance_id =
        Some("USB\\OTHER_SYNTHETIC_PARENT".into());
    registry.observe(&snapshot, &route, 1_002);
    assert!(!binding.current_at(1_002));
}

#[test]
fn stage4_expiry_cannot_be_reversed_by_wall_clock_rollback() {
    let (snapshot, selection, route) = fixture();
    let registry = Registry::default();
    let bound = registry.bind(&snapshot, &selection, &route, 1_000).unwrap();
    assert!(!bound.current_at(bound.expires_at));
    assert!(!bound.current_at(1_001));
}

#[test]
fn stage4_ipv6_requires_selected_scope_and_route_family() {
    let (mut snapshot, mut selection, mut route) = fixture();
    let n = &mut snapshot.devices[0].networks[0];
    n.ipv6_index = 7;
    n.addresses = vec!["fe80::2%7".into()];
    n.prefixes = vec![AddressPrefix {
        address: "fe80::2%7".into(),
        length: 64,
    }];
    n.gateways = vec!["fe80::1%7".into()];
    selection.endpoint = "http://[fe80::1]/".into();
    route.source = "fe80::2%7".into();
    assert!(capture(&snapshot, &selection, &route).is_ok());
    snapshot.devices[0].networks[0].ipv6_index = 8;
    assert!(capture(&snapshot, &selection, &route).is_err());
    snapshot.devices[0].networks[0].ipv6_index = 7;
    route.gateway = Some("192.168.100.1".into());
    assert!(capture(&snapshot, &selection, &route).is_err());
}

#[test]
fn stage4_native_dhcp_schema_legacy_unknown_and_protected_devices() {
    let (mut snapshot, selection, route) = fixture();
    let mut json = serde_json::to_value(&snapshot.devices[0].networks[0]).unwrap();
    json.as_object_mut().unwrap().remove("dhcpServer");
    let old: NetworkAdapter = serde_json::from_value(json).unwrap();
    assert!(old.dhcp_server.is_none());
    assert_eq!(
        serde_json::to_value(old).unwrap()["dhcpServer"],
        serde_json::Value::Null
    );
    let mut protected = serde_json::json!({"instanceId":"synthetic","parentInstanceId":null,"className":"DiskDrive","classGuid":"synthetic","interfaceClasses":[],"hardwareIds":[],"portName":null,"present":true,"manufacturer":null,"compatibleIds":[],"interfacePaths":[]});
    for class in ["DiskDrive", "HIDClass", "Keyboard"] {
        protected["className"] = class.into();
        snapshot.devices[0].interfaces = vec![serde_json::from_value(protected.clone()).unwrap()];
        assert!(capture(&snapshot, &selection, &route).is_err());
    }
}
