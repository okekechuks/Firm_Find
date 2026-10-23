//! A single, separately approved static read. Filename observations never enter identity policy.
use super::*;
use quick_xml::{events::Event, Reader};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StaticMenuObservation {
    pub code: String,
    pub file_name: Option<String>,
    pub source: EvidenceOrigin,
    pub reliability: Reliability,
    pub verification: Verification,
    pub label: String,
}
fn observation(code: &str, file_name: Option<String>) -> StaticMenuObservation {
    StaticMenuObservation {
        code: code.into(),
        file_name,
        source: EvidenceOrigin::EndpointResponse,
        reliability: Reliability::Observed,
        verification: Verification::Observed,
        label: "dashboard file name observed from the static menu; unverified".into(),
    }
}

fn xml_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && bytes.all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
}
pub(super) fn parse_menu(body: &[u8]) -> Option<String> {
    if body.len() > 16384 {
        return None;
    }
    let xml = std::str::from_utf8(body).ok()?;
    // Reject declarations before parsing; no unescape/expansion/resolver is used anywhere.
    if xml.chars().any(|c| {
        (c.is_control() && !matches!(c, '\t' | '\n' | '\r')) || matches!(c, '\u{fffe}' | '\u{ffff}')
    }) {
        return None;
    }
    let upper = xml.to_ascii_uppercase();
    if upper.contains("<!DOCTYPE")
        || upper.contains("<!ENTITY")
        || xml.contains('&')
        || xml.contains("]]>")
    {
        return None;
    }
    let mut reader = Reader::from_str(xml);
    reader.config_mut().check_comments = true;
    reader.config_mut().check_end_names = true;
    reader.config_mut().allow_unmatched_ends = false;
    let mut stack: Vec<(String, bool)> = vec![];
    let mut roots = 0;
    let mut matches = vec![];
    let mut declared = false;
    loop {
        let event = reader.read_event().ok()?;
        match event {
            Event::Start(ref element) | Event::Empty(ref element) => {
                if stack.len() >= 64 {
                    return None;
                }
                let name = element.name().as_ref().to_owned();
                if !xml_name(&name) {
                    return None;
                }
                if stack.is_empty() {
                    roots += 1;
                    if roots > 1 {
                        return None;
                    }
                }
                let mut attrs = BTreeMap::new();
                for attr in element.attributes() {
                    let attr = attr.ok()?;
                    let key = attr.key.as_ref().to_owned();
                    if key == "xmlns" || !xml_name(&key) {
                        return None;
                    }
                    if attr.value.contains('<') {
                        return None;
                    }
                    attrs.insert(key, attr.value.into_owned());
                }
                let simple = name == "Tab"
                    && attrs.contains_key("Name")
                    && attrs.get("type").is_some_and(|v| v == "submenuabsent");
                let submenu = name == "Menu"
                    && attrs.contains_key("id")
                    && stack
                        .iter()
                        .rev()
                        .find(|(name, _)| name == "Tab")
                        .is_some_and(|(_, nested)| *nested);
                if (simple || submenu)
                    && attrs
                        .get("implFunction")
                        .is_some_and(|v| v == "objdashboard")
                {
                    let value = attrs.get("xmlName")?;
                    if value.is_empty()
                        || value.len() > 64
                        || !value
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
                    {
                        return None;
                    }
                    matches.push(value.clone());
                }
                let nested = name == "Tab"
                    && attrs.contains_key("Name")
                    && attrs.get("type").is_some_and(|v| v != "submenuabsent");
                if matches!(event, Event::Start(_)) {
                    stack.push((name, nested));
                }
            }
            Event::End(_) => {
                stack.pop()?;
            }
            Event::Text(text) => {
                if stack.is_empty() && !text.as_ref().bytes().all(|b| b.is_ascii_whitespace()) {
                    return None;
                }
            }
            Event::Decl(decl) if roots == 0 && !declared => {
                if !xml.trim_start_matches('\u{feff}').starts_with("<?xml") {
                    return None;
                }
                let declaration = quick_xml::events::BytesStart::from_content(decl.as_ref(), 3);
                let mut previous = 0;
                let mut version = false;
                for attr in declaration.attributes() {
                    let attr = attr.ok()?;
                    let rank = match attr.key.as_ref() {
                        "version" if attr.value == "1.0" => {
                            version = true;
                            1
                        }
                        "encoding" if attr.value.eq_ignore_ascii_case("utf-8") => 2,
                        "standalone" if attr.value == "yes" || attr.value == "no" => 3,
                        _ => return None,
                    };
                    if rank <= previous || (!version && rank != 1) {
                        return None;
                    }
                    previous = rank;
                }
                if !version {
                    return None;
                }
                declared = true;
            }
            Event::Comment(_) => {}
            Event::Eof => break,
            _ => return None,
        }
    }
    if roots == 1 && stack.is_empty() && matches.len() == 1 {
        matches.pop()
    } else {
        None
    }
}

