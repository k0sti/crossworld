declare module '@workspace/wasm' {
  export function init(): Promise<void>;
  export function pubkey_to_emoji(pubkey_hex: string): string;
  export function load_vox_from_bytes(bytes: Uint8Array, user_npub?: string | null): GeometryData;
  
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
