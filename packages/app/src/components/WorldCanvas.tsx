import * as logger from '../utils/logger';
import { useEffect, useRef, useState } from 'react';
import { Box } from '@chakra-ui/react';
import { SceneManager } from '../renderer/scene';
import { GeometryController } from '../geometry/geometry-controller';
import init, { AvatarEngine } from '@workspace/wasm';
import type { AvatarStateService, AvatarConfig } from '../services/avatar-state';
import type { TeleportAnimationType } from '../renderer/teleport-animation';
import { DebugPanel, type DebugInfo } from './WorldPanel';
import { onDepthChange } from '../config/depth-config';

interface WorldCanvasProps {
  isLoggedIn: boolean;
  isEditMode: boolean;
  isCameraMode: boolean;
  avatarConfig: AvatarConfig;
  teleportAnimationType: TeleportAnimationType;
  avatarStateService?: AvatarStateService;
  currentUserPubkey?: string | null;
  geometryControllerRef?: React.MutableRefObject<any>;
  sceneManagerRef?: React.MutableRefObject<any>;
  speechEnabled?: boolean;
  onSpeechEnabledChange?: (enabled: boolean) => void;
  onWorldCSMUpdate?: (csmText: string) => void;
  timeOfDay: number;
  sunAutoMove: boolean;
  sunSpeed: number;
  onPublishWorld?: () => void;
}

