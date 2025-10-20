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

  /**
   * Parse CSM code and generate mesh data
   */
  export function parse_csm_to_mesh(csm_code: string): MeshResult | ParseError;

  /**
   * Validate CSM code without generating mesh
   */
  export function validate_csm(csm_code: string): ParseError | null;

  /**
   * Initialize the WASM module (default export)
   */
  export default function init(): Promise<void>;
}
