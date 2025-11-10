/**
 * App Initialization Module
 *
 * Centralized initialization system for the Crossworld app
 */

export { AppInitializer } from './AppInitializer';
export {
  loadAllWasmModules,
  isCubeWasmInitialized,
  isPhysicsWasmInitialized,
  areAllWasmModulesInitialized,
} from './WasmLoader';
export { initializePhysics, disposePhysics } from './PhysicsInitializer';
export { initializeRenderer, disposeRenderer } from './RendererInitializer';
export type {
  InitializationState,
  InitializationPhase,
  InitializedSystems,
  AppInitializerConfig,
  InitializationStateCallback,
} from './types';
export { isValidTransition } from './types';
