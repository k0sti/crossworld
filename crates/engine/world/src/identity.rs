//! Identity system for Nostr-based player identification

/// Nostr-based identity trait
///
/// Represents a player's identity information from Nostr.
pub trait Identity {
    /// Get the player's Nostr public key (npub format)
    fn npub(&self) -> &str;

    /// Get the player's display name (from Nostr profile)
    fn display_name(&self) -> Option<&str>;

    /// Get the player's avatar URL (from Nostr profile)
    fn avatar_url(&self) -> Option<&str>;
}
