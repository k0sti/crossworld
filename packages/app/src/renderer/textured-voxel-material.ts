import * as THREE from 'three';

/**
 * Create a WebGL 2.0 2D texture array from individual textures
 */
function createTextureArray(textures: (THREE.Texture | undefined)[], renderer: THREE.WebGLRenderer): THREE.DataArrayTexture | null {
  const gl = renderer.getContext() as WebGL2RenderingContext;

  if (!gl || !gl.texImage3D) {
    console.warn('[textured-voxel-material] WebGL 2.0 not available, cannot create texture array');
    return null;
  }

  // Assume all textures are the same size (standard practice)
  // Find first valid texture to get dimensions
  const firstValidTexture = textures.find(t => t !== undefined && t !== null);
  if (!firstValidTexture || !firstValidTexture.image) {
    console.warn('[textured-voxel-material] No valid textures found');
    return null;
  }

  const width = firstValidTexture.image.width || 256;
  const height = firstValidTexture.image.height || 256;
  const depth = 128; // Support material IDs 0-127

  console.log(`[textured-voxel-material] Creating texture array: ${width}x${height}x${depth}`);

  // Create a canvas to extract pixel data from each texture
  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext('2d', { willReadFrequently: true });

  if (!ctx) {
    console.error('[textured-voxel-material] Failed to get 2D context');
    return null;
  }

  // Allocate array for all texture data (RGBA)
  const data = new Uint8Array(width * height * depth * 4);

  // Fill texture array with data from individual textures
  for (let i = 0; i < depth; i++) {
    const texture = textures[i];

    if (texture && texture.image) {
      // Draw texture to canvas and extract pixel data
      ctx.clearRect(0, 0, width, height);
      ctx.drawImage(texture.image, 0, 0, width, height);
      const imageData = ctx.getImageData(0, 0, width, height);

      // Copy to the appropriate layer in the texture array
      const offset = i * width * height * 4;
      data.set(imageData.data, offset);
    } else {
      // Fill with a default color (pink for missing textures)
      const offset = i * width * height * 4;
      for (let j = 0; j < width * height; j++) {
        data[offset + j * 4 + 0] = 255; // R
        data[offset + j * 4 + 1] = 0;   // G
        data[offset + j * 4 + 2] = 255; // B
        data[offset + j * 4 + 3] = 255; // A
      }
    }
  }

  // Create THREE.js DataArrayTexture
  const textureArray = new THREE.DataArrayTexture(data, width, height, depth);
  textureArray.format = THREE.RGBAFormat;
  textureArray.type = THREE.UnsignedByteType;
  textureArray.minFilter = THREE.NearestFilter;
  textureArray.magFilter = THREE.NearestFilter;
  textureArray.wrapS = THREE.RepeatWrapping;
  textureArray.wrapT = THREE.RepeatWrapping;
  textureArray.flipY = false; // Required for 3D textures
  textureArray.needsUpdate = true;

  console.log('[textured-voxel-material] Texture array created successfully');
  return textureArray;
}

/**
 * Custom shader material for textured voxels using WebGL 2.0 texture arrays
 * Supports both textured materials (2-127) and solid colors (0-1, 128-255)
 */
