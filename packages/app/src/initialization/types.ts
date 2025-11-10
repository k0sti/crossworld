/**
 * Initialization system types
 *
 * Defines the state machine and types for app initialization
 */

import * as THREE from 'three';
import type { World } from '../physics/world';
import type { SceneManager } from '../renderer/scene';
import type { AccountManager } from 'applesauce-accounts';
import type { AvatarStateService } from '../services/avatar-state';

/**
 * Initialization phases in order
 */
export type InitializationPhase =
  | 'idle'       // Not started
  | 'wasm'       // Loading WASM modules (cube + physics)
  | 'rendering'  // Setting up renderer and scene
  | 'network'    // Connecting to Nostr relays
  | 'session'    // User login and session
  | 'world'      // Loading world and materials
  | 'avatar'     // Creating avatar and character controller
  | 'ready'      // Fully initialized and running
  | 'error';     // Initialization failed

/**
 * Sub-component status for parallel initialization tracking
 */
export interface SubComponentStatus {
  /** Component identifier */
  id: string;

  /** Display name */
  name: string;

  /** Badge color */
  color: 'purple' | 'blue' | 'green' | 'orange' | 'cyan' | 'red';

  /** Current status */
  status: 'pending' | 'loading' | 'complete' | 'error';

  /** Progress 0-100 */
  progress: number;

  /** Optional status message */
  message?: string;

  /** Error if status is 'error' */
  error?: Error;
}

/**
 * Current initialization state
 */
export interface InitializationState {
  /** Current phase */
  phase: InitializationPhase;

  /** Progress within current phase (0-100) */
  progress: number;

  /** Human-readable status message */
  message: string;

  /** Error if phase is 'error' */
  error?: Error;

  /** Timestamp of last update */
  timestamp: number;

  /** Sub-component statuses for parallel initialization */
  subComponents: Map<string, SubComponentStatus>;
}

/**
 * Initialized subsystems container
 */
export interface InitializedSystems {
  /** Physics bridge (initialized in 'wasm' phase) */
  physicsBridge: World;

  /** Scene manager (initialized in 'rendering' phase) */
  sceneManager: SceneManager;

  /** WebGL renderer (initialized in 'rendering' phase) */
  renderer: THREE.WebGLRenderer;

  /** Main canvas element */
  canvas: HTMLCanvasElement;

  /** Account manager (initialized in 'network' phase) */
  accountManager?: AccountManager;

  /** Avatar state service (initialized in 'network' phase) */
  avatarStateService?: AvatarStateService;
}

/**
 * Configuration for app initialization
 */
export interface AppInitializerConfig {
  /** Canvas element to render to */
  canvas: HTMLCanvasElement;

  /** Account manager instance for Nostr integration */
  accountManager?: AccountManager;

  /** Whether to auto-login if credentials exist */
  autoLogin?: boolean;

  /** Whether to enable debug logging */
  debug?: boolean;
}

/**
 * State update callback
 */
export type InitializationStateCallback = (state: InitializationState) => void;

/**
 * Phase transition map for validation
 */
export const PHASE_TRANSITIONS: Record<InitializationPhase, InitializationPhase[]> = {
  idle: ['wasm', 'network', 'error'],
  wasm: ['rendering', 'error'],
  rendering: ['session', 'error'],
  network: ['session', 'error'],
  session: ['world', 'error'],
  world: ['avatar', 'error'],
  avatar: ['ready', 'error'],
  ready: ['error'], // Can only transition to error once ready
  error: [], // Terminal state
};

/**
 * Validate phase transition
 */
export function isValidTransition(from: InitializationPhase, to: InitializationPhase): boolean {
  return PHASE_TRANSITIONS[from].includes(to);
}
