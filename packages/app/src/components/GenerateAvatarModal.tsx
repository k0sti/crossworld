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
  Slider,
  SliderTrack,
  SliderFilledTrack,
  SliderThumb,
  Text,
  SimpleGrid,
  Box,
  Badge,
  Tabs,
  TabList,
  TabPanels,
  Tab,
  TabPanel,
} from '@chakra-ui/react';
import { useState, useRef, useEffect } from 'react';
import * as THREE from 'three';
// import init from '@workspace/wasm';

export type GenerationCategory = 'humanoid' | 'animal' | 'geometric' | 'noise' | 'abstract';

export interface GenerationParams {
  category: GenerationCategory;
  preset?: string;
  size: number;
  seed?: string;
  // Category-specific params
  bodyType?: 'slim' | 'normal' | 'bulky';
  headSize?: number;
  limbLength?: number;
  complexity?: number;
  smoothness?: number;
}

interface GenerateAvatarModalProps {
  isOpen: boolean;
  onClose: () => void;
  onGenerate: (params: GenerationParams) => void;
}

const HUMANOID_PRESETS = [
  { id: 'peasant', label: '🧑 Peasant', icon: '🧑' },
  { id: 'warrior', label: '⚔️ Warrior', icon: '⚔️' },
  { id: 'mage', label: '🧙 Mage', icon: '🧙' },
  { id: 'knight', label: '🛡️ Knight', icon: '🛡️' },
  { id: 'archer', label: '🏹 Archer', icon: '🏹' },
  { id: 'robot', label: '🤖 Robot', icon: '🤖' },
];

const ANIMAL_PRESETS = [
  { id: 'cat', label: '🐱 Cat', icon: '🐱' },
  { id: 'dog', label: '🐶 Dog', icon: '🐶' },
  { id: 'bird', label: '🐦 Bird', icon: '🐦' },
  { id: 'fish', label: '🐟 Fish', icon: '🐟' },
  { id: 'dragon', label: '🐉 Dragon', icon: '🐉' },
  { id: 'bear', label: '🐻 Bear', icon: '🐻' },
];

const GEOMETRIC_PRESETS = [
  { id: 'sphere', label: '⚪ Sphere', icon: '⚪' },
  { id: 'cube', label: '🔳 Cube', icon: '🔳' },
  { id: 'pyramid', label: '🔺 Pyramid', icon: '🔺' },
  { id: 'torus', label: '🍩 Torus', icon: '🍩' },
  { id: 'cylinder', label: '🥫 Cylinder', icon: '🥫' },
  { id: 'diamond', label: '💎 Diamond', icon: '💎' },
];

