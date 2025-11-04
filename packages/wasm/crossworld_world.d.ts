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
  static new_with_uvs(vertices: Float32Array, indices: Uint32Array, normals: Float32Array, colors: Float32Array, uvs: Float32Array, material_ids: Uint8Array): GeometryData;
  readonly vertices: Float32Array;
  readonly indices: Uint32Array;
  readonly normals: Float32Array;
  readonly colors: Float32Array;
  readonly uvs: Float32Array;
  readonly materialIds: Uint8Array;
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
  constructor(macro_depth: number, micro_depth: number, border_depth: number);
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
