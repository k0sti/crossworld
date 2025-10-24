# ResponsivePanel Component

A smart panel component that automatically switches to fullscreen mode when content overflows the viewport.

## Features

- **Automatic Overflow Detection**: Uses ResizeObserver to monitor content size
- **Smooth Transitions**: Animated switch between normal and fullscreen modes
- **Responsive**: Reacts to window resize and content changes
- **Click Outside to Close**: Optional click-outside handling
- **Fullscreen Close Button**: Automatically shows close button in fullscreen mode
- **Flexible Positioning**: Supports all CSS position properties
- **Consistent Styling**: Matches existing panel visual style with glass morphism effect

## Usage

### Basic Example

```tsx
import { ResponsivePanel } from './ResponsivePanel'

function MyComponent() {
  const [isOpen, setIsOpen] = useState(false)

  return (
    <ResponsivePanel
      isOpen={isOpen}
      onClose={() => setIsOpen(false)}
      top="60px"
      left="68px"
      minWidth="400px"
      maxWidth="500px"
    >
      <VStack align="stretch" gap={4}>
        <Text>Panel Content</Text>
      </VStack>
    </ResponsivePanel>
  )
}
```

### With Custom Positioning

```tsx
<ResponsivePanel
  isOpen={isOpen}
  onClose={onClose}
  top="60px"
  right="20px"  // Position from right instead of left
  minWidth="300px"
  maxWidth="600px"
  maxHeight="80vh"
  zIndex={2000}
>
  {children}
</ResponsivePanel>
```

### Force Fullscreen Mode

```tsx
<ResponsivePanel
  isOpen={isOpen}
  onClose={onClose}
  forceFullscreen={true}  // Always fullscreen
>
  {children}
</ResponsivePanel>
```

### Disable Click Outside

```tsx
<ResponsivePanel
  isOpen={isOpen}
  onClose={onClose}
  closeOnClickOutside={false}  // Don't close on outside click
>
  {children}
</ResponsivePanel>
```

### Centered Panel

```tsx
<ResponsivePanel
  isOpen={isOpen}
  onClose={onClose}
  centered={true}  // Center in viewport
  minWidth="600px"
  maxWidth="800px"
>
  {children}
</ResponsivePanel>
```

## Props

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `children` | `ReactNode` | required | Panel content |
| `isOpen` | `boolean` | required | Whether panel is visible |
| `onClose` | `() => void` | optional | Close callback |
| `top` | `string \| number` | `"60px"` | Position from top |
| `left` | `string \| number` | optional | Position from left |
| `right` | `string \| number` | optional | Position from right |
| `bottom` | `string \| number` | optional | Position from bottom |
| `minWidth` | `string \| number` | `"400px"` | Min width in normal mode |
| `maxWidth` | `string \| number` | `"500px"` | Max width in normal mode |
| `minHeight` | `string \| number` | optional | Min height in normal mode |
| `maxHeight` | `string \| number` | optional | Max height in normal mode |
| `zIndex` | `number` | `1500` | Z-index for stacking |
| `padding` | `string \| number` | `4` | Panel padding |
| `closeOnClickOutside` | `boolean` | `true` | Enable click-outside to close |
| `forceFullscreen` | `boolean` | `false` | Force fullscreen mode |
| `overflowThreshold` | `number` | `50` | Pixels from edge before fullscreen |
| `centered` | `boolean` | `false` | Center panel in viewport (ignores top/left/right/bottom) |

## How It Works

1. **Initial Render**: Panel renders in normal mode at specified position
2. **Content Monitoring**: ResizeObserver watches for content size changes
3. **Overflow Detection**: Checks if panel extends beyond viewport bounds (with threshold)
4. **Mode Switch**: Automatically transitions to fullscreen if overflow detected
5. **Dynamic Updates**: Re-checks on:
   - Content changes
   - Window resize
   - Children updates

## Overflow Detection Logic

The panel switches to fullscreen when:

```typescript
const overflowsHorizontally =
  rect.right > windowWidth - threshold ||
  rect.left < threshold

const overflowsVertically =
  rect.bottom > windowHeight - threshold ||
  rect.top < threshold

if (overflowsHorizontally || overflowsVertically) {
  // Switch to fullscreen
}
```

Default threshold is 50px, meaning if the panel comes within 50px of any viewport edge, it goes fullscreen.

## Converting Existing Panels

### Before (Manual Panel)

```tsx
export function MyPanel({ onClose }) {
  const panelRef = useRef<HTMLDivElement>(null)

  // Manual click-outside handling
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (panelRef.current && !panelRef.current.contains(event.target as Node)) {
        onClose()
      }
    }
    document.addEventListener('mousedown', handleClickOutside)
    return () => document.removeEventListener('mousedown', handleClickOutside)
  }, [onClose])

  return (
    <Box
      ref={panelRef}
      position="fixed"
      top="60px"
      left="68px"
      zIndex={1500}
      bg="rgba(0, 0, 0, 0.1)"
      backdropFilter="blur(8px)"
      p={4}
      minW="400px"
      maxW="500px"
      _before={{ /* glass morphism styles */ }}
    >
      <VStack>{/* content */}</VStack>
    </Box>
  )
}
```

### After (Using ResponsivePanel)

```tsx
import { ResponsivePanel } from './ResponsivePanel'

export function MyPanel({ isOpen, onClose }) {
  return (
    <ResponsivePanel
      isOpen={isOpen}
      onClose={onClose}
      top="60px"
      left="68px"
      minWidth="400px"
      maxWidth="500px"
    >
      <VStack>{/* content */}</VStack>
    </ResponsivePanel>
  )
}
```

## Benefits

1. **Less Code**: No manual overflow handling or click-outside logic
2. **Better UX**: Automatically adapts to content size
3. **Consistent**: All panels use same behavior and styling
4. **Maintainable**: Single component to update for all panels
5. **Accessible**: Proper fullscreen mode for mobile/small screens

## Examples

See `ResponsivePanelExample.tsx` for a working demonstration.
