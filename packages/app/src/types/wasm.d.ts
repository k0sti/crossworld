declare module '@workspace/wasm' {
  export function init(): void;
  export function load_vox_from_bytes(bytes: Uint8Array, user_npub?: string | null): GeometryData;

  export class AvatarEngine {
    free(): void;
    constructor();
    generate_avatar(user_npub: string): GeometryData;
    clear_cache(): void;
    cache_size(): number;
    set_voxel(x: number, y: number, z: number, color_index: number): void;
    remove_voxel(x: number, y: number, z: number): void;
    regenerate_mesh(user_npub: string): GeometryData;
  }

  export class GeometryData {
    free(): void;
    static new(vertices: Float32Array, indices: Uint32Array, normals: Float32Array, colors: Float32Array): GeometryData;
    readonly vertices: Float32Array;
    readonly indices: Uint32Array;
    readonly normals: Float32Array;
    readonly colors: Float32Array;
  }

  export class GeometryEngine {
    free(): void;
    constructor();
    generate_frame(): GeometryData;
    setGroundRenderMode(use_cube: boolean): void;
    getGroundRenderMode(): boolean;
  }

  export class NetworkClient {
    free(): void;
    constructor();
    connect(_server_url: string, _npub: string, _display_name: string, _avatar_url: string | null | undefined, _initial_x: number, _initial_y: number, _initial_z: number): Promise<void>;
    send_position(_x: number, _y: number, _z: number, _rx: number, _ry: number, _rz: number, _rw: number): void;
    send_chat(_message: string): Promise<void>;
  }

  export default function init(): Promise<any>;
}

