declare module '@workspace/wasm-cube' {
  export interface MeshResult {
    vertices: number[];
    indices: number[];
    normals: number[];
    colors: number[];
  }

  export interface ParseError {
    error: string;
  }

  export interface RaycastResult {
    x: number;
    y: number;
    z: number;
    depth: number;
    world_x: number;
    world_y: number;
    world_z: number;
    normal_x: number;
    normal_y: number;
    normal_z: number;
  }

  export interface Color {
    r: number;
    g: number;
    b: number;
  }

  /**
   * WasmCube - Immutable hierarchical voxel cube structure
   */
  export class WasmCube {
    constructor(value: number);
    static solid(value: number): WasmCube;
    get(x: number, y: number, z: number, depth: number): WasmCube;
    update(x: number, y: number, z: number, depth: number, cube: WasmCube): WasmCube;
    updateDepth(depth: number, offset_x: number, offset_y: number, offset_z: number, scale: number, cube: WasmCube): WasmCube;
    raycast(pos_x: number, pos_y: number, pos_z: number, dir_x: number, dir_y: number, dir_z: number, far: boolean, max_depth: number): RaycastResult | null;
    generateMesh(palette: Color[] | null, max_depth: number): MeshResult | ParseError;
    printScript(optimize: boolean): string;
  }

  /**
   * Load Cubescript (CSM) code into a WasmCube
   */
  export function loadCsm(cubescript: string): WasmCube;

  /**
   * Validate CSM code without creating a cube
   */
  export function validateCsm(cubescript: string): ParseError | null;

  /**
   * Load a .vox file from bytes into a WasmCube
   * @param bytes - .vox file bytes
   * @param align_x - X alignment (0.0-1.0, typically 0.5 for center)
   * @param align_y - Y alignment (0.0-1.0, typically 0.5 for center)
   * @param align_z - Z alignment (0.0-1.0, typically 0.5 for center)
   */
  export function loadVox(bytes: Uint8Array, align_x: number, align_y: number, align_z: number): WasmCube;

  /**
   * Parse CSM code and generate mesh data (deprecated - use WasmCube.generateMesh instead)
   */
  export function parse_csm_to_mesh(csm_code: string): MeshResult | ParseError;

  /**
   * Validate CSM code without generating mesh (deprecated - use validateCsm instead)
   */
  export function validate_csm(csm_code: string): ParseError | null;

  /**
   * Initialize the WASM module (default export)
   */
  export default function init(): Promise<void>;
}
