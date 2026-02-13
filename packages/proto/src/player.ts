// Player controller — WASD/gamepad input, physics, camera
import * as THREE from 'three';
// @ts-ignore — physics WASM module (built separately via wasm-pack)
import initPhysicsWasm, { WasmPhysicsWorld } from '../../wasm-physics/crossworld_physics.js';

// --- Physics state ---
let physics: WasmPhysicsWorld | null = null;
let charId = -1;

// --- Input state ---
const keys = { w: false, a: false, s: false, d: false, space: false, shift: false };
let yaw = 0;
let pitch = 0;
let controlsCanvas: HTMLCanvasElement | null = null;

// --- Gamepad state ---
let lastJumpState = false;

// --- Constants ---
const WALK_SPEED = 8;
const SPRINT_MULTIPLIER = 1.5;
const LOOK_SENSITIVITY = 0.002;
const EYE_HEIGHT = 1.6;
const GAMEPAD_DEADZONE = 0.15;
const GAMEPAD_LOOK_SPEED = 2.5; // radians/sec at full deflection
const PITCH_LIMIT = Math.PI / 2 * 0.98;

// --- Bound event handlers (stored for cleanup) ---
let onKeyDown: ((e: KeyboardEvent) => void) | null = null;
let onKeyUp: ((e: KeyboardEvent) => void) | null = null;
let onMouseMove: ((e: MouseEvent) => void) | null = null;
let onPointerLockChange: (() => void) | null = null;
let onCanvasClick: (() => void) | null = null;

/**
 * Initialize physics WASM, create world with ground plane and player character.
 */
export async function initPlayer(): Promise<void> {
  await initPhysicsWasm();
  physics = new WasmPhysicsWorld(0, -9.81, 0);
  physics.createGroundPlane();
  charId = physics.createCharacter(0, 20, 0, 1.8, 0.3);
}

/**
 * Set up keyboard + mouse controls for first-person movement.
 * Click canvas to lock pointer; Escape to unlock.
 */
export function setupControls(canvas: HTMLCanvasElement, camera: THREE.PerspectiveCamera): void {
  controlsCanvas = canvas;

  // Initialize yaw from camera direction
  const dir = new THREE.Vector3();
  camera.getWorldDirection(dir);
  yaw = Math.atan2(-dir.x, -dir.z);
  pitch = Math.asin(dir.y);

  onKeyDown = (e: KeyboardEvent) => {
    const k = e.key.toLowerCase();
    if (k === 'w') keys.w = true;
    else if (k === 'a') keys.a = true;
    else if (k === 's') keys.s = true;
    else if (k === 'd') keys.d = true;
    else if (k === ' ') keys.space = true;
    else if (k === 'shift') keys.shift = true;
  };

  onKeyUp = (e: KeyboardEvent) => {
    const k = e.key.toLowerCase();
    if (k === 'w') keys.w = false;
    else if (k === 'a') keys.a = false;
    else if (k === 's') keys.s = false;
    else if (k === 'd') keys.d = false;
    else if (k === ' ') keys.space = false;
    else if (k === 'shift') keys.shift = false;
  };

  onMouseMove = (e: MouseEvent) => {
    if (document.pointerLockElement !== canvas) return;
    yaw -= e.movementX * LOOK_SENSITIVITY;
    pitch -= e.movementY * LOOK_SENSITIVITY;
    pitch = Math.max(-PITCH_LIMIT, Math.min(PITCH_LIMIT, pitch));
  };

  onPointerLockChange = () => {
    // Reset keys when pointer lock is lost to avoid stuck movement
    if (document.pointerLockElement !== canvas) {
      keys.w = keys.a = keys.s = keys.d = keys.space = keys.shift = false;
    }
  };

  onCanvasClick = () => {
    canvas.requestPointerLock();
  };

  document.addEventListener('keydown', onKeyDown);
  document.addEventListener('keyup', onKeyUp);
  document.addEventListener('mousemove', onMouseMove);
  document.addEventListener('pointerlockchange', onPointerLockChange);
  canvas.addEventListener('click', onCanvasClick);
}

/**
 * Per-frame update: read input, step physics, update camera.
 * Returns player position for network broadcasting.
 */
