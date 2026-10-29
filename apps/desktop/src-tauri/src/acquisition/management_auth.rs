//! Operation-local authentication. Evidence imports never create route authority.
use digest_auth::{AuthContext, AuthorizationHeader, Qop, WwwAuthenticateHeader};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, Zeroizing};

#[path = "management_auth_flow.rs"]
mod flow;
pub use flow::{
    authenticate_digest, IdentityReadConnector, IdentityReadResponse, ReadAuthorization,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthenticationScheme {
    None,
    Basic,
    Digest,
    CookieSession,
    FormSession,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthError {
    UnqualifiedRoute,
    InvalidChallenge,
    UnsupportedScheme,
    InvalidCredentials,
    AuthorizationExpired,
    DeviceChanged,
    Cancelled,
    Timeout,
    AuthenticationFailed,
    NonceExhausted,
    RetryLimit,
    Transport,
    ApprovalUsed,
}

pub struct VerifiedReadRoute {
    uri: &'static str,
}
#[cfg(test)]
pub(super) fn synthetic_identity_route() -> VerifiedReadRoute {
    VerifiedReadRoute {
        uri: "/synthetic/identity?view=summary",
    }
}
impl VerifiedReadRoute {
    /// Empty until a family-specific identity/status route is independently qualified.
    pub fn for_family(_family: &str) -> Result<Self, AuthError> {
        Err(AuthError::UnqualifiedRoute)
    }
    pub fn uri(&self) -> &str {
        self.uri
    }
}

/// No Debug, Clone or Serialize: callers cannot accidentally include secrets in reports.
pub struct Credentials {
    username: Zeroizing<String>,
    password: Zeroizing<String>,
}
impl Credentials {
    pub fn new(username: String, password: String) -> Result<Self, AuthError> {
        let username = Zeroizing::new(username);
        let password = Zeroizing::new(password);
        if username.is_empty()
            || username.len() > 128
            || password.is_empty()
            || password.len() > 1024
            || username
                .chars()
                .any(|c| c.is_control() || ['"', '\\'].contains(&c))
            || password.chars().any(|c| c.is_control())
        {
            return Err(AuthError::InvalidCredentials);
        }
        Ok(Self { username, password })
    }
}

struct Challenge(WwwAuthenticateHeader);
impl Drop for Challenge {
    fn drop(&mut self) {
        self.0.realm.zeroize();
        self.0.nonce.zeroize();
        if let Some(value) = &mut self.0.opaque {
            value.zeroize();
        }
        if let Some(values) = &mut self.0.domain {
            values.zeroize();
        }
    }
}

pub struct ManagementAuthenticator {
    challenge: Challenge,
    credentials: Credentials,
}

/// Conservatively reject ambiguous parameters and header injection before the
/// maintained parser. Escaped quoted-pairs are presently unsupported.
fn checked_challenge(value: &str) -> Result<Challenge, AuthError> {
    if value.len() > 8192 || value.chars().any(char::is_control) {
        return Err(AuthError::InvalidChallenge);
    }
    let (scheme, parameters) = value.split_once(' ').ok_or(AuthError::InvalidChallenge)?;
    if !scheme.eq_ignore_ascii_case("digest") {
        return Err(AuthError::UnsupportedScheme);
    }
    let mut keys = std::collections::BTreeSet::new();
    let mut input = parameters.trim();
    while !input.is_empty() {
        let (key, rest) = input.split_once('=').ok_or(AuthError::InvalidChallenge)?;
        let key = key.trim().to_ascii_lowercase();
        if key.is_empty()
            || key.len() > 64
            || !key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
            || !keys.insert(key)
            || keys.len() > 16
        {
            return Err(AuthError::InvalidChallenge);
        }
        let rest = rest.trim_start();
        input = if let Some(quoted) = rest.strip_prefix('"') {
            let end = quoted.find('"').ok_or(AuthError::InvalidChallenge)?;
            if quoted[..end].contains('\\') {
                return Err(AuthError::InvalidChallenge);
            }
            let tail = quoted[end + 1..].trim_start();
            if tail.is_empty() {
                ""
            } else {
                tail.strip_prefix(',')
                    .ok_or(AuthError::InvalidChallenge)?
                    .trim_start()
            }
        } else {
            match rest.split_once(',') {
                Some((_, next)) => next.trim_start(),
                None => "",
            }
        };
    }
    let normalized = Zeroizing::new(format!("Digest {parameters}"));
    let mut parsed = digest_auth::parse(&normalized).map_err(|_| AuthError::InvalidChallenge)?;
    if parsed.realm.is_empty() || parsed.nonce.is_empty() || parsed.nonce.len() > 1024 {
        // Wrap before error so all parsed secret-bearing fields are erased.
        drop(Challenge(parsed));
        return Err(AuthError::InvalidChallenge);
    }
    // digest_auth 0.3.1 omits cnonce in its no-qop serialization. A -sess
    // algorithm still needs that value, so this combination is unsupported.
    if parsed.algorithm.sess && parsed.qop.is_none() {
        drop(Challenge(parsed));
        return Err(AuthError::UnsupportedScheme);
    }
    // Choose auth when offered; otherwise preserve auth-int for the empty GET body.
    if parsed.qop.as_ref().is_some_and(|q| q.contains(&Qop::AUTH)) {
        parsed.qop = Some(vec![Qop::AUTH]);
    }
    Ok(Challenge(parsed))
}

fn erase_answer(answer: &mut AuthorizationHeader) {
    answer.realm.zeroize();
    answer.nonce.zeroize();
    answer.response.zeroize();
    answer.username.zeroize();
    answer.uri.zeroize();
    if let Some(value) = &mut answer.opaque {
        value.zeroize();
    }
    if let Some(value) = &mut answer.cnonce {
        value.zeroize();
    }
}

impl ManagementAuthenticator {
    pub fn digest(challenge: &str, credentials: Credentials) -> Result<Self, AuthError> {
        let challenge = checked_challenge(challenge)?;
        if (challenge.0.charset != digest_auth::Charset::UTF8
            && (!credentials.username.is_ascii() || !credentials.password.is_ascii()))
            || (!credentials.username.is_ascii() && !challenge.0.userhash)
        {
            return Err(AuthError::UnsupportedScheme);
        }
        Ok(Self {
            challenge,
            credentials,
        })
    }
    pub fn authorization(
        &mut self,
        route: &VerifiedReadRoute,
    ) -> Result<Zeroizing<String>, AuthError> {
        self.authorization_inner(route, None)
    }
    fn authorization_inner(
        &mut self,
        route: &VerifiedReadRoute,
        cnonce: Option<&str>,
    ) -> Result<Zeroizing<String>, AuthError> {
        if self.challenge.0.nc == u32::MAX {
            return Err(AuthError::NonceExhausted);
        }
        let uri = route.uri();
        if !uri.starts_with('/')
            || uri.starts_with("//")
            || uri.contains('#')
            || uri.contains('"')
            || uri.contains('\\')
            || uri.chars().any(char::is_control)
        {
            return Err(AuthError::UnqualifiedRoute);
        }
        let mut context = AuthContext::new(
            self.credentials.username.as_str(),
            self.credentials.password.as_str(),
            uri,
        );
        // RFC auth-int on GET hashes the empty entity body instead of downgrading.
        context.body = Some((&[][..]).into());
        if let Some(value) = cnonce {
            context.set_custom_cnonce(value);
        }
        let mut answer = self
            .challenge
            .0
            .respond(&context)
            .map_err(|_| AuthError::InvalidChallenge)?;
        let header = Zeroizing::new(answer.to_header_string());
        erase_answer(&mut answer);
        if header.chars().any(char::is_control) {
            return Err(AuthError::InvalidChallenge);
        }
        Ok(header)
    }
    pub fn replace_stale_challenge(&mut self, fresh: &str) -> Result<(), AuthError> {
        let next = checked_challenge(fresh)?;
        if !next.0.stale
            || next.0.nonce == self.challenge.0.nonce
            || next.0.realm != self.challenge.0.realm
            || next.0.algorithm != self.challenge.0.algorithm
            || (self.challenge.0.qop.is_some() && next.0.qop.is_none())
        {
            return Err(AuthError::AuthenticationFailed);
        }
        self.challenge = next;
        Ok(())
    }
}

#[cfg(test)]
#[path = "management_auth_flow_tests.rs"]
mod flow_tests;
#[cfg(test)]
#[path = "management_auth_tests.rs"]
mod tests;
