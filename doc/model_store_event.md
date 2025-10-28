# World Model Storage Event (Nostr)

## Overview

This document specifies a Nostr event type for storing and retrieving voxel world data in Crossworld. The event uses the CSM (Cube Script Model) format for storing octree-based voxel models in an addressable, replaceable event format.

## Event Specification

### Event Kind

**Kind: 30078** (Application-specific Data, Addressable)

This is an addressable/replaceable event, meaning only the latest event for a given combination of `kind`, `pubkey`, and `d` tag will be stored by relays.

### Event Structure

```typescript
interface WorldModelEvent {
  kind: 30078
  pubkey: string              // Creator's public key
  created_at: number          // Unix timestamp
  content: string             // CSM script code
  tags: [
    ['d', string],            // d-tag: world identifier (format below)
    ['a', string],            // Optional: reference to live activity
    ['title', string],        // Optional: human-readable world name
    ['description', string],  // Optional: world description
    ['macro', string],        // MacroDepth value (for quick filtering)
    ['micro', string],        // MicroDepth value (for quick filtering)
  ]
  id: string                  // Event ID
  sig: string                 // Signature
}
```

### d-tag Format

The d-tag uniquely identifies a world configuration:

```
d-tag = [OctantPath] ":" MacroDepth [":" MicroDepth]
```

**Components:**
- `OctantPath` (optional): Octant location within a larger world structure
  - Format: `[a-h]+` (e.g., "abc", "aaa", "efg")
  - Can be empty for root/default worlds
  - See CSM octant layout for coordinate mapping
- `MacroDepth` (required): World octree subdivision levels
  - Range: 1-10
  - Determines world size: 2^macro units
  - Example: 3 → 8×8×8 world units
- `MicroDepth` (optional): Sub-unit voxel subdivisions
  - Range: 0-3
  - Determines subdivisions per world unit: 2^micro voxels
  - Defaults to 0 if omitted
  - Example: 2 → 4×4×4 subdivisions per unit

**Examples:**
```
:3          → Default world, macro=3, micro=0
:3:2        → Default world, macro=3, micro=2
abc:4       → World at octant "abc", macro=4, micro=0
abc:4:1     → World at octant "abc", macro=4, micro=1
:8:3        → Large world, macro=8, micro=3
```

### Content Format

The event `content` field contains CSM (Cube Script Model) script code.

**CSM Format:**
- Text-based octree representation
- Supports hierarchical structures, arrays, epochs, and transformations
- Human-readable and editable
- See `doc/cube-script-model.md` for complete specification

**Example CSM content:**
```csm
# Simple humanoid avatar
>d [100 100 100 100 100 100 100 100]
>dd [150 150 150 150 150 150 150 150]
>c [80 80 80 80 80 80 80 80]
>cd [90 90 90 90 90 90 90 90]
>cf [70 70 70 70 0 0 0 0]
>cg [70 70 70 70 0 0 0 0]
>a [60 60 0 0 0 0 0 0]
>b [60 60 0 0 0 0 0 0]
```

### Tags

#### Required Tags

- **`d` tag**: World identifier using the format specified above
  ```
  ['d', ':3:2']
  ```

#### Optional Tags

- **`a` tag**: Reference to associated live activity (NIP-33)
  ```
  ['a', '30311:<pubkey>:<d-tag>']
  ```

- **`title` tag**: Human-readable world name
  ```
  ['title', 'My Awesome World']
  ```

- **`description` tag**: World description
  ```
  ['description', 'A procedurally generated castle with gardens']
  ```

- **`macro` tag**: Macro depth value (for filtering/querying)
  ```
  ['macro', '3']
  ```

- **`micro` tag**: Micro depth value (for filtering/querying)
  ```
  ['micro', '2']
  ```

## Usage

### Publishing a World

```typescript
import { SimplePool, getPublicKey, finishEvent } from 'nostr-tools'
import { getMacroDepth, getMicroDepth } from '../config/depth-config'
import { getModelCSM } from '../utils/csmSaver'

async function publishWorld(
  signer: any,
  octantPath: string = '',
  title?: string,
  description?: string
) {
  const pool = new SimplePool()
  const pubkey = await signer.getPublicKey()
  const macroDepth = getMacroDepth()
  const microDepth = getMicroDepth()

  // Generate d-tag
  const dTag = microDepth > 0
    ? `${octantPath}:${macroDepth}:${microDepth}`
    : `${octantPath}:${macroDepth}`

  // Get CSM content
  const csmContent = getModelCSM('world')

  // Build event
  const event = {
    kind: 30078,
    created_at: Math.floor(Date.now() / 1000),
    tags: [
      ['d', dTag],
      ['macro', macroDepth.toString()],
      ['micro', microDepth.toString()],
      ...(title ? [['title', title]] : []),
      ...(description ? [['description', description]] : []),
    ],
    content: csmContent,
  }

  const signedEvent = await signer.signEvent(event)
  await pool.publish(RELAYS, signedEvent)

  return signedEvent
}
```

### Fetching Worlds

