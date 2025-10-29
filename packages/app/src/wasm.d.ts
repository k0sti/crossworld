declare module '@workspace/wasm' {
  export function init(): Promise<void>;
  export function load_vox_from_bytes(bytes: Uint8Array, user_npub?: string | null): GeometryData;

  export class GeometryEngine {
    constructor(macro_depth: number, micro_depth: number, border_depth: number);
    generate_frame(): GeometryData;
    setVoxelAtDepth(x: number, y: number, z: number, depth: number, color_index: number): void;
    setVoxel(x: number, y: number, z: number, color_index: number): void;
    removeVoxelAtDepth(x: number, y: number, z: number, depth: number): void;
    removeVoxel(x: number, y: number, z: number): void;
    setFaceMeshMode(enabled: boolean): void;
    setGroundRenderMode(use_cube: boolean): void;
    exportToCSM(): string;
  }

  export class AvatarEngine {
    constructor();
    generate_avatar(user_npub: string): GeometryData;
    clear_cache(): void;
    cache_size(): number;
  }
  
  export class GeometryData {
    readonly vertices: Float32Array;
    readonly indices: Uint32Array;
    readonly normals: Float32Array;
    readonly colors: Float32Array;
  }
  
  export default function __wbg_init(): Promise<void>;
}
