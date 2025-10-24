# Emoji Hash - User Identity Visualization

## Motivation

Display a visually distinctive 4-emoji sequence derived from a user's public key (npub) to help users quickly identify other users in the Crossworld metaverse without requiring them to remember long hexadecimal strings.

## Concept

Convert a hex pubkey string into a deterministic 4-emoji hash that serves as a visual identifier:
- **Easy recognition**: Emojis are more memorable than hex strings
- **Deterministic**: Same pubkey always generates the same emoji sequence
- **Collision-resistant**: Different pubkeys generate different sequences
- **Visually diverse**: Selected emoji set is easily distinguishable

## Implementation Details

### Emoji Set (64 emojis)

Selected for visual diversity and easy distinguishability:
- Nature: 🌟 🔥 💧 🌈 ⚡ 🌙 ☀️ 🌸 🍀 🌺 🌵 🍄 🌊 🏔️ 🌋 ⛰️
- Animals: 🐱 🐶 🐺 🦊 🐯 🦁 🐮 🐷 🐸 🐵 🐔 🐧 🐦 🦅 🦉 🦋
- Sea creatures: 🐢 🐍 🦎 🦀 🐙 🦑 🐠 🐡
- Symbols: ⭐ 💫 ✨ 🔮 💎 🎨 🎭 🎪
- Objects: 🎯 🎲 🎸 🎺 🎻 🥁 🎹 🎤 🚀 🛸 ⚓ 🏹 🗡️ 🛡️ 👑 💍

### Algorithm

1. Hash the pubkey hex string using `DefaultHasher`
2. Generate 4 emojis from the hash:
   - Use modulo operation to select emoji index
   - Use LCG (Linear Congruential Generator) for subsequent values
   - Formula: `remaining = remaining * 6364136223846793005 + 1`

### Properties

- **Deterministic**: Same input always produces same output
- **Distributed**: Hash function ensures even distribution across emoji set
- **Fixed length**: Always 4 emojis
- **No cryptographic guarantees**: Uses standard hash, not cryptographic

## Usage Location

Originally intended for:
- Profile panels
- User identification in UI
- Quick visual reference in chat/network views

## Implementation Files (Pre-removal)

- Rust: `crates/world/src/emoji_hash.rs` - Core algorithm
- Rust: `crates/world/src/lib.rs` - WASM binding `pubkey_to_emoji()`
- TypeScript: `packages/app/src/App.tsx` - Import and usage
- TypeScript: `packages/common/src/components/ProfilePanel.tsx` - Display logic (disabled by `SHOW_EMOJI_HASH = false`)

## Future Considerations

If reimplemented:
- Consider using cryptographic hash for better collision resistance
- Add color variation for additional uniqueness
- Support different emoji sets for theming
- Add accessibility options (text-based alternative)
- Consider using NIP-05 verified names when available as primary identifier
