import * as logger from '../utils/logger';
import { useEffect, useRef, useState } from 'react';
import { Box } from '@chakra-ui/react';
import { SceneManager } from '../renderer/scene';
import { GeometryController } from '../geometry/geometry-controller';
import init from 'crossworld-world';
import type { AvatarStateService, AvatarConfig } from '../services/avatar-state';
import type { TeleportAnimationType } from '../renderer/teleport-animation';
import { DebugPanel, type DebugInfo } from './WorldPanel';
import { onDepthChange, onSeedChange, getSeed } from '../config/depth-config';
import type { MainMode } from '@crossworld/common';

interface WorldCanvasProps {
  isLoggedIn: boolean;
  isEditMode: boolean;
  mainMode: MainMode;
  isCameraMode: boolean;
  avatarConfig: AvatarConfig;
  teleportAnimationType: TeleportAnimationType;
  avatarStateService?: AvatarStateService;
  currentUserPubkey?: string | null;
  geometryControllerRef?: React.MutableRefObject<any>;
  sceneManagerRef?: React.MutableRefObject<any>;
  onWorldCSMUpdate?: (csmText: string) => void;
  timeOfDay: number;
  sunAutoMove: boolean;
  sunSpeed: number;
  onPublishWorld?: () => void;
}

export function WorldCanvas({
  isLoggedIn,
  isEditMode,
  mainMode,
  isCameraMode,
  avatarConfig,
  teleportAnimationType,
  avatarStateService,
  currentUserPubkey,
  geometryControllerRef,
  sceneManagerRef,
  onWorldCSMUpdate,
  timeOfDay,
  sunAutoMove,
  sunSpeed,
  onPublishWorld,
}: WorldCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const localSceneManagerRef = useRef<SceneManager | null>(null);
  const localGeometryControllerRef = useRef<GeometryController | null>(null);
  const animationFrameRef = useRef<number | null>(null);
  const [debugInfo, setDebugInfo] = useState<DebugInfo>({});

  // Scene configuration state with localStorage persistence
  const [worldGridVisible, setWorldGridVisible] = useState(() => {
    const saved = localStorage.getItem('worldPanel.worldGridVisible');
    return saved !== null ? JSON.parse(saved) : false; // Hidden by default
  });
  const [wireframeEnabled, setWireframeEnabled] = useState(() => {
    const saved = localStorage.getItem('worldPanel.wireframeEnabled');
    return saved !== null ? JSON.parse(saved) : false;
  });
  const [texturesEnabled, setTexturesEnabled] = useState(() => {
    const saved = localStorage.getItem('worldPanel.texturesEnabled');
    return saved !== null ? JSON.parse(saved) : false;
  });
  const [avatarTexturesEnabled, setAvatarTexturesEnabled] = useState(() => {
    const saved = localStorage.getItem('worldPanel.avatarTexturesEnabled');
    return saved !== null ? JSON.parse(saved) : false;
  });

  // Save settings to localStorage when they change
  useEffect(() => {
    localStorage.setItem('worldPanel.worldGridVisible', JSON.stringify(worldGridVisible));
  }, [worldGridVisible]);

  useEffect(() => {
    localStorage.setItem('worldPanel.wireframeEnabled', JSON.stringify(wireframeEnabled));
  }, [wireframeEnabled]);

  useEffect(() => {
    localStorage.setItem('worldPanel.texturesEnabled', JSON.stringify(texturesEnabled));
  }, [texturesEnabled]);

  useEffect(() => {
    localStorage.setItem('worldPanel.avatarTexturesEnabled', JSON.stringify(avatarTexturesEnabled));
  }, [avatarTexturesEnabled]);

  const handleApplyDepthSettings = async (worldDepth: number, scaleDepth: number) => {
    const geometryController = localGeometryControllerRef.current;
    const sceneManager = localSceneManagerRef.current;

    if (!geometryController || !sceneManager) {
      logger.error('renderer', 'Cannot apply depth settings: geometry controller or scene manager not initialized');
      return;
    }

    // Calculate macro depth from total depth and micro depth
    const macroDepth = worldDepth - scaleDepth;
    const microDepth = scaleDepth;


    try {
      // Only reinitialize if MACRO depth changed (world size change)
      // Micro depth changes don't need reinit since Rust ignores it and world scale stays constant
      const currentMacro = geometryController.getMacroDepth();
      const currentBorder = geometryController.getBorderDepth();

      if (currentMacro !== macroDepth) {
        await geometryController.reinitialize(macroDepth, microDepth, currentBorder, getSeed(), (geometry) => {
          sceneManager.updateGeometry(
            geometry.vertices,
            geometry.indices,
            geometry.normals,
            geometry.colors,
            geometry.uvs,
            geometry.materialIds
          );
        });
      }
    } catch (error) {
      logger.error('renderer', 'Failed to apply depth settings:', error);
    }
  };

  useEffect(() => {
    if (!canvasRef.current) return;

    const canvas = canvasRef.current;
    const sceneManager = new SceneManager();
    const geometryController = new GeometryController();

    localSceneManagerRef.current = sceneManager;
    localGeometryControllerRef.current = geometryController;

    // Expose geometry controller to parent if ref provided
    if (geometryControllerRef) {
      geometryControllerRef.current = geometryController;
    }

    // Expose scene manager to parent if ref provided
    if (sceneManagerRef) {
      sceneManagerRef.current = sceneManager;
    }

    // Initialize scene (async to allow physics WASM loading)
    sceneManager.initialize(canvas).catch((error: unknown) => {
      logger.error('renderer', 'Failed to initialize scene:', error);
    });

    // Set position update callback and subscribe to state changes
    let unsubscribe: (() => void) | undefined;
    if (avatarStateService) {
      sceneManager.setPositionUpdateCallback((x, y, z, quaternion, moveStyle) => {
        avatarStateService.publishPosition({ x, y, z, quaternion }, moveStyle).catch(console.error);
      });

      // Subscribe to avatar state changes
      unsubscribe = avatarStateService.onChange((states) => {
        sceneManager.updateRemoteAvatars(states);
      });
    }

    // Set voxel edit callback for world cube editing
    sceneManager.setOnVoxelEdit((coord, colorIndex) => {
      if (colorIndex === 0) {
        // Remove voxel at specified depth
        geometryController.removeVoxelAtDepth(coord.x, coord.y, coord.z, coord.depth);
      } else {
        // Set voxel at specified depth
        geometryController.setVoxelAtDepth(coord.x, coord.y, coord.z, coord.depth, colorIndex);
      }
    });

    // Initialize WASM
    init().catch((error: unknown) => {
      logger.error('renderer', 'Failed to initialize WASM:', error);
    });

    // Initialize geometry controller
    geometryController.initialize((geometry) => {
      sceneManager.updateGeometry(
        geometry.vertices,
        geometry.indices,
        geometry.normals,
        geometry.colors,
        geometry.uvs,
        geometry.materialIds
      );
    }, (csmText: string) => {
      // Load into WASM for raycasting
      sceneManager.loadWorldCube(csmText);
      // Call parent callback if provided
      onWorldCSMUpdate?.(csmText);
    }).catch((error) => {
      logger.error('renderer', 'Failed to initialize geometry controller:', error);
    });

    // Animation loop
    const animate = () => {
      sceneManager.render();

      // Update debug info every frame
      setDebugInfo(sceneManager.getDebugInfo());

      animationFrameRef.current = requestAnimationFrame(animate);
    };
    animate();

    // Handle resize
    const handleResize = () => {
      sceneManager.handleResize();
    };
    window.addEventListener('resize', handleResize);

    // Cleanup
    return () => {
      if (animationFrameRef.current) {
        cancelAnimationFrame(animationFrameRef.current);
      }
      window.removeEventListener('resize', handleResize);
      sceneManager.dispose();
      geometryController.destroy();
      if (unsubscribe) {
        unsubscribe();
      }
    };
  }, [avatarStateService]);

  // Subscribe to depth config changes and reinitialize when macro or border depth changes
  useEffect(() => {
    const geometryController = localGeometryControllerRef.current;
    const sceneManager = localSceneManagerRef.current;

    if (!geometryController || !sceneManager) return;

    const unsubscribe = onDepthChange((macroDepth, microDepth, borderDepth) => {
      const currentMacro = geometryController.getMacroDepth();
      const currentBorder = geometryController.getBorderDepth();

      // Reinitialize if macro or border depth changed (these affect world generation)
      if (currentMacro !== macroDepth || currentBorder !== borderDepth) {
        logger.log('renderer', `[WorldCanvas] Depth changed: macro ${currentMacro}->${macroDepth}, border ${currentBorder}->${borderDepth}, reinitializing...`);
        geometryController.reinitialize(macroDepth, microDepth, borderDepth, getSeed(), (geometry) => {
          sceneManager.updateGeometry(
            geometry.vertices,
            geometry.indices,
            geometry.normals,
            geometry.colors,
            geometry.uvs,
            geometry.materialIds
          );
        }).catch((error) => {
          logger.error('renderer', 'Failed to reinitialize geometry controller:', error);
        });
      }
    });

    return unsubscribe;
  }, []);

  // Subscribe to seed changes and reinitialize when seed changes
  useEffect(() => {
    const geometryController = localGeometryControllerRef.current;
    const sceneManager = localSceneManagerRef.current;

    if (!geometryController || !sceneManager) return;

    const unsubscribe = onSeedChange((seed) => {
      logger.log('renderer', `[WorldCanvas] Seed changed to ${seed}, reinitializing world...`);
      geometryController.reinitialize(
        geometryController.getMacroDepth(),
        geometryController.getMicroDepth(),
        geometryController.getBorderDepth(),
        seed,
        (geometry) => {
          sceneManager.updateGeometry(
            geometry.vertices,
            geometry.indices,
            geometry.normals,
            geometry.colors,
            geometry.uvs,
            geometry.materialIds
          );
          setTriangleCount(geometry.stats.triangles);
        }
      ).catch((error) => {
        logger.error('renderer', 'Failed to reinitialize geometry controller:', error);
      });
    });

    return unsubscribe;
  }, []);

  // Handle current user pubkey changes
  useEffect(() => {
    const sceneManager = localSceneManagerRef.current;
    if (!sceneManager) return;

    sceneManager.setCurrentUserPubkey(currentUserPubkey || null);
  }, [currentUserPubkey]);

  // Handle main mode changes (walk/edit/placement)
  useEffect(() => {
    const sceneManager = localSceneManagerRef.current;
    if (!sceneManager) return;

    sceneManager.setMainMode(mainMode);
  }, [mainMode]);

  // Handle edit mode changes (for backward compatibility)
  useEffect(() => {
    const sceneManager = localSceneManagerRef.current;
    if (!sceneManager) return;

    sceneManager.setEditMode(isEditMode);
  }, [isEditMode]);

  // Handle camera mode changes
  useEffect(() => {
    const sceneManager = localSceneManagerRef.current;
    if (!sceneManager) return;

    sceneManager.setCameraMode(isCameraMode);
  }, [isCameraMode]);

  // Handle teleport animation type changes
  useEffect(() => {
    const sceneManager = localSceneManagerRef.current;
    if (!sceneManager) return;

    sceneManager.setTeleportAnimationType(teleportAnimationType);
  }, [teleportAnimationType]);

  // Handle sun system changes
  useEffect(() => {
    const sceneManager = localSceneManagerRef.current;
    if (!sceneManager) return;

    sceneManager.setTimeOfDay(timeOfDay);
  }, [timeOfDay]);

  useEffect(() => {
    const sceneManager = localSceneManagerRef.current;
    if (!sceneManager) return;

    sceneManager.setSunAutoMove(sunAutoMove);
  }, [sunAutoMove]);

  useEffect(() => {
    const sceneManager = localSceneManagerRef.current;
    if (!sceneManager) return;

    sceneManager.setSunSpeed(sunSpeed);
  }, [sunSpeed]);

  // Handle world grid visibility - only show in edit mode
  useEffect(() => {
    const sceneManager = localSceneManagerRef.current;
    if (!sceneManager) return;

    // Only show world grid when in edit mode AND the toggle is enabled
    sceneManager.setWorldGridVisible(isEditMode && worldGridVisible);
  }, [worldGridVisible, isEditMode]);

  // Handle wireframe mode
  useEffect(() => {
    const sceneManager = localSceneManagerRef.current;
    if (!sceneManager) return;

    sceneManager.setWireframe(wireframeEnabled);
  }, [wireframeEnabled]);

  // Handle textures toggle
  useEffect(() => {
    const sceneManager = localSceneManagerRef.current;
    if (!sceneManager) return;

    sceneManager.setTextures(texturesEnabled);
  }, [texturesEnabled]);

  // Handle avatar textures toggle
  useEffect(() => {
    const sceneManager = localSceneManagerRef.current;
    if (!sceneManager) return;

    sceneManager.setAvatarTextures(avatarTexturesEnabled);
  }, [avatarTexturesEnabled]);

  // Handle avatar loading based on new unified avatar config
  useEffect(() => {
    const sceneManager = localSceneManagerRef.current;
    if (!sceneManager) return;

    logger.log('renderer', 'Avatar update triggered:', {
      isLoggedIn,
      avatarType: avatarConfig.avatarType,
      avatarId: avatarConfig.avatarId,
      avatarUrl: avatarConfig.avatarUrl
    });

    const loadAvatar = async () => {
      if (isLoggedIn) {
      // Preserve current transform (position + rotation) if avatar exists
      const currentTransform = sceneManager.getCurrentTransform();

      // Priority loading order:
      // 1. Load from avatarId (predefined models)
      // 2. Load from avatarUrl (custom URL)
      // 3. Load from avatarData (procedural generation - not yet implemented)
      // Finally: Apply avatarMod (modifications - not yet implemented)

      // Use original colors (undefined = original palette)
      const npubForColors = undefined;

      if (avatarConfig.avatarType === 'vox') {
        // Remove old GLB avatar if exists
        sceneManager.removeAvatar();

        // Try to load from avatarId first
        if (avatarConfig.avatarId && avatarConfig.avatarId !== 'file') {
          // Load predefined voxel model from disk
          const { getModelUrl } = await import('../utils/modelConfig');
          const voxUrl = getModelUrl(avatarConfig.avatarId, 'vox');
          if (voxUrl) {
            sceneManager.createVoxelAvatarFromVoxFile(voxUrl, npubForColors, 1.0, currentTransform, undefined, avatarConfig.avatarTexture)
              .then(() => {
                // Refresh profile in case pubkey was set before avatar loaded
                sceneManager.refreshCurrentAvatarProfile();
              })
              .catch(error => {
                logger.error('renderer', '[WorldCanvas] Failed to load voxel avatar from disk:', error);
                // Fallback to generated
                sceneManager.createVoxelAvatar('npub1default', 1.0, currentTransform);
                sceneManager.refreshCurrentAvatarProfile();
              });
          } else {
            logger.warn('renderer', '[WorldCanvas] Unknown voxel model ID:', avatarConfig.avatarId);
            sceneManager.createVoxelAvatar('npub1default', 1.0, currentTransform);
            sceneManager.refreshCurrentAvatarProfile();
          }
        } else if (avatarConfig.avatarUrl) {
          // Load from custom URL
          sceneManager.createVoxelAvatarFromVoxFile(avatarConfig.avatarUrl, npubForColors, 1.0, currentTransform, undefined, avatarConfig.avatarTexture)
            .then(() => {
              sceneManager.refreshCurrentAvatarProfile();
            })
            .catch(error => {
              logger.error('renderer', '[WorldCanvas] Failed to load voxel avatar from URL:', error);
              sceneManager.createVoxelAvatar('npub1default', 1.0, currentTransform);
              sceneManager.refreshCurrentAvatarProfile();
            });
        } else if (avatarConfig.avatarData) {
          // TODO: Generate from avatarData (procedural generation)
          sceneManager.createVoxelAvatar('npub1default', 1.0, currentTransform);
          sceneManager.refreshCurrentAvatarProfile();
        } else {
          // Fallback to simple generated model
          sceneManager.createVoxelAvatar('npub1default', 1.0, currentTransform);
          sceneManager.refreshCurrentAvatarProfile();
        }
      } else if (avatarConfig.avatarType === 'glb') {
        // Remove existing avatar if exists
        sceneManager.removeAvatar();

        // Load GLB avatar
        let glbUrl: string | undefined;

        // Try avatarId first (predefined GLB models)
        if (avatarConfig.avatarId && avatarConfig.avatarId !== 'file') {
          const { getModelUrl } = await import('../utils/modelConfig');
          glbUrl = getModelUrl(avatarConfig.avatarId, 'glb') || undefined;
        }

        // Fallback to avatarUrl
        if (!glbUrl && avatarConfig.avatarUrl) {
          glbUrl = avatarConfig.avatarUrl;
        }

        // Load the GLB
        if (glbUrl) {
          logger.log('renderer', 'Loading GLB avatar:', glbUrl);
          try {
            // Check if file exists before loading
            const checkResponse = await fetch(glbUrl, { method: 'HEAD' });
            if (checkResponse.ok) {
              sceneManager.createAvatar(glbUrl, 1.0, currentTransform);
              sceneManager.refreshCurrentAvatarProfile();
            } else {
              logger.warn('renderer', 'GLB model not found:', glbUrl);
              // Don't create avatar if model doesn't exist
            }
          } catch (error) {
            logger.error('renderer', 'Failed to check/load GLB avatar:', error);
          }
        } else {
          logger.warn('renderer', 'No GLB URL available for avatar');
        }
      } else if (avatarConfig.avatarType === 'csm') {
        // Remove old avatar
        sceneManager.removeAvatar();

        // Get CSM code from avatarData
        if (avatarConfig.avatarData) {
          try {
            const { parseCsmToMesh } = await import('../utils/cubeWasm');
            const result = await parseCsmToMesh(avatarConfig.avatarData);

            if ('error' in result) {
              logger.error('renderer', '[WorldCanvas] CSM parse error:', result.error);
              return;
            }

            // Create CSM avatar from mesh data
            await sceneManager.createCsmAvatar(result, npubForColors, 1.0, currentTransform, undefined, avatarConfig.avatarTexture);
            sceneManager.refreshCurrentAvatarProfile();
          } catch (error) {
            logger.error('renderer', '[WorldCanvas] Failed to load CSM avatar:', error);
          }
        } else {
          logger.warn('renderer', '[WorldCanvas] No avatarData provided for CSM avatar');
        }
      }

      // TODO: Apply avatarMod if present
      if (avatarConfig.avatarMod) {
        logger.log('renderer', 'Avatar modifications not yet implemented');
      }
      } else {
        sceneManager.removeAvatar();
      }
    };

    loadAvatar();
  }, [isLoggedIn, avatarConfig.avatarType, avatarConfig.avatarId, avatarConfig.avatarUrl, avatarConfig.avatarData, avatarConfig.avatarMod, avatarConfig.avatarTexture]);

  return (
    <Box
      position="fixed"
      top={0}
      left={0}
      width="100vw"
      height="100vh"
      zIndex={0}
    >
      <canvas ref={canvasRef} style={{ display: 'block', width: '100%', height: '100%' }} />
      {isEditMode && (
        <DebugPanel
          info={debugInfo}
          onApplyDepthSettings={handleApplyDepthSettings}
          worldGridVisible={worldGridVisible}
          onWorldGridVisibleChange={setWorldGridVisible}
          wireframeEnabled={wireframeEnabled}
          onWireframeEnabledChange={setWireframeEnabled}
          texturesEnabled={texturesEnabled}
          onTexturesEnabledChange={setTexturesEnabled}
          avatarTexturesEnabled={avatarTexturesEnabled}
          onAvatarTexturesEnabledChange={setAvatarTexturesEnabled}
          onPublishWorld={onPublishWorld}
          isLoggedIn={isLoggedIn}
        />
      )}
    </Box>
  );
}

