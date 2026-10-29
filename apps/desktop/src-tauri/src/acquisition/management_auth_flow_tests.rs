//! Synthetic, controlled connector only. No sockets, credentials or device traffic.
use super::*;
use std::{
    collections::VecDeque,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
use zeroize::Zeroizing;

const CHALLENGE: &str = r#"Digest realm="synthetic", nonce="first", algorithm=SHA-256, qop="auth""#;

struct Connector<'a> {
    responses: VecDeque<Result<IdentityReadResponse, AuthError>>,
    requests: Vec<bool>,
    binding: [u8; 32],
    change_on_request: Option<usize>,
    cancel_on_request: Option<(&'a AtomicBool, usize)>,
}
impl IdentityReadConnector for Connector<'_> {
    fn current_binding(&mut self) -> Result<[u8; 32], AuthError> {
        Ok(self.binding)
    }
    fn get(
        &mut self,
        route: &VerifiedReadRoute,
        authorization: Option<&str>,
    ) -> Result<IdentityReadResponse, AuthError> {
        assert_eq!(route.uri(), "/synthetic/identity?view=summary");
        if let Some(value) = authorization {
            assert!(value.starts_with("Digest "));
            assert!(value.contains(r#"uri="/synthetic/identity?view=summary""#));
            assert!(!value.contains("synthetic-password"));
        }
        self.requests.push(authorization.is_some());
        if self.change_on_request == Some(self.requests.len()) {
            self.binding = [9; 32];
        }
        if let Some((cancel, index)) = self.cancel_on_request {
            if index == self.requests.len() {
                cancel.store(true, Ordering::Release);
            }
        }
        self.responses
            .pop_front()
            .expect("bounded expected request count")
    }
}
fn response(
    status: u16,
    challenge: Option<&str>,
    matches: bool,
) -> Result<IdentityReadResponse, AuthError> {
    Ok(IdentityReadResponse {
        status,
        challenge: challenge.map(|s| Zeroizing::new(s.to_string())),
        identity_matches_selected: matches,
    })
}
fn permit() -> ReadAuthorization {
    ReadAuthorization::after_owner_approval(
        VerifiedReadRoute {
            uri: "/synthetic/identity?view=summary",
        },
        [1; 32],
        Instant::now() + Duration::from_secs(10),
    )
}
fn credentials() -> Credentials {
    Credentials::new("synthetic-user".into(), "synthetic-password".into()).unwrap()
}
fn connector<'a>(responses: Vec<Result<IdentityReadResponse, AuthError>>) -> Connector<'a> {
    Connector {
        responses: responses.into(),
        requests: vec![],
        binding: [1; 32],
        change_on_request: None,
        cancel_on_request: None,
    }
}

#[test]
fn controlled_first_read_uses_fresh_challenge_and_validates_identity() {
    let cancel = AtomicBool::new(false);
    let mut connector = connector(vec![
        response(401, Some(CHALLENGE), false),
        response(200, None, true),
    ]);
    let permit = permit();
    authenticate_digest(&permit, credentials(), &cancel, &mut connector).unwrap();
    assert_eq!(connector.requests, [false, true]);
    assert_eq!(
        authenticate_digest(&permit, credentials(), &cancel, &mut connector),
        Err(AuthError::ApprovalUsed)
    );
    assert_eq!(connector.requests, [false, true]);
}

#[test]
fn wrong_credentials_never_retry_and_identity_mismatch_never_succeeds() {
    let cancel = AtomicBool::new(false);
    let mut wrong = connector(vec![
        response(401, Some(CHALLENGE), false),
        response(401, Some(CHALLENGE), false),
    ]);
    assert_eq!(
        authenticate_digest(&permit(), credentials(), &cancel, &mut wrong),
        Err(AuthError::AuthenticationFailed)
    );
    assert_eq!(wrong.requests, [false, true]);
    let mut identity = connector(vec![
        response(401, Some(CHALLENGE), false),
        response(200, None, false),
    ]);
    assert_eq!(
        authenticate_digest(&permit(), credentials(), &cancel, &mut identity),
        Err(AuthError::DeviceChanged)
    );
    assert_eq!(identity.requests, [false, true]);
}

#[test]
fn only_one_fresh_stale_nonce_retry_is_allowed() {
    let cancel = AtomicBool::new(false);
    let stale =
        r#"Digest realm="synthetic",nonce="second",algorithm=SHA-256,qop="auth",stale=true"#;
    let mut accepted = connector(vec![
        response(401, Some(CHALLENGE), false),
        response(401, Some(stale), false),
        response(200, None, true),
    ]);
    authenticate_digest(&permit(), credentials(), &cancel, &mut accepted).unwrap();
    assert_eq!(accepted.requests, [false, true, true]);
    let mut repeat = connector(vec![
        response(401, Some(CHALLENGE), false),
        response(401, Some(stale), false),
        response(401, Some(stale), false),
    ]);
    assert_eq!(
        authenticate_digest(&permit(), credentials(), &cancel, &mut repeat),
        Err(AuthError::RetryLimit)
    );
    assert_eq!(repeat.requests.len(), 3);
}

#[test]
fn expiry_device_change_cancel_and_timeout_are_fail_closed() {
    let cancel = AtomicBool::new(false);
    let mut expired = permit();
    expired.expires = Instant::now();
    let mut no_request = connector(vec![]);
    assert_eq!(
        authenticate_digest(&expired, credentials(), &cancel, &mut no_request),
        Err(AuthError::AuthorizationExpired)
    );
    assert!(no_request.requests.is_empty());
    let mut changed = connector(vec![response(401, Some(CHALLENGE), false)]);
    changed.change_on_request = Some(1);
    assert_eq!(
        authenticate_digest(&permit(), credentials(), &cancel, &mut changed),
        Err(AuthError::DeviceChanged)
    );
    assert_eq!(changed.requests, [false]);
    let mut cancelled = connector(vec![response(401, Some(CHALLENGE), false)]);
    cancelled.cancel_on_request = Some((&cancel, 1));
    assert_eq!(
        authenticate_digest(&permit(), credentials(), &cancel, &mut cancelled),
        Err(AuthError::Cancelled)
    );
    assert_eq!(cancelled.requests, [false]);
    cancel.store(false, Ordering::Release);
    let mut timeout = connector(vec![Err(AuthError::Timeout)]);
    assert_eq!(
        authenticate_digest(&permit(), credentials(), &cancel, &mut timeout),
        Err(AuthError::Timeout)
    );
    assert_eq!(timeout.requests, [false]);
}
