import { useEffect, useRef, useState } from 'react'
import { Box, IconButton, VStack } from '@chakra-ui/react'
import { FiRotateCcw, FiSave, FiDownload } from 'react-icons/fi'
import * as THREE from 'three'
import { PaletteSelector } from './PaletteSelector'
import { BottomBar } from './BottomBar'
import { generateHSVPalette } from '../palettes/hsv'

interface CubeEditorViewProps {
  onSave?: (voxelData: Uint8Array) => void
}

const CUBE_MAX_DEPTH = 4  // 2^4 = 16x16x16
const CUBE_SIZE = 1 << CUBE_MAX_DEPTH  // 16
const MODEL_ID = 'editor-model'

export function CubeEditorView(_props: CubeEditorViewProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const sceneRef = useRef<THREE.Scene | null>(null)
  const cameraRef = useRef<THREE.PerspectiveCamera | null>(null)
  const rendererRef = useRef<THREE.WebGLRenderer | null>(null)
  const geometryMeshRef = useRef<THREE.Mesh | null>(null)
  const gridHelperRef = useRef<THREE.GridHelper | null>(null)
  const previewPlaneRef = useRef<THREE.Mesh | null>(null)
  const raycasterRef = useRef<THREE.Raycaster>(new THREE.Raycaster())
  const mouseRef = useRef<THREE.Vector2>(new THREE.Vector2())
  const [palette, setPalette] = useState<string[]>(() => generateHSVPalette(16))
  const [selectedColor, setSelectedColor] = useState(() => generateHSVPalette(16)[0])
  const [selectedColorIndex, setSelectedColorIndex] = useState(0)
  const [isPaletteOpen, setIsPaletteOpen] = useState(false)
  const [wasmModule, setWasmModule] = useState<any>(null)
  const depth = CUBE_MAX_DEPTH // Always use max depth

  // Initialize WASM module
  useEffect(() => {
    import('@workspace/wasm-cube').then(async (wasmModule) => {
      await wasmModule.default()  // Initialize WASM
      const wasm = wasmModule as any // Type assertion needed for dynamic import
      wasm.create_model(MODEL_ID, CUBE_MAX_DEPTH)
      wasm.set_model_palette(MODEL_ID, palette)
      setWasmModule(wasm)
      console.log('[CubeEditor] WASM module initialized')
    }).catch(console.error)
  }, [palette])

  // Initialize Three.js scene (only once)
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
    camera.position.set(10, 10, 15)
    camera.lookAt(CUBE_SIZE / 2, CUBE_SIZE / 2, CUBE_SIZE / 2)
    cameraRef.current = camera

    // Renderer
    const renderer = new THREE.WebGLRenderer({
      canvas: canvasRef.current,
      antialias: true,
    })
    renderer.setSize(canvasRef.current.clientWidth, canvasRef.current.clientHeight)
    rendererRef.current = renderer

    // Lighting
    const ambientLight = new THREE.AmbientLight(0xffffff, 0.6)
    scene.add(ambientLight)

    const directionalLight = new THREE.DirectionalLight(0xffffff, 0.8)
    directionalLight.position.set(20, 20, 20)
    scene.add(directionalLight)

    // Create helpers (always visible)
    const gridHelper = new THREE.GridHelper(CUBE_SIZE, CUBE_SIZE, 0x444444, 0x222222)
    gridHelper.position.set(CUBE_SIZE / 2, 0, CUBE_SIZE / 2)
    scene.add(gridHelper)
    gridHelperRef.current = gridHelper

    const axesHelper = new THREE.AxesHelper(CUBE_SIZE)
    scene.add(axesHelper)

    // Wireframe box for edit area borders (same color as grid center line)
    const boxGeometry = new THREE.BoxGeometry(CUBE_SIZE, CUBE_SIZE, CUBE_SIZE)
    const boxEdges = new THREE.EdgesGeometry(boxGeometry)
    const boxMaterial = new THREE.LineBasicMaterial({ color: 0x444444 })
    const wireframeBox = new THREE.LineSegments(boxEdges, boxMaterial)
    wireframeBox.position.set(CUBE_SIZE / 2, CUBE_SIZE / 2, CUBE_SIZE / 2)
    scene.add(wireframeBox)

    // Ground plane for raycasting
    const groundGeometry = new THREE.PlaneGeometry(CUBE_SIZE, CUBE_SIZE)
    const groundMaterial = new THREE.MeshBasicMaterial({
      visible: false,
      side: THREE.DoubleSide
    })
    const groundPlane = new THREE.Mesh(groundGeometry, groundMaterial)
    groundPlane.rotation.x = -Math.PI / 2
    groundPlane.position.set(CUBE_SIZE / 2, 0, CUBE_SIZE / 2)
    scene.add(groundPlane)

    // Preview plane (shows where voxel will be placed)
    const voxelSize = CUBE_SIZE / (1 << depth)
    const previewGeometry = new THREE.PlaneGeometry(voxelSize, voxelSize)
    const previewMaterial = new THREE.MeshBasicMaterial({
      color: 0x00ff00,
      transparent: true,
      opacity: 0.3,
      side: THREE.DoubleSide
    })
    const previewPlane = new THREE.Mesh(previewGeometry, previewMaterial)
    previewPlane.rotation.x = -Math.PI / 2
    previewPlane.visible = false
    scene.add(previewPlane)
    previewPlaneRef.current = previewPlane

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

    // Mouse controls - rotation
    let isDragging = false
    let previousMousePosition = { x: 0, y: 0 }

    const handleMouseDown = (e: MouseEvent) => {
      isDragging = true
      previousMousePosition = { x: e.clientX, y: e.clientY }
    }

    const handleMouseMove = (e: MouseEvent) => {
      if (isDragging && camera) {
        const deltaX = e.clientX - previousMousePosition.x
        const deltaY = e.clientY - previousMousePosition.y

        // Rotate camera around the cube center
        const center = new THREE.Vector3(CUBE_SIZE / 2, CUBE_SIZE / 2, CUBE_SIZE / 2)
        const offset = camera.position.clone().sub(center)
        const radius = offset.length()

        const theta = Math.atan2(offset.z, offset.x) - deltaX * 0.01
        const phi = Math.acos(offset.y / radius) + deltaY * 0.01
        const phiClamped = Math.max(0.1, Math.min(Math.PI - 0.1, phi))

        offset.x = radius * Math.sin(phiClamped) * Math.cos(theta)
        offset.y = radius * Math.cos(phiClamped)
        offset.z = radius * Math.sin(phiClamped) * Math.sin(theta)

        camera.position.copy(center.clone().add(offset))
        camera.lookAt(center)

        previousMousePosition = { x: e.clientX, y: e.clientY }
      } else if (!isDragging && canvasRef.current) {
        // Update preview plane position
        const rect = canvasRef.current.getBoundingClientRect()
        mouseRef.current.x = ((e.clientX - rect.left) / rect.width) * 2 - 1
        mouseRef.current.y = -((e.clientY - rect.top) / rect.height) * 2 + 1

        if (camera && previewPlane) {
          raycasterRef.current.setFromCamera(mouseRef.current, camera)
          const intersects = raycasterRef.current.intersectObject(groundPlane)

          if (intersects.length > 0) {
            const point = intersects[0].point
            const voxelSize = CUBE_SIZE / (1 << depth)

            // Convert to grid coordinates and back to world position at voxel center
            const gridX = Math.floor(point.x / voxelSize)
            const gridZ = Math.floor(point.z / voxelSize)

            // Calculate world position at center of voxel
            const worldX = gridX * voxelSize + voxelSize / 2
            const worldZ = gridZ * voxelSize + voxelSize / 2

            // Clamp to bounds
            const clampedX = Math.max(voxelSize / 2, Math.min(CUBE_SIZE - voxelSize / 2, worldX))
            const clampedZ = Math.max(voxelSize / 2, Math.min(CUBE_SIZE - voxelSize / 2, worldZ))

            previewPlane.position.set(clampedX, 0.01, clampedZ)
            previewPlane.visible = true
          } else {
            previewPlane.visible = false
          }
        }
      }
    }

    const handleMouseUp = () => {
      isDragging = false
    }

    const handleWheel = (e: WheelEvent) => {
      e.preventDefault()
      if (!camera) return

      const center = new THREE.Vector3(CUBE_SIZE / 2, CUBE_SIZE / 2, CUBE_SIZE / 2)
      const offset = camera.position.clone().sub(center)
      const radius = offset.length()

      const newRadius = Math.max(5, Math.min(50, radius + e.deltaY * 0.01))
      const scale = newRadius / radius

      offset.multiplyScalar(scale)
      camera.position.copy(center.clone().add(offset))
      camera.lookAt(center)
    }

    canvasRef.current.addEventListener('mousedown', handleMouseDown)
    canvasRef.current.addEventListener('wheel', handleWheel, { passive: false })
    canvasRef.current.addEventListener('mousemove', handleMouseMove)
    window.addEventListener('mouseup', handleMouseUp)

    return () => {
      window.removeEventListener('resize', handleResize)
      canvasRef.current?.removeEventListener('mousedown', handleMouseDown)
      canvasRef.current?.removeEventListener('wheel', handleWheel)
      canvasRef.current?.removeEventListener('mousemove', handleMouseMove)
      window.removeEventListener('mouseup', handleMouseUp)
      renderer.dispose()
    }
  }, []) // Remove depth dependency - scene should only initialize once

  // Update preview plane size when depth changes
  useEffect(() => {
    if (previewPlaneRef.current) {
      const voxelSize = CUBE_SIZE / (1 << depth)
      const newGeometry = new THREE.PlaneGeometry(voxelSize, voxelSize)
      previewPlaneRef.current.geometry.dispose()
      previewPlaneRef.current.geometry = newGeometry
    }
  }, [depth])

  // Update WASM palette when it changes
  useEffect(() => {
    if (wasmModule) {
      wasmModule.set_model_palette(MODEL_ID, palette)
      updateMesh() // Regenerate mesh with new colors
    }
  }, [palette, wasmModule])

  // Handle canvas click to place voxel
  const handleCanvasClick = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (!canvasRef.current || !cameraRef.current || !wasmModule) return

    const rect = canvasRef.current.getBoundingClientRect()
    const x = ((e.clientX - rect.left) / rect.width) * 2 - 1
    const y = -((e.clientY - rect.top) / rect.height) * 2 + 1

    const raycaster = new THREE.Raycaster()
    raycaster.setFromCamera(new THREE.Vector2(x, y), cameraRef.current)

    // Find ground plane
    const scene = sceneRef.current
    if (!scene) return

    const groundPlane = scene.children.find(
      child => child instanceof THREE.Mesh && child.material && !(child.material as THREE.MeshBasicMaterial).visible
    )
    if (!groundPlane) return

    const intersects = raycaster.intersectObject(groundPlane)

    if (intersects.length > 0) {
      const point = intersects[0].point

      // Convert world coordinates to grid coordinates at max depth
      // World coordinates are [0, CUBE_SIZE], grid coordinates are [0, CUBE_SIZE)
      const gridX = Math.floor(point.x)
      const gridY = 0  // Draw on y=0 plane
      const gridZ = Math.floor(point.z)

      // Clamp to valid range [0, CUBE_SIZE)
      const clampedX = Math.max(0, Math.min(CUBE_SIZE - 1, gridX))
      const clampedZ = Math.max(0, Math.min(CUBE_SIZE - 1, gridZ))

      console.log(`[CubeEditor] Drawing at (${clampedX}, ${gridY}, ${clampedZ}) with depth ${depth}, color index ${selectedColorIndex}, color ${selectedColor}`)

      // Call WASM draw function
      const result = wasmModule.draw(MODEL_ID, selectedColorIndex, clampedX, gridY, clampedZ, depth)

      // Check for errors
      if (result && result.error) {
        console.error('[CubeEditor] Draw error:', result.error)
        return
      }

      console.log('[CubeEditor] Draw result:', result)

      // Update mesh
      updateMesh()
    }
  }

  const updateMesh = () => {
    if (!wasmModule || !sceneRef.current) return

    const meshResult = wasmModule.get_model_mesh(MODEL_ID)

    if (meshResult && meshResult.error) {
      console.error('[CubeEditor] Mesh error:', meshResult.error)
      return
    }

    console.log('[CubeEditor] Mesh stats:', {
      vertices: meshResult.vertices.length / 3,
      indices: meshResult.indices.length / 3,
      triangles: meshResult.indices.length / 3,
      colors: meshResult.colors.length / 3,
      normals: meshResult.normals.length / 3,
    })

    // Debug: log first few vertices
    if (meshResult.vertices.length > 0) {
      console.log('[CubeEditor] First vertex:',
        meshResult.vertices[0],
        meshResult.vertices[1],
        meshResult.vertices[2]
      )
      console.log('[CubeEditor] First color:',
        meshResult.colors[0],
        meshResult.colors[1],
        meshResult.colors[2]
      )
    }

    // Remove old geometry mesh
    if (geometryMeshRef.current) {
      sceneRef.current.remove(geometryMeshRef.current)
      geometryMeshRef.current.geometry.dispose()
      if (geometryMeshRef.current.material instanceof THREE.Material) {
        geometryMeshRef.current.material.dispose()
      }
    }

    if (meshResult.vertices.length === 0) {
      console.log('[CubeEditor] Empty mesh, nothing to render')
      geometryMeshRef.current = null
      return
    }

    // Validate mesh data
    if (meshResult.vertices.length % 3 !== 0 ||
        meshResult.normals.length % 3 !== 0 ||
        meshResult.colors.length % 3 !== 0) {
      console.error('[CubeEditor] Invalid mesh data: vertex/normal/color arrays not divisible by 3')
      return
    }

    if (meshResult.vertices.length !== meshResult.normals.length ||
        meshResult.vertices.length !== meshResult.colors.length) {
      console.error('[CubeEditor] Mesh data mismatch:', {
        vertices: meshResult.vertices.length,
        normals: meshResult.normals.length,
        colors: meshResult.colors.length
      })
      return
    }

    try {
      // Create new geometry
      const geometry = new THREE.BufferGeometry()
      geometry.setAttribute('position', new THREE.BufferAttribute(new Float32Array(meshResult.vertices), 3))
      geometry.setAttribute('normal', new THREE.BufferAttribute(new Float32Array(meshResult.normals), 3))
      geometry.setAttribute('color', new THREE.BufferAttribute(new Float32Array(meshResult.colors), 3))
      geometry.setIndex(new THREE.BufferAttribute(new Uint32Array(meshResult.indices), 1))
      geometry.computeBoundingSphere()

      // Create material with vertex colors and lighting
      const material = new THREE.MeshLambertMaterial({
        vertexColors: true,
        side: THREE.DoubleSide
      })

      const mesh = new THREE.Mesh(geometry, material)
      sceneRef.current.add(mesh)
      geometryMeshRef.current = mesh

      console.log('[CubeEditor] Mesh updated successfully')
    } catch (error) {
      console.error('[CubeEditor] Failed to create mesh:', error)
    }
  }

  const handleClear = () => {
    if (!wasmModule) return

    // Recreate model
    wasmModule.create_model(MODEL_ID, CUBE_MAX_DEPTH)

    // Clear mesh
    if (geometryMeshRef.current && sceneRef.current) {
      sceneRef.current.remove(geometryMeshRef.current)
      geometryMeshRef.current.geometry.dispose()
      if (geometryMeshRef.current.material instanceof THREE.Material) {
        geometryMeshRef.current.material.dispose()
      }
      geometryMeshRef.current = null
    }
  }

  const handleSave = () => {
    // TODO: Implement save functionality
    console.log('[CubeEditor] Save not yet implemented')
  }

  const handleExport = () => {
    // TODO: Implement export functionality
    console.log('[CubeEditor] Export not yet implemented')
  }

  const handleColorSelect = (color: string, index: number) => {
    setSelectedColor(color)
    setSelectedColorIndex(index)
  }

  const handleColorChange = (index: number, newColor: string) => {
    const newPalette = [...palette]
    newPalette[index] = newColor
    setPalette(newPalette)

    // Update selected color if it's the one being changed
    if (index === selectedColorIndex) {
      setSelectedColor(newColor)
    }
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
        onColorSelect={handleColorSelect}
      />

      {/* Bottom Bar with Color Grid */}
      <BottomBar
        palette={palette}
        selectedColor={selectedColor}
        selectedColorIndex={selectedColorIndex}
        onColorSelect={handleColorSelect}
        onColorChange={handleColorChange}
      />
    </Box>
  )
}
