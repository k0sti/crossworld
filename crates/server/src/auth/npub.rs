use thiserror::Error;

#[cfg(feature = "nostr")]
use {
    nostr_sdk::prelude::*,
    nostr_sdk::secp256k1::{Message, Secp256k1},
    sha2::{Digest, Sha256},
};

/// Errors related to handling Nostr public keys and signatures.
#[derive(Debug, Error)]
pub enum NpubError {
    #[cfg(feature = "nostr")]
    #[error("invalid nostr public key: {0}")]
    InvalidPublicKey(nostr_sdk::prelude::Error),
    #[cfg(feature = "nostr")]
    #[error("invalid signature: {0}")]
    InvalidSignature(nostr_sdk::secp256k1::Error),
    #[cfg(not(feature = "nostr"))]
    #[error("nostr verification feature disabled")]
    Disabled,
}

#[cfg(feature = "nostr")]
pub fn verify_signature(npub: &str, signature: &[u8], message: &str) -> Result<(), NpubError> {
    let pubkey = if npub.starts_with("npub") {
        PublicKey::from_bech32(npub).map_err(NpubError::InvalidPublicKey)?
    } else {
        PublicKey::from_hex(npub).map_err(NpubError::InvalidPublicKey)?
    };

    let sig = Signature::from_slice(signature).map_err(NpubError::InvalidSignature)?;

    let hash = Sha256::digest(message.as_bytes());
    let msg = Message::from_slice(&hash).map_err(NpubError::InvalidSignature)?;
    let secp = Secp256k1::verification_only();

    secp.verify_schnorr(&sig, &msg, &pubkey.xonly_public_key())
        .map_err(NpubError::InvalidSignature)?;

    Ok(())
}

#[cfg(not(feature = "nostr"))]
pub fn verify_signature(_npub: &str, _signature: &[u8], _message: &str) -> Result<(), NpubError> {
    Err(NpubError::Disabled)
}
