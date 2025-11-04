/* tslint:disable */
/* eslint-disable */
export function init(): void;
/**
 * Load a .vox file from bytes and generate geometry
 */
export function load_vox_from_bytes(bytes: Uint8Array, user_npub?: string | null): GeometryData;
export class AvatarEngine {
  free(): void;
  [Symbol.dispose](): void;
  constructor();
  /**
   * Generate avatar geometry for a specific user
   */
  generate_avatar(user_npub: string): GeometryData;
  /**
   * Clear the avatar cache
   */
  clear_cache(): void;
  /**
   * Get the number of cached avatars
   */
  cache_size(): number;
  /**
   * Set voxel in the base avatar model
   */
  set_voxel(x: number, y: number, z: number, color_index: number): void;
  /**
   * Remove voxel from the base avatar model
   */
  remove_voxel(x: number, y: number, z: number): void;
  /**
   * Regenerate mesh for a user (after modifications)
   */
  regenerate_mesh(user_npub: string): GeometryData;
}
export class GeometryData {
  private constructor();
  free(): void;
  [Symbol.dispose](): void;
  static new(vertices: Float32Array, indices: Uint32Array, normals: Float32Array, colors: Float32Array): GeometryData;
  readonly vertices: Float32Array;
  readonly indices: Uint32Array;
  readonly normals: Float32Array;
  readonly colors: Float32Array;
}
export class NetworkClient {
  free(): void;
  [Symbol.dispose](): void;
  constructor();
  connect(_server_url: string, _npub: string, _display_name: string, _avatar_url: string | null | undefined, _initial_x: number, _initial_y: number, _initial_z: number): Promise<void>;
  send_position(_x: number, _y: number, _z: number, _rx: number, _ry: number, _rz: number, _rw: number): void;
  send_chat(_message: string): Promise<void>;
}
/**
 * WorldCube - The main world terrain cube
 *
 * This replaces the old GeometryEngine with a simpler, direct interface.
 */
export class WorldCube {
  free(): void;
  [Symbol.dispose](): void;
  constructor(macro_depth: number, micro_depth: number, _border_depth: number);
  generateFrame(): GeometryData;
  /**
   * Set voxel in world cube at specified depth
   * depth: octree depth (7=finest detail, 4=coarse, etc.)
   */
  setVoxelAtDepth(x: number, y: number, z: number, depth: number, color_index: number): void;
  /**
   * Remove voxel from world cube at specified depth
   */
  removeVoxelAtDepth(x: number, y: number, z: number, depth: number): void;
  /**
   * Export the current world state to CSM format
   */
  exportToCSM(): string;
  /**
   * Get reference to the root cube (NEW unified interface method)
   *
   * This enables direct manipulation using the unified Cube interface.
   * Returns a serialized cube that can be deserialized on the JS side.
   */
  root(): string;
  /**
   * Set a new root cube (NEW unified interface method)
   *
   * Load a cube from CSM format and replace the entire world.
   *
   * # Arguments
   * * `csm_code` - Cubescript format text
   */
  setRoot(csm_code: string): void;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __wbg_worldcube_free: (a: number, b: number) => void;
  readonly worldcube_new: (a: number, b: number, c: number) => number;
  readonly worldcube_generateFrame: (a: number) => number;
  readonly worldcube_setVoxelAtDepth: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
  readonly worldcube_removeVoxelAtDepth: (a: number, b: number, c: number, d: number, e: number) => void;
  readonly worldcube_exportToCSM: (a: number) => [number, number];
  readonly worldcube_root: (a: number) => [number, number];
  readonly worldcube_setRoot: (a: number, b: number, c: number) => [number, number];
  readonly __wbg_geometrydata_free: (a: number, b: number) => void;
  readonly geometrydata_new: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => number;
  readonly geometrydata_vertices: (a: number) => [number, number];
  readonly geometrydata_indices: (a: number) => [number, number];
  readonly geometrydata_normals: (a: number) => [number, number];
  readonly geometrydata_colors: (a: number) => [number, number];
  readonly __wbg_networkclient_free: (a: number, b: number) => void;
  readonly networkclient_connect: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number) => any;
  readonly networkclient_send_position: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => void;
  readonly networkclient_send_chat: (a: number, b: number, c: number) => any;
  readonly __wbg_avatarengine_free: (a: number, b: number) => void;
  readonly avatarengine_new: () => number;
  readonly avatarengine_generate_avatar: (a: number, b: number, c: number) => number;
  readonly avatarengine_clear_cache: (a: number) => void;
  readonly avatarengine_cache_size: (a: number) => number;
  readonly avatarengine_set_voxel: (a: number, b: number, c: number, d: number, e: number) => void;
  readonly avatarengine_remove_voxel: (a: number, b: number, c: number, d: number) => void;
  readonly load_vox_from_bytes: (a: number, b: number, c: number, d: number) => [number, number, number];
  readonly init: () => void;
  readonly avatarengine_regenerate_mesh: (a: number, b: number, c: number) => number;
  readonly networkclient_new: () => number;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly __externref_table_alloc: () => number;
  readonly __wbindgen_export_2: WebAssembly.Table;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_export_6: WebAssembly.Table;
  readonly __externref_table_dealloc: (a: number) => void;
  readonly closure25_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure110_externref_shim: (a: number, b: number, c: any, d: any) => void;
  readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
