//! Credential-free challenge metadata only. No credential/session material is returned.
use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChallengeObservation {
    pub code: String,
    pub http_status: Option<u16>,
    pub realm: Option<String>,
    pub qop: Option<String>,
    pub algorithm: Option<String>,
    pub nonce_present: bool,
    pub set_cookie_present: bool,
    pub other_schemes_present: bool,
    pub source: EvidenceOrigin,
    pub reliability: Reliability,
    pub verification: Verification,
    pub label: String,
}
impl ChallengeObservation {
    pub(crate) fn failure(code: &str, status: Option<u16>) -> Self {
        Self {
            code: code.into(),
            http_status: status,
            realm: None,
            qop: None,
            algorithm: None,
            nonce_present: false,
            set_cookie_present: false,
            other_schemes_present: false,
            source: EvidenceOrigin::EndpointResponse,
            reliability: Reliability::Observed,
            verification: Verification::Observed,
            label: "authentication challenge observed; unverified".into(),
        }
    }
}
pub(super) fn root_matches(
    base: &AcquisitionCapabilityReport,
    revision: u64,
    input: &DiscoveryRequest,
) -> bool {
    let endpoint = input
        .endpoint
        .as_deref()
        .and_then(|v| connection::parse_address(v).ok())
        .map(|v| v.url);
    base.device_id == input.device_id
        && base.discovery_revision == revision
        && base.expires_at > connection::now()
        && base.probe.attempted
        && base
            .probe
            .http_status
            .is_some_and(|s| (200..300).contains(&s))
        && endpoint.is_some()
        && base.fingerprint.endpoint == endpoint
        && base.fingerprint.interface_id == input.interface_id
        && base.protocols.iter().any(|p| p.family == "asr_duster_xml")
}
pub(super) fn collect(
    base: &AcquisitionCapabilityReport,
    device: &Device,
    revision: u64,
    input: &DiscoveryRequest,
    proof: &AuthChallengeApproval,
    token: &AtomicBool,
    dispatch: impl FnOnce(&connection::Target, &str, &AuthChallengeApproval) -> ChallengeObservation,
) -> Result<AcquisitionCapabilityReport, DiscoveryError> {
    let binding = approval::binding(device, revision, input)?;
    proof.validate(&binding)?;
    if token.load(Ordering::Acquire) {
        return Err(error("CANCELLED", "Challenge check cancelled."));
    }
    let mut report = base.clone();
    if !root_matches(base, revision, input) {
        report.auth_challenge = Some(ChallengeObservation::failure("no_recognized_family", None));
        return Ok(report);
    }
    let target = connection::parse_address(input.endpoint.as_deref().unwrap_or(""))
        .map_err(|_| error("INVALID_ENDPOINT", "Select the checked endpoint."))?;
    let url = format!("{}login.cgi", target.url);
    RequestRegistry::default()
        .validate(
            ProtocolFamilyId::AsrDusterXml,
            RequestId::Challenge,
            &target.url,
            &RequestEnvelope::get(&url),
        )
        .map_err(|_| error("REQUEST_DENIED", "Challenge request is not registered."))?;
    let observation =
        dispatch(&target, input.interface_id.as_deref().unwrap_or(""), proof).sanitized();
    proof.validate(&binding)?;
    if token.load(Ordering::Acquire) {
        return Err(error("CANCELLED", "Challenge check cancelled."));
    }
    if !root_matches(base, revision, input) {
        return Err(error(
            "STALE_RESULT",
            "Root observation expired during challenge.",
        ));
    }
    report.auth_challenge = Some(observation);
    Ok(report)
}
pub(crate) fn valid_realm(value: &str) -> bool {
    !value.is_empty() && value.len() <= 64 && value.bytes().all(|b| (0x20..=0x7e).contains(&b))
}
pub(crate) fn valid_token(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
}
pub(crate) fn valid_qop(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 32
        && value
            .split(',')
            .all(|v| valid_token(v.trim_matches(' '), 32))
}
impl ChallengeObservation {
    /// Revalidate even internal/cached data before diagnostics. Never echo unknown codes.
    pub(crate) fn sanitized(&self) -> Self {
        let code = match self.code.as_str() {
            "observed"
            | "no_recognized_family"
            | "unexpected_status"
            | "redirect_blocked"
            | "timeout"
            | "cancelled"
            | "subnet_conflict"
            | "transport_failed" => self.code.as_str(),
            _ => "challenge_not_usable",
        };
        let mut out = Self::failure(code, self.http_status);
        out.nonce_present = self.nonce_present;
        out.set_cookie_present = self.set_cookie_present;
        out.other_schemes_present = self.other_schemes_present;
        if code == "observed" {
            if self.http_status == Some(401)
                && self.nonce_present
                && !self.other_schemes_present
                && self.realm.as_deref().is_some_and(valid_realm)
                && self.qop.as_deref().is_some_and(valid_qop)
                && self.algorithm.as_deref().is_none_or(|v| valid_token(v, 16))
            {
                out.realm = self.realm.clone();
                out.qop = self.qop.as_deref().map(|v| {
                    v.split(',')
                        .map(|token| token.trim_matches(' '))
                        .collect::<Vec<_>>()
                        .join(",")
                });
                out.algorithm = self.algorithm.clone();
            } else {
                out.code = "challenge_not_usable".into();
            }
        }
        out
    }
}
pub(crate) fn parse_headers(status: u16, raw: &[u8]) -> ChallengeObservation {
    let mut out = ChallengeObservation::failure(
        if (300..400).contains(&status) {
            "redirect_blocked"
        } else if status != 401 {
            "unexpected_status"
        } else {
            "challenge_not_usable"
        },
        Some(status),
    );
    if raw.len() > 8192 || !raw.is_ascii() {
        return out;
    }
    let Ok(text) = std::str::from_utf8(raw) else {
        return out;
    };
    let mut lines = text.split("\r\n");
    if !lines.next().is_some_and(|l| l.starts_with("HTTP/")) {
        return out;
    }
    let mut auth = Vec::new();
    let mut invalid = false;
    for line in lines.filter(|l| !l.is_empty()) {
        let Some((name, value)) = line.split_once(':') else {
            invalid = true;
            continue;
        };
        if !valid_token(name, 128) {
            invalid = true;
            continue;
        }
        if name.eq_ignore_ascii_case("Set-Cookie") {
            out.set_cookie_present = true;
        }
        if name.eq_ignore_ascii_case("WWW-Authenticate") {
            let value = value.trim_matches([' ', '\t']);
            let scheme = value.split_ascii_whitespace().next().unwrap_or("");
            if !scheme.eq_ignore_ascii_case("Digest") {
                out.other_schemes_present = true;
            }
            if line.len() > 2048 {
                invalid = true;
            }
            auth.push(value);
        }
    }
    if invalid || auth.len() != 1 || out.other_schemes_present || status != 401 {
        return out;
    }
    let value = auth[0];
    let Some((scheme, mut rest)) = value.split_once(|c: char| c.is_ascii_whitespace()) else {
        return out;
    };
    if !scheme.eq_ignore_ascii_case("Digest") {
        return out;
    }
    let mut values = BTreeMap::new();
    loop {
        rest = rest.trim_start_matches([' ', '\t']);
        if rest.is_empty() {
            break;
        }
        let count = rest
            .bytes()
            .take_while(|b| b.is_ascii_alphanumeric() || b"_-".contains(b))
            .count();
        if count == 0 || count > 64 {
            return out;
        }
        let key = rest[..count].to_ascii_lowercase();
        rest = rest[count..].trim_start_matches([' ', '\t']);
        if key == "nonce" {
            out.nonce_present = true;
        }
        if !rest.starts_with('=') {
            if !matches!(key.as_str(), "realm" | "nonce" | "qop" | "algorithm") {
                out.other_schemes_present = true;
            }
            return out;
        }
        rest = rest[1..].trim_start_matches([' ', '\t']);
        let Some(quoted) = rest.strip_prefix('"') else {
            return out;
        };
        let Some(end) = quoted.find('"') else {
            return out;
        };
        let val = &quoted[..end];
        if val.len() > 128
            || val
                .bytes()
                .any(|b| !(0x20..=0x7e).contains(&b) || b == b'\\')
            || values.insert(key, val).is_some()
        {
            return out;
        }
        rest = quoted[end + 1..].trim_start_matches([' ', '\t']);
        if rest.is_empty() {
            break;
        }
        let Some(next) = rest.strip_prefix(',') else {
            return out;
        };
        rest = next;
        if rest.trim().is_empty() {
            return out;
        }
    }
    let Some(realm) = values.get("realm").copied().filter(|v| valid_realm(v)) else {
        return out;
    };
    let Some(qop) = values.get("qop").copied().filter(|v| valid_qop(v)) else {
        return out;
    };
    if !values.get("nonce").is_some_and(|v| {
        !v.is_empty() && v.len() <= 128 && v.bytes().all(|b| b.is_ascii_hexdigit())
    }) {
        return out;
    }
    let algorithm = values.get("algorithm").copied();
    if algorithm.is_some_and(|v| !valid_token(v, 16)) {
        return out;
    }
    out.code = "observed".into();
    out.realm = Some(realm.into());
    out.qop = Some(
        qop.split(',')
            .map(|v| v.trim_matches(' '))
            .collect::<Vec<_>>()
            .join(","),
    );
    out.algorithm = algorithm.map(str::to_owned);
    out
}