export function createTexturedVoxelMaterial(textures: (THREE.Texture | undefined)[], enableTextures: boolean = true, renderer?: THREE.WebGLRenderer): THREE.ShaderMaterial | THREE.RawShaderMaterial {
  // Debug: Log texture availability
  const validTextureIndices = textures
    .map((tex, idx) => tex ? idx : -1)
    .filter(idx => idx >= 0);
  console.log('[textured-voxel-material] Creating material with textures at indices:', validTextureIndices);
  console.log('[textured-voxel-material] enableTextures:', enableTextures);

  // Create texture array if renderer is provided (WebGL 2.0 mode)
  const textureArray = renderer ? createTextureArray(textures, renderer) : null;

  const vertexShader = textureArray ? `
    precision highp float;

    // Custom attributes (Three.js provides position, normal, uv automatically)
    in vec3 position;
    in vec3 normal;
    in vec2 uv;
    in vec3 color;
    in float materialId;

    uniform mat4 modelViewMatrix;
    uniform mat4 projectionMatrix;
    uniform mat3 normalMatrix;

    out vec3 vNormal;
    out vec3 vColor;
    out vec2 vUv;
    out float vMaterialId;
    out vec3 vViewPosition;

    void main() {
      // Transform position
      vec4 mvPosition = modelViewMatrix * vec4(position, 1.0);
      gl_Position = projectionMatrix * mvPosition;

      // Pass varying values to fragment shader
      vNormal = normalize(normalMatrix * normal);
      vColor = color;
      vUv = uv;
      vMaterialId = materialId;
      vViewPosition = -mvPosition.xyz;
    }
  ` : `
    precision highp float;

    // Custom attributes (Three.js provides position, normal, uv automatically)
    attribute vec3 color;
    attribute float materialId;

    varying vec3 vNormal;
    varying vec3 vColor;
    varying vec2 vUv;
    varying float vMaterialId;
    varying vec3 vViewPosition;

    void main() {
      // Transform position (Three.js provides position, modelMatrix, viewMatrix, projectionMatrix)
      vec4 mvPosition = modelViewMatrix * vec4(position, 1.0);
      gl_Position = projectionMatrix * mvPosition;

      // Pass varying values to fragment shader (Three.js provides normalMatrix)
      vNormal = normalize(normalMatrix * normal);
      vColor = color;
      vUv = uv;
      vMaterialId = materialId;
      vViewPosition = -mvPosition.xyz;
    }
  `;

  const fragmentShader = textureArray ? `
    precision highp float;
    precision highp sampler2DArray;

    in vec3 vNormal;
    in vec3 vColor;
    in vec2 vUv;
    in float vMaterialId;
    in vec3 vViewPosition;

    uniform vec3 ambientLightColor;
    uniform vec3 directionalLightColor;
    uniform vec3 directionalLightDirection;
    uniform bool enableTextures;
    uniform sampler2DArray textureArray;

    out vec4 fragColor;

    void main() {
      vec3 baseColor = vColor;
      int matId = int(vMaterialId + 0.5); // Round to nearest int

      // Check if this is a textured material (2-127) and textures are enabled
      if (enableTextures && matId >= 2 && matId <= 127) {
        // Sample texture array using material ID as the layer index
        vec4 texColor = texture(textureArray, vec3(vUv, float(matId)));
        // Multiply texture with vertex color - texture is base, vertex color adjusts it
        baseColor = texColor.rgb * vColor;
      }
      // Otherwise use vertex color (solid color materials 0-1, 128-255 or when textures disabled)

      // Very minimal lighting to preserve texture colors
      vec3 normal = normalize(vNormal);

      // Calculate light direction and diffuse factor (this creates shadows)
      vec3 lightDir = normalize(directionalLightDirection);
      float diff = max(dot(normal, lightDir), 0.0);

      // Calculate total light intensity to detect day vs night
      float totalLightIntensity = length(ambientLightColor) + length(directionalLightColor);
      float dayFactor = clamp((totalLightIntensity - 1.0) / 2.0, 0.0, 1.0);

      // Much lower lighting values to preserve texture colors
      // Lighting is purely directional (no view-dependent effects)
      float ambientStrength = mix(0.1, 0.25, dayFactor);
      float diffuseStrength = mix(0.3, 0.25, dayFactor);

      // Ambient - very minimal base lighting
      vec3 ambient = ambientLightColor * baseColor * ambientStrength;

      // Diffuse - creates shadow effect ONLY based on surface normal vs light direction
      // This is view-independent
      vec3 diffuse = directionalLightColor * diff * baseColor * diffuseStrength;

      // Combine lighting (no specular, no view-dependent effects)
      vec3 finalColor = ambient + diffuse;

      fragColor = vec4(finalColor, 1.0);
    }
  ` : `
    precision highp float;

    varying vec3 vNormal;
    varying vec3 vColor;
    varying vec2 vUv;
    varying float vMaterialId;
    varying vec3 vViewPosition;

    uniform vec3 ambientLightColor;
    uniform vec3 directionalLightColor;
    uniform vec3 directionalLightDirection;
    uniform bool enableTextures;

    // Texture uniforms - fallback for WebGL 1.0 (limited support)
    uniform sampler2D texture2;
    uniform sampler2D texture3;
    uniform sampler2D texture4;
    uniform sampler2D texture5;
    uniform sampler2D texture6;
    uniform sampler2D texture7;
    uniform sampler2D texture8;
    uniform sampler2D texture9;
    uniform sampler2D texture10;

    void main() {
      vec3 baseColor = vColor;
      int matId = int(vMaterialId + 0.5);

      // WebGL 1.0 fallback - only supports materials 2-10
      if (enableTextures && matId >= 2 && matId <= 10) {
        vec4 texColor = vec4(1.0);

        if (matId == 2) texColor = texture2D(texture2, vUv);
        else if (matId == 3) texColor = texture2D(texture3, vUv);
        else if (matId == 4) texColor = texture2D(texture4, vUv);
        else if (matId == 5) texColor = texture2D(texture5, vUv);
        else if (matId == 6) texColor = texture2D(texture6, vUv);
        else if (matId == 7) texColor = texture2D(texture7, vUv);
        else if (matId == 8) texColor = texture2D(texture8, vUv);
        else if (matId == 9) texColor = texture2D(texture9, vUv);
        else if (matId == 10) texColor = texture2D(texture10, vUv);

        // Multiply texture with vertex color - texture is base, vertex color adjusts it
        baseColor = texColor.rgb * vColor;
      }

      vec3 normal = normalize(vNormal);
      vec3 lightDir = normalize(directionalLightDirection);
      float diff = max(dot(normal, lightDir), 0.0);

      float totalLightIntensity = length(ambientLightColor) + length(directionalLightColor);
      float dayFactor = clamp((totalLightIntensity - 1.0) / 2.0, 0.0, 1.0);

      float ambientStrength = mix(0.1, 0.25, dayFactor);
      float diffuseStrength = mix(0.3, 0.25, dayFactor);

      vec3 ambient = ambientLightColor * baseColor * ambientStrength;
      vec3 diffuse = directionalLightColor * diff * baseColor * diffuseStrength;
      vec3 finalColor = ambient + diffuse;

      gl_FragColor = vec4(finalColor, 1.0);
    }
  `;

  // Create uniforms
  const uniforms: any = {
    ambientLightColor: { value: new THREE.Color(0x404040) },
    directionalLightColor: { value: new THREE.Color(0xffffff) },
    directionalLightDirection: { value: new THREE.Vector3(1, 1, 1).normalize() },
    enableTextures: { value: enableTextures }
  };

  if (textureArray) {
    // WebGL 2.0 mode - use texture array
    uniforms.textureArray = { value: textureArray };
  } else {
    // WebGL 1.0 fallback - individual textures
    uniforms.texture2 = { value: textures[2] || null };
    uniforms.texture3 = { value: textures[3] || null };
    uniforms.texture4 = { value: textures[4] || null };
    uniforms.texture5 = { value: textures[5] || null };
    uniforms.texture6 = { value: textures[6] || null };
    uniforms.texture7 = { value: textures[7] || null };
    uniforms.texture8 = { value: textures[8] || null };
    uniforms.texture9 = { value: textures[9] || null };
    uniforms.texture10 = { value: textures[10] || null };
  }

  // Use RawShaderMaterial for GLSL 300 es (WebGL 2.0) to avoid Three.js prepending shader chunks
  if (textureArray) {
    return new THREE.RawShaderMaterial({
      vertexShader,
      fragmentShader,
      uniforms,
      side: THREE.FrontSide,
      glslVersion: THREE.GLSL3
    });
  }

  // Use regular ShaderMaterial for GLSL 100 (WebGL 1.0)
  return new THREE.ShaderMaterial({
    vertexShader,
    fragmentShader,
    uniforms,
    lights: false, // We're handling lighting manually
    side: THREE.FrontSide,
  });
}

/**
 * Update shader material lighting based on scene lights
 */
export function updateShaderLighting(material: THREE.ShaderMaterial | THREE.RawShaderMaterial, scene: THREE.Scene): void {
  // Find ambient and directional lights in the scene
  let ambientColor = new THREE.Color(0x404040);
  let directionalColor = new THREE.Color(0xffffff);
  let directionalDirection = new THREE.Vector3(1, 1, 1).normalize();

  scene.traverse((object) => {
    if (object instanceof THREE.AmbientLight) {
      ambientColor = object.color.clone().multiplyScalar(object.intensity);
    } else if (object instanceof THREE.DirectionalLight) {
      directionalColor = object.color.clone().multiplyScalar(object.intensity);
      directionalDirection = object.position.clone().normalize();
    }
  });

  // Update uniforms
  if (material.uniforms.ambientLightColor) {
    material.uniforms.ambientLightColor.value = ambientColor;
  }
  if (material.uniforms.directionalLightColor) {
    material.uniforms.directionalLightColor.value = directionalColor;
  }
  if (material.uniforms.directionalLightDirection) {
    material.uniforms.directionalLightDirection.value = directionalDirection;
  }
}
