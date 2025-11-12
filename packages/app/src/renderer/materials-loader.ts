import * as THREE from 'three';
import * as logger from '../utils/logger';

interface Material {
  index: number;
  id: string;
  color: string;
  description: string;
}

interface MaterialsData {
  generated: string;
  count: number;
  materials: Material[];
}

/**
 * Manages loading materials.json and textures for textured voxels
 */
export class MaterialsLoader {
  private materialsData: MaterialsData | null = null;
  private textureArray: THREE.Texture[] = [];
  private materialIdToTexture: Map<number, THREE.Texture> = new Map();
  private loadedTextureIndices: Set<number> = new Set();

  /**
   * Load materials.json from assets
   */
  async loadMaterialsJson(): Promise<void> {
    try {
      const response = await fetch('/crossworld/assets/materials.json');
      if (!response.ok) {
        throw new Error(`Failed to load materials.json: ${response.statusText}`);
      }
      this.materialsData = await response.json();
      logger.log('renderer', `Loaded ${this.materialsData?.materials.length} materials from materials.json`);
    } catch (error) {
      logger.error('renderer', 'Failed to load materials.json:', error);
      throw error;
    }
  }

  /**
   * Load textures for materials 2-127 (textured materials)
   * @param useHighRes - If true, load high-res textures from /assets/textures (for avatars)
   *                     If false, load low-res textures from /assets/textures5 (for world)
   */
  async loadTextures(useHighRes: boolean = false): Promise<void> {
    if (!this.materialsData) {
      throw new Error('Materials data not loaded. Call loadMaterialsJson first.');
    }

    const textureLoader = new THREE.TextureLoader();
    const texturedMaterials = this.materialsData.materials.filter(
      m => m.index >= 2 && m.index <= 127
    );

    const textureDir = useHighRes ? 'textures' : 'textures5';
    logger.log('renderer', `Loading ${texturedMaterials.length} ${useHighRes ? 'high-res' : 'low-res'} textures from ${textureDir}...`);

    // Initialize texture array with placeholders
    this.textureArray = new Array(128);

    // Load textures in parallel
    const loadPromises = texturedMaterials.map(async (material) => {
      const texturePath = `/crossworld/assets/${textureDir}/${material.id}.webp`;

      try {
        const texture = await textureLoader.loadAsync(texturePath);

        // Configure texture for seamless tiling with nearest neighbor filtering (pixelated look)
        texture.wrapS = THREE.RepeatWrapping;
        texture.wrapT = THREE.RepeatWrapping;
        texture.magFilter = THREE.NearestFilter;
        texture.minFilter = THREE.NearestFilter;
        texture.generateMipmaps = false;

        // Store texture at material index
        this.textureArray[material.index] = texture;
        this.materialIdToTexture.set(material.index, texture);
        this.loadedTextureIndices.add(material.index);
      } catch (error) {
        logger.warn('renderer', `Failed to load texture for material ${material.index} (${material.id}):`, error);

        // Create fallback colored texture
        const canvas = document.createElement('canvas');
        canvas.width = 32;
        canvas.height = 32;
        const ctx = canvas.getContext('2d')!;

        // Parse color from material (format: #RRGGBBAA or #RRGGBB)
        const colorHex = material.color.startsWith('#') ? material.color.substring(1, 7) : 'CCCCCC';
        ctx.fillStyle = '#' + colorHex;
        ctx.fillRect(0, 0, 32, 32);

        const fallbackTexture = new THREE.CanvasTexture(canvas);
        fallbackTexture.wrapS = THREE.RepeatWrapping;
        fallbackTexture.wrapT = THREE.RepeatWrapping;
        fallbackTexture.magFilter = THREE.NearestFilter;
        fallbackTexture.minFilter = THREE.NearestFilter;

        this.textureArray[material.index] = fallbackTexture;
        this.materialIdToTexture.set(material.index, fallbackTexture);
      }
    });

    await Promise.all(loadPromises);
    logger.log('renderer', `Loaded ${this.loadedTextureIndices.size} textures successfully`);
  }

  /**
   * Get texture for a material index
   */
  getTexture(materialId: number): THREE.Texture | undefined {
    return this.materialIdToTexture.get(materialId);
  }

  /**
   * Get all loaded textures as an array
   */
  getTextureArray(): THREE.Texture[] {
    return this.textureArray;
  }

  /**
   * Get material info by index
   */
  getMaterial(index: number): Material | undefined {
    return this.materialsData?.materials.find(m => m.index === index);
  }

  /**
   * Check if textures are loaded
   */
  isLoaded(): boolean {
    return this.loadedTextureIndices.size > 0;
  }
}
