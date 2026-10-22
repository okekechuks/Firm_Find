use crate::adapters::{AdapterError, AdapterErrorCode};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

/// No raw command variant. Configuration values are selected on the router before transmission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadQuery {
    Board,
    Network,
    RuntimeDns,
    DhcpDns,
    Wireless,
    WirelessRadios,
    WirelessRuntime,
    Partitions,
}

pub const INSPECTION_QUERIES: [ReadQuery; 9] = [
    ReadQuery::Board,
    ReadQuery::Network,
    ReadQuery::Wireless,
    ReadQuery::RuntimeDns,
    ReadQuery::DhcpDns,
    ReadQuery::WirelessRadios,
    ReadQuery::WirelessRuntime,
    ReadQuery::Partitions,
    ReadQuery::Board,
];
pub const BATCH_LIMIT: usize = 512 * 1024;

/// Fixed allowlisted commands only. Byte framing keeps one query's data out of another.
pub fn inspection_command() -> String {
    let mut script = String::from("LC_ALL=C; export LC_ALL; ");
    for (index, query) in INSPECTION_QUERIES.iter().enumerate() {
        script.push_str(&format!("value=$({}); code=$?; printf 'OBTAINER1 {index} %s %s\\n' \"$code\" \"${{#value}}\"; printf '%s' \"$value\"; unset value; ", query.command()));
    }
    script
}

pub type InspectionReplies = Vec<Result<Vec<u8>, AdapterError>>;
pub fn inspection_replies(mut bytes: &[u8]) -> Result<InspectionReplies, AdapterError> {
    if bytes.len() > BATCH_LIMIT {
        return Err(invalid());
    }
    let mut replies = Vec::new();
    for (index, query) in INSPECTION_QUERIES.iter().enumerate() {
        let end = bytes
            .iter()
            .position(|b| *b == b'\n')
            .filter(|n| *n <= 80)
            .ok_or_else(invalid)?;
        let header = std::str::from_utf8(&bytes[..end]).map_err(|_| invalid())?;
        let parts: Vec<_> = header.split(' ').collect();
        if parts.len() != 4 || parts[0] != "OBTAINER1" || parts[1] != index.to_string() {
            return Err(invalid());
        }
        let code: u8 = parts[2].parse().map_err(|_| invalid())?;
        let size: usize = parts[3].parse().map_err(|_| invalid())?;
        bytes = &bytes[end + 1..];
        if size > query.output_limit() || size > bytes.len() {
            return Err(invalid());
        }
        let (data, rest) = bytes.split_at(size);
        bytes = rest;
        replies.push(if code == 0 {
            Ok(data.to_vec())
        } else {
            Err(AdapterError::new(
                AdapterErrorCode::TransportError,
                "Allowlisted metadata query unavailable.",
                None,
                true,
                "Review inspection limitations.",
            ))
        });
    }
    if !bytes.is_empty() {
        return Err(invalid());
    }
    Ok(replies)
}

#[cfg(test)]
mod batch_tests {
    use super::*;
    fn frame() -> Vec<u8> {
        let mut bytes = Vec::new();
        for index in 0..INSPECTION_QUERIES.len() {
            bytes.extend_from_slice(format!("OBTAINER1 {index} 0 2\n[]").as_bytes());
        }
        bytes
    }
    #[test]
    fn single_batch_has_fixed_queries_and_strict_independent_frames() {
        let replies = inspection_replies(&frame()).unwrap();
        assert_eq!(replies.len(), 9);
        assert!(replies.iter().all(|r| r.as_ref().unwrap() == b"[]"));
        let script = inspection_command();
        assert_eq!(script.matches("value=$(").count(), 9);
        assert!(script.contains("LC_ALL=C"));
        for query in INSPECTION_QUERIES {
            assert!(script.contains(query.command()));
        }
        for bad in [
            b"garbage".to_vec(),
            frame()[..20].to_vec(),
            [frame(), b"extra".to_vec()].concat(),
            String::from_utf8(frame())
                .unwrap()
                .replacen("0 0 2", "1 0 2", 1)
                .into_bytes(),
            String::from_utf8(frame())
                .unwrap()
                .replacen("0 0 2", "0 0 999999", 1)
                .into_bytes(),
        ] {
            assert!(inspection_replies(&bad).is_err());
        }
    }
    #[test]
    fn command_permission_failure_is_separate_from_ssh_login() {
        let bytes = String::from_utf8(frame())
            .unwrap()
            .replacen("4 0 2", "4 1 2", 1);
        let replies = inspection_replies(bytes.as_bytes()).unwrap();
        assert_eq!(
            replies[4].as_ref().unwrap_err().code,
            AdapterErrorCode::TransportError
        );
        assert!(replies[3].is_ok());
        assert!(replies[5].is_ok());
    }
}