pub(super) fn collect(
    base: &AcquisitionCapabilityReport,
    device: &Device,
    revision: u64,
    input: &DiscoveryRequest,
    proof: &StaticMenuApproval,
    token: &AtomicBool,
    dispatch: impl FnOnce(
        &connection::Target,
        &str,
        &StaticMenuApproval,
    ) -> (connection::EndpointResult, Option<Vec<u8>>),
) -> Result<AcquisitionCapabilityReport, DiscoveryError> {
    let binding = approval::binding(device, revision, input)?;
    proof.validate(&binding)?;
    let target = connection::parse_address(input.endpoint.as_deref().unwrap_or(""))
        .map_err(|_| error("INVALID_ENDPOINT", "Select the checked endpoint."))?;
    if base.device_id != device.id
        || base.discovery_revision != revision
        || base.expires_at <= connection::now()
        || base.fingerprint.endpoint.as_deref() != Some(&target.url)
        || base.fingerprint.interface_id != input.interface_id
    {
        return Err(error(
            "STALE_RESULT",
            "Run the root check again for this endpoint.",
        ));
    }
    let mut report = base.clone();
    if !base.protocols.iter().any(|p| p.family == "asr_duster_xml") {
        report.static_menu = Some(observation("no_recognized_family", None));
        return Ok(report);
    }
    if token.load(Ordering::Acquire) {
        return Err(error("CANCELLED", "Static menu check cancelled."));
    }
    let registry = RequestRegistry::default();
    let url = format!("{}xml/ui_mifi.xml", target.url);
    registry
        .validate(
            ProtocolFamilyId::AsrDusterXml,
            RequestId::StaticMenu,
            &target.url,
            &RequestEnvelope::get(&url),
        )
        .map_err(|_| error("REQUEST_DENIED", "Static menu request is not registered."))?;
    let (result, body) = dispatch(&target, input.interface_id.as_deref().unwrap_or(""), proof);
    proof.validate(&binding)?;
    if base.expires_at <= connection::now() {
        return Err(error(
            "STALE_RESULT",
            "Root observation expired during menu check.",
        ));
    }
    if token.load(Ordering::Acquire) {
        return Err(error("CANCELLED", "Static menu check cancelled."));
    }
    let (code, name) = if result.status == Some(401) {
        ("menu_not_readable_without_login", None)
    } else if result.status == Some(403) {
        ("forbidden", None)
    } else if result.status.is_some_and(|s| (300..400).contains(&s)) {
        ("redirect_blocked", None)
    } else if result.code == "identity_unknown"
        && result.status.is_some_and(|s| (200..300).contains(&s))
    {
        match body.as_deref().and_then(parse_menu) {
            Some(name) => ("observed", Some(name)),
            None => ("no_unique_dashboard_entry", None),
        }
    } else {
        (
            match result.code.as_str() {
                "truncated_metadata" => "truncated_metadata",
                "wrong_content_type" => "wrong_content_type",
                "metadata_too_large" => "metadata_too_large",
                "timeout" => "timeout",
                "cancelled" => "cancelled",
                "subnet_conflict" => "subnet_conflict",
                _ => "transport_failed",
            },
            None,
        )
    };
    report.static_menu = Some(observation(code, name));
    Ok(report)
}
