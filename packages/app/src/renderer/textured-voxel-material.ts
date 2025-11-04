import * as THREE from 'three';

/**
 * Custom shader material for textured voxels
 * Supports both textured materials (2-127) and solid colors (0-1, 128-255)
 */
export function createTexturedVoxelMaterial(textures: (THREE.Texture | undefined)[], enableTextures: boolean = true): THREE.ShaderMaterial {
  // Create a texture array for sampling
  // We'll use a 2D texture atlas approach since WebGL1 may not support texture arrays
  // For now, use a simple multi-texture approach with conditional sampling

  const vertexShader = `
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

  const fragmentShader = `
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

    // Texture uniforms - we'll support up to 16 textures for now
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
      int matId = int(vMaterialId + 0.5); // Round to nearest int

      // Check if this is a textured material (2-127) and textures are enabled
      if (enableTextures && matId >= 2 && matId <= 127) {
        // Sample the appropriate texture based on material ID
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
        else texColor = vec4(baseColor, 1.0); // Fallback to vertex color

        baseColor = texColor.rgb;
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

      gl_FragColor = vec4(finalColor, 1.0);
    }
  `;

  // Create uniforms
  const uniforms = {
    ambientLightColor: { value: new THREE.Color(0x404040) },
    directionalLightColor: { value: new THREE.Color(0xffffff) },
    directionalLightDirection: { value: new THREE.Vector3(1, 1, 1).normalize() },
    enableTextures: { value: enableTextures },
    // Texture uniforms
    texture2: { value: textures[2] || null },
    texture3: { value: textures[3] || null },
    texture4: { value: textures[4] || null },
    texture5: { value: textures[5] || null },
    texture6: { value: textures[6] || null },
    texture7: { value: textures[7] || null },
    texture8: { value: textures[8] || null },
    texture9: { value: textures[9] || null },
    texture10: { value: textures[10] || null },
  };

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
export function updateShaderLighting(material: THREE.ShaderMaterial, scene: THREE.Scene): void {
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
