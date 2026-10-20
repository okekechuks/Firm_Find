use super::*;
use std::sync::Barrier;

#[test]
fn post_review_cancel_idle_releases_busy_and_allows_reads() {
    let s = connected();
    s.control(Control::Cancel).unwrap();
    assert!(!s.runtime().unwrap().busy.load(Ordering::Acquire));
    assert!(request(&s, Action::ReadFeatures)
        .unwrap()
        .features
        .is_some());
}

#[test]
fn post_review_scenario_reserves_busy_before_mutating_state() {
    let s = service();
    let r = s.runtime().unwrap();
    let pause = r.test_hooks.arm(TestPoint::Scenario);
    std::thread::scope(|scope| {
        let worker = scope.spawn(|| {
            s.control(Control::Scenario {
                scenario: Scenario::Healthy,
                repeat: false,
            })
        });
        pause.entered();
        let owned = r.busy.load(Ordering::Acquire);
        drop(pause);
        worker.join().unwrap().unwrap();
        assert!(owned, "scenario must acquire busy before touching state");
    });
    assert!(!r.busy.load(Ordering::Acquire));
}

fn input(s: &SimulatorService, action: Action) -> Request {
    let status = s.status().unwrap();
    Request {
        target: DEVICE_ID.into(),
        endpoint: status.device.unwrap().endpoint,
        epoch: status.epoch,
        revision: status.revision,
        action,
    }
}
fn quiescent(s: &SimulatorService, connections: usize) {
    let r = s.runtime().unwrap();
    let status = s.status().unwrap();
    assert!(!status.busy);
    assert!(!r.busy.load(Ordering::Acquire));
    assert!(lock(&r.cancellation).unwrap().is_none());
    assert_eq!(status.active_connections, connections);
    assert_eq!(r.resources.load(Ordering::Acquire), connections);
}
#[test]
fn post_review_cancel_active_slow_read_then_reconnect_read_and_dns() {
    let s = connected();
    scenario(&s, Scenario::SlowRead);
    let r = s.runtime().unwrap();
    let read = input(&s, Action::ReadFeatures);
    let delay = r.test_hooks.arm(TestPoint::Delay);
    std::thread::scope(|scope| {
        let worker = scope.spawn(|| s.request(read));
        delay.entered();
        let interrupt = r.test_hooks.arm(TestPoint::Interrupt);
        let cancel = scope.spawn(|| s.control(Control::Cancel));
        interrupt.entered();
        drop(delay);
        drop(interrupt);
        assert_eq!(error(worker.join().unwrap()), ErrorCode::Cancelled);
        cancel.join().unwrap().unwrap();
    });
    quiescent(&s, 0);
    scenario(&s, Scenario::Healthy);
    connect(&s);
    assert!(request(&s, Action::ReadFeatures)
        .unwrap()
        .features
        .is_some());
    assert!(apply(&s).outcome.unwrap().verified);
    request(&s, Action::Disconnect).unwrap();
    quiescent(&s, 0);
}
#[test]
fn post_review_repeated_cancel_then_valid_operation_and_cleanup() {
    let s = connected();
    for _ in 0..2 {
        s.control(Control::Cancel).unwrap();
        quiescent(&s, 1);
    }
    assert!(request(&s, Action::ReadFeatures).is_ok());
    request(&s, Action::Disconnect).unwrap();
    quiescent(&s, 0);
}
#[test]
fn post_review_scenario_excludes_connection_and_other_scenarios() {
    let s = service();
    let approval = request(&s, Action::ApproveHandshake)
        .unwrap()
        .approval_id
        .unwrap();
    let connect_request = input(
        &s,
        Action::Connect {
            approval_id: approval,
            credential: "lab-only".into(),
        },
    );
    let r = s.runtime().unwrap();
    let pause = r.test_hooks.arm(TestPoint::Scenario);
    std::thread::scope(|scope| {
        let scenario = scope.spawn(|| {
            s.control(Control::Scenario {
                scenario: Scenario::Healthy,
                repeat: false,
            })
        });
        pause.entered();
        assert_eq!(error(s.request(connect_request)), ErrorCode::Busy);
        assert_eq!(
            error(s.control(Control::Scenario {
                scenario: Scenario::WrongIdentity,
                repeat: false
            })),
            ErrorCode::Busy
        );
        assert!(
            r.busy.load(Ordering::Acquire),
            "rejected contender cannot release scenario ownership"
        );
        drop(pause);
        scenario.join().unwrap().unwrap();
    });
    quiescent(&s, 0);
    connect(&s);
    request(&s, Action::Disconnect).unwrap();
    quiescent(&s, 0);
}
#[test]
fn post_review_scenario_excludes_dns_apply_and_stales_queued_results() {
    let s = connected();
    let preview = request(
        &s,
        Action::PreviewDns {
            configuration: proposal("9.9.9.9"),
        },
    )
    .unwrap()
    .preview
    .unwrap();
    let apply = input(
        &s,
        Action::ApplyDns {
            preview_id: preview.id,
        },
    );
    let stale = input(&s, Action::ReadFeatures);
    let r = s.runtime().unwrap();
    let pause = r.test_hooks.arm(TestPoint::Scenario);
    std::thread::scope(|scope| {
        let change = scope.spawn(|| {
            s.control(Control::Scenario {
                scenario: Scenario::Healthy,
                repeat: false,
            })
        });
        pause.entered();
        assert_eq!(error(s.request(apply)), ErrorCode::Busy);
        assert!(r.busy.load(Ordering::Acquire));
        drop(pause);
        change.join().unwrap().unwrap();
    });
    assert_eq!(error(s.request(stale)), ErrorCode::Stale);
    let status = s.status().unwrap();
    assert!(status.preview.is_none() && status.outcome.is_none() && status.features.is_none());
    assert_eq!(
        lock(&r.state).unwrap().configuration.dns,
        model::dns_fixture()
    );
    request(&s, Action::Disconnect).unwrap();
    quiescent(&s, 0);
}
#[test]
fn post_review_running_connection_or_dns_excludes_scenario_and_advance() {
    for dns in [false, true] {
        let s = if dns { connected() } else { service() };
        let action = if dns {
            let preview = request(
                &s,
                Action::PreviewDns {
                    configuration: proposal("9.9.9.9"),
                },
            )
            .unwrap()
            .preview
            .unwrap();
            Action::ApplyDns {
                preview_id: preview.id,
            }
        } else {
            let approval = request(&s, Action::ApproveHandshake)
                .unwrap()
                .approval_id
                .unwrap();
            Action::Connect {
                approval_id: approval,
                credential: "lab-only".into(),
            }
        };
        let operation = input(&s, action);
        let r = s.runtime().unwrap();
        let before = s.status().unwrap();
        let pause = r.test_hooks.arm(TestPoint::Request);
        std::thread::scope(|scope| {
            let worker = scope.spawn(|| s.request(operation));
            pause.entered();
            assert_eq!(
                error(s.control(Control::Scenario {
                    scenario: Scenario::Healthy,
                    repeat: false
                })),
                ErrorCode::Busy
            );
            assert_eq!(
                error(s.control(Control::Advance {
                    milliseconds: 31_000
                })),
                ErrorCode::Busy
            );
            assert!(r.busy.load(Ordering::Acquire));
            assert_eq!(lock(&r.state).unwrap().clock, before.virtual_time);
            drop(pause);
            worker.join().unwrap().unwrap();
        });
        quiescent(&s, 1);
        request(&s, Action::Disconnect).unwrap();
        quiescent(&s, 0);
    }
}
#[test]
fn post_review_reset_or_stop_during_delayed_read_rejects_old_success() {
    for stop in [false, true] {
        let s = connected();
        scenario(&s, Scenario::SlowRead);
        let read = input(&s, Action::ReadFeatures);
        let stale = input(&s, Action::ReadFeatures);
        let r = s.runtime().unwrap();
        let delay = r.test_hooks.arm(TestPoint::Delay);
        std::thread::scope(|scope| {
            let worker = scope.spawn(|| s.request(read));
            delay.entered();
            let interrupt = r.test_hooks.arm(TestPoint::Interrupt);
            let control =
                scope.spawn(|| s.control(if stop { Control::Stop } else { Control::Reset }));
            interrupt.entered();
            drop(delay);
            drop(interrupt);
            assert_eq!(error(worker.join().unwrap()), ErrorCode::Cancelled);
            control.join().unwrap().unwrap();
        });
        assert_eq!(r.resources.load(Ordering::Acquire), 0);
        assert!(!r.busy.load(Ordering::Acquire));
        assert!(lock(&r.cancellation).unwrap().is_none());
        assert_eq!(
            error(r.perform(stale)),
            if stop {
                ErrorCode::Disabled
            } else {
                ErrorCode::Stale
            }
        );
        if stop {
            s.control(Control::Start).unwrap();
        }
        quiescent(&s, 0);
        connect(&s);
        assert!(request(&s, Action::ReadFeatures).is_ok());
        request(&s, Action::Disconnect).unwrap();
        quiescent(&s, 0);
    }
}
#[test]
fn post_review_cancel_wait_timeout_never_releases_another_owner() {
    let s = connected();
    let read = input(&s, Action::ReadFeatures);
    let r = s.runtime().unwrap();
    let pause = r.test_hooks.arm(TestPoint::Request);
    std::thread::scope(|scope| {
        let worker = scope.spawn(|| s.request(read));
        pause.entered();
        assert_eq!(error(r.begin(true, Duration::ZERO)), ErrorCode::Timeout);
        assert!(r.busy.load(Ordering::Acquire));
        assert!(lock(&r.cancellation)
            .unwrap()
            .as_ref()
            .unwrap()
            .is_cancelled());
        assert_eq!(error(r.begin(false, Duration::ZERO)), ErrorCode::Busy);
        drop(pause);
        assert_eq!(error(worker.join().unwrap()), ErrorCode::Cancelled);
    });
    quiescent(&s, 0);
    connect(&s);
    request(&s, Action::Disconnect).unwrap();
    quiescent(&s, 0);
}
#[test]
fn post_review_virtual_expiry_boundary_revokes_reads_without_status_poll() {
    let s = connected();
    let r = s.runtime().unwrap();
    s.control(Control::Advance {
        milliseconds: 29_999,
    })
    .unwrap();
    assert!(request(&s, Action::ReadFeatures).is_ok());
    let read = input(&s, Action::ReadFeatures);
    // Runtime controls reap expiry themselves; no intervening status IPC is required.
    r.control(Control::Advance { milliseconds: 1 }).unwrap();
    assert_eq!(error(r.perform(read)), ErrorCode::Unauthorized);
    let status = s.status().unwrap();
    assert!(!status.read_authorized && status.features.is_none());
    quiescent(&s, 0);
    connect(&s);
    assert!(request(&s, Action::ReadFeatures).is_ok());
    request(&s, Action::Disconnect).unwrap();
    quiescent(&s, 0);
}
#[test]
fn post_review_wrong_identity_summary_reports_actual_configuration() {
    let s = service();
    scenario(&s, Scenario::WrongIdentity);
    assert_eq!(
        s.status().unwrap().device.unwrap().model,
        "Wrong Virtual Device"
    );
    quiescent(&s, 0);
}
#[test]
fn post_review_feature_mutations_require_their_own_read_grants() {
    for (operation, action) in [
        (
            AdapterOperation::ReadWifiLimit,
            Action::SetWifi {
                radio: "sim-radio-2g".into(),
                limit: 12,
            },
        ),
        (
            AdapterOperation::ReadApn,
            Action::SetApn {
                source: ConfigurationSource::Manual,
                apn: "own.lab".into(),
                authentication: ApnAuthentication::None,
                ip_family: IpFamily::Ipv4,
            },
        ),
        (
            AdapterOperation::ReadUssdCapability,
            Action::UssdStart {
                code: "*123#".into(),
            },
        ),
    ] {
        for authorized in [false, true] {
            let s = connected();
            let r = s.runtime().unwrap();
            let d = r.data().unwrap();
            let before = s.status().unwrap().revision;
            let mut access = lock(&r.access).unwrap();
            access.grant = Some(
                r.manager
                    .confirm_session_reads(
                        access.session.unwrap(),
                        &r.registry,
                        &d.ctx(&r.profiles),
                        vec![if authorized {
                            operation
                        } else {
                            AdapterOperation::ReadDns
                        }],
                    )
                    .unwrap(),
            );
            drop(access);
            // Recreate the action for each grant; serialized IPC is the source of truth.
            let action: Action = serde_json::from_value(match &action {
                Action::SetWifi { .. } => serde_json::json!({"kind":"set_wifi","radio":"sim-radio-2g","limit":12}),
                Action::SetApn { .. } => serde_json::json!({"kind":"set_apn","source":"manual","apn":"own.lab","authentication":"none","ipFamily":"ipv4"}),
                _ => serde_json::json!({"kind":"ussd_start","code":"*123#"}),
            }).unwrap();
            let result = request(&s, action);
            if authorized {
                result.unwrap();
            } else {
                assert_eq!(error(result), ErrorCode::Unauthorized);
                assert_eq!(lock(&r.state).unwrap().revision, before);
                assert!(lock(&r.state).unwrap().ussd.is_none());
            }
            request(&s, Action::Disconnect).unwrap();
            quiescent(&s, 0);
        }
    }
}
#[test]
fn post_review_rejections_timeout_and_operation_unwind_release_resources() {
    let s = connected();
    assert_eq!(
        error(s.control(Control::Advance {
            milliseconds: 300_001
        })),
        ErrorCode::InvalidInput
    );
    quiescent(&s, 1);
    let r = s.runtime().unwrap();
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = r.begin(false, Duration::ZERO).unwrap();
        panic!("controlled operation unwind");
    }))
    .is_err());
    quiescent(&s, 1);
    scenario(&s, Scenario::SlowRead);
    assert_eq!(error(request(&s, Action::ReadFeatures)), ErrorCode::Timeout);
    quiescent(&s, 0);
    connect(&s);
    request(&s, Action::Disconnect).unwrap();
    quiescent(&s, 0);
}
#[test]
fn post_review_simulation_ipc_cannot_select_production_physical_adapters() {
    let s = service();
    for endpoint in [
        "http://192.168.100.1/",
        "ssh://192.0.2.1",
        "serial://COM3",
        "usb://1286:4e31",
    ] {
        let mut forged = input(&s, Action::ApproveHandshake);
        forged.endpoint = endpoint.into();
        assert_eq!(error(s.request(forged)), ErrorCode::EndpointUntrusted);
        quiescent(&s, 0);
    }
    for kind in [
        "extract_firmware",
        "flash_firmware",
        "execute_ussd",
        "apply_configuration",
    ] {
        assert!(serde_json::from_value::<Action>(serde_json::json!({"kind":kind})).is_err());
    }
    let normal = SimulatorService::new(false, "production".into());
    assert_eq!(
        error(normal.request(input(&s, Action::ApproveHandshake))),
        ErrorCode::Disabled
    );
    quiescent(&s, 0);
}
#[test]
fn live_identity_and_endpoint_changes_expire_sessions_and_old_snapshots() {
    for (sc, code) in [
        (Scenario::WrongIdentity, ErrorCode::IdentityConflict),
        (Scenario::EndpointSubstitution, ErrorCode::EndpointUntrusted),
    ] {
        let s = connected();
        let previous = apply(&s);
        let old = previous.device.unwrap();
        let epoch = previous.epoch;
        let snapshot = previous.outcome.unwrap().snapshot_id.unwrap();
        scenario(&s, sc);
        let status = s.status().unwrap();
        assert!(status.epoch > epoch);
        assert!(!status.read_authorized);
        assert_eq!(status.active_connections, 0);
        assert!(status.features.is_none());
        assert!(status.preview.is_none());
        assert!(status.outcome.is_none());
        assert_eq!(
            error(s.request(Request {
                target: DEVICE_ID.into(),
                endpoint: old.endpoint,
                epoch,
                revision: previous.revision,
                action: Action::ReadFeatures
            })),
            ErrorCode::Stale
        );
        let p = request(&s, Action::ApproveHandshake)
            .unwrap()
            .approval_id
            .unwrap();
        assert_eq!(
            error(request(
                &s,
                Action::Connect {
                    approval_id: p,
                    credential: "lab-only".into()
                }
            )),
            code
        );
        scenario(&s, Scenario::Healthy);
        connect(&s);
        assert_eq!(
            error(request(
                &s,
                Action::Restore {
                    snapshot_id: snapshot
                }
            )),
            ErrorCode::Stale
        );
        assert_eq!(
            request(&s, Action::ReadFeatures)
                .unwrap()
                .features
                .unwrap()
                .dns
                .value
                .unwrap()
                .upstream_ipv4,
            ["9.9.9.9"]
        );
    }
}
#[test]
fn cellular_service_restart_recovers_only_after_scheduled_virtual_time() {
    let s = connected();
    scenario(&s, Scenario::CellularRestart);
    let outcome = apply(&s).outcome.unwrap();
    assert!(outcome.configuration_matches);
    assert!(outcome.service_healthy);
    assert!(!outcome.resolution_successful);
    assert!(!outcome.verified);
    let f = request(&s, Action::ReadFeatures).unwrap().features.unwrap();
    assert_eq!(f.modem.service, "restarting");
    assert!(!f.modem.data_connected);
    assert_eq!(f.apn.error.unwrap().code, ErrorCode::DataConnectionFailed);
    s.control(Control::Advance { milliseconds: 2999 }).unwrap();
    assert_eq!(
        lock(&s.runtime().unwrap().state)
            .unwrap()
            .configuration
            .modem
            .service,
        "restarting"
    );
    s.control(Control::Advance { milliseconds: 1 }).unwrap();
    connect(&s);
    let f = request(&s, Action::ReadFeatures).unwrap().features.unwrap();
    assert_eq!(f.modem.service, "running");
    assert!(f.modem.data_connected);
    assert_eq!(f.apn.value.unwrap().apn.as_deref(), Some("lab.example"));
    assert!(
        request(&s, Action::VerifyDns)
            .unwrap()
            .outcome
            .unwrap()
            .verified
    );
}
fn service() -> SimulatorService {
    let s = SimulatorService::new(true, "unit-isolated".into());
    s.control(Control::Start).unwrap();
    s
}
fn request(s: &SimulatorService, action: Action) -> Result<Status> {
    let status = s.status()?;
    s.request(Request {
        target: DEVICE_ID.into(),
        endpoint: status.device.unwrap().endpoint,
        epoch: status.epoch,
        revision: status.revision,
        action,
    })
}
fn connected() -> SimulatorService {
    let s = service();
    connect(&s);
    s
}
fn connect(s: &SimulatorService) {
    let p = request(s, Action::ApproveHandshake).unwrap();
    request(
        s,
        Action::Connect {
            approval_id: p.approval_id.unwrap(),
            credential: "lab-only".into(),
        },
    )
    .unwrap();
    request(s, Action::ApproveReads).unwrap();
}
fn scenario(s: &SimulatorService, scenario: Scenario) {
    s.control(Control::Scenario {
        scenario,
        repeat: false,
    })
    .unwrap();
}
fn proposal(ip: &str) -> DnsConfiguration {
    let mut d = model::dns_fixture();
    d.upstream_source_ipv4 = ConfigurationSource::Manual;
    d.upstream_ipv4 = vec![ip.into()];
    d
}
fn apply(s: &SimulatorService) -> Status {
    let p = request(
        s,
        Action::PreviewDns {
            configuration: proposal("9.9.9.9"),
        },
    )
    .unwrap();
    request(
        s,
        Action::ApplyDns {
            preview_id: p.preview.unwrap().id,
        },
    )
    .unwrap()
}
fn error<T>(r: Result<T>) -> ErrorCode {
    r.err().expect("must fail").code
}

