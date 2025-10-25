import { useEffect, useRef } from 'react';
import { Box } from '@chakra-ui/react';
import { SceneManager } from '../renderer/scene';
import { GeometryController } from '../geometry/geometry-controller';
import init, { AvatarEngine } from '@workspace/wasm';
import type { AvatarStateService, AvatarConfig } from '../services/avatar-state';
import type { TeleportAnimationType } from '../renderer/teleport-animation';

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
}: WorldCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const localSceneManagerRef = useRef<SceneManager | null>(null);
  const localGeometryControllerRef = useRef<GeometryController | null>(null);
  const avatarEngineRef = useRef<AvatarEngine | null>(null);
  const animationFrameRef = useRef<number | null>(null);

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
    sceneManager.setOnVoxelEdit((x, y, z, colorIndex) => {
      if (colorIndex === 0) {
        geometryController.removeVoxel(x, y, z);
      } else {
        geometryController.setVoxel(x, y, z, colorIndex);
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
    }).catch((error) => {
      console.error('Failed to initialize geometry controller:', error);
    });

    // Animation loop
    const animate = () => {
      sceneManager.render();
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
              })
              .catch(error => {
                console.error('[WorldCanvas] Failed to load voxel avatar from disk:', error);
                // Fallback to generated
                console.log('[WorldCanvas] Falling back to generated model');
                sceneManager.createVoxelAvatar('npub1default', 1.0, currentTransform);
              });
          } else {
            console.warn('[WorldCanvas] Unknown voxel model ID:', avatarConfig.avatarId);
            sceneManager.createVoxelAvatar('npub1default', 1.0, currentTransform);
          }
        } else if (avatarConfig.avatarUrl) {
          // Load from custom URL
          console.log('[WorldCanvas] Loading voxel avatar from URL:', avatarConfig.avatarUrl);
          sceneManager.createVoxelAvatarFromVoxFile(avatarConfig.avatarUrl, npubForColors, 1.0, currentTransform)
            .then(() => {
              console.log('[WorldCanvas] Successfully loaded voxel avatar from URL');
            })
            .catch(error => {
              console.error('[WorldCanvas] Failed to load voxel avatar from URL:', error);
              sceneManager.createVoxelAvatar('npub1default', 1.0, currentTransform);
            });
        } else if (avatarConfig.avatarData) {
          // TODO: Generate from avatarData (procedural generation)
          console.log('[WorldCanvas] Avatar generation from avatarData not yet implemented');
          sceneManager.createVoxelAvatar('npub1default', 1.0, currentTransform);
        } else {
          // Fallback to simple generated model
          console.log('[WorldCanvas] Using simple generated voxel avatar');
          sceneManager.createVoxelAvatar('npub1default', 1.0, currentTransform);
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
    </Box>
  );
}

