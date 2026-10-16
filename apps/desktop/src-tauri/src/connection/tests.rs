use super::*;
use crate::discovery::AddressPrefix;
#[test]
fn addresses_reject_credentials_paths_public_destinations_and_bad_ports() {
    for address in [
        "",
        "file:///etc/passwd",
        "http://user:pass@192.168.1.1/",
        "http://192.168.1.1/apply",
        "http://192.168.1.1/?reset=1",
        "http://127.0.0.1",
        "http://[::1]",
        "http://[::ffff:127.0.0.1]",
        "http://169.254.169.254",
        "http://8.8.8.8",
        "http://192.168.1.1:0",
        "http://192.168.1.1:65536",
        "http://192.168.1.1\\@evil.com",
        "http://192.168.1.1#x",
    ] {
        assert!(parse_address(address).is_err(), "{address}");
    }
    assert_eq!(
        parse_address("192.168.100.1").unwrap().url,
        "http://192.168.100.1/"
    );
    assert_eq!(
        parse_address("https://router.local:8443").unwrap().port,
        8443
    );
    assert!(parse_address("http://[fd12::1]").is_ok());
    assert!(parse_address("fe80::1").is_ok());
}
fn adapter(id: &str) -> NetworkAdapter {
    NetworkAdapter {
        id: id.into(),
        link_up: true,
        interface_index: 5,
        prefixes: vec![AddressPrefix {
            address: "192.168.100.2".into(),
            length: 24,
        }],
        ..Default::default()
    }
}
#[test]
fn scope_requires_on_link_physical_interface_and_handles_overlap() {
    let a = adapter("one");
    let mut b = adapter("two");
    let ip = "192.168.100.1".parse().unwrap();
    assert_eq!(
        select_interface(ip, None, &[a.clone(), b.clone()])
            .unwrap_err()
            .code,
        "ambiguous_interface"
    );
    assert_eq!(
        select_interface(ip, Some("one"), &[a.clone(), b.clone()])
            .unwrap()
            .id,
        "one"
    );
    b.virtual_interface = true;
    assert_eq!(
        select_interface(ip, None, &[b]).unwrap_err().code,
        "no_route"
    );
    assert_eq!(
        select_interface("10.5.2.1".parse().unwrap(), None, &[a])
            .unwrap_err()
            .code,
        "no_route"
    );
}
#[test]
fn cancellation_supersession_concurrency_and_removal_protect_persisted_result() {
    let temporary = tempfile::tempdir().unwrap();
    let repo = Mutex::new(Repository::open(&temporary.path().join("data.db")).unwrap());
    let saved = prepare_saved(
        ManualInput {
            address: "192.168.100.1".into(),
            label: "User supplied".into(),
            interface_id: None,
        },
        &DiscoverySnapshot::default(),
    )
    .unwrap();
    repo.lock().unwrap().save_router(&saved).unwrap();
    let service = ConnectionService::default();
    let first = service.begin(&saved.id).unwrap();
    let second = service.begin(&saved.id).unwrap();
    assert!(first.cancelled.load(Ordering::Acquire));
    assert!(service.begin("other").is_err());
    let observation = EndpointResult::failure("test", "Test observation");
    assert!(!service.publish(&first, &observation, &repo).unwrap());
    assert!(service.publish(&second, &observation, &repo).unwrap());
    service.cancel(&saved.id);
    assert!(!service
        .publish(&second, &EndpointResult::failure("stale", "old"), &repo)
        .unwrap());
    assert_eq!(
        repo.lock()
            .unwrap()
            .router(&saved.id)
            .unwrap()
            .unwrap()
            .last_result
            .unwrap()
            .code,
        "test"
    );
    drop(first);
    drop(second);
    let third = service.begin(&saved.id).unwrap();
    service.remove(&saved.id, &repo).unwrap();
    assert!(!service.publish(&third, &observation, &repo).unwrap());
    assert!(repo.lock().unwrap().routers().unwrap().is_empty());
}
#[test]
fn saved_targets_deduplicate_and_persist_without_claiming_live_connectivity() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("data.db");
    let input = |address: &str| ManualInput {
        address: address.into(),
        label: "My router".into(),
        interface_id: None,
    };
    let one = prepare_saved(input("192.168.100.1"), &DiscoverySnapshot::default()).unwrap();
    let two = prepare_saved(
        input("http://192.168.100.1/"),
        &DiscoverySnapshot::default(),
    )
    .unwrap();
    assert_eq!(one.id, two.id);
    {
        let mut repo = Repository::open(&path).unwrap();
        repo.save_router(&one).unwrap();
        repo.save_router(&two).unwrap();
        repo.save_router_result(&one.id, &EndpointResult::failure("timeout", "Old timeout"))
            .unwrap();
    }
    let repo = Repository::open(&path).unwrap();
    let entries = repo.routers().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].state, "discovered");
    assert_eq!(entries[0].last_result.as_ref().unwrap().code, "timeout");
}
