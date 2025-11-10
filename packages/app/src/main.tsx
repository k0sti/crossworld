import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { Root } from './Root.tsx'

// WASM initialization is now handled by AppInitializer in WorldCanvas
// This allows for better progress tracking and unified initialization
createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <Root />
  </StrictMode>,
)
