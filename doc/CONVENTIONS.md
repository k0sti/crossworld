# CrossWorld Development Conventions

This document describes the coding conventions and design principles used throughout the CrossWorld project.

## Data Validation and Backwards Compatibility

### Philosophy

**Reset over Migration**: When encountering old, invalid, or corrupted data, it is acceptable and preferred to reset to the initial state rather than attempting complex migrations or maintaining backwards compatibility.

### Rationale

- **Simplicity**: Avoiding legacy code paths keeps the codebase clean and maintainable
- **Development Speed**: No need to write and test migration code for every data format change
- **Early Stage**: During active development, data formats may change frequently
- **User Impact**: Users can easily recreate their state by making new selections

### Guidelines

1. **No Hardcoded Legacy Mappings**: Don't maintain mappings of old identifiers to new ones
2. **Validation Simplicity**: When validating data, check for current valid formats only
3. **Fail Fast**: If data doesn't match expected format, treat it as invalid
4. **Clear Initial State**: Ensure the application has a well-defined, safe initial state
5. **User Notification**: When possible, inform users that their data was reset (but don't block them)

### Examples

**Good**:
```typescript
// Load avatar, reset to initial state if ID doesn't exist
const avatarId = loadAvatarId();
if (!isValidAvatarId(avatarId)) {
  return DEFAULT_AVATAR_ID;
}
```

**Avoid**:
```typescript
// Legacy mapping that adds complexity
const LEGACY_MAP = {
  'boy': 'chr_army1',
  'girl': 'chr_lady1',
  // ... grows over time
};
const avatarId = LEGACY_MAP[loadAvatarId()] || loadAvatarId();
```

### When to Maintain Compatibility

Only maintain backwards compatibility when:
- The cost of user data loss is high (e.g., purchased items, persistent world state)
- The migration is trivial and doesn't add complexity
- The data format is considered stable/released

## Code Organization

### File Naming

- React components: `PascalCase.tsx` (e.g., `SelectAvatar.tsx`)
- Utilities and services: `kebab-case.ts` (e.g., `avatar-state.ts`)
- Types and interfaces: Co-locate with implementation or in dedicated `types/` directory

### Import Organization

Group imports in this order:
1. External dependencies (React, Three.js, etc.)
2. Internal absolute imports (from `src/`)
3. Relative imports (from `./` or `../`)

```typescript
import { useState } from 'react';
import * as THREE from 'three';

import { AvatarService } from '../services/avatar-service';

import { ModelConfig } from './types';
```

## Numeric Values with Units

When handling numeric values that have units (time, distance, angles, etc.), always include the unit in the variable name unless there is no ambiguity.

### Examples

**Good**:
```typescript
const timeout_ms = 5000;
const distance_m = 10.5;
const angle_rad = Math.PI / 4;
const duration_s = 30;
```

**Acceptable** (when type makes unit clear):
```typescript
type Duration = number; // milliseconds (documented in type)
const duration: Duration = 5000;
```

**Avoid**:
```typescript
const timeout = 5000;  // What unit?
const distance = 10.5; // Meters? Pixels? Units?
```

## State Management

### Local vs Global State

- Use local component state (`useState`) for UI-only state
- Use props for parent-child communication
- Use services for shared business logic and state
- Avoid global state unless absolutely necessary

### State Updates

- Keep state updates atomic and predictable
- Avoid deep object mutations; prefer creating new objects
- Use TypeScript to enforce state shape

## Error Handling

### User-Facing Errors

- Show clear, actionable error messages
- Provide fallback UI when components fail
- Log detailed errors to console for debugging

### Development Errors

- Use console logging with prefixes for easy filtering (e.g., `[AvatarState]`)
- Include relevant context in error messages
- Fail fast during development; fail gracefully in production

## Documentation

### Code Comments

- Explain *why*, not *what* (code should be self-documenting)
- Document non-obvious behavior or workarounds
- Keep comments up-to-date with code changes

### Documentation Files

Keep documentation in `doc/` directory:
- `QUICKSTART.md` - Getting started guide
- `project-structure.md` - Architecture overview
- Feature-specific docs (e.g., `avatar_model.md`)
- This file (`CONVENTIONS.md`) - Development conventions

## Testing Philosophy

While we don't currently have extensive automated tests:
- Write code that is easy to test (pure functions, dependency injection)
- Test manually through the UI during development
- Document testing procedures for complex features

## Related Documents

- [Project Structure](./project-structure.md) - Overall project organization
- [Avatar Model Format](./avatar_model.md) - Avatar data format specification