#[test]
fn isolated_activation_and_cold_bootstrap_do_not_seed_authentication() {
    let denied = SimulatorService::new(false, "normal".into());
    assert_eq!(error(denied.control(Control::Start)), ErrorCode::Disabled);
    let s = service();
    let runtime = s.runtime().unwrap();
    let d = runtime.data().unwrap();
    assert!(d.report.evidence.is_empty());
    assert_eq!(d.report.confidence, profiles::MatchConfidence::NoMatch);
    assert!(d.device.networks.is_empty());
    assert!(d.device.interfaces.is_empty());
    assert_eq!(d.device.transports, ["simulation"]);
    assert!(!s.status().unwrap().read_authorized);
    assert_eq!(
        error(request(&s, Action::ReadFeatures)),
        ErrorCode::Unauthorized
    );
    connect(&s);
    let status = request(&s, Action::ReadFeatures).unwrap();
    assert!(status.read_authorized);
    assert_eq!(status.authentication, AuthenticationState::Authenticated);
    let f = status.features.unwrap();
    assert_eq!(f.dns.value.unwrap(), model::dns_fixture());
    assert_eq!(f.wifi.value.unwrap().len(), 2);
    assert_eq!(f.apn.value.unwrap().apn.as_deref(), Some("lab.example"));
    assert_eq!(
        f.ussd.value.unwrap().assessment,
        profiles::Assessment::Supported
    );
    assert!(runtime.data().unwrap().report.evidence.is_empty());
    assert_eq!(status.active_connections, 1);
    request(&s, Action::Disconnect).unwrap();
    assert_eq!(runtime.resources.load(Ordering::Acquire), 0);
}
#[test]
fn approval_replay_wrong_credentials_expiry_and_separate_read_gate() {
    let s = service();
    let p = request(&s, Action::ApproveHandshake)
        .unwrap()
        .approval_id
        .unwrap();
    assert_eq!(
        error(request(
            &s,
            Action::Connect {
                approval_id: p.clone(),
                credential: "wrong".into()
            }
        )),
        ErrorCode::AuthenticationFailed
    );
    assert_eq!(
        error(request(
            &s,
            Action::Connect {
                approval_id: p,
                credential: "lab-only".into()
            }
        )),
        ErrorCode::Unauthorized
    );
    let p = request(&s, Action::ApproveHandshake)
        .unwrap()
        .approval_id
        .unwrap();
    s.control(Control::Advance {
        milliseconds: 31_000,
    })
    .unwrap();
    assert_eq!(
        error(request(
            &s,
            Action::Connect {
                approval_id: p,
                credential: "lab-only".into()
            }
        )),
        ErrorCode::Expired
    );
    let p = request(&s, Action::ApproveHandshake)
        .unwrap()
        .approval_id
        .unwrap();
    request(
        &s,
        Action::Connect {
            approval_id: p,
            credential: "lab-only".into(),
        },
    )
    .unwrap();
    assert_eq!(
        error(request(&s, Action::ReadFeatures)),
        ErrorCode::Unauthorized
    );
    request(&s, Action::ApproveReads).unwrap();
    s.control(Control::Advance {
        milliseconds: 31_000,
    })
    .unwrap();
    assert_eq!(s.status().unwrap().active_connections, 0);
    assert_eq!(
        error(request(&s, Action::ReadFeatures)),
        ErrorCode::Unauthorized
    );
}
#[test]
fn physical_targets_provenance_and_malformed_ipc_are_rejected() {
    let s = service();
    let r = s.runtime().unwrap();
    let d = r.data().unwrap();
    let mut b = r.binding(&d);
    let real = discovery::replay_fixture(include_str!("../../fixtures/discovery-lifecycle.json"))
        .unwrap()[1]
        .devices[0]
        .clone();
    let report = profiles::evaluate(&r.profiles, &real, vec![], d.now, 1);
    let physical = DeviceContext {
        device: &real,
        profile: &report,
        profiles: &r.profiles,
        revision: 1,
        now: d.now,
    };
    b.device_id = real.id.clone();
    assert!(!r.registry.simulation_target(&physical, &b));
    let mut prod = Registry::production().unwrap();
    let adapter = SimulatorAdapter {
        state: r.state.clone(),
        authority: r.authority.clone(),
        resources: r.resources.clone(),
        test_hooks: r.test_hooks.clone(),
    };
    assert!(prod.register(adapter).is_err());
    assert!(s
        .request(Request {
            target: real.id,
            endpoint: r.authority.endpoint.clone(),
            epoch: 1,
            revision: 1,
            action: Action::ApproveHandshake
        })
        .is_err());
    assert!(s
        .request(Request {
            target: DEVICE_ID.into(),
            endpoint: "http://192.168.100.1/".into(),
            epoch: 1,
            revision: 1,
            action: Action::ApproveHandshake
        })
        .is_err());
    assert!(serde_json::from_value::<Request>(serde_json::json!({"target":DEVICE_ID,"endpoint":"fake","epoch":1,"revision":1,"simulated":true,"action":{"kind":"approve_handshake"}})).is_err());
    assert!(serde_json::from_value::<Control>(
        serde_json::json!({"kind":"execute","command":"AT+CUSD"})
    )
    .is_err());
    connect(&s);
    let d = r.data().unwrap();
    let a = lock(&r.access).unwrap();
    assert!(r
        .manager
        .confirm_session_reads(
            a.session.unwrap(),
            &r.registry,
            &d.ctx(&r.profiles),
            vec![AdapterOperation::ModifyConfiguration]
        )
        .is_err());
}
#[test]
fn endpoint_and_identity_faults_never_publish_connected_state() {
    for (sc, code) in [
        (
            Scenario::AuthenticationFailure,
            ErrorCode::AuthenticationFailed,
        ),
        (Scenario::WrongIdentity, ErrorCode::IdentityConflict),
        (Scenario::EndpointSubstitution, ErrorCode::EndpointUntrusted),
    ] {
        let s = service();
        scenario(&s, sc);
        let p = request(&s, Action::ApproveHandshake)
            .unwrap()
            .approval_id
            .unwrap();
        assert_eq!(
            error(request(
                &s,
                Action::Connect {
                    approval_id: p,
                    credential: "lab-only".into()
                }
            )),
            code
        );
        assert_eq!(s.runtime().unwrap().resources.load(Ordering::Acquire), 0);
        assert_eq!(s.status().unwrap().active_connections, 0);
    }
}
#[test]
fn connection_and_authentication_delays_are_bounded() {
    for sc in [Scenario::ConnectionTimeout, Scenario::SlowAuthentication] {
        let s = service();
        scenario(&s, sc);
        let p = request(&s, Action::ApproveHandshake)
            .unwrap()
            .approval_id
            .unwrap();
        assert_eq!(
            error(request(
                &s,
                Action::Connect {
                    approval_id: p,
                    credential: "lab-only".into()
                }
            )),
            ErrorCode::Timeout
        );
        assert_eq!(s.runtime().unwrap().resources.load(Ordering::Acquire), 0);
    }
}
#[test]
fn read_timeout_cancel_and_reset_prevent_stale_success() {
    let s = connected();
    scenario(&s, Scenario::SlowRead);
    assert_eq!(error(request(&s, Action::ReadFeatures)), ErrorCode::Timeout);
    for reset in [false, true] {
        let s = connected();
        scenario(&s, Scenario::SlowRead);
        let gate = Arc::new(Barrier::new(2));
        std::thread::scope(|scope| {
            let g = gate.clone();
            let service_ref = &s;
            let worker = scope.spawn(move || {
                g.wait();
                request(service_ref, Action::ReadFeatures)
            });
            gate.wait();
            let r = s.runtime().unwrap();
            let deadline = std::time::Instant::now() + Duration::from_millis(300);
            while !r.busy.load(Ordering::Acquire) {
                assert!(std::time::Instant::now() < deadline);
                std::thread::yield_now();
            }
            s.control(if reset {
                Control::Reset
            } else {
                Control::Cancel
            })
            .unwrap();
            assert!(matches!(
                error(worker.join().unwrap()),
                ErrorCode::Cancelled | ErrorCode::Stale
            ));
        });
        assert_eq!(s.runtime().unwrap().resources.load(Ordering::Acquire), 0);
    }
}
#[test]
fn stop_start_and_reset_expire_old_runtime_requests() {
    let s = connected();
    let old = s.status().unwrap();
    let old_endpoint = old.device.unwrap().endpoint;
    let r = s.runtime().unwrap();
    s.control(Control::Stop).unwrap();
    assert_eq!(r.resources.load(Ordering::Acquire), 0);
    s.control(Control::Start).unwrap();
    assert_eq!(
        error(s.request(Request {
            target: DEVICE_ID.into(),
            endpoint: old_endpoint,
            epoch: old.epoch,
            revision: old.revision,
            action: Action::ApproveHandshake
        })),
        ErrorCode::EndpointUntrusted
    );
    connect(&s);
    s.control(Control::Reset).unwrap();
    assert_eq!(
        error(request(&s, Action::ReadFeatures)),
        ErrorCode::Unauthorized
    );
    connect(&s);
    assert_eq!(
        request(&s, Action::ReadFeatures)
            .unwrap()
            .features
            .unwrap()
            .dns
            .value
            .unwrap(),
        model::dns_fixture()
    );
}
#[test]
fn dns_validation_ip_families_empty_duplicate_bounds_and_automatic_sources() {
    let mut candidates = vec![
        proposal("bad"),
        proposal("::1"),
        proposal("0.0.0.0"),
        proposal("224.0.0.1"),
    ];
    let mut empty = proposal("9.9.9.9");
    empty.upstream_ipv4.clear();
    candidates.push(empty);
    let mut duplicate = proposal("9.9.9.9");
    duplicate.upstream_ipv4.push("9.9.9.9".into());
    candidates.push(duplicate);
    let mut long = proposal("9.9.9.9");
    long.upstream_ipv4 = vec!["9.9.9.9".into(); 5];
    candidates.push(long);
    let mut wrong_v6 = proposal("9.9.9.9");
    wrong_v6.upstream_source_ipv6 = ConfigurationSource::Manual;
    wrong_v6.upstream_ipv6 = vec!["1.1.1.1".into()];
    candidates.push(wrong_v6);
    for c in candidates {
        assert!(model::validate_dns(c).is_err());
    }
    let mut d = proposal("9.9.9.9");
    d.upstream_source_ipv4 = ConfigurationSource::Automatic;
    assert_eq!(model::validate_dns(d).unwrap(), model::dns_fixture());
    let mut d = proposal("9.9.9.9");
    d.upstream_source_ipv6 = ConfigurationSource::Manual;
    d.upstream_ipv6 = vec!["2001:db8:0:0::53".into()];
    assert_eq!(
        model::validate_dns(d).unwrap().upstream_ipv6,
        ["2001:db8::53"]
    );
}
#[test]
fn complete_dns_preview_snapshot_apply_readback_verify_restore() {
    let s = connected();
    let previous = request(&s, Action::ReadFeatures)
        .unwrap()
        .features
        .unwrap()
        .dns
        .value
        .unwrap();
    let p = request(
        &s,
        Action::PreviewDns {
            configuration: proposal("9.9.9.9"),
        },
    )
    .unwrap()
    .preview
    .unwrap();
    assert_eq!(p.previous, previous);
    assert_eq!(p.proposed.upstream_ipv4, ["9.9.9.9"]);
    assert_eq!(p.changes.len(), 2);
    let applied = request(
        &s,
        Action::ApplyDns {
            preview_id: p.id.clone(),
        },
    )
    .unwrap();
    let outcome = applied.outcome.unwrap();
    assert!(outcome.verified);
    assert_eq!(applied.revision, 2);
    assert_eq!(applied.snapshots.len(), 1);
    let current = request(&s, Action::ReadFeatures).unwrap();
    assert_eq!(
        current.features.unwrap().dns.value.unwrap().upstream_ipv4,
        ["9.9.9.9"]
    );
    assert!(
        request(&s, Action::VerifyDns)
            .unwrap()
            .outcome
            .unwrap()
            .verified
    );
    let restored = request(
        &s,
        Action::Restore {
            snapshot_id: outcome.snapshot_id.unwrap(),
        },
    )
    .unwrap();
    assert_eq!(restored.outcome.unwrap().rollback, Rollback::Succeeded);
    assert_eq!(
        request(&s, Action::ReadFeatures)
            .unwrap()
            .features
            .unwrap()
            .dns
            .value
            .unwrap(),
        previous
    );
}
#[test]
fn cancelled_draft_stale_revision_wrong_snapshot_and_replay_deny() {
    let s = connected();
    let p = request(
        &s,
        Action::PreviewDns {
            configuration: proposal("9.9.9.9"),
        },
    )
    .unwrap()
    .preview
    .unwrap();
    request(&s, Action::CancelDraft).unwrap();
    assert_eq!(
        error(request(&s, Action::ApplyDns { preview_id: p.id })),
        ErrorCode::Stale
    );
    let p = request(
        &s,
        Action::PreviewDns {
            configuration: proposal("9.9.9.9"),
        },
    )
    .unwrap()
    .preview
    .unwrap();
    scenario(&s, Scenario::ExternalRevision);
    assert_eq!(
        error(request(&s, Action::ApplyDns { preview_id: p.id })),
        ErrorCode::Stale
    );
    assert_eq!(
        error(request(
            &s,
            Action::Restore {
                snapshot_id: "physical-or-other-snapshot".into()
            }
        )),
        ErrorCode::Stale
    );
    s.control(Control::Reset).unwrap();
    connect(&s);
    let result = apply(&s);
    let p = result.preview.unwrap();
    assert_eq!(
        error(request(&s, Action::ApplyDns { preview_id: p.id })),
        ErrorCode::Stale
    );
}
#[test]
fn dns_apply_failure_does_not_mutate_configuration() {
    let s = connected();
    scenario(&s, Scenario::DnsApplyFailure);
    let result = apply(&s).outcome.unwrap();
    assert!(!result.applied);
    assert!(!result.verified);
    assert_eq!(
        request(&s, Action::ReadFeatures)
            .unwrap()
            .features
            .unwrap()
            .dns
            .value
            .unwrap(),
        model::dns_fixture()
    );
    assert_eq!(
        request(
            &s,
            Action::Restore {
                snapshot_id: result.snapshot_id.unwrap()
            }
        )
        .unwrap()
        .outcome
        .unwrap()
        .rollback,
        Rollback::Succeeded
    );
}
#[test]
fn verification_service_resolution_and_rollback_failures_are_distinct() {
    for sc in [
        Scenario::DnsVerificationMismatch,
        Scenario::DnsServiceFailure,
        Scenario::ResolutionFailure,
        Scenario::RollbackFailure,
    ] {
        let s = connected();
        scenario(&s, sc);
        let o = apply(&s).outcome.unwrap();
        assert!(o.applied);
        assert!(!o.verified);
        if sc == Scenario::DnsServiceFailure {
            assert!(o.configuration_matches);
            assert!(!o.service_healthy);
        }
        if sc == Scenario::ResolutionFailure {
            assert!(o.configuration_matches);
            assert!(o.service_healthy);
            assert!(!o.resolution_successful);
        }
        let restored = request(
            &s,
            Action::Restore {
                snapshot_id: o.snapshot_id.unwrap(),
            },
        )
        .unwrap()
        .outcome
        .unwrap();
        if sc == Scenario::RollbackFailure {
            assert_eq!(restored.rollback, Rollback::Failed);
            assert!(restored.requires_reset);
            assert_eq!(
                error(request(
                    &s,
                    Action::ApplyDns {
                        preview_id: request(
                            &s,
                            Action::PreviewDns {
                                configuration: proposal("1.0.0.1")
                            }
                        )
                        .unwrap()
                        .preview
                        .unwrap()
                        .id
                    }
                )),
                ErrorCode::RecoveryRequired
            );
        } else {
            assert_eq!(restored.rollback, Rollback::Succeeded);
        }
    }
}
#[test]
fn fault_trigger_counts_repeat_and_scenario_reset_are_deterministic() {
    let s = connected();
    scenario(&s, Scenario::DnsApplyFailure);
    assert!(!apply(&s).outcome.unwrap().applied);
    assert!(apply(&s).outcome.unwrap().verified);
    s.control(Control::Scenario {
        scenario: Scenario::DnsApplyFailure,
        repeat: true,
    })
    .unwrap();
    assert!(!apply(&s).outcome.unwrap().applied);
    assert!(!apply(&s).outcome.unwrap().applied);
    s.control(Control::Reset).unwrap();
    connect(&s);
    assert!(apply(&s).outcome.unwrap().verified);
}
#[test]
fn wifi_multibss_ranges_associated_clients_and_capacity_are_modeled() {
    let s = connected();
    let f = request(&s, Action::ReadFeatures)
        .unwrap()
        .features
        .unwrap()
        .wifi
        .value
        .unwrap();
    assert_eq!(f.len(), 2);
    assert_eq!(f[0].connected_clients, Some(3));
    for (radio, limit) in [
        ("unknown", 8),
        ("sim-radio-2g", 0),
        ("sim-radio-2g", 33),
        ("sim-radio-2g", 2),
    ] {
        assert!(request(
            &s,
            Action::SetWifi {
                radio: radio.into(),
                limit
            }
        )
        .is_err());
    }
    request(
        &s,
        Action::SetWifi {
            radio: "sim-radio-2g".into(),
            limit: 12,
        },
    )
    .unwrap();
    assert_eq!(
        request(&s, Action::ReadFeatures)
            .unwrap()
            .features
            .unwrap()
            .wifi
            .value
            .unwrap()[0]
            .station_limit,
        Some(12)
    );
    scenario(&s, Scenario::WifiCapacity);
    assert_eq!(
        error(request(
            &s,
            Action::SetWifi {
                radio: "sim-radio-2g".into(),
                limit: 10
            }
        )),
        ErrorCode::InvalidInput
    );
}
#[test]
fn wifi_reload_disconnect_requires_fresh_authorization_after_virtual_recovery() {
    let s = connected();
    scenario(&s, Scenario::WifiReloadInterruption);
    assert_eq!(
        error(request(
            &s,
            Action::SetWifi {
                radio: "sim-radio-2g".into(),
                limit: 12
            }
        )),
        ErrorCode::Offline
    );
    assert!(request(&s, Action::ReadFeatures).is_err());
    s.control(Control::Advance { milliseconds: 5000 }).unwrap();
    assert_eq!(
        error(request(&s, Action::ReadFeatures)),
        ErrorCode::Unauthorized
    );
    connect(&s);
    assert!(request(&s, Action::ReadFeatures).is_ok());
}
#[test]
fn apn_operator_lookup_manual_authentication_and_ip_family_validation() {
    let s = connected();
    request(
        &s,
        Action::SetApn {
            source: ConfigurationSource::Manual,
            apn: "manual.lab".into(),
            authentication: ApnAuthentication::Chap,
            ip_family: IpFamily::Ipv4,
        },
    )
    .unwrap();
    let apn = request(&s, Action::ReadFeatures)
        .unwrap()
        .features
        .unwrap()
        .apn
        .value
        .unwrap();
    assert_eq!(apn.apn.as_deref(), Some("manual.lab"));
    assert!(apn.authentication_required);
    request(
        &s,
        Action::SetApn {
            source: ConfigurationSource::Automatic,
            apn: String::new(),
            authentication: ApnAuthentication::None,
            ip_family: IpFamily::DualStack,
        },
    )
    .unwrap();
    assert_eq!(
        request(&s, Action::ReadFeatures)
            .unwrap()
            .features
            .unwrap()
            .apn
            .value
            .unwrap()
            .apn
            .as_deref(),
        Some("lab.example")
    );
    assert!(request(
        &s,
        Action::SetApn {
            source: ConfigurationSource::Manual,
            apn: "bad APN;command".into(),
            authentication: ApnAuthentication::None,
            ip_family: IpFamily::Ipv4
        }
    )
    .is_err());
    scenario(&s, Scenario::Ipv4Only);
    assert_eq!(
        error(request(
            &s,
            Action::SetApn {
                source: ConfigurationSource::Manual,
                apn: "lab".into(),
                authentication: ApnAuthentication::None,
                ip_family: IpFamily::Ipv6
            }
        )),
        ErrorCode::Unsupported
    );
}
#[test]
fn modem_failure_scenarios_return_real_errors() {
    for (sc, code) in [
        (Scenario::SimMissing, ErrorCode::SimMissing),
        (Scenario::SimPinRequired, ErrorCode::SimPinRequired),
        (Scenario::UnknownOperator, ErrorCode::UnknownOperator),
        (Scenario::MissingApn, ErrorCode::MissingApn),
        (Scenario::ModemAbsent, ErrorCode::ModemAbsent),
        (Scenario::RegistrationFailure, ErrorCode::RegistrationFailed),
        (
            Scenario::DataConnectionFailure,
            ErrorCode::DataConnectionFailed,
        ),
    ] {
        let s = connected();
        scenario(&s, sc);
        let apn = request(&s, Action::ReadFeatures)
            .unwrap()
            .features
            .unwrap()
            .apn;
        assert_eq!(apn.error.unwrap().code, code);
        assert!(apn.value.is_none());
    }
}
#[test]
fn ussd_scripts_followup_busy_cancellation_and_timeout_use_virtual_time_only() {
    let s = connected();
    let first = request(
        &s,
        Action::UssdStart {
            code: "*123#".into(),
        },
    )
    .unwrap()
    .ussd
    .unwrap();
    assert_eq!(first.state, "active");
    assert_eq!(
        error(request(
            &s,
            Action::UssdStart {
                code: "*100#".into()
            }
        )),
        ErrorCode::Busy
    );
    let complete = request(
        &s,
        Action::UssdReply {
            session_id: first.id,
            text: "1".into(),
        },
    )
    .unwrap()
    .ussd
    .unwrap();
    assert!(complete.text.contains("fictional balance"));
    assert_eq!(complete.state, "completed");
    let one = request(
        &s,
        Action::UssdStart {
            code: "*100#".into(),
        },
    )
    .unwrap()
    .ussd
    .unwrap();
    assert_eq!(one.state, "completed");
    let active = request(
        &s,
        Action::UssdStart {
            code: "*123#".into(),
        },
    )
    .unwrap()
    .ussd
    .unwrap();
    assert_eq!(
        request(
            &s,
            Action::UssdCancel {
                session_id: active.id
            }
        )
        .unwrap()
        .ussd
        .unwrap()
        .state,
        "cancelled"
    );
    scenario(&s, Scenario::UssdTimeout);
    request(
        &s,
        Action::UssdStart {
            code: "*123#".into(),
        },
    )
    .unwrap();
    assert_eq!(
        s.control(Control::Advance { milliseconds: 1000 })
            .unwrap()
            .ussd
            .unwrap()
            .state,
        "timed_out"
    );
    assert!(request(
        &s,
        Action::UssdStart {
            code: "AT+CUSD".into()
        }
    )
    .is_err());
}
#[test]
fn ussd_unsupported_modem_and_network_failure_are_truthful() {
    let s = connected();
    scenario(&s, Scenario::UnsupportedModem);
    assert_eq!(
        request(&s, Action::ReadFeatures)
            .unwrap()
            .features
            .unwrap()
            .ussd
            .value
            .unwrap()
            .assessment,
        profiles::Assessment::Unsupported
    );
    assert_eq!(
        error(request(
            &s,
            Action::UssdStart {
                code: "*123#".into()
            }
        )),
        ErrorCode::Unsupported
    );
    scenario(&s, Scenario::UssdNetworkFailure);
    assert_eq!(
        request(
            &s,
            Action::UssdStart {
                code: "*123#".into()
            }
        )
        .unwrap()
        .ussd
        .unwrap()
        .state,
        "failed"
    );
}
