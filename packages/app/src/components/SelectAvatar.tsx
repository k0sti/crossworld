import {
  Modal,
  ModalOverlay,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalCloseButton,
  VStack,
  HStack,
  Button,
  Text,
  SimpleGrid,
  Box,
  Badge,
  Tabs,
  TabList,
  TabPanels,
  Tab,
  TabPanel,
  Divider,
  Input,
  IconButton,
  Textarea,
} from '@chakra-ui/react';
import { useState, useRef, useEffect } from 'react';
import * as THREE from 'three';
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js';
import { ReadyPlayerMeService } from '../services/ready-player-me';
import type { TeleportAnimationType } from '../renderer/teleport-animation';
import { ENABLE_AVATAR_COLOR_SELECTION } from '../constants/features';

export interface AvatarSelection {
  avatarType: 'vox' | 'glb' | 'csm';
  avatarId?: string;
  avatarUrl?: string;
  csmCode?: string;
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

export function SelectAvatar({ isOpen, onClose, onSave, currentSelection }: SelectAvatarProps) {
  const [avatarType, setAvatarType] = useState<'vox' | 'glb' | 'csm'>(currentSelection?.avatarType || 'vox');
  const [selectedId, setSelectedId] = useState<string>(currentSelection?.avatarId || '');
  const [avatarUrl, setAvatarUrl] = useState<string>(currentSelection?.avatarUrl || '');
  const [inputUrl, setInputUrl] = useState<string>('');
  const [csmCode, setCsmCode] = useState<string>(currentSelection?.csmCode || '>a [1 2 3 4 5 6 7 8]');
  const [teleportAnimationType, setTeleportAnimationType] = useState<TeleportAnimationType>(
    currentSelection?.teleportAnimationType || 'fade'
  );
  const [voxModels, setVoxModels] = useState<ModelItem[]>([]);
  const [glbModels, setGlbModels] = useState<ModelItem[]>([]);
  const [modelsLoaded, setModelsLoaded] = useState(false);

  const fileInputRef = useRef<HTMLInputElement>(null);

  // Load models configuration
  useEffect(() => {
    console.log('[SelectAvatar] Loading models config...');
    loadModelsConfig().then(config => {
      console.log('[SelectAvatar] Models config loaded:', config);
      const vox = config.vox.map(([label, filename]) => ({
        id: filename.replace('.vox', ''),
        label,
        filename
      }));
      const glb = config.glb.map(([label, filename]) => ({
        id: filename.replace('.glb', ''),
        label,
        filename
      }));
      setVoxModels(vox);
      setGlbModels(glb);
      setModelsLoaded(true);

      // Set default selection if none exists or if current selection is invalid
      if ((!selectedId || selectedId === 'boy' || selectedId === 'girl') && vox.length > 0) {
        console.log('[SelectAvatar] Setting default selection to:', vox[0].id);
        setSelectedId(vox[0].id);
      }
    }).catch(error => {
      console.error('[SelectAvatar] Failed to load models config:', error);
      setModelsLoaded(true); // Still set to true to show the error state
    });
  }, []);

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
    console.log('[SelectAvatar] Scene init effect:', { isOpen, hasCanvas: !!previewCanvas });

    if (!isOpen || !previewCanvas) {
      setSceneReady(false);
      return;
    }

    const canvas = previewCanvas;
    const scene = new THREE.Scene();
    scene.background = new THREE.Color(0x1a1a1e);

    const camera = new THREE.PerspectiveCamera(50, 1, 0.1, 1000);
    const cameraDistance = 3;
    camera.position.set(0, 1, cameraDistance);
    camera.lookAt(0, 0.5, 0);

    // Get container size for responsive canvas
    const containerWidth = previewContainerRef.current?.clientWidth || 400;
    const canvasSize = Math.min(containerWidth - 32, 400); // Max 400px, with padding

    const renderer = new THREE.WebGLRenderer({ canvas, antialias: true });
    renderer.setSize(canvasSize, canvasSize);
    renderer.setPixelRatio(window.devicePixelRatio);

    // Lighting
    const ambientLight = new THREE.AmbientLight(0xffffff, 0.6);
    scene.add(ambientLight);

    const directionalLight = new THREE.DirectionalLight(0xffffff, 0.8);
    directionalLight.position.set(5, 10, 5);
    scene.add(directionalLight);

    previewSceneRef.current = { scene, camera, renderer, cameraDistance };
    console.log('[SelectAvatar] Scene initialized successfully');
    setSceneReady(true);

    // Mouse wheel zoom handler
    const handleWheel = (event: WheelEvent) => {
      if (!previewSceneRef.current) return;
      event.preventDefault();

      const { camera, cameraDistance: currentDistance } = previewSceneRef.current;
      const zoomSpeed = 0.001;
      const newDistance = Math.max(1, Math.min(10, currentDistance + event.deltaY * zoomSpeed));

      previewSceneRef.current.cameraDistance = newDistance;
      camera.position.set(0, 1, newDistance);
      camera.lookAt(0, 0.5, 0);
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
    console.log('[SelectAvatar] Preview effect triggered:', { isOpen, selectedId, avatarUrl, sceneReady, hasScene: !!previewSceneRef.current });

    if (!isOpen || !sceneReady || !previewSceneRef.current) {
      console.log('[SelectAvatar] Preview effect skipped - modal closed or scene not ready');
      return;
    }

    // Don't reload preview if no model is selected yet
    // CSM models are an exception - they always have code to render
    if (!selectedId && !avatarUrl && avatarType !== 'csm') {
      console.log('[SelectAvatar] Preview effect skipped - no model selected');
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

      // Handle CSM models first (they don't use URLs)
      if (avatarType === 'csm') {
        try {
          const { parseCsmToMesh } = await import('../utils/cubeWasm');
          const result = await parseCsmToMesh(csmCode);

          if (!previewSceneRef.current) return; // Component unmounted

          // Check if result contains an error
          if ('error' in result) {
            console.error('[SelectAvatar] CSM parse error:', result.error);
            throw new Error(result.error);
          }

          // Create geometry from CSM data
          const geometry = new THREE.BufferGeometry();
          geometry.setAttribute('position', new THREE.BufferAttribute(new Float32Array(result.vertices), 3));
          geometry.setAttribute('normal', new THREE.BufferAttribute(new Float32Array(result.normals), 3));
          geometry.setAttribute('color', new THREE.BufferAttribute(new Float32Array(result.colors), 3));
          geometry.setIndex(new THREE.BufferAttribute(new Uint32Array(result.indices), 1));

          // CENTER THE GEOMETRY so rotation happens around geometric center
          geometry.computeBoundingBox();
          const geoCenter = new THREE.Vector3();
          geometry.boundingBox!.getCenter(geoCenter);
          geometry.translate(-geoCenter.x, 0, -geoCenter.z); // Only center horizontally

          const material = new THREE.MeshPhongMaterial({
            vertexColors: true,
            specular: 0x111111,
            shininess: 30,
            side: THREE.DoubleSide,
          });

          const mesh = new THREE.Mesh(geometry, material);

          // Calculate bounding box for sizing and positioning
          geometry.computeBoundingBox();
          const box = geometry.boundingBox!;
          const size = box.getSize(new THREE.Vector3());

          // Lift mesh so bottom is at y=0 (geometry is already centered in X/Z)
          mesh.position.y = -box.min.y;

          // Scale to fit in view
          const maxDim = Math.max(size.x, size.y, size.z);
          const scale = 1.5 / maxDim;
          mesh.scale.setScalar(scale);

          // Add directly to scene
          mesh.rotation.y = Math.PI; // Rotate 180 degrees to show front
          scene.add(mesh);
          previewSceneRef.current.mesh = mesh;
          console.log('[SelectAvatar] CSM model loaded successfully');
          return;
        } catch (error) {
          console.error('[SelectAvatar] Failed to load CSM model:', error);
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
          return;
        }
      }

      let modelUrl: string | undefined;

      // Determine the model URL based on selection
      if (avatarUrl) {
        modelUrl = avatarUrl;
        console.log('[SelectAvatar] Loading preview from URL:', modelUrl);
      } else if (selectedId && selectedId !== 'file') {
        if (avatarType === 'vox') {
          const model = voxModels.find(m => m.id === selectedId);
          if (model) {
            modelUrl = `${import.meta.env.BASE_URL}assets/models/vox/${model.filename}`;
            console.log('[SelectAvatar] Loading VOX preview:', modelUrl);
          }
        } else if (avatarType === 'glb') {
          const model = glbModels.find(m => m.id === selectedId);
          if (model) {
            modelUrl = `${import.meta.env.BASE_URL}assets/models/glb/${model.filename}`;
            console.log('[SelectAvatar] Loading GLB preview:', modelUrl);
          }
        }
      }

      if (!modelUrl) {
        console.log('[SelectAvatar] No model URL, showing placeholder');
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
      if (avatarType === 'glb' || modelUrl.endsWith('.glb')) {
        // Load GLB model
        const loader = new GLTFLoader();
        try {
          const gltf = await loader.loadAsync(modelUrl);
          if (!previewSceneRef.current) return; // Component unmounted

          const model = gltf.scene;

          // Calculate bounding box for the model
          const box = new THREE.Box3().setFromObject(model);
          const center = box.getCenter(new THREE.Vector3());
          const size = box.getSize(new THREE.Vector3());

          // Create a wrapper group at origin for centered rotation
          const wrapper = new THREE.Group();

          // Position model inside wrapper so its geometric center is at wrapper origin
          // Keep bottom at y=0 of the wrapper
          model.position.set(-center.x, -box.min.y, -center.z);

          // Scale to fit in view
          const maxDim = Math.max(size.x, size.y, size.z);
          const scale = 1.5 / maxDim;
          model.scale.setScalar(scale);

          wrapper.add(model);
          wrapper.rotation.y = Math.PI; // Rotate 180 degrees to show front
          scene.add(wrapper);
          previewSceneRef.current.mesh = wrapper;
          console.log('[SelectAvatar] GLB model loaded successfully');
        } catch (error) {
          console.error('[SelectAvatar] Failed to load GLB model:', error);
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
      } else if (avatarType === 'vox' || modelUrl.endsWith('.vox')) {
        // Load VOX file
        try {
          const { loadVoxFromUrl } = await import('../utils/voxLoader');
          const geometryData = await loadVoxFromUrl(modelUrl);

          if (!previewSceneRef.current) return; // Component unmounted

          // Create geometry from voxel data
          const geometry = new THREE.BufferGeometry();
          geometry.setAttribute('position', new THREE.BufferAttribute(geometryData.vertices, 3));
          geometry.setAttribute('normal', new THREE.BufferAttribute(geometryData.normals, 3));
          geometry.setAttribute('color', new THREE.BufferAttribute(geometryData.colors, 3));
          geometry.setIndex(new THREE.BufferAttribute(geometryData.indices, 1));

          // CENTER THE GEOMETRY so rotation happens around geometric center
          geometry.computeBoundingBox();
          const geoCenter = new THREE.Vector3();
          geometry.boundingBox!.getCenter(geoCenter);
          geometry.translate(-geoCenter.x, 0, -geoCenter.z); // Only center horizontally

          const material = new THREE.MeshPhongMaterial({
            vertexColors: true,
            specular: 0x111111,
            shininess: 30,
          });

          const mesh = new THREE.Mesh(geometry, material);

          // Calculate bounding box for sizing and positioning
          geometry.computeBoundingBox();
          const box = geometry.boundingBox!;
          const size = box.getSize(new THREE.Vector3());

          // Lift mesh so bottom is at y=0 (geometry is already centered in X/Z)
          mesh.position.y = -box.min.y;

          // Scale to fit in view
          const maxDim = Math.max(size.x, size.y, size.z);
          const scale = 1.5 / maxDim;
          mesh.scale.setScalar(scale);

          // Add directly to scene - no group needed since geometry is centered
          mesh.rotation.y = Math.PI; // Rotate 180 degrees to show front
          scene.add(mesh);
          previewSceneRef.current.mesh = mesh;
          console.log('[SelectAvatar] VOX model loaded successfully');
        } catch (error) {
          console.error('[SelectAvatar] Failed to load VOX model:', error);
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
  }, [isOpen, sceneReady, selectedId, avatarUrl, voxModels, glbModels, csmCode, avatarType]);

  const handleSave = () => {
    const selection: AvatarSelection = {
      avatarType,
      avatarId: avatarType === 'vox' ? selectedId : (avatarType === 'glb' && selectedId !== 'file' ? selectedId : undefined),
      avatarUrl: avatarUrl || undefined,
      csmCode: avatarType === 'csm' ? csmCode : undefined,
      teleportAnimationType,
    };
    console.log('[SelectAvatar] Saving avatar selection:', selection);
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

  const renderModelGrid = (models: ModelItem[]) => (
    <SimpleGrid columns={6} spacing={2} maxHeight="300px" overflowY="auto">
      {models.map((model) => (
        <Box
          key={model.id}
          as="button"
          onClick={() => {
            console.log('[SelectAvatar] Model selected:', model.id, 'type:', avatarType);
            setSelectedId(model.id);
            setAvatarUrl('');
          }}
          p={2}
          bg={selectedId === model.id && !avatarUrl ? 'rgba(100, 150, 250, 0.3)' : 'rgba(80, 80, 80, 0.1)'}
          border="1px solid"
          borderColor={selectedId === model.id && !avatarUrl ? 'rgba(100, 150, 250, 0.5)' : 'rgba(255, 255, 255, 0.1)'}
          borderRadius="md"
          _hover={{
            bg: selectedId === model.id && !avatarUrl ? 'rgba(100, 150, 250, 0.4)' : 'rgba(120, 120, 120, 0.2)',
          }}
          transition="all 0.2s"
          cursor="pointer"
          title={model.filename}
        >
          <Text fontSize="xs" color="white" noOfLines={1}>
            {model.label}
          </Text>
        </Box>
      ))}
    </SimpleGrid>
  );

  return (
    <Modal isOpen={isOpen} onClose={onClose} size="xl" closeOnOverlayClick={false}>
      <ModalOverlay />
      <ModalContent bg="rgba(20, 20, 30, 0.95)" backdropFilter="blur(10px)">
        <ModalHeader color="white">Select Avatar</ModalHeader>
        <ModalCloseButton color="white" />
        <ModalBody pb={6}>
          <VStack align="stretch" spacing={4}>
            {/* Preview Canvas */}
            <Box
              ref={previewContainerRef}
              display="flex"
              justifyContent="center"
              alignItems="center"
              bg="rgba(26, 26, 30, 0.8)"
              borderRadius="md"
              p={4}
              border="1px solid"
              borderColor="rgba(255, 255, 255, 0.1)"
              minHeight="300px"
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

            {/* Tabs */}
            <Tabs
              variant="soft-rounded"
              colorScheme="blue"
              index={avatarType === 'vox' ? 0 : avatarType === 'glb' ? 1 : 2}
              onChange={(index) => {
                const types: ('vox' | 'glb' | 'csm')[] = ['vox', 'glb', 'csm'];
                setAvatarType(types[index]);
                setAvatarUrl('');
                // Clear selectedId when switching tabs to prevent showing wrong type
                setSelectedId('');
              }}
            >
              <TabList>
                <Tab fontSize="xs">VOX</Tab>
                <Tab fontSize="xs">GLB</Tab>
                <Tab fontSize="xs">Cube</Tab>
              </TabList>

              <TabPanels>
                {/* VOX */}
                <TabPanel>
                  {modelsLoaded ? renderModelGrid(voxModels) : (
                    <Text fontSize="sm" color="gray.400" textAlign="center" py={4}>
                      Loading models...
                    </Text>
                  )}
                </TabPanel>

                {/* GLB */}
                <TabPanel>
                  {modelsLoaded ? renderModelGrid(glbModels) : (
                    <Text fontSize="sm" color="gray.400" textAlign="center" py={4}>
                      Loading models...
                    </Text>
                  )}
                </TabPanel>

                {/* CSM (Cube Script Model) */}
                <TabPanel>
                  <VStack align="stretch" spacing={3}>
                    <Text fontSize="sm" fontWeight="semibold" color="white">
                      Cube Script Model (CSM)
                    </Text>
                    <Textarea
                      value={csmCode}
                      onChange={(e) => setCsmCode(e.target.value)}
                      placeholder="Enter CSM code..."
                      size="sm"
                      fontSize="xs"
                      fontFamily="monospace"
                      bg="rgba(0, 0, 0, 0.3)"
                      color="white"
                      borderColor="rgba(255, 255, 255, 0.2)"
                      _hover={{ borderColor: 'rgba(255, 255, 255, 0.3)' }}
                      _focus={{ borderColor: 'blue.400', boxShadow: '0 0 0 1px #3182ce' }}
                      rows={8}
                      spellCheck={false}
                    />
                    <Text fontSize="xs" color="gray.400">
                      Example: <code>&gt;a [1 2 3 4 5 6 7 8]</code>
                    </Text>
                  </VStack>
                </TabPanel>
              </TabPanels>
            </Tabs>

            <Divider borderColor="rgba(255, 255, 255, 0.1)" />

            {/* Load File/URL Section */}
            <VStack align="stretch" spacing={2}>
              <Text fontSize="sm" fontWeight="semibold" color="white">
                Load Model
              </Text>

              <Input
                value={inputUrl}
                onChange={(e) => setInputUrl(e.target.value)}
                placeholder="https://models.readyplayer.me/..."
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

                <Divider borderColor="rgba(255, 255, 255, 0.1)" />
              </>
            )}

            {/* Teleport Animation */}
            <VStack align="stretch" spacing={2}>
              <HStack>
                <Text fontSize="sm" fontWeight="semibold" color="white">
                  Teleport
                </Text>
                <Badge colorScheme="gray" fontSize="2xs" px={2}>
                  CTRL+Click
                </Badge>
              </HStack>

              <HStack spacing={2} justify="center">
                <IconButton
                  aria-label="Fade animation"
                  icon={<Text fontSize="lg">🌫️</Text>}
                  size="sm"
                  bg={teleportAnimationType === 'fade' ? 'rgba(100, 150, 250, 0.3)' : 'rgba(80, 80, 80, 0.1)'}
                  borderColor={teleportAnimationType === 'fade' ? 'rgba(100, 150, 250, 0.5)' : 'rgba(255, 255, 255, 0.1)'}
                  borderWidth="1px"
                  _hover={{
                    bg: teleportAnimationType === 'fade' ? 'rgba(100, 150, 250, 0.4)' : 'rgba(120, 120, 120, 0.2)',
                  }}
                  onClick={() => setTeleportAnimationType('fade')}
                />

                <IconButton
                  aria-label="Scale animation"
                  icon={<Text fontSize="lg">⚫</Text>}
                  size="sm"
                  bg={teleportAnimationType === 'scale' ? 'rgba(100, 150, 250, 0.3)' : 'rgba(80, 80, 80, 0.1)'}
                  borderColor={teleportAnimationType === 'scale' ? 'rgba(100, 150, 250, 0.5)' : 'rgba(255, 255, 255, 0.1)'}
                  borderWidth="1px"
                  _hover={{
                    bg: teleportAnimationType === 'scale' ? 'rgba(100, 150, 250, 0.4)' : 'rgba(120, 120, 120, 0.2)',
                  }}
                  onClick={() => setTeleportAnimationType('scale')}
                />

                <IconButton
                  aria-label="Spin animation"
                  icon={<Text fontSize="lg">🌀</Text>}
                  size="sm"
                  bg={teleportAnimationType === 'spin' ? 'rgba(100, 150, 250, 0.3)' : 'rgba(80, 80, 80, 0.1)'}
                  borderColor={teleportAnimationType === 'spin' ? 'rgba(100, 150, 250, 0.5)' : 'rgba(255, 255, 255, 0.1)'}
                  borderWidth="1px"
                  _hover={{
                    bg: teleportAnimationType === 'spin' ? 'rgba(100, 150, 250, 0.4)' : 'rgba(120, 120, 120, 0.2)',
                  }}
                  onClick={() => setTeleportAnimationType('spin')}
                />

                <IconButton
                  aria-label="Slide animation"
                  icon={<Text fontSize="lg">⬇️</Text>}
                  size="sm"
                  bg={teleportAnimationType === 'slide' ? 'rgba(100, 150, 250, 0.3)' : 'rgba(80, 80, 80, 0.1)'}
                  borderColor={teleportAnimationType === 'slide' ? 'rgba(100, 150, 250, 0.5)' : 'rgba(255, 255, 255, 0.1)'}
                  borderWidth="1px"
                  _hover={{
                    bg: teleportAnimationType === 'slide' ? 'rgba(100, 150, 250, 0.4)' : 'rgba(120, 120, 120, 0.2)',
                  }}
                  onClick={() => setTeleportAnimationType('slide')}
                />

                <IconButton
                  aria-label="Burst animation"
                  icon={<Text fontSize="lg">✨</Text>}
                  size="sm"
                  bg={teleportAnimationType === 'burst' ? 'rgba(100, 150, 250, 0.3)' : 'rgba(80, 80, 80, 0.1)'}
                  borderColor={teleportAnimationType === 'burst' ? 'rgba(100, 150, 250, 0.5)' : 'rgba(255, 255, 255, 0.1)'}
                  borderWidth="1px"
                  _hover={{
                    bg: teleportAnimationType === 'burst' ? 'rgba(100, 150, 250, 0.4)' : 'rgba(120, 120, 120, 0.2)',
                  }}
                  onClick={() => setTeleportAnimationType('burst')}
                />
              </HStack>
            </VStack>

            {/* Action Buttons */}
            <HStack spacing={3} pt={4}>
              <Button flex={1} onClick={onClose} variant="ghost" color="white">
                Cancel
              </Button>
              <Button flex={1} colorScheme="blue" onClick={handleSave}>
                Save
              </Button>
            </HStack>
          </VStack>
        </ModalBody>
      </ModalContent>
    </Modal>
  );
}

