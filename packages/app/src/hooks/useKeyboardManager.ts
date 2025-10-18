import { useEffect, useCallback, useRef } from 'react'

export interface KeysPressed {
  forward: boolean
  backward: boolean
  left: boolean
  right: boolean
  jump: boolean
  run: boolean
}

export interface KeyboardManagerCallbacks {
  onOpenNetworkPanel?: () => void
  onOpenProfilePanel?: () => void
  onOpenAvatarPanel?: () => void
  onToggleChat?: () => void
  onLogout?: () => void
  onClosePanel?: () => void
  onToggleCameraView?: () => void
  onToggleHelp?: () => void
  onToggleFullscreen?: () => void
}

export function useKeyboardManager(
  isLoggedIn: boolean,
  isChatOpen: boolean,
  callbacks: KeyboardManagerCallbacks = {}
) {
  // Use ref for keys pressed so we can read them every frame without causing re-renders
  const keysPressed = useRef<KeysPressed>({
    forward: false,
    backward: false,
    left: false,
    right: false,
    jump: false,
    run: false,
  })

  // Check if we should capture keyboard input (not in modal, not in other input)
  const shouldCaptureInput = useCallback(
    (event: KeyboardEvent): boolean => {
      const target = event.target as HTMLElement
      // Don't capture if typing in input/textarea
      if (
        target.tagName === 'INPUT' ||
        target.tagName === 'TEXTAREA' ||
        target.isContentEditable
      ) {
        return false
      }
      return true
    },
    []
  )

  // Get current keys pressed state
  const getKeysPressed = useCallback(() => {
    return keysPressed.current
  }, [])

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      const key = event.key
      const alt = event.altKey

      // Global shortcuts (work in any mode)
      if (key === 'F1') {
        event.preventDefault()
        callbacks.onToggleHelp?.()
        return
      }
      if (key === 'F11') {
        event.preventDefault()
        callbacks.onToggleFullscreen?.()
        return
      }

      // Global ALT shortcuts (only if logged in)
      if (isLoggedIn && alt) {
        if (key === 'n' || key === 'N') {
          event.preventDefault()
          callbacks.onOpenNetworkPanel?.()
          return
        }
        if (key === 'p' || key === 'P') {
          event.preventDefault()
          callbacks.onOpenProfilePanel?.()
          return
        }
        if (key === 'a' || key === 'A') {
          event.preventDefault()
          callbacks.onOpenAvatarPanel?.()
          return
        }
        if (key === 'c' || key === 'C') {
          event.preventDefault()
          callbacks.onToggleChat?.()
          return
        }
        if (key === 'q' || key === 'Q') {
          event.preventDefault()
          callbacks.onLogout?.()
          return
        }
        if (key === 'h' || key === 'H') {
          event.preventDefault()
          callbacks.onToggleHelp?.()
          return
        }
      }

      // Prevent TAB from cycling through UI elements in walk mode
      if (key === 'Tab' && shouldCaptureInput(event)) {
        event.preventDefault()
        return
      }

      if (key === 'Escape') {
        event.preventDefault()
        if (isChatOpen) {
          callbacks.onToggleChat?.()
        } else {
          callbacks.onClosePanel?.()
        }
        return
      }

      // Walk mode handling (only if not in chat)
      if (!isChatOpen && shouldCaptureInput(event)) {
        // Walk mode shortcuts
        if (key === 'f' || key === 'F') {
          event.preventDefault()
          callbacks.onToggleCameraView?.()
          return
        }

        // WASD movement - update keys pressed ref
        if (key === 'w' || key === 'W') {
          event.preventDefault()
          keysPressed.current.forward = true
          return
        }
        if (key === 's' || key === 'S') {
          event.preventDefault()
          keysPressed.current.backward = true
          return
        }
        if (key === 'a' || key === 'A') {
          event.preventDefault()
          keysPressed.current.left = true
          return
        }
        if (key === 'd' || key === 'D') {
          event.preventDefault()
          keysPressed.current.right = true
          return
        }
        if (key === ' ') {
          event.preventDefault()
          keysPressed.current.jump = true
          return
        }

        // Track Shift for running
        if (key === 'Shift') {
          keysPressed.current.run = true
          return
        }
      }
    }

    const handleKeyUp = (event: KeyboardEvent) => {
      const key = event.key

      // Release keys for movement
      if (key === 'w' || key === 'W') {
        keysPressed.current.forward = false
      }
      if (key === 's' || key === 'S') {
        keysPressed.current.backward = false
      }
      if (key === 'a' || key === 'A') {
        keysPressed.current.left = false
      }
      if (key === 'd' || key === 'D') {
        keysPressed.current.right = false
      }
      if (key === ' ') {
        keysPressed.current.jump = false
      }
      if (key === 'Shift') {
        keysPressed.current.run = false
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    window.addEventListener('keyup', handleKeyUp)

    return () => {
      window.removeEventListener('keydown', handleKeyDown)
      window.removeEventListener('keyup', handleKeyUp)
    }
  }, [
    isChatOpen,
    isLoggedIn,
    shouldCaptureInput,
    callbacks,
  ])

  return {
    getKeysPressed,
  }
}
