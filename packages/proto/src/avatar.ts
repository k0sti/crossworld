// Avatar system — load .vox files via cube WASM, generate Three.js meshes
import * as THREE from 'three';
import cubeInit from 'cube';
import * as cubeWasm from 'cube';

interface MeshResult {
  vertices: number[];
  indices: number[];
  normals: number[];
  colors: number[];
  uvs: number[];
  material_ids: number[];
}

interface AvatarManifestEntry {
  name: string;
  file: string;
}

const AVATAR_TARGET_HEIGHT = 1.8;
const ASSETS_BASE = '/crossworld/assets/avatars';

let initialized = false;
const avatarCache = new Map<string, THREE.Group>();

export async function initAvatars(): Promise<void> {
  if (initialized) return;
  await cubeInit();
  initialized = true;
}

export async function loadAvatar(url: string): Promise<THREE.Group> {
  const cached = avatarCache.get(url);
  if (cached) return cached.clone();

  if (!initialized) await initAvatars();

  const response = await fetch(url);
  if (!response.ok) throw new Error(`Failed to fetch avatar: ${url} (${response.status})`);

  const bytes = new Uint8Array(await response.arrayBuffer());

  // @ts-ignore - WASM module exports loadVoxBox
  const cubeBox = cubeWasm.loadVoxBox(bytes);
  const result = cubeBox.generateMesh(null) as MeshResult | { error: string };

  if ('error' in result) throw new Error(`Failed to generate avatar mesh: ${result.error}`);
  if (result.vertices.length === 0) throw new Error(`Avatar has no geometry: ${url}`);

  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute('position', new THREE.BufferAttribute(new Float32Array(result.vertices), 3));
  geometry.setAttribute('normal', new THREE.BufferAttribute(new Float32Array(result.normals), 3));
  geometry.setIndex(new THREE.BufferAttribute(new Uint32Array(result.indices), 1));

  // Colors from WASM may be RGBA (4 per vertex) — extract RGB for Three.js
  const vertexCount = result.vertices.length / 3;
  const isRGBA = result.colors.length === vertexCount * 4;
  let rgb: Float32Array;
  if (isRGBA) {
    rgb = new Float32Array(vertexCount * 3);
    for (let i = 0, j = 0; i < result.colors.length; i += 4, j += 3) {
      rgb[j] = result.colors[i];
      rgb[j + 1] = result.colors[i + 1];
      rgb[j + 2] = result.colors[i + 2];
    }
  } else {
    rgb = new Float32Array(result.colors);
  }
  geometry.setAttribute('color', new THREE.BufferAttribute(rgb, 3));

  const material = new THREE.MeshStandardMaterial({ vertexColors: true });
  const mesh = new THREE.Mesh(geometry, material);

  // Scale avatar so it's approximately AVATAR_TARGET_HEIGHT tall
  const bbox = new THREE.Box3().setFromBufferAttribute(
    geometry.getAttribute('position') as THREE.BufferAttribute,
  );
  const size = new THREE.Vector3();
  bbox.getSize(size);
  const rawHeight = size.y;
  const scale = rawHeight > 0 ? AVATAR_TARGET_HEIGHT / rawHeight : 1;
  mesh.scale.set(scale, scale, scale);

  // Center horizontally, feet on ground
  const scaledMin = bbox.min.clone().multiplyScalar(scale);
  const scaledSize = size.clone().multiplyScalar(scale);
  mesh.position.set(
    -(scaledMin.x + scaledSize.x * 0.5),
    -scaledMin.y,
    -(scaledMin.z + scaledSize.z * 0.5),
  );

  const group = new THREE.Group();
  group.add(mesh);

  avatarCache.set(url, group);
  return group.clone();
}

export function createAvatarInstance(pubkey: string, avatarGroup: THREE.Group): THREE.Group {
  const instance = avatarGroup.clone();

  // Hash pubkey to a hue value (0–360)
  let hash = 0;
  for (let i = 0; i < pubkey.length; i++) {
    hash = ((hash << 5) - hash + pubkey.charCodeAt(i)) | 0;
  }
  const hue = ((hash % 360) + 360) % 360;

  // Apply hue tint to vertex colors
  const tintColor = new THREE.Color();
  tintColor.setHSL(hue / 360, 0.4, 0.7);

  instance.traverse((child) => {
    if (!(child instanceof THREE.Mesh)) return;
    const geo = child.geometry as THREE.BufferGeometry;
    const colorAttr = geo.getAttribute('color');
    if (!colorAttr) return;

    // Clone geometry so we don't mutate the cached original
    child.geometry = geo.clone();
    const clonedColors = child.geometry.getAttribute('color') as THREE.BufferAttribute;
    const colors = clonedColors.array as Float32Array;

    const src = new THREE.Color();
    const hsl = { h: 0, s: 0, l: 0 };
    for (let i = 0; i < colors.length; i += 3) {
      src.setRGB(colors[i], colors[i + 1], colors[i + 2]);
      src.getHSL(hsl);
      // Shift hue while preserving saturation and lightness
      src.setHSL(hue / 360, Math.max(hsl.s, 0.3), hsl.l);
      colors[i] = src.r;
      colors[i + 1] = src.g;
      colors[i + 2] = src.b;
    }
    clonedColors.needsUpdate = true;
  });

  return instance;
}

export function updateAvatarPosition(
  group: THREE.Group,
  x: number,
  y: number,
  z: number,
  rotY: number,
): void {
  group.position.set(x, y, z);
  group.rotation.y = rotY;
}

export async function getDefaultAvatar(): Promise<THREE.Group> {
  if (!initialized) await initAvatars();

  try {
    const response = await fetch(`${ASSETS_BASE}/../avatars.json`);
    if (response.ok) {
      const manifest: AvatarManifestEntry[] = await response.json();
      if (manifest.length > 0) {
        return loadAvatar(`${ASSETS_BASE}/${manifest[0].file}`);
      }
    }
  } catch {
    // Fall through to hardcoded default
  }

  return loadAvatar(`${ASSETS_BASE}/default.vox`);
}