export function updatePlayer(
  dt: number,
  camera: THREE.PerspectiveCamera,
): { x: number; y: number; z: number } {
  if (!physics || charId < 0) {
    return { x: 0, y: 0, z: 0 };
  }

  // --- Gamepad input ---
  pollGamepad(dt);

  // --- Calculate movement velocity from WASD relative to camera yaw ---
  const forward = new THREE.Vector3(-Math.sin(yaw), 0, -Math.cos(yaw));
  const right = new THREE.Vector3(Math.cos(yaw), 0, -Math.sin(yaw));
  const vel = new THREE.Vector3();

  if (keys.w) vel.add(forward);
  if (keys.s) vel.sub(forward);
  if (keys.d) vel.add(right);
  if (keys.a) vel.sub(right);

  if (vel.lengthSq() > 0) {
    vel.normalize();
  }

  const speed = keys.shift ? WALK_SPEED * SPRINT_MULTIPLIER : WALK_SPEED;
  vel.multiplyScalar(speed);

  // --- Jump ---
  if (keys.space && physics.isObjectGrounded(charId)) {
    physics.jumpCharacter(charId);
  }

  // --- Step physics ---
  physics.moveCharacter(charId, vel.x, vel.z, dt);
  physics.step(dt);

  // --- Read back position ---
  const pos = physics.getCharacterPosition(charId);
  const px = pos[0];
  const py = pos[1];
  const pz = pos[2];

  // --- Update camera (first-person: at eye height) ---
  camera.position.set(px, py + EYE_HEIGHT, pz);
  camera.rotation.order = 'YXZ';
  camera.rotation.y = yaw;
  camera.rotation.x = pitch;

  return { x: px, y: py, z: pz };
}

/**
 * Get current player position.
 */
export function getPlayerPosition(): { x: number; y: number; z: number } {
  if (!physics || charId < 0) return { x: 0, y: 0, z: 0 };
  const pos = physics.getCharacterPosition(charId);
  return { x: pos[0], y: pos[1], z: pos[2] };
}

/**
 * Get current player Y rotation in radians.
 */
export function getPlayerRotation(): number {
  return yaw;
}

/**
 * Remove all event listeners.
 */
export function cleanup(): void {
  if (onKeyDown) document.removeEventListener('keydown', onKeyDown);
  if (onKeyUp) document.removeEventListener('keyup', onKeyUp);
  if (onMouseMove) document.removeEventListener('mousemove', onMouseMove);
  if (onPointerLockChange) document.removeEventListener('pointerlockchange', onPointerLockChange);
  if (controlsCanvas && onCanvasClick) controlsCanvas.removeEventListener('click', onCanvasClick);

  onKeyDown = onKeyUp = onMouseMove = onPointerLockChange = onCanvasClick = null;
  controlsCanvas = null;
}

// --- Gamepad helpers (internal) ---

function applyDeadzone(value: number): number {
  return Math.abs(value) < GAMEPAD_DEADZONE ? 0 : value;
}

function pollGamepad(dt: number): void {
  const gamepads = navigator.getGamepads();
  const gp = gamepads[0] ?? gamepads[1] ?? gamepads[2] ?? gamepads[3];
  if (!gp) return;

  // Left stick → movement (override keyboard if non-zero)
  const lx = applyDeadzone(gp.axes[0] ?? 0);
  const ly = applyDeadzone(gp.axes[1] ?? 0);

  if (lx !== 0 || ly !== 0) {
    // Map stick to WASD-like keys — forward is negative Y
    keys.w = ly < -GAMEPAD_DEADZONE;
    keys.s = ly > GAMEPAD_DEADZONE;
    keys.a = lx < -GAMEPAD_DEADZONE;
    keys.d = lx > GAMEPAD_DEADZONE;
  }

  // Right stick → look
  const rx = applyDeadzone(gp.axes[2] ?? 0);
  const ry = applyDeadzone(gp.axes[3] ?? 0);
  if (rx !== 0 || ry !== 0) {
    yaw -= rx * GAMEPAD_LOOK_SPEED * dt;
    pitch -= ry * GAMEPAD_LOOK_SPEED * dt;
    pitch = Math.max(-PITCH_LIMIT, Math.min(PITCH_LIMIT, pitch));
  }

  // A button (0) → jump (edge-detected)
  const jumpNow = gp.buttons[0]?.pressed ?? false;
  if (jumpNow && !lastJumpState) {
    keys.space = true;
  }
  lastJumpState = jumpNow;

  // RT trigger (7) → sprint
  keys.shift = (gp.buttons[7]?.value ?? 0) > 0.3;
}
