import * as THREE from 'three';

/**
 * GamepadController - Handles gamepad input for avatar movement
 *
 * Button mapping (standard gamepad):
 * - Left Stick (axes 0, 1): Walk/move avatar
 * - RT Trigger (button 7): Run/sprint
 * - A button (button 0): Jump
 */
export class GamepadController {
  private deadzone = 0.15; // Ignore stick values below this threshold
  private runThreshold = 0.3; // RT trigger threshold for running

  // Movement state
  private moveDirection = new THREE.Vector2(0, 0);
  private isRunning = false;
  private jumpPressed = false;
  private lastJumpState = false;

  constructor() {
    // Listen for gamepad connection events
    window.addEventListener('gamepadconnected', this.handleGamepadConnected.bind(this));
    window.addEventListener('gamepaddisconnected', this.handleGamepadDisconnected.bind(this));
  }

  private handleGamepadConnected(e: GamepadEvent): void {
    console.log('[GamepadController] Connected:', e.gamepad.id);
  }

  private handleGamepadDisconnected(e: GamepadEvent): void {
    console.log('[GamepadController] Disconnected:', e.gamepad.id);
  }

  /**
   * Update gamepad state - call this every frame
   */
  update(): void {
    const gamepads = navigator.getGamepads();
    const gamepad = gamepads[0] || gamepads[1] || gamepads[2] || gamepads[3];

    if (!gamepad) {
      // No gamepad connected, reset state
      this.moveDirection.set(0, 0);
      this.isRunning = false;
      this.jumpPressed = false;
      this.lastJumpState = false;
      return;
    }

    // Read left stick (axes 0 and 1)
    let axisX = gamepad.axes[0] || 0;
    let axisY = gamepad.axes[1] || 0;

    // Apply deadzone
    if (Math.abs(axisX) < this.deadzone) axisX = 0;
    if (Math.abs(axisY) < this.deadzone) axisY = 0;

    // Store movement direction (Y is inverted for forward/backward)
    this.moveDirection.set(axisX, -axisY);

    // Read RT trigger (button 7) for running
    const rtButton = gamepad.buttons[7];
    this.isRunning = rtButton && rtButton.value > this.runThreshold;

    // Read A button (button 0) for jumping - detect button press (not hold)
    const aButton = gamepad.buttons[0];
    const currentJumpState = aButton && aButton.pressed;
    this.jumpPressed = currentJumpState && !this.lastJumpState; // Only true on press, not hold
    this.lastJumpState = currentJumpState;
  }

  /**
   * Get the current movement direction from left stick
   * Returns normalized vector (0,0) if below deadzone
   */
  getMoveDirection(): THREE.Vector2 {
    return this.moveDirection.clone();
  }

  /**
   * Check if running (RT trigger pressed)
   */
  isRunPressed(): boolean {
    return this.isRunning;
  }

  /**
   * Check if jump button was just pressed this frame
   */
  wasJumpPressed(): boolean {
    return this.jumpPressed;
  }

  /**
   * Check if any movement input is active
   */
  hasMovementInput(): boolean {
    return this.moveDirection.length() > 0;
  }

  /**
   * Dispose and cleanup
   */
  dispose(): void {
    // Event listeners will be automatically cleaned up when the window is closed
  }
}
