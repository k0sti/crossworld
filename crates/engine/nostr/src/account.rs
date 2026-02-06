//! Nostr account state management
//!
//! Provides account abstraction and state tracking for the editor.

use nostr::prelude::*;

/// Represents a logged-in Nostr account
///
/// Can be created from full keys (for local signing) or just a public key
/// (for NIP-46 remote signing).
#[derive(Debug, Clone)]
pub struct NostrAccount {
    /// The Nostr keypair (None for remote-signer accounts)
    keys: Option<Keys>,
    /// Public key (always available)
    pubkey: PublicKey,
}

impl NostrAccount {
    /// Create an account from existing keys (full signing capability)
    pub fn from_keys(keys: Keys) -> Self {
        let pubkey = keys.public_key();
        Self {
            keys: Some(keys),
            pubkey,
        }
    }

    /// Create an account from just a public key (for NIP-46 remote signing)
    ///
    /// This creates an account that can display the user's identity but
    /// requires a remote signer for any signing operations.
    pub fn from_pubkey(pubkey: PublicKey) -> Self {
        Self { keys: None, pubkey }
    }

    /// Check if this account has local signing keys
    pub fn has_local_keys(&self) -> bool {
        self.keys.is_some()
    }

    /// Get the public key in npub format (bech32)
    pub fn npub(&self) -> String {
        self.pubkey
            .to_bech32()
            .unwrap_or_else(|_| self.public_key_hex())
    }

    /// Get the private key in nsec format (bech32)
    /// Returns None for remote-signer accounts
    pub fn nsec(&self) -> Option<String> {
        self.keys.as_ref().map(|k| {
            k.secret_key()
                .to_bech32()
                .unwrap_or_else(|_| "error".to_string())
        })
    }

    /// Get the public key in hex format
    pub fn public_key_hex(&self) -> String {
        self.pubkey.to_hex()
    }

    /// Get the short display name (first 12 chars of npub + ...)
    pub fn short_npub(&self) -> String {
        let npub = self.npub();
        format!("{}...", &npub[..12.min(npub.len())])
    }

    /// Get a reference to the underlying keys (if available)
    pub fn keys(&self) -> Option<&Keys> {
        self.keys.as_ref()
    }

    /// Get the public key
    pub fn public_key(&self) -> &PublicKey {
        &self.pubkey
    }
}

/// Current account state for the application
#[derive(Debug, Clone, Default)]
pub struct AccountState {
    /// The currently logged-in account, if any
    account: Option<NostrAccount>,
    /// Whether the login dialog should be shown
    show_login_dialog: bool,
    /// Pending nsec input in the login dialog
    nsec_input: String,
    /// Error message to display in the login dialog
    error_message: Option<String>,
}

impl AccountState {
    /// Create a new empty account state
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if a user is logged in
    pub fn is_logged_in(&self) -> bool {
        self.account.is_some()
    }

    /// Get the current account, if logged in
    pub fn account(&self) -> Option<&NostrAccount> {
        self.account.as_ref()
    }

    /// Log in with an account
    pub fn login(&mut self, account: NostrAccount) {
        self.account = Some(account);
        self.show_login_dialog = false;
        self.nsec_input.clear();
        self.error_message = None;
    }

    /// Log out the current account
    pub fn logout(&mut self) {
        self.account = None;
    }

    /// Get display text for the login button
    pub fn button_text(&self) -> String {
        if let Some(account) = &self.account {
            account.short_npub()
        } else {
            "Nostr Login".to_string()
        }
    }

    /// Check if the login dialog should be shown
    pub fn should_show_dialog(&self) -> bool {
        self.show_login_dialog
    }

    /// Open the login dialog
    pub fn open_dialog(&mut self) {
        self.show_login_dialog = true;
        self.error_message = None;
    }

    /// Close the login dialog
    pub fn close_dialog(&mut self) {
        self.show_login_dialog = false;
        self.nsec_input.clear();
        self.error_message = None;
    }

    /// Get mutable reference to nsec input
    pub fn nsec_input_mut(&mut self) -> &mut String {
        &mut self.nsec_input
    }

    /// Get the nsec input
    pub fn nsec_input(&self) -> &str {
        &self.nsec_input
    }

    /// Set an error message
    pub fn set_error(&mut self, message: String) {
        self.error_message = Some(message);
    }

    /// Get the current error message
    pub fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }

    /// Clear the error message
    pub fn clear_error(&mut self) {
        self.error_message = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_from_keys() {
        let keys = Keys::generate();
        let account = NostrAccount::from_keys(keys.clone());

        assert!(account.npub().starts_with("npub1"));
        assert!(account.nsec().unwrap().starts_with("nsec1"));
        assert_eq!(account.public_key_hex().len(), 64);
        assert!(account.has_local_keys());
    }

    #[test]
    fn test_account_from_pubkey() {
        let keys = Keys::generate();
        let pubkey = keys.public_key();
        let account = NostrAccount::from_pubkey(pubkey);

        assert!(account.npub().starts_with("npub1"));
        assert!(account.nsec().is_none()); // No private key for remote accounts
        assert_eq!(account.public_key_hex().len(), 64);
        assert!(!account.has_local_keys());
    }

    #[test]
    fn test_short_npub() {
        let keys = Keys::generate();
        let account = NostrAccount::from_keys(keys);

        let short = account.short_npub();
        assert!(short.ends_with("..."));
        assert!(short.len() <= 16);
    }

    #[test]
    fn test_account_state_login_logout() {
        let mut state = AccountState::new();

        // Initially not logged in
        assert!(!state.is_logged_in());
        assert!(state.account().is_none());

        // Log in
        let keys = Keys::generate();
        let account = NostrAccount::from_keys(keys);
        state.login(account);

        assert!(state.is_logged_in());
        assert!(state.account().is_some());

        // Log out
        state.logout();
        assert!(!state.is_logged_in());
        assert!(state.account().is_none());
    }

    #[test]
    fn test_account_state_button_text() {
        let mut state = AccountState::new();

        // Not logged in
        assert_eq!(state.button_text(), "Nostr Login");

        // Logged in
        let keys = Keys::generate();
        let account = NostrAccount::from_keys(keys);
        state.login(account);

        let text = state.button_text();
        assert!(text.starts_with("npub1"));
        assert!(text.ends_with("..."));
    }

    #[test]
    fn test_account_state_dialog() {
        let mut state = AccountState::new();

        // Dialog initially closed
        assert!(!state.should_show_dialog());

        // Open dialog
        state.open_dialog();
        assert!(state.should_show_dialog());

        // Close dialog
        state.close_dialog();
        assert!(!state.should_show_dialog());
    }

    #[test]
    fn test_account_state_error() {
        let mut state = AccountState::new();

        // No error initially
        assert!(state.error_message().is_none());

        // Set error
        state.set_error("Test error".to_string());
        assert_eq!(state.error_message(), Some("Test error"));

        // Clear error
        state.clear_error();
        assert!(state.error_message().is_none());
    }
}
