import { useEffect, useRef, useState } from 'react';
import { Box } from '@chakra-ui/react';
import { SceneManager } from '../renderer/scene';
import { GeometryController } from '../geometry/geometry-controller';
import { AvatarDebugPanel } from './AvatarDebugPanel';
import init, { AvatarEngine } from '@workspace/wasm';

interface WorldCanvasProps {
  isLoggedIn: boolean;
}

export function WorldCanvas({ isLoggedIn }: WorldCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const sceneManagerRef = useRef<SceneManager | null>(null);
  const geometryControllerRef = useRef<GeometryController | null>(null);
  const avatarEngineRef = useRef<AvatarEngine | null>(null);
  const animationFrameRef = useRef<number | null>(null);
  const [avatarUrl, setAvatarUrl] = useState<string | undefined>();
  const [useVoxelAvatar, setUseVoxelAvatar] = useState(false);

  useEffect(() => {
    if (!canvasRef.current) return;

    const canvas = canvasRef.current;
    const sceneManager = new SceneManager();
    const geometryController = new GeometryController();

    sceneManagerRef.current = sceneManager;
    geometryControllerRef.current = geometryController;

    // Initialize scene
    sceneManager.initialize(canvas);

    // Initialize WASM and avatar engine
    init().then(() => {
      const avatarEngine = new AvatarEngine();
      avatarEngineRef.current = avatarEngine;
      sceneManager.setAvatarEngine(avatarEngine);
      console.log('Avatar engine initialized');
    }).catch((error) => {
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
    };
  }, []);

  // Handle login state changes
  useEffect(() => {
    const sceneManager = sceneManagerRef.current;
    if (!sceneManager) return;

    if (isLoggedIn) {
      if (useVoxelAvatar) {
        // Create voxel avatar using a test npub
        const testNpub = 'npub1test' + Math.random().toString(36).substring(7);
        sceneManager.createVoxelAvatar(testNpub, 1.0);
      } else {
        sceneManager.createAvatar(avatarUrl, 1.0);
      }
    } else {
      sceneManager.removeAvatar();
      sceneManager.removeVoxelAvatar();
    }
  }, [isLoggedIn, avatarUrl, useVoxelAvatar]);

  const handleAvatarUrlChange = (url: string) => {
    setAvatarUrl(url);
    setUseVoxelAvatar(false);
  };

  const handleCreateVoxelAvatar = () => {
    setUseVoxelAvatar(true);
    setAvatarUrl(undefined);
  };

  return (
    <>
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
      {isLoggedIn && (
        <AvatarDebugPanel
          onAvatarUrlChange={handleAvatarUrlChange}
          onCreateVoxelAvatar={handleCreateVoxelAvatar}
          currentUrl={avatarUrl}
        />
      )}
    </>
  );
}
