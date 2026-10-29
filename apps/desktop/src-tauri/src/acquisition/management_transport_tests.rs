//! Synthetic route facts. The explicitly named socket test uses loopback only.
use super::*;
use crate::discovery::{AddressPrefix, Inventory, RawNode};

fn snapshot() -> DiscoverySnapshot {
    let inventory = Inventory {
        nodes: vec![RawNode {
            instance_id: "USB\\VID_1234&PID_5678\\synthetic".into(),
            container_id: Some("synthetic-container".into()),
            display_name: "Synthetic router".into(),
            class_name: "Net".into(),
            adapter_id: Some("{11111111-2222-3333-4444-555555555555}".into()),
            ..RawNode::default()
        }],
        adapters: vec![NetworkAdapter {
            id: "{11111111-2222-3333-4444-555555555555}".into(),
            interface_index: 17,
            interface_type: 6,
            link_up: true,
            addresses: vec!["192.168.100.2".into()],
            luid: "12345".into(),
            prefixes: vec![AddressPrefix {
                address: "192.168.100.2".into(),
                length: 24,
            }],
            ..NetworkAdapter::default()
        }],
    };
    DiscoverySnapshot {
        devices: crate::discovery::assemble(inventory, connection::now()),
        status: DiscoveryStatus::Ready,
        scanned_at: Some(connection::now()),
        ..DiscoverySnapshot::default()
    }
}

#[test]
fn binding_fails_closed_without_proven_windows_route() {
    let snapshot = snapshot();
    let device = &snapshot.devices[0];
    let network = &device.networks[0];
    assert!(SourceBinding::capture_with(
        &snapshot,
        &device.id,
        &network.id,
        "http://192.168.100.1",
        |_, _, _| Err(BindingFailure::RouteUnproven)
    )
    .is_err());
}

fn good_route(
    network: &NetworkAdapter,
    source: Ipv4Addr,
    _: Ipv4Addr,
) -> Result<RouteFacts, BindingFailure> {
    Ok(RouteFacts {
        source,
        index: network.interface_index,
        luid: network.luid.parse().unwrap(),
        next_hop: Ipv4Addr::UNSPECIFIED,
    })
}

#[test]
fn selected_source_binding_uses_exact_adapter_and_rechecks_changes() {
    let mut snapshot = snapshot();
    let device = &snapshot.devices[0];
    let binding = SourceBinding::capture_with(
        &snapshot,
        &device.id,
        &device.networks[0].id,
        "http://192.168.100.1",
        good_route,
    )
    .unwrap();
    assert_eq!(
        binding.source_ipv4(),
        "192.168.100.2".parse::<Ipv4Addr>().unwrap()
    );
    binding.recheck_with(&snapshot, good_route).unwrap();
    snapshot.devices[0].networks[0].addresses = vec!["192.168.100.3".into()];
    assert_eq!(
        binding.recheck_with(&snapshot, good_route),
        Err(BindingFailure::DeviceChanged)
    );
}

#[test]
fn source_substitution_and_overlapping_subnet_are_rejected() {
    let mut snapshot = snapshot();
    let id = snapshot.devices[0].id.clone();
    let interface = snapshot.devices[0].networks[0].id.clone();
    let wrong = |network: &NetworkAdapter, _: Ipv4Addr, destination: Ipv4Addr| {
        good_route(network, "192.168.100.99".parse().unwrap(), destination)
    };
    assert!(
        SourceBinding::capture_with(&snapshot, &id, &interface, "http://192.168.100.1", wrong)
            .is_err()
    );
    let mut other = snapshot.devices[0].networks[0].clone();
    other.id = "{99999999-2222-3333-4444-555555555555}".into();
    other.interface_index = 18;
    other.luid = "12346".into();
    other.addresses = vec!["192.168.100.3".into()];
    other.prefixes[0].address = "192.168.100.3".into();
    snapshot.devices[0].networks.push(other);
    assert!(matches!(
        SourceBinding::capture_with(
            &snapshot,
            &id,
            &interface,
            "http://192.168.100.1",
            good_route
        ),
        Err(BindingFailure::OverlappingSubnet)
    ));
}

#[test]
#[cfg(windows)]
fn windows_reqwest_socket_uses_selected_loopback_source() {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
    };
    // bind() completes before client startup: no timing-based server readiness.
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let destination = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let fixture = snapshot();
    assert!(matches!(
        SourceBinding::capture_with(
            &fixture,
            &fixture.devices[0].id,
            &fixture.devices[0].networks[0].id,
            &format!("http://{destination}/"),
            good_route
        ),
        Err(BindingFailure::InvalidEndpoint)
    ));
    // Production deliberately refuses loopback device targets. Construct only a
    // test-local binding to exercise the socket builder, preserving that policy.
    let source: Ipv4Addr = "127.0.0.2".parse().unwrap();
    let mut network = fixture.devices[0].networks[0].clone();
    network.addresses = vec![source.to_string()];
    network.prefixes = vec![AddressPrefix {
        address: source.to_string(),
        length: 8,
    }];
    let binding = SourceBinding {
        device_id: fixture.devices[0].id.clone(),
        interface_id: network.id.clone(),
        endpoint: format!("http://{destination}/"),
        source,
        route: good_route(&network, source, Ipv4Addr::LOCALHOST).unwrap(),
        network,
        device_stamp: [0; 32],
        deadline: Instant::now() + Duration::from_secs(15),
    };
    let route = super::super::management_auth::synthetic_identity_route();
    let client = binding.identity_client(&route).unwrap();
    let request = client.qualified_request(&route).unwrap();
    let server = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(5);
        let (mut stream, peer) = loop {
            match listener.accept() {
                Ok(value) => break value,
                Err(error)
                    if error.kind() == std::io::ErrorKind::WouldBlock
                        && Instant::now() < deadline =>
                {
                    thread::yield_now()
                }
                Err(error) => panic!("Bound loopback request did not arrive: {error}"),
            }
        };
        // Windows inherits the listener's nonblocking flag on accepted sockets.
        stream.set_nonblocking(false).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut header = Vec::new();
        while !header.ends_with(b"\r\n\r\n") {
            let mut byte = [0u8];
            stream.read_exact(&mut byte).unwrap();
            header.push(byte[0]);
            assert!(header.len() <= 8192);
        }
        assert!(header.starts_with(b"GET /synthetic/identity?view=summary HTTP/1.1\r\n"));
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
            .unwrap();
        peer.ip()
    });
    // execute is accessible only inside this test module, never through production IPC.
    let response = client.client.execute(request);
    let peer = server.join();
    let response = response.unwrap();
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(response.text().unwrap(), "ok");
    assert_eq!(peer.unwrap(), "127.0.0.2".parse::<IpAddr>().unwrap());
}
