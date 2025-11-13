use crate::protocol::{handshake_message, AuthLevel, Handshake};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

mod npub;
pub use npub::{verify_signature, NpubError};

/// Authentication configuration driven by environment variables.
#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub admin_npubs: Vec<String>,
    pub user_npubs: Option<Vec<String>>,
    pub allow_anonymous_read: bool,
    pub max_timestamp_age: u64,
}

impl AuthConfig {
    pub fn from_env() -> Self {
        let admin_npubs = read_list("CROSSWORLD_ADMIN_NPUBS").unwrap_or_default();
        let user_npubs = read_list("CROSSWORLD_USER_NPUBS");
        let allow_anonymous_read = std::env::var("CROSSWORLD_ALLOW_ANON")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        let max_timestamp_age = std::env::var("CROSSWORLD_MAX_TS_AGE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(60);

        Self {
            admin_npubs,
            user_npubs,
            allow_anonymous_read,
            max_timestamp_age,
        }
    }

    pub fn determine_auth_level(&self, npub: &str) -> AuthLevel {
        if self.admin_npubs.iter().any(|n| n == npub) {
            return AuthLevel::Admin;
        }

        if let Some(users) = &self.user_npubs {
            if users.iter().any(|n| n == npub) {
                return AuthLevel::User;
            }
            return if self.allow_anonymous_read {
                AuthLevel::ReadOnly
            } else {
                AuthLevel::ReadOnly
            };
        }

        AuthLevel::User
    }
}

fn read_list(var: &str) -> Option<Vec<String>> {
    std::env::var(var)
        .ok()
        .and_then(|value| {
            let items: Vec<String> = value
                .split(',')
                .filter_map(|entry| {
                    let trimmed = entry.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                })
                .collect();
            if items.is_empty() {
                None
            } else {
                Some(items)
            }
        })
}

/// Errors that can occur during authentication.
#[derive(Debug, Error)]
pub enum AuthError {
    #[error("handshake timestamp is too old")]
    ExpiredTimestamp,
    #[error("invalid signature: {0}")]
    InvalidSignature(NpubError),
    #[error("nostr verification disabled at build time")]
    VerificationUnavailable,
    #[error("authentication is required")]
    AuthenticationRequired,
}

/// Authentication manager responsible for verifying handshakes.
#[derive(Debug, Clone)]
pub struct AuthManager {
    config: AuthConfig,
}

impl AuthManager {
    pub fn new(config: AuthConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &AuthConfig {
        &self.config
    }

    pub fn verify_handshake(&self, server_url: &str, handshake: &Handshake) -> Result<AuthLevel, AuthError> {
        if handshake.npub.is_empty() {
            if self.config.allow_anonymous_read {
                return Ok(AuthLevel::ReadOnly);
            }
            return Err(AuthError::AuthenticationRequired);
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is set before UNIX epoch")
            .as_secs();

        if now.saturating_sub(handshake.timestamp) > self.config.max_timestamp_age {
            return Err(AuthError::ExpiredTimestamp);
        }

        let message = handshake_message(server_url, handshake.timestamp);
        if let Err(err) = npub::verify_signature(&handshake.npub, &handshake.signature, &message) {
            if matches!(err, NpubError::Disabled) {
                return Err(AuthError::VerificationUnavailable);
            }
            return Err(AuthError::InvalidSignature(err));
        }

        Ok(self.config.determine_auth_level(&handshake.npub))
    }
}
