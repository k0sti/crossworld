use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Predefined set of emojis for hash generation
/// Selected to be easily distinguishable and visually diverse
const EMOJI_SET: &[&str] = &[
    "🌟", "🔥", "💧", "🌈", "⚡", "🌙", "☀️", "🌸",
    "🍀", "🌺", "🌵", "🍄", "🌊", "🏔️", "🌋", "⛰️",
    "🐱", "🐶", "🐺", "🦊", "🐯", "🦁", "🐮", "🐷",
    "🐸", "🐵", "🐔", "🐧", "🐦", "🦅", "🦉", "🦋",
    "🐢", "🐍", "🦎", "🦀", "🐙", "🦑", "🐠", "🐡",
    "⭐", "💫", "✨", "🔮", "💎", "🎨", "🎭", "🎪",
    "🎯", "🎲", "🎸", "🎺", "🎻", "🥁", "🎹", "🎤",
    "🚀", "🛸", "⚓", "🏹", "🗡️", "🛡️", "👑", "💍",
];

/// Convert a hex pubkey string to a 4-emoji hash
pub fn pubkey_to_emoji_hash(pubkey_hex: &str) -> String {
    // Hash the pubkey
    let mut hasher = DefaultHasher::new();
    pubkey_hex.hash(&mut hasher);
    let hash = hasher.finish();

    // Generate 4 emojis from the hash
    let mut result = String::new();
    let mut remaining = hash;

    for _ in 0..4 {
        let index = (remaining % (EMOJI_SET.len() as u64)) as usize;
        result.push_str(EMOJI_SET[index]);
        remaining = remaining.wrapping_mul(6364136223846793005).wrapping_add(1);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emoji_hash_length() {
        let hash = pubkey_to_emoji_hash("test_pubkey_123");
        assert_eq!(hash.chars().count(), 4);
    }

    #[test]
    fn test_emoji_hash_deterministic() {
        let hash1 = pubkey_to_emoji_hash("same_pubkey");
        let hash2 = pubkey_to_emoji_hash("same_pubkey");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_emoji_hash_different() {
        let hash1 = pubkey_to_emoji_hash("pubkey1");
        let hash2 = pubkey_to_emoji_hash("pubkey2");
        assert_ne!(hash1, hash2);
    }
}
