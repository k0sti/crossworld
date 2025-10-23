import { useEffect, useRef, useState } from 'react'
import { Box, IconButton, VStack, HStack } from '@chakra-ui/react'
import { FiRotateCcw, FiSave, FiDownload } from 'react-icons/fi'
import * as THREE from 'three'
import { PaletteSelector } from './PaletteSelector'

interface CubeEditorViewProps {
  onSave?: (voxelData: Uint8Array) => void
}

const GRID_SIZE = 16
const VOXEL_SIZE = 0.1

export function CubeEditorView({ onSave }: CubeEditorViewProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const sceneRef = useRef<THREE.Scene | null>(null)
  const cameraRef = useRef<THREE.PerspectiveCamera | null>(null)
  const rendererRef = useRef<THREE.WebGLRenderer | null>(null)
  const voxelMeshesRef = useRef<Map<string, THREE.Mesh>>(new Map())
  const [selectedColor, setSelectedColor] = useState('#FF0000')
  const [isPaletteOpen, setIsPaletteOpen] = useState(false)

  // Initialize Three.js scene
  useEffect(() => {
    if (!canvasRef.current) return

    // Scene
    const scene = new THREE.Scene()
    scene.background = new THREE.Color(0x1a1a1a)
    sceneRef.current = scene

    // Camera
    const camera = new THREE.PerspectiveCamera(
      75,
      canvasRef.current.clientWidth / canvasRef.current.clientHeight,
      0.1,
      1000
    )
    camera.position.set(2, 2, 3)
    camera.lookAt(0, 0, 0)
    cameraRef.current = camera

    // Renderer
    const renderer = new THREE.WebGLRenderer({
      canvas: canvasRef.current,
      antialias: true,
    })
    renderer.setSize(canvasRef.current.clientWidth, canvasRef.current.clientHeight)
    rendererRef.current = renderer

    // Lighting
    const ambientLight = new THREE.AmbientLight(0xffffff, 0.5)
    scene.add(ambientLight)

    const directionalLight = new THREE.DirectionalLight(0xffffff, 0.8)
    directionalLight.position.set(5, 5, 5)
    scene.add(directionalLight)

    // Grid helper
    const gridHelper = new THREE.GridHelper(GRID_SIZE * VOXEL_SIZE, GRID_SIZE, 0x444444, 0x222222)
    scene.add(gridHelper)

    // Axes helper
    const axesHelper = new THREE.AxesHelper(1)
    scene.add(axesHelper)

    // Animation loop
    const animate = () => {
      requestAnimationFrame(animate)
      renderer.render(scene, camera)
    }
    animate()

    // Handle resize
    const handleResize = () => {
      if (!canvasRef.current || !camera || !renderer) return
      camera.aspect = canvasRef.current.clientWidth / canvasRef.current.clientHeight
      camera.updateProjectionMatrix()
      renderer.setSize(canvasRef.current.clientWidth, canvasRef.current.clientHeight)
    }
    window.addEventListener('resize', handleResize)

    // Mouse controls - simple rotation
    let isDragging = false
    let previousMousePosition = { x: 0, y: 0 }

    const handleMouseDown = (e: MouseEvent) => {
      isDragging = true
      previousMousePosition = { x: e.clientX, y: e.clientY }
    }

    const handleMouseMove = (e: MouseEvent) => {
      if (!isDragging || !camera) return

      const deltaX = e.clientX - previousMousePosition.x
      const deltaY = e.clientY - previousMousePosition.y

      // Rotate camera around the origin
      const radius = Math.sqrt(
        camera.position.x ** 2 + camera.position.y ** 2 + camera.position.z ** 2
      )

      const theta = Math.atan2(camera.position.z, camera.position.x) - deltaX * 0.01
      const phi = Math.acos(camera.position.y / radius) + deltaY * 0.01

      camera.position.x = radius * Math.sin(phi) * Math.cos(theta)
      camera.position.y = radius * Math.cos(phi)
      camera.position.z = radius * Math.sin(phi) * Math.sin(theta)

      camera.lookAt(0, 0, 0)

      previousMousePosition = { x: e.clientX, y: e.clientY }
    }

    const handleMouseUp = () => {
      isDragging = false
    }

    canvasRef.current.addEventListener('mousedown', handleMouseDown)
    window.addEventListener('mousemove', handleMouseMove)
    window.addEventListener('mouseup', handleMouseUp)

    return () => {
      window.removeEventListener('resize', handleResize)
      canvasRef.current?.removeEventListener('mousedown', handleMouseDown)
      window.removeEventListener('mousemove', handleMouseMove)
      window.removeEventListener('mouseup', handleMouseUp)
      renderer.dispose()
    }
  }, [])

  // Handle canvas click to place voxel
  const handleCanvasClick = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (!sceneRef.current || !cameraRef.current || !canvasRef.current) return

    const rect = canvasRef.current.getBoundingClientRect()
    const x = ((e.clientX - rect.left) / rect.width) * 2 - 1
    const y = -((e.clientY - rect.top) / rect.height) * 2 + 1

    const raycaster = new THREE.Raycaster()
    raycaster.setFromCamera(new THREE.Vector2(x, y), cameraRef.current)

    // Check intersection with existing voxels
    const meshArray = Array.from(voxelMeshesRef.current.values())
    const intersects = raycaster.intersectObjects(meshArray)

    if (intersects.length > 0) {
      // Place voxel adjacent to clicked voxel
      const intersect = intersects[0]
      const normal = intersect.face?.normal
      if (!normal) return

      const gridPosition = intersect.object.position.clone()
      gridPosition.add(normal.clone().multiplyScalar(VOXEL_SIZE))

      // Snap to grid
      const voxelX = Math.round(gridPosition.x / VOXEL_SIZE)
      const voxelY = Math.round(gridPosition.y / VOXEL_SIZE)
      const voxelZ = Math.round(gridPosition.z / VOXEL_SIZE)

      placeVoxel(voxelX, voxelY, voxelZ)
    } else {
      // Place voxel at origin if no intersection
      placeVoxel(0, 0, 0)
    }
  }

  const placeVoxel = (x: number, y: number, z: number) => {
    if (!sceneRef.current) return

    const key = `${x},${y},${z}`

    // Check if voxel already exists
    if (voxelMeshesRef.current.has(key)) {
      // Remove existing voxel
      const mesh = voxelMeshesRef.current.get(key)!
      sceneRef.current.remove(mesh)
      mesh.geometry.dispose()
      ;(mesh.material as THREE.Material).dispose()
      voxelMeshesRef.current.delete(key)
    } else {
      // Add new voxel
      const geometry = new THREE.BoxGeometry(VOXEL_SIZE, VOXEL_SIZE, VOXEL_SIZE)
      const material = new THREE.MeshLambertMaterial({ color: selectedColor })
      const mesh = new THREE.Mesh(geometry, material)

      mesh.position.set(x * VOXEL_SIZE, y * VOXEL_SIZE, z * VOXEL_SIZE)
      sceneRef.current.add(mesh)
      voxelMeshesRef.current.set(key, mesh)
    }
  }

  const handleClear = () => {
    if (!sceneRef.current) return

    voxelMeshesRef.current.forEach(mesh => {
      sceneRef.current!.remove(mesh)
      mesh.geometry.dispose()
      ;(mesh.material as THREE.Material).dispose()
    })
    voxelMeshesRef.current.clear()
  }

  const handleSave = () => {
    // TODO: Implement save functionality
    console.log('Save voxel data')
    if (onSave) {
      // Convert voxel meshes to data format
      const data = new Uint8Array(GRID_SIZE * GRID_SIZE * GRID_SIZE * 4)
      onSave(data)
    }
  }

  const handleExport = () => {
    // TODO: Implement export functionality
    console.log('Export as CSM')
  }

  return (
    <Box position="relative" w="100%" h="100vh">
      {/* 3D Canvas */}
      <canvas
        ref={canvasRef}
        onClick={handleCanvasClick}
        style={{
          width: '100%',
          height: '100%',
          display: 'block',
        }}
      />

      {/* Editor Controls */}
      <VStack
        position="fixed"
        left={4}
        top="80px"
        spacing={2}
        bg="rgba(0, 0, 0, 0.5)"
        backdropFilter="blur(8px)"
        p={2}
        borderRadius="md"
      >
        <IconButton
          aria-label="Toggle palette"
          icon={<Box w="20px" h="20px" bg={selectedColor} borderRadius="sm" />}
          onClick={() => setIsPaletteOpen(!isPaletteOpen)}
          size="md"
        />
        <IconButton
          aria-label="Clear all"
          icon={<FiRotateCcw />}
          onClick={handleClear}
          size="md"
        />
        <IconButton
          aria-label="Save"
          icon={<FiSave />}
          onClick={handleSave}
          size="md"
          colorScheme="green"
        />
        <IconButton
          aria-label="Export"
          icon={<FiDownload />}
          onClick={handleExport}
          size="md"
          colorScheme="blue"
        />
      </VStack>

      {/* Palette Selector */}
      <PaletteSelector
        isOpen={isPaletteOpen}
        onClose={() => setIsPaletteOpen(false)}
        selectedColor={selectedColor}
        onColorSelect={setSelectedColor}
      />
    </Box>
  )
}