```typescript
import { SimplePool } from 'nostr-tools'

async function fetchUserWorlds(pubkey: string, relays: string[]) {
  const pool = new SimplePool()

  const events = await pool.querySync(relays, {
    kinds: [30078],
    authors: [pubkey],
  })

  return events.map(event => ({
    pubkey: event.pubkey,
    dTag: event.tags.find(t => t[0] === 'd')?.[1] || '',
    title: event.tags.find(t => t[0] === 'title')?.[1],
    description: event.tags.find(t => t[0] === 'description')?.[1],
    macroDepth: parseInt(event.tags.find(t => t[0] === 'macro')?.[1] || '3'),
    microDepth: parseInt(event.tags.find(t => t[0] === 'micro')?.[1] || '0'),
    csmCode: event.content,
    createdAt: event.created_at,
  }))
}

async function fetchSpecificWorld(
  pubkey: string,
  dTag: string,
  relays: string[]
) {
  const pool = new SimplePool()

  const event = await pool.get(relays, {
    kinds: [30078],
    authors: [pubkey],
    '#d': [dTag],
  })

  return event
}
```

### Loading a World

```typescript
import { loadWorldFromCSM } from '../utils/csmSaver'
import { setMacroDepth, setMicroDepth } from '../config/depth-config'

async function loadWorldFromEvent(event: Event) {
  const dTag = event.tags.find(t => t[0] === 'd')?.[1]
  if (!dTag) throw new Error('Missing d-tag')

  // Parse d-tag
  const parts = dTag.split(':')
  let octantPath = ''
  let macroDepth = 3
  let microDepth = 0

  if (parts.length === 2) {
    // Format: ":3" or "abc:3"
    octantPath = parts[0]
    macroDepth = parseInt(parts[1])
  } else if (parts.length === 3) {
    // Format: ":3:2" or "abc:3:2"
    octantPath = parts[0]
    macroDepth = parseInt(parts[1])
    microDepth = parseInt(parts[2])
  }

  // Update depth configuration
  setMacroDepth(macroDepth)
  setMicroDepth(microDepth)

  // Load CSM content
  const totalDepth = macroDepth + microDepth
  loadWorldFromCSM(event.content, 'world', totalDepth)

  return {
    octantPath,
    macroDepth,
    microDepth,
    totalDepth
  }
}
```

### Auto-load Current Configuration

On application start or when user switches worlds, automatically load the world matching current configuration:

```typescript
async function autoLoadWorld(pubkey: string, relays: string[]) {
  const macroDepth = getMacroDepth()
  const microDepth = getMicroDepth()
  const octantPath = '' // Default to root

  // Build d-tag for current config
  const dTag = microDepth > 0
    ? `${octantPath}:${macroDepth}:${microDepth}`
    : `${octantPath}:${macroDepth}`

  // Fetch matching event
  const event = await fetchSpecificWorld(pubkey, dTag, relays)

  if (event) {
    await loadWorldFromEvent(event)
    return true
  }

  return false // No saved world for this configuration
}
```

## Implementation Notes

### World Configuration Matching

The implementation should:
1. Fetch all world events for the current user's pubkey
2. Filter events where d-tag matches current world configuration
3. If match found, replace current world with loaded CSM data
4. If no match found, keep current world (or start with empty world)

### Depth Changes

When user changes macro/micro depth settings:
1. Save current world to event with old configuration d-tag
2. Query for event with new configuration d-tag
3. Load new world if found, otherwise start fresh

### Multiple Worlds

Users can maintain multiple worlds:
- Different octant paths for different locations
- Different depth configurations for different scales
- Each combination gets its own addressable event

### Relay Selection

Consider using dedicated relays for world data:
- World data can be large (CSM can be verbose)
- May want separate relay configuration from chat/social
- Could use `WORLD_RELAYS` from existing config

## Design Considerations

### Addressed in Specification

✅ Event kind selection (30078 for application-specific data)
✅ Addressable event format with d-tag
✅ d-tag format including octantpath, macrodepth, microdepth
✅ CSM content format
✅ Event metadata (tags)
✅ Fetching and filtering logic
✅ Loading world from event data

### Potential Enhancements

**1. Content Size Concerns**
- Large CSM files may exceed Nostr event size limits (~100KB typical)
- Consider compression (gzip) or chunking for very large worlds
- Could use content-addressed storage (IPFS/blossom) with hash in content field

**2. Version Control**
- Current spec only keeps latest version (addressable/replaceable)
- Consider adding version tags or separate event kinds for history

**3. Permissions & Collaboration**
- Current spec is single-author only
- Could add delegation tags for multi-author worlds
- Could reference parent/fork events for world templates

**4. Indexing & Discovery**
- Consider additional tags for categorization (genre, complexity, etc.)
- Hashtags for discoverability
- Thumbnail/preview in image tag

**5. Delta Updates**
- Instead of replacing entire world, consider incremental update events
- Reference previous state and apply CSM diffs
- Useful for collaborative editing

**6. World Linking**
- Worlds could reference other worlds via a-tags
- Enable portals/connections between worlds
- Build multi-world universes

## References

- [NIP-01: Basic protocol flow](https://github.com/nostr-protocol/nips/blob/master/01.md)
- [NIP-33: Parameterized Replaceable Events](https://github.com/nostr-protocol/nips/blob/master/33.md)
- [NIP-78: Application-specific data](https://github.com/nostr-protocol/nips/blob/master/78.md)
- `doc/cube-script-model.md` - CSM format specification
- `doc/csm-examples.md` - CSM code examples
- `packages/app/src/config/depth-config.ts` - Depth configuration
