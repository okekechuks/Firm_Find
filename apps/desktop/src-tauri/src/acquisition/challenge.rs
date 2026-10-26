//! Credential-free challenge metadata only. No credential/session material is returned.
use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HeaderCount {
    #[serde(rename = "0")]
    Zero,
    #[serde(rename = "1")]
    One,
    #[serde(rename = "2")]
    Two,
    #[serde(rename = "3+")]
    Many,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Scheme {
    Digest,
    Basic,
    Negotiate,
    Ntlm,
    Other,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ParameterName {
    Realm,
    Nonce,
    Qop,
    Algorithm,
    Opaque,
    Stale,
    Charset,
    Domain,
    Userhash,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ShapeReason {
    NoHeader,
    MultipleHeaders,
    NonDigest,
    MissingRealm,
    MissingNonce,
    BadNonceShape,
    BadQop,
    BadParameterSyntax,
    DuplicateParameter,
    Oversized,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ParameterShape {
    pub name: ParameterName,
    pub quoted: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChallengeShape {
    pub header_count: HeaderCount,
    pub scheme: Option<Scheme>,
    pub parameters: Vec<ParameterShape>,
    pub other_parameter_count: u16,
    pub reason: Option<ShapeReason>,
}
impl Default for ChallengeShape {
    fn default() -> Self {
        Self {
            header_count: HeaderCount::Zero,
            scheme: None,
            parameters: vec![],
            other_parameter_count: 0,
            reason: Some(ShapeReason::NoHeader),
        }
    }
}
fn parameter_name(key: &str) -> Option<ParameterName> {
    Some(match key {
        "realm" => ParameterName::Realm,
        "nonce" => ParameterName::Nonce,
        "qop" => ParameterName::Qop,
        "algorithm" => ParameterName::Algorithm,
        "opaque" => ParameterName::Opaque,
        "stale" => ParameterName::Stale,
        "charset" => ParameterName::Charset,
        "domain" => ParameterName::Domain,
        "userhash" => ParameterName::Userhash,
        _ => return None,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChallengeObservation {
    #[serde(default)]
    pub shape: Option<ChallengeShape>,
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
    pub(crate) fn header_failure(status: Option<u16>, oversized: bool) -> Self {
        let mut out = Self::failure(
            if status.is_some_and(|s| (300..400).contains(&s)) {
                "redirect_blocked"
            } else if status.is_some_and(|s| s != 401) {
                "unexpected_status"
            } else {
                "challenge_not_usable"
            },
            status,
        );
        out.shape = Some(ChallengeShape {
            reason: Some(if oversized {
                ShapeReason::Oversized
            } else {
                ShapeReason::BadParameterSyntax
            }),
            ..ChallengeShape::default()
        });
        out
    }
    pub(crate) fn failure(code: &str, status: Option<u16>) -> Self {
        Self {
            shape: if code == "challenge_not_usable" {
                Some(ChallengeShape::default())
            } else {
                None
            },
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
        && !["conflicting", "ambiguous"].contains(&base.identity.confidence.as_str())
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
    binding: &approval::Binding,
    revision: u64,
    input: &DiscoveryRequest,
    proof: &AuthChallengeApproval,
    token: &AtomicBool,
    dispatch: impl FnOnce(&connection::Target, &str, &AuthChallengeApproval) -> ChallengeObservation,
) -> Result<AcquisitionCapabilityReport, DiscoveryError> {
    if base.discovery_revision != revision {
        return Err(error(
            "STALE_RESULT",
            "Root observation no longer matches this inventory.",
        ));
    }
    proof.validate(binding)?;
    if !binding.matches_request(input) {
        return Err(error(
            "TARGET_MISMATCH",
            "Request differs from the approved selection.",
        ));
    }
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
    proof.validate(binding)?;
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
        out.shape = self.shape.clone().or(out.shape).map(|mut shape| {
            shape.parameters.truncate(256);
            shape.other_parameter_count = shape.other_parameter_count.min(8192);
            shape
        });
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
                out.shape = Some(ChallengeShape {
                    reason: Some(ShapeReason::BadParameterSyntax),
                    ..out.shape.unwrap_or_default()
                });
            }
        }
        out
    }
}
// Facts cover bounded, syntactically reachable parameters in header order. A
// malformed tail is never searched heuristically for additional names/values.
fn scan_parameter_shapes(value: &str, shape: &mut ChallengeShape) {
    let Some((_, mut rest)) = value.split_once(|c: char| c.is_ascii_whitespace()) else {
        return;
    };
    loop {
        rest = rest.trim_start_matches([' ', '\t']);
        let end = rest
            .bytes()
            .take_while(|b| b.is_ascii_alphanumeric() || b"_-".contains(b))
            .count();
        if end == 0 || end > 64 {
            return;
        }
        let key = rest[..end].to_ascii_lowercase();
        rest = rest[end..].trim_start_matches([' ', '\t']);
        let Some(next) = rest.strip_prefix('=') else {
            return;
        };
        rest = next.trim_start_matches([' ', '\t']);
        let quoted = rest.starts_with('"');
        if let Some(name) = parameter_name(&key) {
            if shape.parameters.len() >= 256 {
                return;
            }
            shape.parameters.push(ParameterShape { name, quoted });
        } else {
            shape.other_parameter_count = shape.other_parameter_count.saturating_add(1);
        }
        if quoted {
            let Some(end) = rest[1..].find('"') else {
                return;
            };
            rest = &rest[end + 2..];
        } else {
            let end = rest.find([',', ' ', '\t']).unwrap_or(rest.len());
            if end == 0 {
                return;
            }
            rest = &rest[end..];
        }
        rest = rest.trim_start_matches([' ', '\t']);
        let Some(next) = rest.strip_prefix(',') else {
            return;
        };
        rest = next;
    }
}
fn rejected(
    mut out: ChallengeObservation,
    mut shape: ChallengeShape,
    reason: ShapeReason,
) -> ChallengeObservation {
    shape.reason = Some(reason);
    out.shape = Some(shape);
    out
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
    let mut shape = ChallengeShape::default();
    if raw.len() > 8192 {
        return rejected(out, shape, ShapeReason::Oversized);
    }
    if !raw.is_ascii() {
        return rejected(out, shape, ShapeReason::BadParameterSyntax);
    }
    let Ok(text) = std::str::from_utf8(raw) else {
        return rejected(out, shape, ShapeReason::BadParameterSyntax);
    };
    let mut lines = text.split("\r\n");
    if !lines.next().is_some_and(|l| l.starts_with("HTTP/")) {
        return rejected(out, shape, ShapeReason::BadParameterSyntax);
    }
    let mut auth = Vec::new();
    let mut invalid = false;
    let mut oversized = false;
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
            let kind = if scheme.eq_ignore_ascii_case("Digest") {
                Scheme::Digest
            } else if scheme.eq_ignore_ascii_case("Basic") {
                Scheme::Basic
            } else if scheme.eq_ignore_ascii_case("Negotiate") {
                Scheme::Negotiate
            } else if scheme.eq_ignore_ascii_case("NTLM") {
                Scheme::Ntlm
            } else {
                Scheme::Other
            };
            if kind != Scheme::Digest {
                out.other_schemes_present = true;
            }
            if auth.is_empty() {
                shape.scheme = Some(kind);
            }
            if line.len() > 2048 {
                oversized = true;
            }
            auth.push(value);
        }
    }
    shape.header_count = match auth.len() {
        0 => HeaderCount::Zero,
        1 => HeaderCount::One,
        2 => HeaderCount::Two,
        _ => HeaderCount::Many,
    };
    if oversized {
        return rejected(out, shape, ShapeReason::Oversized);
    }
    for value in &auth {
        scan_parameter_shapes(value, &mut shape);
    }
    if invalid {
        return rejected(out, shape, ShapeReason::BadParameterSyntax);
    }
    if auth.is_empty() {
        return rejected(out, shape, ShapeReason::NoHeader);
    }
    if auth.len() != 1 {
        return rejected(out, shape, ShapeReason::MultipleHeaders);
    }
    if out.other_schemes_present {
        return rejected(out, shape, ShapeReason::NonDigest);
    }
    let Some((_, mut rest)) = auth[0].split_once(|c: char| c.is_ascii_whitespace()) else {
        return rejected(out, shape, ShapeReason::BadParameterSyntax);
    };
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
            return rejected(out, shape, ShapeReason::BadParameterSyntax);
        }
        let key = rest[..count].to_ascii_lowercase();
        rest = rest[count..].trim_start_matches([' ', '\t']);
        if key == "nonce" {
            out.nonce_present = true;
        }
        if !rest.starts_with('=') {
            if parameter_name(&key).is_none() {
                out.other_schemes_present = true;
            }
            return rejected(out, shape, ShapeReason::BadParameterSyntax);
        }
        rest = rest[1..].trim_start_matches([' ', '\t']);
        let quoted = rest.starts_with('"');
        let val;
        if quoted {
            let inside = &rest[1..];
            let Some(end) = inside.find('"') else {
                return rejected(out, shape, ShapeReason::BadParameterSyntax);
            };
            val = &inside[..end];
            rest = inside[end + 1..].trim_start_matches([' ', '\t']);
            if val.len() > 128 {
                return rejected(out, shape, ShapeReason::Oversized);
            }
            if val
                .bytes()
                .any(|b| !(0x20..=0x7e).contains(&b) || b == b'\\')
            {
                return rejected(out, shape, ShapeReason::BadParameterSyntax);
            }
        } else {
            if !matches!(key.as_str(), "algorithm" | "stale" | "charset" | "userhash") {
                return rejected(out, shape, ShapeReason::BadParameterSyntax);
            }
            let end = rest.find([',', ' ', '\t']).unwrap_or(rest.len());
            val = &rest[..end];
            rest = rest[end..].trim_start_matches([' ', '\t']);
            if val.len() > 32 {
                return rejected(out, shape, ShapeReason::Oversized);
            }
            if !valid_token(val, 32) {
                return rejected(out, shape, ShapeReason::BadParameterSyntax);
            }
        }
        if values.insert(key, val).is_some() {
            return rejected(out, shape, ShapeReason::DuplicateParameter);
        }
        if rest.is_empty() {
            break;
        }
        let Some(next) = rest.strip_prefix(',') else {
            return rejected(out, shape, ShapeReason::BadParameterSyntax);
        };
        rest = next;
        if rest.trim().is_empty() {
            return rejected(out, shape, ShapeReason::BadParameterSyntax);
        }
    }
    let Some(realm) = values.get("realm").copied() else {
        return rejected(out, shape, ShapeReason::MissingRealm);
    };
    if realm.len() > 64 {
        return rejected(out, shape, ShapeReason::Oversized);
    }
    if !valid_realm(realm) {
        return rejected(out, shape, ShapeReason::BadParameterSyntax);
    }
    let Some(nonce) = values.get("nonce").copied() else {
        return rejected(out, shape, ShapeReason::MissingNonce);
    };
    if nonce.is_empty() || nonce.len() > 128 || !nonce.bytes().all(|b| b.is_ascii_hexdigit()) {
        return rejected(out, shape, ShapeReason::BadNonceShape);
    }
    let Some(qop) = values.get("qop").copied().filter(|v| valid_qop(v)) else {
        return rejected(out, shape, ShapeReason::BadQop);
    };
    let algorithm = values.get("algorithm").copied();
    if algorithm.is_some_and(|v| v.len() > 16) {
        return rejected(out, shape, ShapeReason::Oversized);
    }
    if algorithm.is_some_and(|v| !valid_token(v, 16)) {
        return rejected(out, shape, ShapeReason::BadParameterSyntax);
    }
    for key in ["stale", "charset", "userhash"] {
        if values.get(key).is_some_and(|v| !valid_token(v, 32)) {
            return rejected(out, shape, ShapeReason::BadParameterSyntax);
        }
    }
    shape.reason = None;
    out.shape = Some(shape);
    if status == 401 {
        out.code = "observed".into();
        out.realm = Some(realm.into());
        out.qop = Some(
            qop.split(',')
                .map(|v| v.trim_matches(' '))
                .collect::<Vec<_>>()
                .join(","),
        );
        out.algorithm = algorithm.map(str::to_owned);
    }
    out
}
