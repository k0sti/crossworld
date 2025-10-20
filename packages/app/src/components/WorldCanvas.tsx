import { useEffect, useRef, useState } from 'react';
import { Box } from '@chakra-ui/react';
import { SceneManager } from '../renderer/scene';
import { GeometryController } from '../geometry/geometry-controller';
import init, { AvatarEngine } from '@workspace/wasm';
import type { AvatarStateService, AvatarConfig } from '../services/avatar-state';
import type { TeleportAnimationType } from '../renderer/teleport-animation';

interface WorldCanvasProps {
  isLoggedIn: boolean;
  isEditMode: boolean;
  avatarConfig: AvatarConfig;
  teleportAnimationType: TeleportAnimationType;
  avatarStateService?: AvatarStateService;
  currentUserPubkey?: string | null;
}

export function WorldCanvas({
  isLoggedIn,
  isEditMode,
  avatarConfig,
  teleportAnimationType,
  avatarStateService,
  currentUserPubkey,
}: WorldCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const sceneManagerRef = useRef<SceneManager | null>(null);
  const geometryControllerRef = useRef<GeometryController | null>(null);
  const avatarEngineRef = useRef<AvatarEngine | null>(null);
  const animationFrameRef = useRef<number | null>(null);

  useEffect(() => {
    if (!canvasRef.current) return;

    const canvas = canvasRef.current;
    const sceneManager = new SceneManager();
    const geometryController = new GeometryController();

    sceneManagerRef.current = sceneManager;
    geometryControllerRef.current = geometryController;

    // Initialize scene
    sceneManager.initialize(canvas);

    // Set position update callback and subscribe to state changes
    let unsubscribe: (() => void) | undefined;
    if (avatarStateService) {
      sceneManager.setPositionUpdateCallback((x, y, z, quaternion) => {
        avatarStateService.publishPosition({ x, y, z, quaternion }).catch(console.error);
      });

      // Subscribe to avatar state changes
      unsubscribe = avatarStateService.onChange((states) => {
        sceneManager.updateRemoteAvatars(states);
      });
    }

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
    const sceneManager = sceneManagerRef.current;
    if (!sceneManager) return;

    sceneManager.setCurrentUserPubkey(currentUserPubkey || null);
  }, [currentUserPubkey]);

  // Handle edit mode changes
  useEffect(() => {
    const sceneManager = sceneManagerRef.current;
    if (!sceneManager) return;

    sceneManager.setEditMode(isEditMode);
  }, [isEditMode]);

  // Handle teleport animation type changes
  useEffect(() => {
    const sceneManager = sceneManagerRef.current;
    if (!sceneManager) return;

    sceneManager.setTeleportAnimationType(teleportAnimationType);
  }, [teleportAnimationType]);

  // Handle avatar loading based on new unified avatar config
  useEffect(() => {
    const sceneManager = sceneManagerRef.current;
    if (!sceneManager) return;

    console.log('Avatar update triggered:', { isLoggedIn, avatarConfig });

    if (isLoggedIn) {
      // Preserve current transform (position + rotation) if avatar exists
      let currentTransform: any = undefined;
      const currentVoxelAvatar = sceneManager.getVoxelAvatar();
      if (currentVoxelAvatar) {
        currentTransform = currentVoxelAvatar.getTransform();
      }

      // Priority loading order:
      // 1. Load from avatarId (predefined models)
      // 2. Load from avatarUrl (custom URL)
      // 3. Load from avatarData (procedural generation - not yet implemented)
      // Finally: Apply avatarMod (modifications - not yet implemented)

      // Use original colors (undefined = original palette)
      const npubForColors = undefined;

      if (avatarConfig.avatarType === 'voxel') {
        // Remove old GLB avatar if exists
        sceneManager.removeAvatar();

        // Try to load from avatarId first
        if (avatarConfig.avatarId && avatarConfig.avatarId !== 'file') {
          // Load predefined voxel model from disk
          const voxFilename = getVoxFilename(avatarConfig.avatarId);
          if (voxFilename) {
            const voxUrl = `${import.meta.env.BASE_URL}assets/models/vox/${voxFilename}`;
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
        // Remove voxel avatar if exists
        sceneManager.removeVoxelAvatar();

        // Load GLB avatar
        let glbUrl: string | undefined;

        // Try avatarId first (predefined GLB models)
        if (avatarConfig.avatarId && avatarConfig.avatarId !== 'file') {
          glbUrl = getGLBUrl(avatarConfig.avatarId);
        }

        // Fallback to avatarUrl
        if (!glbUrl && avatarConfig.avatarUrl) {
          glbUrl = avatarConfig.avatarUrl;
        }

        // Load the GLB
        if (glbUrl) {
          console.log('Loading GLB avatar:', glbUrl);
          sceneManager.createAvatar(glbUrl, 1.0, currentTransform);
        } else {
          console.warn('No GLB URL available for avatar');
          // Fallback to default man model
          const defaultUrl = `${import.meta.env.BASE_URL}assets/models/man.glb`;
          sceneManager.createAvatar(defaultUrl, 1.0, currentTransform);
        }
      }

      // TODO: Apply avatarMod if present
      if (avatarConfig.avatarMod) {
        console.log('Avatar modifications not yet implemented');
      }
    } else {
      sceneManager.removeAvatar();
      sceneManager.removeVoxelAvatar();
    }
  }, [isLoggedIn, avatarConfig]);

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

/**
 * Get .vox filename for a given avatar ID
 */
function getVoxFilename(avatarId: string): string | null {
  const voxModels: Record<string, string> = {
    'boy': 'chr_peasant_guy_blackhair.vox',
    'girl': 'chr_peasant_girl_orangehair.vox',
  };

  return voxModels[avatarId] || null;
}

/**
 * Get GLB URL for a given avatar ID
 */
function getGLBUrl(avatarId: string): string | null {
  const glbModels: Record<string, string> = {
    'man': `${import.meta.env.BASE_URL}assets/models/man.glb`,
  };

  return glbModels[avatarId] || null;
}
