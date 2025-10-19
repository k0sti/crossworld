import { useEffect, useRef, useState } from 'react';
import { Box } from '@chakra-ui/react';
import { SceneManager } from '../renderer/scene';
import { GeometryController } from '../geometry/geometry-controller';
import init, { AvatarEngine } from '@workspace/wasm';
import type { AvatarStateService } from '../services/avatar-state';

export type VoxelModelType = 'boy' | 'girl';

interface WorldCanvasProps {
  isLoggedIn: boolean;
  useVoxelAvatar: boolean;
  onToggleAvatarType: (useVoxel: boolean) => void;
  isEditMode: boolean;
  voxelModel: VoxelModelType;
  onVoxelModelChange: (model: VoxelModelType) => void;
  useVoxFile: boolean;
  onVoxFileChange: (useVox: boolean) => void;
  useOriginalColors: boolean;
  onColorModeChange: (useOriginal: boolean) => void;
  onAvatarUrlChange: (url: string) => void;
  avatarUrl?: string;
  colorChangeCounter?: number;
  avatarStateService?: AvatarStateService;
  currentUserPubkey?: string | null;
}

export function WorldCanvas({
  isLoggedIn,
  useVoxelAvatar,
  isEditMode,
  voxelModel,
  useVoxFile,
  useOriginalColors,
  avatarUrl,
  colorChangeCounter,
  avatarStateService,
  currentUserPubkey
}: WorldCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const sceneManagerRef = useRef<SceneManager | null>(null);
  const geometryControllerRef = useRef<GeometryController | null>(null);
  const avatarEngineRef = useRef<AvatarEngine | null>(null);
  const animationFrameRef = useRef<number | null>(null);
  const [colorSeed, setColorSeed] = useState(() => Math.random().toString(36).substring(7));

  // Update color seed when counter changes
  useEffect(() => {
    if (colorChangeCounter !== undefined && colorChangeCounter > 0) {
      setColorSeed(Math.random().toString(36).substring(7));
    }
  }, [colorChangeCounter]);

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

  // Handle login state changes and avatar updates
  useEffect(() => {
    const sceneManager = sceneManagerRef.current;
    if (!sceneManager) return;

    console.log('Avatar update triggered:', { isLoggedIn, useVoxelAvatar, voxelModel, useVoxFile, useOriginalColors });

    if (isLoggedIn) {
      // Preserve current transform (position + rotation) if avatar exists
      let currentTransform: any = undefined; // Transform type
      const currentVoxelAvatar = sceneManager.getVoxelAvatar();
      if (currentVoxelAvatar) {
        currentTransform = currentVoxelAvatar.getTransform();
      }

      if (useVoxelAvatar) {
        // Remove old GLB avatar if exists
        sceneManager.removeAvatar();

        // Create voxel avatar with color customization
        // Use undefined for npub to get original colors, or a seed for randomized colors
        const npubForColors = useOriginalColors ? undefined : `npub1seed${colorSeed}`;
        // For generated avatars, always use a npub for color variation
        const npubForGenerated = npubForColors || 'npub1default';

        if (useVoxFile) {
          // Load from .vox file - can use undefined for original colors
          const voxFilename = voxelModel === 'boy'
            ? 'chr_peasant_guy_blackhair.vox'
            : 'chr_peasant_girl_orangehair.vox';
          const voxUrl = `${import.meta.env.BASE_URL}assets/models/vox/${voxFilename}`;

          console.log('Loading voxel avatar from file:', voxUrl, 'with colors:', useOriginalColors ? 'original' : 'randomized');

          sceneManager.createVoxelAvatarFromVoxFile(voxUrl, npubForColors, 1.0, currentTransform)
            .then(() => {
              console.log('Successfully loaded voxel avatar from file');
            })
            .catch(error => {
              console.error('Failed to load voxel avatar from file:', error);
              // Fallback to generated model
              console.log('Falling back to generated model');
              sceneManager.createVoxelAvatar(npubForGenerated, 1.0, currentTransform);
            });
        } else {
          // Use procedurally generated model
          console.log('Creating procedurally generated voxel avatar with npub:', npubForGenerated);
          sceneManager.createVoxelAvatar(npubForGenerated, 1.0, currentTransform);
        }
      } else {
        // Remove voxel avatar if exists
        sceneManager.removeVoxelAvatar();
        console.log('Creating GLB avatar:', avatarUrl);
        sceneManager.createAvatar(avatarUrl, 1.0, currentTransform);
      }
    } else {
      sceneManager.removeAvatar();
      sceneManager.removeVoxelAvatar();
    }
  }, [isLoggedIn, avatarUrl, useVoxelAvatar, voxelModel, useVoxFile, useOriginalColors, colorSeed]);

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
