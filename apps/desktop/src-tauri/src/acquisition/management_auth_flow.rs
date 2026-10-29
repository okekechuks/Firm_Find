//! Bounded authentication driver. The caller supplies a trusted connector, never HAR URLs.
use super::{AuthError, Credentials, ManagementAuthenticator, VerifiedReadRoute};
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};
use zeroize::Zeroizing;

/// Opaque operation permit; construction requires a compiled, qualified read route.
pub struct ReadAuthorization {
    pub(super) route: VerifiedReadRoute,
    pub(super) binding: [u8; 32],
    pub(super) expires: Instant,
    used: AtomicBool,
}

impl ReadAuthorization {
    /// Call only after the dedicated native per-session approval. This is not an IPC DTO.
    pub fn after_owner_approval(
        route: VerifiedReadRoute,
        binding: [u8; 32],
        expires: Instant,
    ) -> Self {
        Self {
            route,
            binding,
            expires,
            used: AtomicBool::new(false),
        }
    }
}

/// Secret-bearing response fields cannot enter diagnostics through Debug/Serialize.
pub struct IdentityReadResponse {
    pub status: u16,
    pub challenge: Option<Zeroizing<String>>,
    /// A qualified family parser compares its selected expected identity after binding.
    pub identity_matches_selected: bool,
}

pub trait IdentityReadConnector {
    /// Re-query selected adapter/interface/source/route and identity consistency.
    fn current_binding(&mut self) -> Result<[u8; 32], AuthError>;
    /// Must perform exactly this qualified GET, without redirects or automatic retries.
    fn get(
        &mut self,
        route: &VerifiedReadRoute,
        authorization: Option<&str>,
    ) -> Result<IdentityReadResponse, AuthError>;
}

fn checkpoint(
    permit: &ReadAuthorization,
    cancel: &AtomicBool,
    connector: &mut impl IdentityReadConnector,
) -> Result<(), AuthError> {
    if cancel.load(Ordering::Acquire) {
        return Err(AuthError::Cancelled);
    }
    if Instant::now() >= permit.expires {
        return Err(AuthError::AuthorizationExpired);
    }
    if connector.current_binding()? != permit.binding {
        return Err(AuthError::DeviceChanged);
    }
    Ok(())
}

/// Exactly one fresh unauthenticated identity GET, one response, and at most one
/// fresh stale-nonce retry. No login action or cookie/form inference is performed.
pub fn authenticate_digest(
    permit: &ReadAuthorization,
    credentials: Credentials,
    cancel: &AtomicBool,
    connector: &mut impl IdentityReadConnector,
) -> Result<(), AuthError> {
    permit
        .used
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .map_err(|_| AuthError::ApprovalUsed)?;
    checkpoint(permit, cancel, connector)?;
    let first = connector.get(&permit.route, None)?;
    checkpoint(permit, cancel, connector)?;
    if first.status != 401 {
        return Err(AuthError::InvalidChallenge);
    }
    let challenge = first
        .challenge
        .as_deref()
        .ok_or(AuthError::InvalidChallenge)?;
    let mut auth = ManagementAuthenticator::digest(challenge, credentials)?;
    for attempt in 0..2 {
        checkpoint(permit, cancel, connector)?;
        let authorization = auth.authorization(&permit.route)?;
        let response = connector.get(&permit.route, Some(&authorization))?;
        checkpoint(permit, cancel, connector)?;
        if (200..300).contains(&response.status) {
            return if response.identity_matches_selected {
                Ok(())
            } else {
                Err(AuthError::DeviceChanged)
            };
        }
        if response.status != 401 {
            return Err(AuthError::AuthenticationFailed);
        }
        if attempt != 0 {
            return Err(AuthError::RetryLimit);
        }
        auth.replace_stale_challenge(
            response
                .challenge
                .as_deref()
                .ok_or(AuthError::AuthenticationFailed)?,
        )?;
    }
    Err(AuthError::RetryLimit)
}
