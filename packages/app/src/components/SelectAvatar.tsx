import * as logger from '../utils/logger';
import {
  VStack,
  HStack,
  Button,
  Text,
  Box,
  Badge,
  Divider,
  Input,
  Collapse,
  Popover,
  PopoverTrigger,
  PopoverContent,
  PopoverBody,
} from '@chakra-ui/react';
import { useState, useRef, useEffect } from 'react';
import * as THREE from 'three';
import { ReadyPlayerMeService } from '../services/ready-player-me';
import type { TeleportAnimationType } from '../renderer/teleport-animation';
import { ENABLE_AVATAR_COLOR_SELECTION } from '../constants/features';
import { ResponsivePanel } from './ResponsivePanel';
import { MaterialsLoader } from '../renderer/materials-loader';
import { createTexturedVoxelMaterial, updateShaderLighting } from '../renderer/textured-voxel-material';
import { getWorldPanelSetting } from '../config/world-panel-settings';

export interface AvatarSelection {
  avatarType: 'vox' | 'glb' | 'csm';
  avatarId?: string;
  avatarUrl?: string;
  avatarData?: string;  // For CSM: contains the CSM code
  avatarTexture?: string;  // Texture name (0 = only colors, or texture name like 'grass', 'stone')
  teleportAnimationType: TeleportAnimationType;
}

interface SelectAvatarProps {
  isOpen: boolean;
  onClose: () => void;
  onSave: (selection: AvatarSelection) => void;
  currentSelection?: AvatarSelection;
}

import { loadModelsConfig } from '../utils/modelConfig';

interface ModelItem {
  id: string;
  label: string;
  filename: string;
}

// Available textures for avatars
const AVATAR_TEXTURES = [
  'hay', 'grass', 'glass', 'force_field', 'fabric_pink', 'dirt', 'diorite',
  'concrete', 'coal', 'ash', 'andesite', 'amber', 'wool_white', 'wool_red',
  'wool_gray', 'wool_blue', 'wood_oak', 'wood_jungle', 'wood_birch', 'wax',
  'vine', 'topaz', 'sulfur', 'stone', 'stained_glass_blue', 'sponge', 'snow',
  'slime', 'sandstone', 'salt', 'redstone', 'pearl', 'moss', 'melon', 'marble',
  'magma', 'leaves_spruce', 'leather_tan'
];

