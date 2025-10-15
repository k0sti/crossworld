export const DEFAULT_RELAYS = [
  'wss://relay.damus.io',
  'wss://nos.lol',
  'wss://relay.primal.net',
]

// Avatar voxel model constants
export const AVATAR_VOXEL_CONSTANTS = {
  // Voxel grid size (width, height, depth in voxels)
  GRID_SIZE_X: 16,
  GRID_SIZE_Y: 32,
  GRID_SIZE_Z: 16,

  // Voxel size in world units
  VOXEL_SIZE: 0.1,

  // Model dimensions in world units
  get MODEL_WIDTH() { return this.GRID_SIZE_X * this.VOXEL_SIZE; },
  get MODEL_HEIGHT() { return this.GRID_SIZE_Y * this.VOXEL_SIZE; },
  get MODEL_DEPTH() { return this.GRID_SIZE_Z * this.VOXEL_SIZE; },
}