impl ReadQuery {
    pub fn command(self) -> &'static str {
        match self {
            Self::Board => "/bin/ubus call system board",
            Self::Network => {
                r#"/usr/bin/ucode -e 'import { cursor } from "uci"; let c = cursor(); let out = []; c.foreach("network", "interface", function(s) { push(out, {name:s[".name"],proto:s.proto,dns:s.dns,peerdns:s.peerdns,apn:s.apn,auto_apn:s.auto_apn,pdptype:s.pdptype,auth:s.auth}); }); print(sprintf("%J\n",out));'"#
            }
            Self::RuntimeDns => {
                r#"/usr/bin/ucode -e 'import { connect } from "ubus"; let c=connect(null,2); if(!c) exit(1); let d=c.call("network.interface","dump",{}); if(type(d)!="object"||type(d.interface)!="array") exit(2); let out=[]; for(let i in d.interface) push(out,{name:i.interface,up:i.up,proto:i.proto,dns:i["dns-server"]}); c.disconnect(); print(sprintf("%J\n",out));'"#
            }
            Self::DhcpDns => {
                r#"/usr/bin/ucode -e 'import { cursor } from "uci"; let c=cursor(); let out=[]; c.foreach("dhcp","dhcp",function(s) { let v=[]; for(let k in [s.dhcp_option,s.dhcp_option_force]) { if(type(k)=="string") k=[k]; if(type(k)=="array") for(let x in k) if(type(x)=="string" && (substr(x,0,2)=="6," || substr(x,0,18)=="option:dns-server,")) push(v,x); } push(out,{name:s[".name"],interface:s.interface,ipv4_options:v,ipv6_dns:s.dns,ra:s.ra,dhcpv6:s.dhcpv6}); }); print(sprintf("%J\n",out));'"#
            }
            Self::Wireless => {
                r#"/usr/bin/ucode -e 'import { cursor } from "uci"; let c = cursor(); let out = []; c.foreach("wireless", "wifi-iface", function(s) { push(out, {name:s[".name"],radio:s.device,ssid:s.ssid,maxassoc:s.maxassoc,mode:s.mode,disabled:s.disabled}); }); print(sprintf("%J\n",out));'"#
            }
            Self::WirelessRadios => {
                r#"/usr/bin/ucode -e 'import { cursor } from "uci"; let c=cursor(); let out=[]; c.foreach("wireless","wifi-device",function(s) { push(out,{name:s[".name"],driver:s.type,maxassoc:s.maxassoc,disabled:s.disabled}); }); print(sprintf("%J\n",out));'"#
            }
            Self::WirelessRuntime => {
                r#"/usr/bin/ucode -e 'import { connect } from "ubus"; let c=connect(null,2); if(!c) exit(1); let names=c.list(); if(type(names)!="array" || length(names)>1024) exit(2); let out=[]; for(let n in names) { if(type(n)!="string" || !match(n,/^hostapd\.[A-Za-z0-9_.-]{1,64}$/)) continue; if(length(out)>=64) exit(3); let s=c.call(n,"get_status",{}); if(type(s)!="object") s={}; let r=c.call(n,"get_clients",{}); let count=null; if(type(r)=="object" && type(r.clients)=="object") { count=0; for(let mac,v in r.clients) if(v.assoc===true || v.assoc===1) count++; } push(out,{name:n,driver:s.driver,phy:s.phy,status:s.status,associated_clients:count}); } c.disconnect(); print(sprintf("%J\n",out));'"#
            }
            Self::Partitions => "/bin/cat /proc/mtd",
        }
    }
    pub fn output_limit(self) -> usize {
        match self {
            Self::Board | Self::Partitions => 16 * 1024,
            Self::Network
            | Self::Wireless
            | Self::RuntimeDns
            | Self::DhcpDns
            | Self::WirelessRadios
            | Self::WirelessRuntime => 64 * 1024,
        }
    }
}

