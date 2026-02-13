// World rendering — WASM WorldCube → Three.js BufferGeometry
import * as THREE from 'three';
import initWorldWasm, { WorldCube as WasmWorldCube } from 'crossworld-world';

const WORLD_SCALE = 128;
const WORLD_OFFSET = WORLD_SCALE / 2;

let worldCube: WasmWorldCube | null = null;

export async function initWorld(): Promise<void> {
  await initWorldWasm();
  worldCube = new WasmWorldCube(3, 0, 0, 42);
}

export function getWorldCube(): WasmWorldCube | null {
  return worldCube;
}

function buildGeometry(): THREE.BufferGeometry {
  if (!worldCube) throw new Error('World not initialized — call initWorld() first');

  const geo = worldCube.generateFrame();
  const vertices = geo.vertices;
  const indices = geo.indices;
  const normals = geo.normals;
  const colors = geo.colors;

  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute('position', new THREE.BufferAttribute(new Float32Array(vertices), 3));
  geometry.setAttribute('normal', new THREE.BufferAttribute(new Float32Array(normals), 3));

  // Colors from WASM are RGBA (4 floats per vertex) — extract RGB for Three.js
  const vertexCount = vertices.length / 3;
  const rgb = new Float32Array(vertexCount * 3);
  for (let i = 0; i < vertexCount; i++) {
    rgb[i * 3] = colors[i * 4];
    rgb[i * 3 + 1] = colors[i * 4 + 1];
    rgb[i * 3 + 2] = colors[i * 4 + 2];
  }
  geometry.setAttribute('color', new THREE.BufferAttribute(rgb, 3));

  geometry.setIndex(new THREE.BufferAttribute(new Uint32Array(indices), 1));

  return geometry;
}

export function createWorldMesh(scene: THREE.Scene): THREE.Mesh {
  const geometry = buildGeometry();
  const material = new THREE.MeshStandardMaterial({
    vertexColors: true,
    side: THREE.DoubleSide,
  });

  const mesh = new THREE.Mesh(geometry, material);
  mesh.scale.set(WORLD_SCALE, WORLD_SCALE, WORLD_SCALE);
  mesh.position.set(-WORLD_OFFSET, 0, -WORLD_OFFSET);

  scene.add(mesh);
  return mesh;
}

export function updateWorldMesh(mesh: THREE.Mesh): void {
  const geometry = buildGeometry();
  mesh.geometry.dispose();
  mesh.geometry = geometry;
}
