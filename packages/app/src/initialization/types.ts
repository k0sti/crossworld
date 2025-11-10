/**
 * Initialization system types
 *
 * Defines the state machine and types for app initialization
 */

import * as THREE from 'three';
import type { World } from '../physics/world';
import type { SceneManager } from '../renderer/scene';

/**
 * Initialization phases in order
 */
export type InitializationPhase =
  | 'idle'       // Not started
  | 'wasm'       // Loading WASM modules (cube + physics)
  | 'rendering'  // Setting up renderer and scene
  | 'session'    // User login and session
  | 'world'      // Loading world and materials
  | 'avatar'     // Creating avatar and character controller
  | 'ready'      // Fully initialized and running
  | 'error';     // Initialization failed

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
}

/**
 * Configuration for app initialization
 */
export interface AppInitializerConfig {
  /** Canvas element to render to */
  canvas: HTMLCanvasElement;

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
  idle: ['wasm', 'error'],
  wasm: ['rendering', 'error'],
  rendering: ['session', 'error'],
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
