use super::{AdapterError, AdapterErrorCode, AuthenticationState};
/// Only the backend connector can advance authentication; this type is never deserialized.
pub struct AuthenticationLifecycle {
    state: AuthenticationState,
}
impl Default for AuthenticationLifecycle {
    fn default() -> Self {
        Self {
            state: AuthenticationState::Required,
        }
    }
}
impl AuthenticationLifecycle {
    pub fn state(&self) -> AuthenticationState {
        self.state
    }
    pub fn begin(&mut self) -> Result<(), AdapterError> {
        if self.state != AuthenticationState::Required {
            return Err(AdapterError::new(
                AdapterErrorCode::AuthenticationFailed,
                "Authentication transition is invalid.",
                None,
                false,
                "Start a fresh connection.",
            ));
        }
        self.state = AuthenticationState::Authenticating;
        Ok(())
    }
    pub fn accept(&mut self) -> Result<(), AdapterError> {
        if self.state != AuthenticationState::Authenticating {
            return Err(AdapterError::new(
                AdapterErrorCode::AuthenticationRequired,
                "Authentication handshake has not started.",
                None,
                false,
                "Start a fresh connection.",
            ));
        }
        self.state = AuthenticationState::Authenticated;
        Ok(())
    }
    pub fn reject(&mut self) {
        self.state = AuthenticationState::Failed;
    }
    pub fn expire(&mut self) {
        self.state = AuthenticationState::Expired;
    }
}