export function SelectAvatar({ isOpen, onClose, onSave, currentSelection }: SelectAvatarProps) {
  const [avatarType, setAvatarType] = useState<'vox' | 'glb' | 'csm'>(currentSelection?.avatarType || 'vox');
  const [selectedId, setSelectedId] = useState<string>(currentSelection?.avatarId || '');
  const [avatarUrl, setAvatarUrl] = useState<string>(currentSelection?.avatarUrl || '');
  const [inputUrl, setInputUrl] = useState<string>('');
  const [avatarTexture, setAvatarTexture] = useState<string>(currentSelection?.avatarTexture || '0');
  const [teleportAnimationType, setTeleportAnimationType] = useState<TeleportAnimationType>(
    currentSelection?.teleportAnimationType || 'fade'
  );
  const [voxModels, setVoxModels] = useState<ModelItem[]>([]);
  const [glbModels, setGlbModels] = useState<ModelItem[]>([]);
  const [showMoreSettings, setShowMoreSettings] = useState(false);
  const [materialsLoader] = useState(() => new MaterialsLoader());
  const [texturesLoaded, setTexturesLoaded] = useState(false);
  const [textureDepth, setTextureDepth] = useState<number>(0); // 0-5, scales texture by 2^depth
  const [avatarTexturesEnabled] = useState(() => getWorldPanelSetting('avatarTexturesEnabled'));

  const fileInputRef = useRef<HTMLInputElement>(null);

  // Check if user has a previous avatar selection
  const hasPreviousSelection = !!(currentSelection?.avatarId || currentSelection?.avatarUrl || currentSelection?.avatarData);

  // Handle texture selection with logging
  const handleTextureSelect = (texture: string) => {
    setAvatarTexture(texture);
    logger.log('ui', `[SelectAvatar] Texture selected: "${texture === '0' ? 'None (vertex colors only)' : texture}"`);
  };

  // Randomize avatar selection
  const randomizeAvatar = () => {
    logger.log('ui', '[SelectAvatar] Randomizing avatar:', {
      avatarType,
      voxModelsCount: voxModels.length,
      glbModelsCount: glbModels.length,
    });

    // Randomize avatar model
    let selectedModel = '';
    if (avatarType === 'vox' && voxModels.length > 0) {
      const randomIndex = Math.floor(Math.random() * voxModels.length);
      selectedModel = voxModels[randomIndex].label;
      logger.log('ui', '[SelectAvatar] Selected random VOX:', {
        index: randomIndex,
        id: voxModels[randomIndex].id,
        label: selectedModel,
      });
      setSelectedId(voxModels[randomIndex].id);
      setAvatarUrl('');
    } else if (avatarType === 'glb' && glbModels.length > 0) {
      const randomIndex = Math.floor(Math.random() * glbModels.length);
      selectedModel = glbModels[randomIndex].label;
      logger.log('ui', '[SelectAvatar] Selected random GLB:', {
        index: randomIndex,
        id: glbModels[randomIndex].id,
        label: selectedModel,
      });
      setSelectedId(glbModels[randomIndex].id);
      setAvatarUrl('');
    } else {
      logger.warn('ui', '[SelectAvatar] Cannot randomize: no models available');
    }

    // Randomize texture (only if textures are enabled)
    let selectedTexture = '0';
    if (avatarTexturesEnabled) {
      const randomTextureIndex = Math.floor(Math.random() * AVATAR_TEXTURES.length);
      selectedTexture = AVATAR_TEXTURES[randomTextureIndex];
      setAvatarTexture(selectedTexture);
    } else {
      setAvatarTexture('0');
    }

    // Randomize teleport animation
    const teleportTypes: TeleportAnimationType[] = ['fade', 'scale', 'spin', 'slide', 'burst'];
    const randomTeleportIndex = Math.floor(Math.random() * teleportTypes.length);
    const selectedTeleport = teleportTypes[randomTeleportIndex];
    setTeleportAnimationType(selectedTeleport);

    logger.log('ui', `[SelectAvatar] Randomized avatar: model="${selectedModel}", texture="${selectedTexture}", teleport="${selectedTeleport}"`);
  };

  // Load materials and textures (only if avatar textures are enabled)
  useEffect(() => {
    if (!isOpen) return;

    const loadTextures = async () => {
      try {
        if (avatarTexturesEnabled) {
          logger.log('ui', '[SelectAvatar] Loading materials and textures...');
          await materialsLoader.loadMaterialsJson();
          await materialsLoader.loadTextures(true); // Use high-res textures for avatar selector
          setTexturesLoaded(true);
          logger.log('ui', '[SelectAvatar] Materials and textures loaded successfully');
        } else {
          logger.log('ui', '[SelectAvatar] Avatar textures disabled, skipping texture load');
          setTexturesLoaded(false);
        }
      } catch (error) {
        logger.error('ui', '[SelectAvatar] Failed to load materials/textures:', error);
      }
    };

    loadTextures();
  }, [isOpen, materialsLoader, avatarTexturesEnabled]);

  // Load models configuration
  useEffect(() => {
    logger.log('ui', '[SelectAvatar] Loading models config...');
    loadModelsConfig().then(config => {
      const vox = config.vox?.map(([label, filename]) => ({
        id: filename.replace('.vox', ''),
        label,
        filename
      })) || [];
      const glb = config.glb?.map(([label, filename]) => ({
        id: filename.replace('.glb', ''),
        label,
        filename
      })) || [];

      logger.log('ui', '[SelectAvatar] Models loaded:', {
        voxCount: vox.length,
        glbCount: glb.length,
      });

      setVoxModels(vox);
      setGlbModels(glb);

      // Only auto-select if no previous selection exists
      if (!hasPreviousSelection && vox.length > 0) {
        const randomIndex = Math.floor(Math.random() * vox.length);
        const selectedModel = vox[randomIndex];
        logger.log('ui', '[SelectAvatar] Auto-selecting random model (no previous selection):', {
          id: selectedModel.id,
          label: selectedModel.label,
        });
        setSelectedId(selectedModel.id);
        setAvatarType('vox');
      } else if (hasPreviousSelection) {
        logger.log('ui', '[SelectAvatar] Keeping previous selection:', {
          avatarType: currentSelection?.avatarType,
          avatarId: currentSelection?.avatarId,
          avatarUrl: currentSelection?.avatarUrl,
        });
      } else {
        logger.warn('ui', '[SelectAvatar] No VOX models available for auto-selection');
      }
    }).catch(error => {
      logger.error('ui', '[SelectAvatar] Failed to load models config:', error);
    });
  }, [hasPreviousSelection, currentSelection]);

  // Preview canvas - use callback ref to detect when canvas is mounted
  const [previewCanvas, setPreviewCanvas] = useState<HTMLCanvasElement | null>(null);
  const previewSceneRef = useRef<{
    scene: THREE.Scene;
    camera: THREE.PerspectiveCamera;
    renderer: THREE.WebGLRenderer;
    mesh?: THREE.Object3D;
    animationId?: number;
    cameraDistance: number;
  } | null>(null);
  const [sceneReady, setSceneReady] = useState(false);
  const previewContainerRef = useRef<HTMLDivElement>(null);

  // Initialize preview scene when canvas is ready
  useEffect(() => {
    if (!isOpen || !previewCanvas) {
      setSceneReady(false);
      return;
    }

    const canvas = previewCanvas;
    const scene = new THREE.Scene();
    scene.background = new THREE.Color(0x87ceeb); // Sky blue - same as world

    const camera = new THREE.PerspectiveCamera(50, 1, 0.1, 1000);
    const cameraDistance = 3;
    camera.position.set(0, 0, cameraDistance);
    camera.lookAt(0, 0, 0); // Look at center since avatar pivot is now centered

    // Set canvas size to 600px
    const canvasSize = 600;

    const renderer = new THREE.WebGLRenderer({ canvas, antialias: true });
    renderer.setSize(canvasSize, canvasSize);
    renderer.setPixelRatio(window.devicePixelRatio);

    // Lighting - much brighter to compensate for shader's low lighting multipliers
    // The textured voxel shader multiplies by ~0.4-0.5, so we need lights ~2-3x brighter
    const ambientLightObj = new THREE.AmbientLight(0xffffff, 1.5);
    scene.add(ambientLightObj);

    const directionalLightObj = new THREE.DirectionalLight(0xffffff, 2.0);
    directionalLightObj.position.set(5, 10, 5);
    scene.add(directionalLightObj);

    previewSceneRef.current = {
      scene,
      camera,
      renderer,
      cameraDistance,
    };
    setSceneReady(true);

    // Mouse wheel zoom handler
    const handleWheel = (event: WheelEvent) => {
      if (!previewSceneRef.current) return;
      event.preventDefault();

      const { camera, cameraDistance: currentDistance } = previewSceneRef.current;
      const zoomSpeed = 0.001;
      const newDistance = Math.max(1, Math.min(10, currentDistance + event.deltaY * zoomSpeed));

      previewSceneRef.current.cameraDistance = newDistance;
      camera.position.set(0, 0, newDistance);
      camera.lookAt(0, 0, 0);
    };

    canvas.addEventListener('wheel', handleWheel, { passive: false });

    // Animation loop
    const animate = () => {
      if (!previewSceneRef.current) return;

      const { scene, camera, renderer, mesh } = previewSceneRef.current;

      // Rotate mesh slowly around Y axis
      if (mesh) {
        mesh.rotation.y += 0.01;
      }

      renderer.render(scene, camera);
      previewSceneRef.current.animationId = requestAnimationFrame(animate);
    };
    animate();

    return () => {
      canvas.removeEventListener('wheel', handleWheel);
      if (previewSceneRef.current?.animationId) {
        cancelAnimationFrame(previewSceneRef.current.animationId);
      }
      renderer.dispose();
      setSceneReady(false);
    };
  }, [isOpen, previewCanvas]);

  // Update preview when selection changes (but NOT when just switching tabs)
  useEffect(() => {
    logger.log('ui', '[SelectAvatar] Preview effect triggered:', {
      isOpen,
      sceneReady,
      hasScene: !!previewSceneRef.current,
      selectedId,
      avatarUrl,
      avatarType,
      voxModelsCount: voxModels.length,
      glbModelsCount: glbModels.length,
    });

    if (!isOpen || !sceneReady || !previewSceneRef.current) {
      logger.log('ui', '[SelectAvatar] Preview effect skipped: not ready');
      return;
    }

    // Don't reload preview if no model is selected yet
    if (!selectedId && !avatarUrl) {
      logger.log('ui', '[SelectAvatar] Preview effect skipped: no model selected');
      return;
    }

    const { scene, mesh: oldMesh } = previewSceneRef.current;

    // Remove old mesh and dispose resources
    if (oldMesh) {
      scene.remove(oldMesh);

      // Recursively dispose geometries and materials
      oldMesh.traverse((child) => {
        if ((child as THREE.Mesh).isMesh) {
          const mesh = child as THREE.Mesh;
          if (mesh.geometry) {
            mesh.geometry.dispose();
          }
          if (mesh.material) {
            if (Array.isArray(mesh.material)) {
              mesh.material.forEach((m) => m.dispose());
            } else {
              mesh.material.dispose();
            }
          }
        }
      });

      previewSceneRef.current.mesh = undefined;
    }

    // Load the selected model
    const loadPreview = async () => {
      if (!previewSceneRef.current) return;

      let modelUrl: string | undefined;

      logger.log('ui', '[SelectAvatar] Loading preview:', {
        selectedId,
        avatarUrl,
        avatarType,
        voxModelsCount: voxModels.length,
        glbModelsCount: glbModels.length,
      });

      // Determine the model URL based on selection
      if (avatarUrl) {
        modelUrl = avatarUrl;
        logger.log('ui', '[SelectAvatar] Using avatarUrl:', modelUrl);
      } else if (selectedId && selectedId !== 'file') {
        if (avatarType === 'vox') {
          const model = voxModels.find(m => m.id === selectedId);
          logger.log('ui', '[SelectAvatar] Looking for VOX model:', {
            selectedId,
            found: !!model,
            model: model?.filename,
          });
          if (model) {
            modelUrl = `${import.meta.env.BASE_URL}assets/models/vox/${model.filename}`;
          }
        } else if (avatarType === 'glb') {
          const model = glbModels.find(m => m.id === selectedId);
          logger.log('ui', '[SelectAvatar] Looking for GLB model:', {
            selectedId,
            found: !!model,
            model: model?.filename,
          });
          if (model) {
            modelUrl = `${import.meta.env.BASE_URL}assets/models/glb/${model.filename}`;
          }
        }
      }

      logger.log('ui', '[SelectAvatar] Final modelUrl:', modelUrl);

      if (!modelUrl) {
        logger.warn('ui', '[SelectAvatar] No modelUrl found, showing placeholder');
        // Show placeholder (wrapped in group for consistent rotation)
        const geometry = new THREE.BoxGeometry(0.5, 1, 0.5);
        const material = new THREE.MeshStandardMaterial({ color: 0x6496fa });
        const mesh = new THREE.Mesh(geometry, material);
        mesh.position.y = 0.5;

        const group = new THREE.Group();
        group.add(mesh);
        group.rotation.y = Math.PI; // Rotate 180 degrees to show front
        scene.add(group);
        previewSceneRef.current.mesh = group;
        return;
      }

      // Load based on type
      if (avatarType === 'vox' || modelUrl.endsWith('.vox')) {
        // Load VOX file
        try {
          logger.log('ui', '[SelectAvatar] Loading VOX file:', modelUrl);
          const { loadVoxFromUrl } = await import('../utils/voxLoader');
          // Use maxDepth=6 for avatar selector to support larger models in preview
          const geometryData = await loadVoxFromUrl(modelUrl, undefined, 6);
          logger.log('ui', '[SelectAvatar] VOX file loaded:', {
            vertices: geometryData.vertices.length / 3,
            indices: geometryData.indices.length,
          });

          // Check for empty geometry
          if (geometryData.vertices.length === 0 || geometryData.indices.length === 0) {
            logger.error('ui', '[SelectAvatar] VOX file has no geometry (empty model):', modelUrl);
            // Show error placeholder (wrapped in group for consistent rotation)
            const geometry = new THREE.BoxGeometry(0.5, 1, 0.5);
            const material = new THREE.MeshStandardMaterial({ color: 0xff9900 }); // Orange for "empty file"
            const mesh = new THREE.Mesh(geometry, material);
            mesh.position.y = 0.5;

            const group = new THREE.Group();
            group.add(mesh);
            group.rotation.y = Math.PI;
            scene.add(group);
            previewSceneRef.current.mesh = group;
            return;
          }

          if (!previewSceneRef.current) return; // Component unmounted

          // Create geometry from voxel data
          const geometry = new THREE.BufferGeometry();
          geometry.setAttribute('position', new THREE.BufferAttribute(geometryData.vertices, 3));
          geometry.setAttribute('normal', new THREE.BufferAttribute(geometryData.normals, 3));
          geometry.setAttribute('color', new THREE.BufferAttribute(geometryData.colors, 3));
          geometry.setIndex(new THREE.BufferAttribute(geometryData.indices, 1));

          // Generate UV coordinates - combine both approaches
          // Use face normals to determine which axes to use, plus add normal contribution for variation
          const uvs = new Float32Array(geometryData.vertices.length / 3 * 2);

          for (let i = 0; i < geometryData.vertices.length / 3; i++) {
            const nx = geometryData.normals[i * 3];
            const ny = geometryData.normals[i * 3 + 1];
            const nz = geometryData.normals[i * 3 + 2];

            const x = geometryData.vertices[i * 3];
            const y = geometryData.vertices[i * 3 + 1];
            const z = geometryData.vertices[i * 3 + 2];

            // Determine dominant axis from normal (face direction)
            const absNx = Math.abs(nx);
            const absNy = Math.abs(ny);
            const absNz = Math.abs(nz);

            let u: number, v: number;

            // Use position divided by a scale factor for texture tiling
            // Scale by 2^textureDepth to increase texture frequency
            const textureScale = Math.pow(2, textureDepth);
            const scale = 10 * textureScale; // Higher scale = larger UVs = smaller texture

            if (absNy > absNx && absNy > absNz) {
              // Top/Bottom face (Y-dominant)
              u = x / scale + nx * 0.1;
              v = z / scale + nz * 0.1;
            } else if (absNx > absNz) {
              // Left/Right face (X-dominant)
              u = z / scale + nz * 0.1;
              v = y / scale + ny * 0.1;
            } else {
              // Front/Back face (Z-dominant)
              u = x / scale + nx * 0.1;
              v = y / scale + ny * 0.1;
            }

            // Use fractional part for tiling (0-1 range per texture repeat)
            uvs[i * 2] = u - Math.floor(u);
            uvs[i * 2 + 1] = v - Math.floor(v);
          }

          geometry.setAttribute('uv', new THREE.BufferAttribute(uvs, 2));

          // Get material ID from selected texture (only if textures are enabled)
          let materialId = 0; // Default to vertex colors only
          if (avatarTexturesEnabled && avatarTexture && avatarTexture !== '0' && texturesLoaded) {
            const materialsData = (materialsLoader as any).materialsData;
            if (materialsData) {
              const material = materialsData.materials.find((m: any) => m.id === avatarTexture);
              if (material) {
                materialId = material.index;
                logger.log('ui', `[SelectAvatar] Using material ID ${materialId} for texture '${avatarTexture}'`);
              }
            }
          }

          // Add materialId attribute for all vertices
          const materialIds = new Float32Array(geometryData.vertices.length / 3).fill(materialId);
          geometry.setAttribute('materialId', new THREE.BufferAttribute(materialIds, 1));

          // CENTER THE GEOMETRY so rotation happens around geometric center
          geometry.computeBoundingBox();
          const geoCenter = new THREE.Vector3();
          geometry.boundingBox!.getCenter(geoCenter);
          geometry.translate(-geoCenter.x, 0, -geoCenter.z); // Only center horizontally

          // Create material using textured shader if textures are enabled and loaded
          let material: THREE.Material;
          if (avatarTexturesEnabled && texturesLoaded && previewSceneRef.current.renderer) {
            const textureArray = materialsLoader.getTextureArray();
            material = createTexturedVoxelMaterial(textureArray, true, previewSceneRef.current.renderer);
            logger.log('ui', '[SelectAvatar] Using textured material');
          } else {
            material = new THREE.MeshPhongMaterial({
              vertexColors: true,
              specular: 0x111111,
              shininess: 30,
            });
            logger.log('ui', '[SelectAvatar] Using vertex color material (textures disabled)');
          }

          const mesh = new THREE.Mesh(geometry, material);

          // Calculate bounding box for sizing and positioning
          geometry.computeBoundingBox();
          const box = geometry.boundingBox!;
          const size = box.getSize(new THREE.Vector3());

          // Scale to fit in view BEFORE positioning
          const maxDim = Math.max(size.x, size.y, size.z);
          const scale = 1.5 / maxDim;
          mesh.scale.setScalar(scale);

          // Recalculate bounding box after scaling
          const scaledBox = new THREE.Box3().setFromObject(mesh);
          const scaledSize = new THREE.Vector3();
          scaledBox.getSize(scaledSize);

          // Position mesh with pivot at center (0.5, 0.5, 0.5) for all axes
          // Formula: position = -(min + size * pivot)
          const pivot = new THREE.Vector3(0.5, 0.5, 0.5);
          mesh.position.x = -(scaledBox.min.x + scaledSize.x * pivot.x);
          mesh.position.y = -(scaledBox.min.y + scaledSize.y * pivot.y);
          mesh.position.z = -(scaledBox.min.z + scaledSize.z * pivot.z);

          // Add directly to scene - no group needed since geometry is centered
          mesh.rotation.y = Math.PI; // Rotate 180 degrees to show front
          scene.add(mesh);
          previewSceneRef.current.mesh = mesh;

          logger.log('ui', '[SelectAvatar] VOX mesh added to scene:', {
            position: mesh.position.toArray(),
            scale: mesh.scale.toArray(),
            materialType: material.type,
          });

          // Update shader lighting AFTER mesh is added to scene
          if (material instanceof THREE.ShaderMaterial) {
            updateShaderLighting(material, scene);
            logger.log('ui', '[SelectAvatar] Updated shader lighting');
          }
        } catch (error) {
          logger.error('ui', '[SelectAvatar] Failed to load VOX model:', error);
          // Show error placeholder (wrapped in group for consistent rotation)
          const geometry = new THREE.BoxGeometry(0.5, 1, 0.5);
          const material = new THREE.MeshStandardMaterial({ color: 0xff0000 });
          const mesh = new THREE.Mesh(geometry, material);
          mesh.position.y = 0.5;

          const group = new THREE.Group();
          group.add(mesh);
          group.rotation.y = Math.PI; // Rotate 180 degrees to show front
          scene.add(group);
          previewSceneRef.current.mesh = group;
        }
      }
    };

    loadPreview();
  }, [isOpen, sceneReady, selectedId, avatarUrl, voxModels, glbModels, avatarType, avatarTexture, texturesLoaded, materialsLoader, textureDepth, avatarTexturesEnabled]);

  const handleSave = () => {
    const selection: AvatarSelection = {
      avatarType,
      avatarId: avatarType === 'vox' ? selectedId : (avatarType === 'glb' && selectedId !== 'file' ? selectedId : undefined),
      avatarUrl: avatarUrl || undefined,
      avatarData: undefined, // CSM removed
      avatarTexture: avatarTexture !== '0' ? avatarTexture : undefined,
      teleportAnimationType,
    };
    onSave(selection);
    onClose();
  };

  const handleLoadUrl = () => {
    if (inputUrl.trim()) {
      setAvatarUrl(inputUrl.trim());
      setSelectedId('file');
    }
  };

  const handleFileSelect = (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (file && (file.name.endsWith('.glb') || file.name.endsWith('.vox'))) {
      const url = URL.createObjectURL(file);
      setAvatarUrl(url);
      setSelectedId('file');
      setInputUrl(file.name);
    }
  };

  const handleOpenCreator = () => {
    ReadyPlayerMeService.openAvatarCreator();
  };

  return (
    <ResponsivePanel
      isOpen={isOpen}
      onClose={hasPreviousSelection ? onClose : undefined}
      closeOnClickOutside={hasPreviousSelection}
      closeOnEsc={hasPreviousSelection}
      forceFullscreen={true}
      padding={6}
      zIndex={2000}
      title="Select Avatar"
      actions={
        <VStack spacing={2} width="100%" maxW="800px" mx="auto">
          <Button width="100%" colorScheme="blue" onClick={handleSave} size="lg">
            That's me!
          </Button>
          <Button width="100%" variant="outline" onClick={randomizeAvatar} size="lg">
            Not me
          </Button>
        </VStack>
      }
    >
      <Box
        maxW="1200px"
        mx="auto"
        w="full"
      >
        <VStack align="stretch" spacing={4}>
          {/* Preview Canvas with Cycle Buttons */}
          <Box position="relative" display="flex" justifyContent="center">
              <Box
                ref={previewContainerRef}
                bg="rgba(26, 26, 30, 0.8)"
                borderRadius="md"
                p={2}
                border="1px solid"
                borderColor="rgba(255, 255, 255, 0.1)"
                display="inline-block"
              >
                <canvas
                  ref={setPreviewCanvas}
                  style={{
                    borderRadius: '8px',
                    maxWidth: '100%',
                    height: 'auto',
                    display: 'block',
                  }}
                />
              </Box>

            </Box>

            {/* More Settings Toggle */}
            <Button
              onClick={() => setShowMoreSettings(!showMoreSettings)}
              variant="ghost"
              size="sm"
              color="white"
            >
              {showMoreSettings ? '▼' : '▶'} More Settings
            </Button>

            {/* More Settings Collapsible */}
            <Collapse in={showMoreSettings} animateOpacity>
              <VStack align="stretch" spacing={4}>
            {/* Model Selector */}
            <HStack spacing={2} justify="space-between">
              <Text fontSize="sm" color="white">
                Model
              </Text>
              <Popover placement="top">
                {({ onClose }) => (
                  <>
                    <PopoverTrigger>
                      <Badge
                        colorScheme="blue"
                        fontSize="xs"
                        cursor="pointer"
                        _hover={{ opacity: 0.8 }}
                      >
                        {voxModels.find(m => m.id === selectedId)?.label || 'Select...'}
                      </Badge>
                    </PopoverTrigger>
                    <PopoverContent bg="gray.800" borderColor="blue.500" width="auto" maxH="300px" overflowY="auto">
                      <PopoverBody p={1}>
                        <VStack spacing={1}>
                          {voxModels.map((model) => (
                            <Button
                              key={model.id}
                              size="xs"
                              variant={selectedId === model.id ? 'solid' : 'ghost'}
                              colorScheme="blue"
                              onClick={() => {
                                setSelectedId(model.id);
                                setAvatarUrl('');
                                onClose();
                              }}
                              width="100%"
                            >
                              {model.label}
                            </Button>
                          ))}
                        </VStack>
                      </PopoverBody>
                    </PopoverContent>
                  </>
                )}
              </Popover>
            </HStack>

            {/* Material Selector - Only show when textures are enabled */}
            {avatarTexturesEnabled && (
              <>
                <HStack spacing={2} justify="space-between">
                  <Text fontSize="sm" color="white">
                    Material
                  </Text>
                  <Popover placement="top">
                    {({ onClose }) => (
                      <>
                        <PopoverTrigger>
                          <Badge
                            colorScheme="purple"
                            fontSize="xs"
                            cursor="pointer"
                            _hover={{ opacity: 0.8 }}
                          >
                            {avatarTexture === '0' ? 'None' : avatarTexture.replace(/_/g, ' ')}
                          </Badge>
                        </PopoverTrigger>
                        <PopoverContent bg="gray.800" borderColor="purple.500" width="auto" maxH="300px" overflowY="auto">
                          <PopoverBody p={1}>
                            <VStack spacing={1}>
                              <Button
                                size="xs"
                                variant={avatarTexture === '0' ? 'solid' : 'ghost'}
                                colorScheme="purple"
                                onClick={() => {
                                  handleTextureSelect('0');
                                  onClose();
                                }}
                                width="100%"
                              >
                                None
                              </Button>
                              {AVATAR_TEXTURES.map((texture) => (
                                <Button
                                  key={texture}
                                  size="xs"
                                  variant={avatarTexture === texture ? 'solid' : 'ghost'}
                                  colorScheme="purple"
                                  onClick={() => {
                                    handleTextureSelect(texture);
                                    onClose();
                                  }}
                                  width="100%"
                                >
                                  {texture.replace(/_/g, ' ')}
                                </Button>
                              ))}
                            </VStack>
                          </PopoverBody>
                        </PopoverContent>
                      </>
                    )}
                  </Popover>
                </HStack>

                {/* Texture Scale Control */}
                <HStack spacing={2} justify="space-between">
                  <Text fontSize="sm" color="white">
                    Texture Scale
                  </Text>
                  <Popover placement="top">
                    {({ onClose }) => (
                      <>
                        <PopoverTrigger>
                          <Badge
                            colorScheme="orange"
                            fontSize="xs"
                            cursor="pointer"
                            _hover={{ opacity: 0.8 }}
                          >
                            {Math.pow(2, textureDepth).toFixed(1)}x
                          </Badge>
                        </PopoverTrigger>
                        <PopoverContent bg="gray.800" borderColor="orange.500" width="auto">
                          <PopoverBody p={1}>
                            <VStack spacing={1}>
                              {[0, 1, 2, 3, 4, 5].map((depth) => (
                                <Button
                                  key={depth}
                                  size="xs"
                                  variant={textureDepth === depth ? 'solid' : 'ghost'}
                                  colorScheme="orange"
                                  onClick={() => {
                                    setTextureDepth(depth);
                                    onClose();
                                  }}
                                  width="100%"
                                >
                                  {Math.pow(2, depth).toFixed(1)}x
                                </Button>
                              ))}
                            </VStack>
                          </PopoverBody>
                        </PopoverContent>
                      </>
                    )}
                  </Popover>
                </HStack>
              </>
            )}

            {/* Teleport Animation */}
            <HStack spacing={2} justify="space-between">
              <HStack spacing={2}>
                <Text fontSize="sm" color="white">
                  Teleport
                </Text>
                <Badge colorScheme="gray" fontSize="2xs" px={2}>
                  CTRL+Click
                </Badge>
              </HStack>
              <Popover placement="top">
                {({ onClose }) => {
                  const teleportOptions: { type: TeleportAnimationType; icon: string; label: string }[] = [
                    { type: 'fade', icon: '🌫️', label: 'Fade' },
                    { type: 'scale', icon: '⚫', label: 'Scale' },
                    { type: 'spin', icon: '🌀', label: 'Spin' },
                    { type: 'slide', icon: '⬇️', label: 'Slide' },
                    { type: 'burst', icon: '✨', label: 'Burst' },
                  ];
                  const currentOption = teleportOptions.find(opt => opt.type === teleportAnimationType);

                  return (
                    <>
                      <PopoverTrigger>
                        <Badge
                          colorScheme="green"
                          fontSize="xs"
                          cursor="pointer"
                          _hover={{ opacity: 0.8 }}
                        >
                          {currentOption?.icon} {currentOption?.label}
                        </Badge>
                      </PopoverTrigger>
                      <PopoverContent bg="gray.800" borderColor="green.500" width="auto">
                        <PopoverBody p={1}>
                          <VStack spacing={1}>
                            {teleportOptions.map((option) => (
                              <Button
                                key={option.type}
                                size="xs"
                                variant={teleportAnimationType === option.type ? 'solid' : 'ghost'}
                                colorScheme="green"
                                onClick={() => {
                                  setTeleportAnimationType(option.type);
                                  onClose();
                                }}
                                width="100%"
                                leftIcon={<Text fontSize="sm">{option.icon}</Text>}
                              >
                                {option.label}
                              </Button>
                            ))}
                          </VStack>
                        </PopoverBody>
                      </PopoverContent>
                    </>
                  );
                }}
              </Popover>
            </HStack>

            {/* Load Model Section */}
            <VStack align="stretch" spacing={2}>
              <Text fontSize="sm" fontWeight="semibold" color="white">
                Load Custom Model
              </Text>

              <Input
                value={inputUrl}
                onChange={(e) => setInputUrl(e.target.value)}
                placeholder="https://models.readyplayer.me/... or .vox/.glb URL"
                size="sm"
                fontSize="xs"
                bg="rgba(255, 255, 255, 0.05)"
                color="white"
                borderColor="rgba(255, 255, 255, 0.2)"
                _hover={{ borderColor: 'rgba(255, 255, 255, 0.3)' }}
                _focus={{ borderColor: 'blue.400', boxShadow: '0 0 0 1px #3182ce' }}
              />

              <HStack spacing={2}>
                <Button
                  size="sm"
                  fontSize="xs"
                  colorScheme="blue"
                  onClick={handleLoadUrl}
                  flex={1}
                  isDisabled={!inputUrl.trim()}
                >
                  Load URL
                </Button>

                <input
                  ref={fileInputRef}
                  type="file"
                  accept=".glb,.vox"
                  onChange={handleFileSelect}
                  style={{ display: 'none' }}
                />

                <Button
                  size="sm"
                  fontSize="xs"
                  colorScheme="green"
                  onClick={() => fileInputRef.current?.click()}
                  flex={1}
                >
                  Load File
                </Button>

                <Button
                  size="sm"
                  fontSize="xs"
                  colorScheme="purple"
                  onClick={handleOpenCreator}
                  flex={1}
                >
                  RPM Creator
                </Button>
              </HStack>
            </VStack>

            {ENABLE_AVATAR_COLOR_SELECTION && (
              <>
                <Divider borderColor="rgba(255, 255, 255, 0.1)" />

                {/* Color Options */}
                <VStack align="stretch" spacing={2}>
                  <Text fontSize="sm" fontWeight="semibold" color="white">
                    Color Options
                  </Text>
                  <Text fontSize="xs" color="gray.400">
                    Feature disabled
                  </Text>
                </VStack>
              </>
            )}
              </VStack>
            </Collapse>
        </VStack>
      </Box>
    </ResponsivePanel>
  );
}

