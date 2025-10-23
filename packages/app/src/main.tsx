import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { Root } from './Root.tsx'
import { ensureCubeWasmInitialized } from './utils/cubeWasm'

// Initialize WASM before React renders
ensureCubeWasmInitialized()
  .then(() => {
    createRoot(document.getElementById('root')!).render(
      <StrictMode>
        <Root />
      </StrictMode>,
    )
  })
  .catch((error) => {
    // Show user-friendly error instead of blank screen
    const root = document.getElementById('root')!
    root.innerHTML = `
      <div style="display:flex;align-items:center;justify-content:center;height:100vh;background:#000;color:#fff;font-family:sans-serif;text-align:center;padding:20px">
        <div>
          <h1>Initialization Failed</h1>
          <p>Failed to load required modules. Please refresh the page.</p>
          <pre style="color:#f88;margin:20px 0">${error.message}</pre>
          <button onclick="location.reload()" style="padding:10px 20px;font-size:16px;cursor:pointer">Reload Page</button>
        </div>
      </div>
    `
    console.error('[Init] Failed to initialize app:', error)
  })
