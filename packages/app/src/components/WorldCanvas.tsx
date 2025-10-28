import { useEffect, useRef, useState } from 'react';
import { Box } from '@chakra-ui/react';
import { SceneManager } from '../renderer/scene';
import { GeometryController } from '../geometry/geometry-controller';
import init, { AvatarEngine } from '@workspace/wasm';
import type { AvatarStateService, AvatarConfig } from '../services/avatar-state';
import type { TeleportAnimationType } from '../renderer/teleport-animation';
import { DebugPanel, type DebugInfo } from './WorldPanel';
import { setMacroDepth, setMicroDepth } from '../config/depth-config';

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
}: WorldCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const localSceneManagerRef = useRef<SceneManager | null>(null);
  const localGeometryControllerRef = useRef<GeometryController | null>(null);
  const avatarEngineRef = useRef<AvatarEngine | null>(null);
  const animationFrameRef = useRef<number | null>(null);
  const [debugInfo, setDebugInfo] = useState<DebugInfo>({});

  // Scene configuration state
  const [timeOfDay, setTimeOfDay] = useState(0.35); // Start slightly after sunrise
  const [sunAutoMove, setSunAutoMove] = useState(false); // Start with sun fixed
  const [sunSpeed, setSunSpeed] = useState(0.01);
  const [internalSpeechEnabled, setInternalSpeechEnabled] = useState(false); // Disabled by default
  const [worldGridVisible, setWorldGridVisible] = useState(true); // Show helpers by default
  const [faceMeshEnabled, setFaceMeshEnabled] = useState(true); // Enabled by default
  const [wireframeEnabled, setWireframeEnabled] = useState(false); // Disabled by default
  const [triangleCount, setTriangleCount] = useState<number | undefined>(undefined);

  // Use external speechEnabled if provided, otherwise use internal state
  const speechEnabled = externalSpeechEnabled ?? internalSpeechEnabled;
  const setSpeechEnabled = externalOnSpeechEnabledChange ?? setInternalSpeechEnabled;

  const handleApplyDepthSettings = async (worldDepth: number, scaleDepth: number) => {
    const geometryController = localGeometryControllerRef.current;
    const sceneManager = localSceneManagerRef.current;

    if (!geometryController || !sceneManager) {
      console.error('Cannot apply depth settings: geometry controller or scene manager not initialized');
      return;
    }

    // Calculate macro depth from total depth and micro depth
    const macroDepth = worldDepth - scaleDepth;
    const microDepth = scaleDepth;

    console.log(`Applying depth settings: macroDepth=${macroDepth}, microDepth=${microDepth}, totalDepth=${worldDepth}`);

    try {
      // Only reinitialize if MACRO depth changed (world size change)
      // Micro depth changes don't need reinit since Rust ignores it and world scale stays constant
      const currentMacro = geometryController['macroDepth']; // Access private field for comparison

      if (currentMacro !== macroDepth) {
        console.log(`Macro depth changed from ${currentMacro} to ${macroDepth}, reinitializing...`);
        await geometryController.reinitialize(macroDepth, microDepth, (geometry) => {
          sceneManager.updateGeometry(
            geometry.vertices,
            geometry.indices,
            geometry.normals,
            geometry.colors
          );
        });
        console.log('Geometry reinitialized successfully');
      } else {
        console.log('Macro depth unchanged, skipping reinitialize (micro depth only affects coordinate precision)');
      }
    } catch (error) {
      console.error('Failed to apply depth settings:', error);
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
      console.log('[WorldCanvas Voxel Edit]', { coord, colorIndex });
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
      console.log('Avatar engine initialized');
    }).catch((error: unknown) => {
      console.error('Failed to initialize WASM/Avatar engine:', error);
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
      console.error('Failed to initialize geometry controller:', error);
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

  // Handle world grid visibility
  useEffect(() => {
    const sceneManager = localSceneManagerRef.current;
    if (!sceneManager) return;

    sceneManager.setWorldGridVisible(worldGridVisible);
  }, [worldGridVisible]);

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

    console.log('Avatar update triggered:', {
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
            console.log('[WorldCanvas] Loading voxel avatar from disk:', voxUrl);

            sceneManager.createVoxelAvatarFromVoxFile(voxUrl, npubForColors, 1.0, currentTransform)
              .then(() => {
                console.log('[WorldCanvas] Successfully loaded voxel avatar from disk');
                // Refresh profile in case pubkey was set before avatar loaded
                sceneManager.refreshCurrentAvatarProfile();
              })
              .catch(error => {
                console.error('[WorldCanvas] Failed to load voxel avatar from disk:', error);
                // Fallback to generated
                console.log('[WorldCanvas] Falling back to generated model');
                sceneManager.createVoxelAvatar('npub1default', 1.0, currentTransform);
                sceneManager.refreshCurrentAvatarProfile();
              });
          } else {
            console.warn('[WorldCanvas] Unknown voxel model ID:', avatarConfig.avatarId);
            sceneManager.createVoxelAvatar('npub1default', 1.0, currentTransform);
            sceneManager.refreshCurrentAvatarProfile();
          }
        } else if (avatarConfig.avatarUrl) {
          // Load from custom URL
          console.log('[WorldCanvas] Loading voxel avatar from URL:', avatarConfig.avatarUrl);
          sceneManager.createVoxelAvatarFromVoxFile(avatarConfig.avatarUrl, npubForColors, 1.0, currentTransform)
            .then(() => {
              console.log('[WorldCanvas] Successfully loaded voxel avatar from URL');
              sceneManager.refreshCurrentAvatarProfile();
            })
            .catch(error => {
              console.error('[WorldCanvas] Failed to load voxel avatar from URL:', error);
              sceneManager.createVoxelAvatar('npub1default', 1.0, currentTransform);
              sceneManager.refreshCurrentAvatarProfile();
            });
        } else if (avatarConfig.avatarData) {
          // TODO: Generate from avatarData (procedural generation)
          console.log('[WorldCanvas] Avatar generation from avatarData not yet implemented');
          sceneManager.createVoxelAvatar('npub1default', 1.0, currentTransform);
          sceneManager.refreshCurrentAvatarProfile();
        } else {
          // Fallback to simple generated model
          console.log('[WorldCanvas] Using simple generated voxel avatar');
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
          console.log('Loading GLB avatar:', glbUrl);
          try {
            // Check if file exists before loading
            const checkResponse = await fetch(glbUrl, { method: 'HEAD' });
            if (checkResponse.ok) {
              sceneManager.createAvatar(glbUrl, 1.0, currentTransform);
              sceneManager.refreshCurrentAvatarProfile();
            } else {
              console.warn('GLB model not found:', glbUrl);
              // Don't create avatar if model doesn't exist
            }
          } catch (error) {
            console.error('Failed to check/load GLB avatar:', error);
          }
        } else {
          console.warn('No GLB URL available for avatar');
        }
      } else if (avatarConfig.avatarType === 'csm') {
        // Remove old avatar
        sceneManager.removeAvatar();

        // Get CSM code from avatarData
        if (avatarConfig.avatarData) {
          console.log('[WorldCanvas] Loading CSM avatar from avatarData');
          try {
            const { parseCsmToMesh } = await import('../utils/cubeWasm');
            const result = await parseCsmToMesh(avatarConfig.avatarData);

            if ('error' in result) {
              console.error('[WorldCanvas] CSM parse error:', result.error);
              return;
            }

            // Create CSM avatar from mesh data
            sceneManager.createCsmAvatar(result, npubForColors, 1.0, currentTransform);
            sceneManager.refreshCurrentAvatarProfile();
            console.log('[WorldCanvas] Successfully loaded CSM avatar');
          } catch (error) {
            console.error('[WorldCanvas] Failed to load CSM avatar:', error);
          }
        } else {
          console.warn('[WorldCanvas] No avatarData provided for CSM avatar');
        }
      }

      // TODO: Apply avatarMod if present
      if (avatarConfig.avatarMod) {
        console.log('Avatar modifications not yet implemented');
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
      <DebugPanel
        info={debugInfo}
        onApplyDepthSettings={handleApplyDepthSettings}
        timeOfDay={timeOfDay}
        onTimeOfDayChange={setTimeOfDay}
        sunAutoMove={sunAutoMove}
        onSunAutoMoveChange={setSunAutoMove}
        sunSpeed={sunSpeed}
        onSunSpeedChange={setSunSpeed}
        speechEnabled={speechEnabled}
        onSpeechEnabledChange={setSpeechEnabled}
        worldGridVisible={worldGridVisible}
        onWorldGridVisibleChange={setWorldGridVisible}
        faceMeshEnabled={faceMeshEnabled}
        onFaceMeshEnabledChange={setFaceMeshEnabled}
        wireframeEnabled={wireframeEnabled}
        onWireframeEnabledChange={setWireframeEnabled}
        triangleCount={triangleCount}
      />
    </Box>
  );
}