export function WorldCanvas({
  isLoggedIn,
  isEditMode,
  isCameraMode,
  avatarConfig,
  teleportAnimationType,
  avatarStateService,
  currentUserPubkey,
  geometryControllerRef,
  sceneManagerRef,
  speechEnabled: externalSpeechEnabled,
  onSpeechEnabledChange: externalOnSpeechEnabledChange,
  onWorldCSMUpdate,
  timeOfDay,
  sunAutoMove,
  sunSpeed,
  onPublishWorld,
}: WorldCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const localSceneManagerRef = useRef<SceneManager | null>(null);
  const localGeometryControllerRef = useRef<GeometryController | null>(null);
  const avatarEngineRef = useRef<AvatarEngine | null>(null);
  const animationFrameRef = useRef<number | null>(null);
  const [debugInfo, setDebugInfo] = useState<DebugInfo>({});

  // Scene configuration state with localStorage persistence
  const [internalSpeechEnabled, setInternalSpeechEnabled] = useState(() => {
    const saved = localStorage.getItem('worldPanel.speechEnabled');
    return saved !== null ? JSON.parse(saved) : false;
  });
  const [worldGridVisible, setWorldGridVisible] = useState(() => {
    const saved = localStorage.getItem('worldPanel.worldGridVisible');
    return saved !== null ? JSON.parse(saved) : false; // Hidden by default
  });
  const [faceMeshEnabled, setFaceMeshEnabled] = useState(() => {
    const saved = localStorage.getItem('worldPanel.faceMeshEnabled');
    return saved !== null ? JSON.parse(saved) : true;
  });
  const [wireframeEnabled, setWireframeEnabled] = useState(() => {
    const saved = localStorage.getItem('worldPanel.wireframeEnabled');
    return saved !== null ? JSON.parse(saved) : false;
  });
  const [triangleCount, setTriangleCount] = useState<number | undefined>(undefined);

  // Save settings to localStorage when they change
  useEffect(() => {
    localStorage.setItem('worldPanel.speechEnabled', JSON.stringify(internalSpeechEnabled));
  }, [internalSpeechEnabled]);

  useEffect(() => {
    localStorage.setItem('worldPanel.worldGridVisible', JSON.stringify(worldGridVisible));
  }, [worldGridVisible]);

  useEffect(() => {
    localStorage.setItem('worldPanel.faceMeshEnabled', JSON.stringify(faceMeshEnabled));
  }, [faceMeshEnabled]);

  useEffect(() => {
    localStorage.setItem('worldPanel.wireframeEnabled', JSON.stringify(wireframeEnabled));
  }, [wireframeEnabled]);

  // Use external speechEnabled if provided, otherwise use internal state
  const speechEnabled = externalSpeechEnabled ?? internalSpeechEnabled;
  const setSpeechEnabled = externalOnSpeechEnabledChange ?? setInternalSpeechEnabled;

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
        await geometryController.reinitialize(macroDepth, microDepth, currentBorder, (geometry) => {
          sceneManager.updateGeometry(
            geometry.vertices,
            geometry.indices,
            geometry.normals,
            geometry.colors
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

    // Initialize scene
    sceneManager.initialize(canvas);

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

    // Initialize WASM and avatar engine
    init().then(() => {
      const avatarEngine = new AvatarEngine();
      avatarEngineRef.current = avatarEngine;
      sceneManager.setAvatarEngine(avatarEngine);
    }).catch((error: unknown) => {
      logger.error('renderer', 'Failed to initialize WASM/Avatar engine:', error);
    });

    // Initialize geometry controller
    geometryController.initialize((geometry) => {
      sceneManager.updateGeometry(
        geometry.vertices,
        geometry.indices,
        geometry.normals,
        geometry.colors
      );
      // Update triangle count
      setTriangleCount(geometry.stats.triangles);
    }, onWorldCSMUpdate).then(() => {
      // Set initial face mesh mode (enabled by default)
      geometryController.setFaceMeshMode(true);
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
        geometryController.reinitialize(macroDepth, microDepth, borderDepth, (geometry) => {
          sceneManager.updateGeometry(
            geometry.vertices,
            geometry.indices,
            geometry.normals,
            geometry.colors
          );
          setTriangleCount(geometry.stats.triangles);
        }).catch((error) => {
          logger.error('renderer', 'Failed to reinitialize geometry controller:', error);
        });
      }
    });

    return unsubscribe;
  }, []);

  // Handle current user pubkey changes
  useEffect(() => {
    const sceneManager = localSceneManagerRef.current;
    if (!sceneManager) return;

    sceneManager.setCurrentUserPubkey(currentUserPubkey || null);
  }, [currentUserPubkey]);

  // Handle edit mode changes
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

  // Handle face mesh mode
  useEffect(() => {
    const geometryController = localGeometryControllerRef.current;
    if (!geometryController) return;

    geometryController.setFaceMeshMode(faceMeshEnabled);
  }, [faceMeshEnabled]);

  // Handle wireframe mode
  useEffect(() => {
    const sceneManager = localSceneManagerRef.current;
    if (!sceneManager) return;

    sceneManager.setWireframe(wireframeEnabled);
  }, [wireframeEnabled]);

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

            sceneManager.createVoxelAvatarFromVoxFile(voxUrl, npubForColors, 1.0, currentTransform)
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
          sceneManager.createVoxelAvatarFromVoxFile(avatarConfig.avatarUrl, npubForColors, 1.0, currentTransform)
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
            sceneManager.createCsmAvatar(result, npubForColors, 1.0, currentTransform);
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
  }, [isLoggedIn, avatarConfig.avatarType, avatarConfig.avatarId, avatarConfig.avatarUrl, avatarConfig.avatarData, avatarConfig.avatarMod]);

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
          speechEnabled={speechEnabled}
          onSpeechEnabledChange={setSpeechEnabled}
          worldGridVisible={worldGridVisible}
          onWorldGridVisibleChange={setWorldGridVisible}
          faceMeshEnabled={faceMeshEnabled}
          onFaceMeshEnabledChange={setFaceMeshEnabled}
          wireframeEnabled={wireframeEnabled}
          onWireframeEnabledChange={setWireframeEnabled}
          triangleCount={triangleCount}
          onPublishWorld={onPublishWorld}
          isLoggedIn={isLoggedIn}
        />
      )}
    </Box>
  );
}

