use super::types::*;
use crate::adapters::*;
use std::{
    collections::BTreeSet,
    net::{Ipv4Addr, Ipv6Addr},
};

#[derive(Clone)]
pub(super) struct Configuration {
    pub identity: AdapterIdentity,
    pub dns: DnsConfiguration,
    pub wifi: Vec<WifiClientLimitConfiguration>,
    pub apn: ApnConfiguration,
    pub modem: Modem,
    pub dns_service: String,
    pub wifi_service: String,
    pub resolver_ok: bool,
}
impl Configuration {
    pub fn resolves(&self) -> bool {
        self.dns_service == "running"
            && self.resolver_ok
            && self.modem.present
            && self.modem.sim_present
            && !self.modem.pin_required
            && self.modem.registered
            && self.modem.data_connected
    }
}
#[derive(Clone)]
pub(super) struct Snapshot {
    pub info: SnapshotInfo,
    pub configuration: Configuration,
    pub epoch: u64,
}
pub(super) struct Router {
    pub configuration: Configuration,
    pub revision: u64,
    pub clock: u64,
    pub available: bool,
    pub scenario: Scenario,
    pub faults: Vec<FaultPlan>,
    pub snapshots: Vec<Snapshot>,
    pub preview: Option<DnsPreview>,
    pub outcome: Option<DnsOutcome>,
    pub features: Option<Features>,
    pub ussd: Option<UssdSession>,
    pub events: Vec<Event>,
    pub sequence: u64,
    pub recovery_required: bool,
    pub reconnect_at: Option<u64>,
    pub cellular_reconnect_at: Option<u64>,
    pub ussd_requests: Vec<u64>,
}
pub(super) fn dns_fixture() -> DnsConfiguration {
    DnsConfiguration {
        upstream_ipv4: vec!["1.1.1.1".into(), "8.8.8.8".into()],
        upstream_ipv6: vec!["2606:4700:4700::1111".into()],
        lan_ipv4: vec!["192.0.2.1".into()],
        lan_ipv6: vec!["2001:db8::1".into()],
        upstream_source_ipv4: ConfigurationSource::Automatic,
        upstream_source_ipv6: ConfigurationSource::Automatic,
        lan_source_ipv4: ConfigurationSource::Automatic,
        lan_source_ipv6: ConfigurationSource::Automatic,
        observed_by: "SIMULATION: virtual DNS adapter".into(),
    }
}
pub(super) fn identity_fixture() -> AdapterIdentity {
    AdapterIdentity {
        manufacturer: "OBTAINER".into(),
        model: "Virtual Router Lab".into(),
        hardware_revision: "SIM-1".into(),
        firmware_version: "SIM-0.1".into(),
        firmware_family: "Simulated OpenWrt-like".into(),
        management_identity: "obtainer,virtual-router-lab".into(),
        operating_system: "Simulated Embedded Linux".into(),
        modem_identity: Some("OBTAINER Virtual Modem".into()),
        simulated: true,
    }
}
pub(super) fn modem_fixture() -> Modem {
    Modem {
        present: true,
        sim_present: true,
        pin_required: false,
        registered: true,
        operator: Some("SIM-001".into()),
        data_connected: true,
        ipv6_supported: true,
        service: "running".into(),
    }
}
impl Router {
    pub fn new(clock: u64) -> Self {
        Self {
            configuration: Configuration {
                identity: identity_fixture(),
                dns: dns_fixture(),
                wifi: [
                    ("sim-radio-2g", "OBTAINER-LAB", 8, 32, 3),
                    ("sim-radio-5g", "OBTAINER-LAB-5G", 16, 64, 5),
                ]
                .into_iter()
                .map(
                    |(radio, ssid, limit, max, clients)| WifiClientLimitConfiguration {
                        radio: radio.into(),
                        ssid: Some(ssid.into()),
                        bss_id: Some(format!("{radio}-bss")),
                        station_limit: Some(limit),
                        verified_minimum: Some(1),
                        verified_maximum: Some(max),
                        connected_clients: Some(clients),
                        observed_by: "SIMULATION: virtual wireless driver".into(),
                    },
                )
                .collect(),
                apn: ApnConfiguration {
                    source: ConfigurationSource::Automatic,
                    apn: Some("lab.example".into()),
                    operator: Some("SIM-001".into()),
                    authentication_required: false,
                    ip_family: IpFamily::DualStack,
                    authentication_method: ApnAuthentication::None,
                    observed_by: "SIMULATION: operator fixture SIM-001".into(),
                },
                modem: modem_fixture(),
                dns_service: "running".into(),
                wifi_service: "running".into(),
                resolver_ok: true,
            },
            revision: 1,
            clock,
            available: true,
            scenario: Scenario::Healthy,
            faults: vec![],
            snapshots: vec![],
            preview: None,
            outcome: None,
            features: None,
            ussd: None,
            events: vec![],
            sequence: 0,
            recovery_required: false,
            reconnect_at: None,
            cellular_reconnect_at: None,
            ussd_requests: vec![],
        }
    }
    pub fn id(&mut self, kind: &str) -> String {
        self.sequence += 1;
        format!("sim-{kind}-{}-{}", self.clock, self.sequence)
    }
    pub fn event(&mut self, message: &str) {
        self.events.push(Event {
            time: self.clock,
            message: message.into(),
        });
        if self.events.len() > 100 {
            self.events.remove(0);
        }
    }
    pub fn fault(&mut self, stage: FaultStage) -> bool {
        if let Some(f) = self.faults.iter_mut().find(|f| f.stage == stage) {
            if f.repeat {
                true
            } else if f.remaining > 0 {
                f.remaining -= 1;
                true
            } else {
                false
            }
        } else {
            false
        }
    }
    pub fn scenario(&mut self, s: Scenario, repeat: bool) {
        self.scenario = s;
        self.faults.clear();
        self.features = None;
        let stages: &[FaultStage] = match s {
            Scenario::ConnectionTimeout => &[FaultStage::Connect],
            Scenario::SlowAuthentication => &[FaultStage::Authenticate],
            Scenario::SlowRead => &[FaultStage::Read],
            Scenario::DnsApplyFailure => &[FaultStage::Apply],
            Scenario::DnsVerificationMismatch => &[FaultStage::Verify],
            Scenario::DnsServiceFailure => &[FaultStage::Apply],
            Scenario::ResolutionFailure => &[FaultStage::Resolve],
            Scenario::RollbackFailure => &[FaultStage::Verify, FaultStage::Restore],
            Scenario::WifiReloadInterruption => &[FaultStage::Wifi],
            Scenario::UssdTimeout | Scenario::UssdNetworkFailure => &[FaultStage::Ussd],
            _ => &[],
        };
        self.faults = stages
            .iter()
            .map(|s| FaultPlan {
                stage: *s,
                remaining: 1,
                repeat,
            })
            .collect();
        self.configuration.modem = modem_fixture();
        self.configuration.identity = identity_fixture();
        if s == Scenario::WrongIdentity {
            self.configuration.identity.model = "Wrong Virtual Device".into();
        }
        self.cellular_reconnect_at = None;
        let m = &mut self.configuration.modem;
        match s {
            Scenario::SimMissing => m.sim_present = false,
            Scenario::SimPinRequired => m.pin_required = true,
            Scenario::UnknownOperator => m.operator = Some("SIM-UNKNOWN".into()),
            Scenario::MissingApn => m.operator = Some("SIM-NO-APN".into()),
            Scenario::ModemAbsent => m.present = false,
            Scenario::RegistrationFailure => m.registered = false,
            Scenario::DataConnectionFailure => m.data_connected = false,
            Scenario::CellularRestart => {
                m.service = "restarting".into();
                m.data_connected = false;
                self.cellular_reconnect_at = Some(self.clock + 3000);
            }
            Scenario::Ipv4Only => m.ipv6_supported = false,
            Scenario::RouterDisconnected => self.available = false,
            Scenario::ExternalRevision => {
                self.revision += 1;
                self.configuration.dns.upstream_ipv4 = vec!["9.9.9.9".into()];
            }
            Scenario::WifiCapacity => self.configuration.wifi[0].connected_clients = Some(20),
            _ => {}
        }
        if !m.present || !m.sim_present || m.pin_required || !m.registered {
            m.data_connected = false;
        }
        self.event("Simulation scenario selected; no physical action.");
    }
    pub fn modem_error(&self) -> Option<SimError> {
        let m = &self.configuration.modem;
        let (code, text) = if m.service == "restarting" {
            (
                ErrorCode::DataConnectionFailed,
                "Virtual cellular service restarting; advance 3000 ms, then read again.",
            )
        } else if !m.present {
            (ErrorCode::ModemAbsent, "Virtual modem absent.")
        } else if !m.sim_present {
            (ErrorCode::SimMissing, "Virtual SIM missing.")
        } else if m.pin_required {
            (
                ErrorCode::SimPinRequired,
                "Virtual SIM requires PIN; simulator does not unlock SIMs.",
            )
        } else if !m.registered {
            (
                ErrorCode::RegistrationFailed,
                "Virtual cellular registration failed.",
            )
        } else if m.operator.as_deref() == Some("SIM-UNKNOWN") {
            (
                ErrorCode::UnknownOperator,
                "Synthetic operator has no mapping.",
            )
        } else if m.operator.as_deref() == Some("SIM-NO-APN") {
            (
                ErrorCode::MissingApn,
                "Synthetic operator has no APN definition.",
            )
        } else if !m.data_connected {
            (
                ErrorCode::DataConnectionFailed,
                "Virtual data session failed.",
            )
        } else {
            return None;
        };
        Some(fail(code, text))
    }
}
pub(super) fn validate_dns(mut d: DnsConfiguration) -> Result<DnsConfiguration> {
    let defaults = dns_fixture();
    let sets = [
        (
            &mut d.upstream_ipv4,
            &d.upstream_source_ipv4,
            false,
            defaults.upstream_ipv4,
        ),
        (
            &mut d.upstream_ipv6,
            &d.upstream_source_ipv6,
            true,
            defaults.upstream_ipv6,
        ),
        (
            &mut d.lan_ipv4,
            &d.lan_source_ipv4,
            false,
            defaults.lan_ipv4,
        ),
        (&mut d.lan_ipv6, &d.lan_source_ipv6, true, defaults.lan_ipv6),
    ];
    for (values, source, v6, automatic) in sets {
        if *source == ConfigurationSource::Unknown {
            return Err(fail(
                ErrorCode::Unsupported,
                "Unknown DNS source cannot be applied.",
            ));
        }
        if values.len() > 4 {
            return Err(fail(
                ErrorCode::InvalidInput,
                "At most four resolvers per DNS list.",
            ));
        }
        if *source == ConfigurationSource::Automatic {
            *values = automatic;
            continue;
        }
        if values.is_empty() {
            return Err(fail(
                ErrorCode::InvalidInput,
                "Manual DNS requires at least one resolver per selected family.",
            ));
        }
        let mut seen = BTreeSet::new();
        for value in values.iter_mut() {
            let normalized = if v6 {
                value
                    .parse::<Ipv6Addr>()
                    .ok()
                    .filter(|a| !a.is_unspecified() && !a.is_multicast() && !a.is_loopback())
                    .map(|a| a.to_string())
            } else {
                value
                    .parse::<Ipv4Addr>()
                    .ok()
                    .filter(|a| {
                        !a.is_unspecified()
                            && !a.is_multicast()
                            && !a.is_loopback()
                            && *a != Ipv4Addr::BROADCAST
                    })
                    .map(|a| a.to_string())
            }
            .ok_or_else(|| {
                fail(
                    ErrorCode::InvalidInput,
                    "Invalid DNS address or wrong IP family.",
                )
            })?;
            if !seen.insert(normalized.clone()) {
                return Err(fail(ErrorCode::InvalidInput, "Duplicate DNS resolver."));
            }
            *value = normalized;
        }
    }
    d.observed_by = "SIMULATION: virtual DNS adapter".into();
    Ok(d)
}