pub(crate) fn invalid() -> AdapterError {
    AdapterError::new(
        AdapterErrorCode::IdentityUnverified,
        "OpenWrt returned missing, malformed or excessive inspection data.",
        None,
        false,
        "Check the supported firmware target. Remote output is not included in logs.",
    )
}

fn text(value: &str, maximum: usize) -> bool {
    !value.is_empty() && value.len() <= maximum && !value.chars().any(char::is_control)
}
fn decode<T: for<'a> Deserialize<'a>>(bytes: &[u8], query: ReadQuery) -> Result<T, AdapterError> {
    if bytes.len() > query.output_limit() {
        return Err(invalid());
    }
    serde_json::from_slice(bytes).map_err(|_| invalid())
}

/// These fields are authenticated server reports, not independent hardware certification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Board {
    pub model: String,
    pub board_name: String,
    pub release: Release,
    #[serde(default)]
    pub rootfs_type: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Release {
    pub distribution: String,
    pub version: String,
    pub revision: String,
    pub target: String,
}
impl Board {
    pub fn parse(bytes: &[u8]) -> Result<Self, AdapterError> {
        let board: Self = decode(bytes, ReadQuery::Board)?;
        if [
            &board.model,
            &board.board_name,
            &board.release.distribution,
            &board.release.version,
            &board.release.revision,
            &board.release.target,
        ]
        .iter()
        .any(|s| !text(s, 256))
            || board.release.distribution != "OpenWrt"
            || board.rootfs_type.as_ref().is_some_and(|s| !text(s, 64))
        {
            return Err(invalid());
        }
        Ok(board)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StringList {
    One(String),
    Many(Vec<String>),
}
impl StringList {
    pub fn addresses(&self) -> Result<Vec<IpAddr>, AdapterError> {
        let values: Vec<&str> = match self {
            Self::One(s) => s.split_ascii_whitespace().collect(),
            Self::Many(v) => v.iter().map(String::as_str).collect(),
        };
        if values.len() > 16 {
            return Err(invalid());
        }
        values
            .into_iter()
            .map(|s| s.parse().map_err(|_| invalid()))
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Network {
    pub name: String,
    pub proto: Option<String>,
    pub dns: Option<StringList>,
    pub peerdns: Option<String>,
    pub apn: Option<String>,
    pub auto_apn: Option<String>,
    pub pdptype: Option<String>,
    pub auth: Option<String>,
}
pub fn networks(bytes: &[u8]) -> Result<Vec<Network>, AdapterError> {
    let rows: Vec<Network> = decode(bytes, ReadQuery::Network)?;
    if rows.len() > 64 {
        return Err(invalid());
    }
    let mut names = std::collections::BTreeSet::new();
    for row in &rows {
        if !text(&row.name, 128)
            || !names.insert(&row.name)
            || [
                &row.proto,
                &row.peerdns,
                &row.apn,
                &row.auto_apn,
                &row.pdptype,
                &row.auth,
            ]
            .into_iter()
            .flatten()
            .any(|s| !text(s, 256))
        {
            return Err(invalid());
        }
        if let Some(dns) = &row.dns {
            dns.addresses()?;
        }
    }
    Ok(rows)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeDns {
    pub name: String,
    pub up: bool,
    pub proto: Option<String>,
    pub dns: Option<StringList>,
}
pub fn runtime_dns(bytes: &[u8]) -> Result<Vec<RuntimeDns>, AdapterError> {
    let rows: Vec<RuntimeDns> = decode(bytes, ReadQuery::RuntimeDns)?;
    if rows.len() > 64 {
        return Err(invalid());
    }
    let mut names = std::collections::BTreeSet::new();
    for row in &rows {
        if !text(&row.name, 128)
            || !names.insert(&row.name)
            || row.proto.as_ref().is_some_and(|s| !text(s, 128))
        {
            return Err(invalid());
        }
        if let Some(dns) = &row.dns {
            dns.addresses()?;
        }
    }
    Ok(rows)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DhcpDns {
    pub name: String,
    pub interface: Option<String>,
    pub ipv4_options: Vec<String>,
    pub ipv6_dns: Option<StringList>,
    pub ra: Option<String>,
    pub dhcpv6: Option<String>,
}
pub fn dhcp_dns(bytes: &[u8]) -> Result<Vec<DhcpDns>, AdapterError> {
    let rows: Vec<DhcpDns> = decode(bytes, ReadQuery::DhcpDns)?;
    if rows.len() > 64 {
        return Err(invalid());
    }
    let mut names = std::collections::BTreeSet::new();
    for row in &rows {
        if !text(&row.name, 128)
            || !names.insert(&row.name)
            || [&row.interface, &row.ra, &row.dhcpv6]
                .into_iter()
                .flatten()
                .any(|s| !text(s, 128))
            || row.ipv4_options.len() > 16
        {
            return Err(invalid());
        }
        if let Some(dns) = &row.ipv6_dns {
            if dns.addresses()?.iter().any(IpAddr::is_ipv4) {
                return Err(invalid());
            }
        }
        for option in &row.ipv4_options {
            let values = option
                .strip_prefix("6,")
                .or_else(|| option.strip_prefix("option:dns-server,"))
                .ok_or_else(invalid)?;
            let values: Vec<_> = values.split(',').collect();
            if values.is_empty()
                || values.len() > 16
                || values
                    .iter()
                    .any(|s| s.parse::<std::net::Ipv4Addr>().is_err())
            {
                return Err(invalid());
            }
        }
    }
    Ok(rows)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Radio {
    pub name: String,
    pub driver: Option<String>,
    pub maxassoc: Option<String>,
    pub disabled: Option<String>,
}
pub fn radios(bytes: &[u8]) -> Result<Vec<Radio>, AdapterError> {
    let rows: Vec<Radio> = decode(bytes, ReadQuery::WirelessRadios)?;
    if rows.len() > 64 {
        return Err(invalid());
    }
    let mut names = std::collections::BTreeSet::new();
    for row in &rows {
        if !text(&row.name, 128)
            || !names.insert(&row.name)
            || [&row.driver, &row.disabled]
                .into_iter()
                .flatten()
                .any(|s| !text(s, 128))
            || row
                .maxassoc
                .as_ref()
                .is_some_and(|s| s.parse::<u32>().is_err())
        {
            return Err(invalid());
        }
    }
    Ok(rows)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WirelessRuntime {
    pub name: String,
    pub driver: Option<String>,
    pub phy: Option<String>,
    pub status: Option<String>,
    pub associated_clients: Option<u32>,
}
pub fn wireless_runtime(bytes: &[u8]) -> Result<Vec<WirelessRuntime>, AdapterError> {
    let rows: Vec<WirelessRuntime> = decode(bytes, ReadQuery::WirelessRuntime)?;
    if rows.len() > 64 {
        return Err(invalid());
    }
    let mut names = std::collections::BTreeSet::new();
    for row in &rows {
        if !row.name.starts_with("hostapd.")
            || !text(&row.name, 72)
            || !names.insert(&row.name)
            || !row
                .name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
            || [&row.driver, &row.phy, &row.status]
                .into_iter()
                .flatten()
                .any(|s| !text(s, 128))
        {
            return Err(invalid());
        }
    }
    Ok(rows)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wireless {
    pub name: String,
    pub radio: Option<String>,
    pub ssid: Option<String>,
    pub maxassoc: Option<String>,
    pub mode: Option<String>,
    pub disabled: Option<String>,
}
pub fn wireless(bytes: &[u8]) -> Result<Vec<Wireless>, AdapterError> {
    let rows: Vec<Wireless> = decode(bytes, ReadQuery::Wireless)?;
    if rows.len() > 64 {
        return Err(invalid());
    }
    let mut names = std::collections::BTreeSet::new();
    for row in &rows {
        if !text(&row.name, 128)
            || !names.insert(&row.name)
            || [&row.radio, &row.mode, &row.disabled]
                .into_iter()
                .flatten()
                .any(|s| !text(s, 128))
            || row
                .ssid
                .as_ref()
                .is_some_and(|s| s.len() > 32 || s.chars().any(char::is_control))
            || row
                .maxassoc
                .as_ref()
                .is_some_and(|s| s.parse::<u32>().is_err())
        {
            return Err(invalid());
        }
    }
    Ok(rows)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Partition {
    pub name: String,
    pub identifier: String,
    pub bytes: u64,
    pub erase_bytes: u64,
}
pub fn partitions(bytes: &[u8]) -> Result<Vec<Partition>, AdapterError> {
    if bytes.len() > ReadQuery::Partitions.output_limit() {
        return Err(invalid());
    }
    let mut lines = std::str::from_utf8(bytes).map_err(|_| invalid())?.lines();
    if lines
        .next()
        .map(|s| s.split_whitespace().collect::<Vec<_>>())
        != Some(vec!["dev:", "size", "erasesize", "name"])
    {
        return Err(invalid());
    }
    let mut rows = Vec::new();
    let mut ids = std::collections::BTreeSet::new();
    for line in lines {
        let (fields, quoted) = line.split_once('"').ok_or_else(invalid)?;
        let name = quoted.strip_suffix('"').ok_or_else(invalid)?;
        let parts: Vec<_> = fields.split_whitespace().collect();
        if parts.len() != 3 || !text(name, 128) || rows.len() >= 128 {
            return Err(invalid());
        }
        let id = parts[0].strip_suffix(':').ok_or_else(invalid)?;
        let number = id.strip_prefix("mtd").ok_or_else(invalid)?;
        if number.is_empty()
            || !number.bytes().all(|b| b.is_ascii_digit())
            || !ids.insert(id.to_owned())
        {
            return Err(invalid());
        }
        rows.push(Partition {
            name: name.into(),
            identifier: id.into(),
            bytes: u64::from_str_radix(parts[1], 16).map_err(|_| invalid())?,
            erase_bytes: u64::from_str_radix(parts[2], 16).map_err(|_| invalid())?,
        });
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn board_missing_fields_and_vendor_claims_fail_closed() {
        assert!(Board::parse(br#"{}"#).is_err());
        let valid = br#"{"model":"QEMU Standard PC","board_name":"qemu-standard-pc","release":{"distribution":"OpenWrt","version":"25.12.6","revision":"r1","target":"x86/64"}}"#;
        let parsed = Board::parse(valid).unwrap();
        assert_eq!(parsed.rootfs_type, None);
        assert!(Board::parse(&vec![b' '; 16385]).is_err());
        assert!(Board::parse(
            String::from_utf8_lossy(valid)
                .replace("OpenWrt", "Vendor")
                .as_bytes()
        )
        .is_err());
    }
    #[test]
    fn multiple_named_interfaces_and_missing_dns_are_preserved() {
        let rows = networks(br#"[{"name":"uplink","proto":"dhcp","dns":["1.1.1.1","2606:4700:4700::1111"]},{"name":"cellular","proto":"qmi","apn":"internet"}]"#).unwrap();
        assert_eq!(rows[0].dns.as_ref().unwrap().addresses().unwrap().len(), 2);
        assert!(rows[1].dns.is_none());
        assert!(networks(br#"[{"name":"wan","dns":["not-an-ip"]}]"#).is_err());
        assert!(networks(br#"[{"name":"wan","password":"secret"}]"#).is_err());
        assert!(networks(br#"[{"name":"wan"},{"name":"wan"}]"#).is_err());
    }
    #[test]
    fn wireless_is_configuration_not_driver_capacity() {
        let rows =
            wireless(br#"[{"name":"default_radio0","radio":"radio0","maxassoc":"32"}]"#).unwrap();
        assert_eq!(rows[0].maxassoc.as_deref(), Some("32"));
        assert!(wireless(br#"[{"name":"x","maxassoc":"-1"}]"#).is_err());
        assert!(wireless(br#"[{"name":"x","key":"secret"}]"#).is_err());
    }
    #[test]
    fn mtd_metadata_never_implies_installable_bytes() {
        let rows =
            partitions(b"dev: size erasesize name\nmtd0: 00040000 00010000 \"u-boot\"\n").unwrap();
        assert_eq!(rows[0].bytes, 262144);
        assert!(partitions(b"bad").is_err());
        assert!(partitions(b"dev: size erasesize name\nmtd0: xyz 100 \"firmware\"\n").is_err());
    }
    #[test]
    fn remote_registry_has_no_frontend_command_or_write_variant() {
        for query in [
            ReadQuery::Board,
            ReadQuery::Network,
            ReadQuery::RuntimeDns,
            ReadQuery::DhcpDns,
            ReadQuery::Wireless,
            ReadQuery::WirelessRadios,
            ReadQuery::WirelessRuntime,
            ReadQuery::Partitions,
        ] {
            let command = query.command();
            for forbidden in [
                "uci set",
                "uci commit",
                "sysupgrade",
                "/dev/mtd",
                "reboot",
                "password",
                "pincode",
                "s.key",
            ] {
                assert!(!command.contains(forbidden));
            }
        }
    }

    #[test]
    fn configured_effective_and_lan_dns_are_distinct_bounded_sources() {
        let runtime = runtime_dns(br#"[{"name":"custom_uplink","up":true,"proto":"dhcp","dns":["1.1.1.1","2606:4700:4700::1111"]},{"name":"downlink","up":false}]"#).unwrap();
        assert_eq!(
            runtime[0].dns.as_ref().unwrap().addresses().unwrap().len(),
            2
        );
        assert!(runtime[1].dns.is_none());
        assert!(runtime_dns(br#"[{"name":"uplink","up":true,"dns":["not-address"]}]"#).is_err());
        let dhcp = dhcp_dns(br#"[{"name":"lan","interface":"local","ipv4_options":["6,1.1.1.1,8.8.8.8"],"ipv6_dns":["2606:4700:4700::1111"]}]"#).unwrap();
        assert_eq!(dhcp[0].interface.as_deref(), Some("local"));
        assert!(dhcp_dns(br#"[{"name":"lan","ipv4_options":["3,192.168.1.1"]}]"#).is_err());
        assert!(dhcp_dns(br#"[{"name":"lan","ipv4_options":[],"ipv6_dns":["1.1.1.1"]}]"#).is_err());
        assert!(runtime_dns(br#"[{"name":"wan","up":true,"password":"secret"}]"#).is_err());
    }

    #[test]
    fn radio_and_bss_limits_are_not_runtime_driver_guarantees() {
        let radio = radios(br#"[{"name":"radio0","driver":"mac80211","maxassoc":"64"}]"#).unwrap();
        assert_eq!(radio[0].maxassoc.as_deref(), Some("64"));
        let runtime = wireless_runtime(br#"[{"name":"hostapd.wlan0","driver":"nl80211","phy":"phy0","status":"ENABLED","associated_clients":2}]"#).unwrap();
        assert_eq!(runtime[0].associated_clients, Some(2));
        assert!(
            wireless_runtime(br#"[{"name":"hostapd.wlan0;cmd","associated_clients":0}]"#).is_err()
        );
        assert!(
            wireless_runtime(br#"[{"name":"hostapd.wlan0","associated_clients":-1}]"#).is_err()
        );
        assert!(radios(br#"[{"name":"radio0","maxassoc":"999999999999999999"}]"#).is_err());
    }
}