export function GenerateAvatarModal({ isOpen, onClose, onGenerate }: GenerateAvatarModalProps) {
  const [category, setCategory] = useState<GenerationCategory>('humanoid');
  const [preset, setPreset] = useState<string>('peasant');
  const [size, setSize] = useState<number>(16);
  const [bodyType, setBodyType] = useState<'slim' | 'normal' | 'bulky'>('normal');
  const [complexity, setComplexity] = useState<number>(50);
  const [smoothness, setSmoothness] = useState<number>(50);

  // Preview canvas
  const previewCanvasRef = useRef<HTMLCanvasElement>(null);
  const previewSceneRef = useRef<{
    scene: THREE.Scene;
    camera: THREE.PerspectiveCamera;
    renderer: THREE.WebGLRenderer;
    mesh?: THREE.Mesh;
    animationId?: number;
  } | null>(null);

  // Initialize preview scene
  useEffect(() => {
    if (!isOpen || !previewCanvasRef.current) return;

    const canvas = previewCanvasRef.current;
    const scene = new THREE.Scene();
    scene.background = new THREE.Color(0x1a1a1e);

    const camera = new THREE.PerspectiveCamera(50, 1, 0.1, 1000);
    camera.position.set(0, 1, 3);
    camera.lookAt(0, 0.5, 0);

    const renderer = new THREE.WebGLRenderer({ canvas, antialias: true });
    renderer.setSize(256, 256);
    renderer.setPixelRatio(window.devicePixelRatio);

    // Lighting
    const ambientLight = new THREE.AmbientLight(0xffffff, 0.6);
    scene.add(ambientLight);

    const directionalLight = new THREE.DirectionalLight(0xffffff, 0.8);
    directionalLight.position.set(5, 10, 5);
    scene.add(directionalLight);

    previewSceneRef.current = { scene, camera, renderer };

    // Animation loop
    const animate = () => {
      if (!previewSceneRef.current) return;

      const { scene, camera, renderer, mesh } = previewSceneRef.current;

      // Rotate mesh slowly
      if (mesh) {
        mesh.rotation.y += 0.01;
      }

      renderer.render(scene, camera);
      previewSceneRef.current.animationId = requestAnimationFrame(animate);
    };
    animate();

    return () => {
      if (previewSceneRef.current?.animationId) {
        cancelAnimationFrame(previewSceneRef.current.animationId);
      }
      renderer.dispose();
    };
  }, [isOpen]);

  // Update preview when parameters change
  // TODO: Re-enable once WASM import issues are resolved
  useEffect(() => {
    if (!isOpen || !previewSceneRef.current) return;

    // Placeholder: Show a simple cube for now
    const { scene, mesh: oldMesh } = previewSceneRef.current;

    // Remove old mesh
    if (oldMesh) {
      scene.remove(oldMesh);
      oldMesh.geometry.dispose();
      if (Array.isArray(oldMesh.material)) {
        oldMesh.material.forEach((m: THREE.Material) => m.dispose());
      } else {
        oldMesh.material.dispose();
      }
    }

    // Create placeholder cube
    const geometry = new THREE.BoxGeometry(1, 1, 1);
    const material = new THREE.MeshStandardMaterial({ color: 0x6496fa });
    const mesh = new THREE.Mesh(geometry, material);
    scene.add(mesh);

    previewSceneRef.current.mesh = mesh;
  }, [isOpen, category, preset, size, bodyType, complexity]);

  const handleGenerate = () => {
    const params: GenerationParams = {
      category,
      preset,
      size,
      bodyType,
      complexity,
      smoothness,
      seed: Math.random().toString(36).substring(7),
    };
    onGenerate(params);
  };

  const handlePresetClick = (presetId: string) => {
    setPreset(presetId);
  };

  const renderPresetGrid = (presets: typeof HUMANOID_PRESETS) => (
    <SimpleGrid columns={3} spacing={3}>
      {presets.map((p) => (
        <Box
          key={p.id}
          as="button"
          onClick={() => handlePresetClick(p.id)}
          p={4}
          bg={preset === p.id ? 'rgba(100, 150, 250, 0.3)' : 'rgba(80, 80, 80, 0.1)'}
          border="1px solid"
          borderColor={preset === p.id ? 'rgba(100, 150, 250, 0.5)' : 'rgba(255, 255, 255, 0.1)'}
          borderRadius="md"
          _hover={{
            bg: preset === p.id ? 'rgba(100, 150, 250, 0.4)' : 'rgba(120, 120, 120, 0.2)',
          }}
          transition="all 0.2s"
          cursor="pointer"
        >
          <VStack spacing={2}>
            <Text fontSize="3xl">{p.icon}</Text>
            <Text fontSize="xs" color="white">
              {p.label.split(' ')[1]}
            </Text>
          </VStack>
        </Box>
      ))}
    </SimpleGrid>
  );

  return (
    <Modal isOpen={isOpen} onClose={onClose} size="xl">
      <ModalOverlay />
      <ModalContent bg="rgba(20, 20, 30, 0.95)" backdropFilter="blur(10px)">
        <ModalHeader color="white">Generate Avatar</ModalHeader>
        <ModalCloseButton color="white" />
        <ModalBody pb={6}>
          <VStack align="stretch" spacing={4}>
            {/* Preview Canvas */}
            <Box
              display="flex"
              justifyContent="center"
              alignItems="center"
              bg="rgba(26, 26, 30, 0.8)"
              borderRadius="md"
              p={4}
              border="1px solid"
              borderColor="rgba(255, 255, 255, 0.1)"
            >
              <canvas
                ref={previewCanvasRef}
                width={256}
                height={256}
                style={{
                  borderRadius: '8px',
                  maxWidth: '100%',
                  height: 'auto',
                }}
              />
            </Box>

            {/* Category Tabs */}
            <Tabs
              variant="soft-rounded"
              colorScheme="blue"
              onChange={(index) => {
                const categories: GenerationCategory[] = ['humanoid', 'animal', 'geometric', 'noise', 'abstract'];
                setCategory(categories[index]);
              }}
            >
              <TabList>
                <Tab fontSize="xs">🧑 Humanoid</Tab>
                <Tab fontSize="xs">🐾 Animal</Tab>
                <Tab fontSize="xs">📐 Geometric</Tab>
                <Tab fontSize="xs">🌊 Noise</Tab>
                <Tab fontSize="xs">✨ Abstract</Tab>
              </TabList>

              <TabPanels>
                {/* Humanoid */}
                <TabPanel>
                  {renderPresetGrid(HUMANOID_PRESETS)}
                </TabPanel>

                {/* Animal */}
                <TabPanel>
                  {renderPresetGrid(ANIMAL_PRESETS)}
                </TabPanel>

                {/* Geometric */}
                <TabPanel>
                  {renderPresetGrid(GEOMETRIC_PRESETS)}
                </TabPanel>

                {/* Noise */}
                <TabPanel>
                  <VStack align="stretch" spacing={4}>
                    <Text fontSize="sm" color="white">
                      Noise-based procedural generation
                    </Text>
                    <Box>
                      <Text fontSize="xs" color="gray.400" mb={2}>
                        Complexity: {complexity}%
                      </Text>
                      <Slider value={complexity} onChange={setComplexity} min={0} max={100}>
                        <SliderTrack bg="gray.700">
                          <SliderFilledTrack bg="blue.500" />
                        </SliderTrack>
                        <SliderThumb />
                      </Slider>
                    </Box>
                  </VStack>
                </TabPanel>

                {/* Abstract */}
                <TabPanel>
                  <VStack align="stretch" spacing={4}>
                    <Text fontSize="sm" color="white">
                      Abstract form generation
                    </Text>
                    <Box>
                      <Text fontSize="xs" color="gray.400" mb={2}>
                        Smoothness: {smoothness}%
                      </Text>
                      <Slider value={smoothness} onChange={setSmoothness} min={0} max={100}>
                        <SliderTrack bg="gray.700">
                          <SliderFilledTrack bg="purple.500" />
                        </SliderTrack>
                        <SliderThumb />
                      </Slider>
                    </Box>
                  </VStack>
                </TabPanel>
              </TabPanels>
            </Tabs>

            {/* Common Parameters */}
            <VStack align="stretch" spacing={3} pt={4} borderTop="1px solid" borderColor="rgba(255,255,255,0.1)">
              <Text fontSize="sm" fontWeight="semibold" color="white">
                Common Parameters
              </Text>

              {/* Size */}
              <Box>
                <HStack justify="space-between" mb={2}>
                  <Text fontSize="xs" color="gray.400">
                    Size
                  </Text>
                  <Badge colorScheme="blue" fontSize="xs">
                    {size} voxels
                  </Badge>
                </HStack>
                <Slider value={size} onChange={setSize} min={4} max={32} step={2}>
                  <SliderTrack bg="gray.700">
                    <SliderFilledTrack bg="blue.500" />
                  </SliderTrack>
                  <SliderThumb />
                </Slider>
              </Box>

              {/* Body Type (for humanoid/animal) */}
              {(category === 'humanoid' || category === 'animal') && (
                <Box>
                  <Text fontSize="xs" color="gray.400" mb={2}>
                    Body Type
                  </Text>
                  <HStack spacing={2}>
                    {['slim', 'normal', 'bulky'].map((type) => (
                      <Button
                        key={type}
                        size="sm"
                        fontSize="xs"
                        flex={1}
                        colorScheme={bodyType === type ? 'blue' : 'gray'}
                        onClick={() => setBodyType(type as any)}
                      >
                        {type.charAt(0).toUpperCase() + type.slice(1)}
                      </Button>
                    ))}
                  </HStack>
                </Box>
              )}
            </VStack>

            {/* Action Buttons */}
            <HStack spacing={3} pt={4}>
              <Button flex={1} onClick={onClose} variant="ghost" color="white">
                Cancel
              </Button>
              <Button flex={1} colorScheme="blue" onClick={handleGenerate}>
                Generate & Apply
              </Button>
            </HStack>
          </VStack>
        </ModalBody>
      </ModalContent>
    </Modal>
  );
}
