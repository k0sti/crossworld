## ADDED Requirements

### Requirement: Unified Renderer Trait

The renderer crate SHALL provide a unified `Renderer` trait that all renderer implementations implement, supporting both GL-based and software rendering.

#### Scenario: Software renderer usage
- **WHEN** a renderer does not require OpenGL context (e.g., CpuTracer, BcfTracer)
- **THEN** it SHALL implement `render()` and `render_with_camera()` methods that write to an internal image buffer
- **AND** `supports_gl()` SHALL return false
- **AND** `supports_image_output()` SHALL return true

#### Scenario: GL-based renderer usage
- **WHEN** a renderer requires OpenGL context (e.g., GlTracer, ComputeTracer, MeshRenderer)
- **THEN** it SHALL implement `init_gl()` for initialization with GL context
- **AND** it SHALL implement `destroy_gl()` for cleanup
- **AND** it SHALL implement `render_to_framebuffer()` for rendering to GL framebuffer
- **AND** `supports_gl()` SHALL return true

#### Scenario: Renderer name identification
- **WHEN** any renderer is queried via `name()` method
- **THEN** it SHALL return a human-readable string identifier (e.g., "CPU Tracer", "GL Tracer", "Mesh Renderer")

### Requirement: Renderers Module Organization

Renderer implementations SHALL be organized in a dedicated `renderers` module for clear separation of concerns.

#### Scenario: Module structure
- **WHEN** accessing renderer implementations
- **THEN** all renderer types SHALL be available under `renderer::renderers::{CpuTracer, GlTracer, BcfTracer, ComputeTracer, MeshRenderer}`
- **AND** the Renderer trait SHALL be available at `renderer::Renderer`
- **AND** backward-compatible re-exports SHALL exist at the crate root for existing code

#### Scenario: Common types accessibility
- **WHEN** using renderer functionality
- **THEN** CameraConfig SHALL be available from `renderer::camera::CameraConfig`
- **AND** lighting constants SHALL be available from `renderer::lighting::{AMBIENT, DIFFUSE_STRENGTH, LIGHT_DIR, BACKGROUND_COLOR}`
- **AND** they SHALL also be re-exported at crate root for backward compatibility

### Requirement: GL Renderer Lifecycle

GL-based renderers SHALL support proper initialization and cleanup of OpenGL resources.

#### Scenario: Initialization
- **WHEN** `init_gl(&Context)` is called on a GL-based renderer
- **THEN** it SHALL create all required GL resources (shaders, buffers, textures)
- **AND** it SHALL return Result<(), String> indicating success or failure
- **AND** the renderer SHALL be ready for rendering operations

#### Scenario: Cleanup
- **WHEN** `destroy_gl(&Context)` is called on a GL-based renderer
- **THEN** it SHALL release all GL resources (delete shaders, buffers, textures)
- **AND** it SHALL be safe to call multiple times (idempotent)

#### Scenario: Rendering to framebuffer
- **WHEN** `render_to_framebuffer()` is called on an initialized GL renderer
- **THEN** it SHALL render to the currently bound framebuffer
- **AND** it SHALL accept width, height, and camera or time parameters
- **AND** it SHALL not modify framebuffer bindings after completion

### Requirement: MeshRenderer Trait Implementation

The MeshRenderer SHALL implement the Renderer trait with full support for GL-based rendering.

#### Scenario: MeshRenderer as Renderer
- **WHEN** MeshRenderer is used through the Renderer trait
- **THEN** it SHALL support `init_gl()` for shader and buffer initialization
- **AND** it SHALL support `render_to_framebuffer()` for mesh rendering
- **AND** it SHALL support `render_with_camera()` using CameraConfig
- **AND** `supports_gl()` SHALL return true
- **AND** `supports_image_output()` SHALL return false

#### Scenario: Mesh upload and rendering
- **WHEN** `upload_mesh(&Cube<u8>, depth)` is called
- **THEN** it SHALL generate and upload mesh geometry to GPU
- **AND** return a mesh index for later rendering
- **AND** rendering via `render_to_framebuffer()` SHALL use uploaded meshes

### Requirement: Cross-Crate Interface Usage

The Renderer interface SHALL be usable from other crates without exposing internal implementation details.

#### Scenario: Using renderer in proto-gl
- **WHEN** proto-gl creates a renderer instance
- **THEN** it SHALL be able to use `renderer::renderers::GlTracer::new()`
- **AND** initialize it with `init_gl(&gl_context)`
- **AND** render using unified `render_to_framebuffer()` method
- **AND** clean up with `destroy_gl(&gl_context)`

#### Scenario: Generic renderer code
- **WHEN** code needs to work with any renderer implementation
- **THEN** it SHALL be able to accept `&mut dyn Renderer`
- **AND** call methods from the Renderer trait
- **AND** query capabilities with `supports_gl()` and `supports_image_output()`

### Requirement: File Output Support

All renderers SHALL support saving their output to image files regardless of rendering backend.

#### Scenario: Software renderer file save
- **WHEN** a software renderer (CpuTracer, BcfTracer) has rendered a frame to its image buffer
- **THEN** `save_to_file(path)` SHALL save the image buffer to the specified file path
- **AND** it SHALL support common image formats (PNG, JPEG) via the image crate
- **AND** return Result<(), String> indicating success or failure

#### Scenario: GL renderer file save
- **WHEN** a GL renderer (GlTracer, ComputeTracer, MeshRenderer) has rendered to a framebuffer
- **THEN** `save_to_file(&Context, path)` SHALL read pixels from the currently bound framebuffer
- **AND** convert the pixel data to an image format
- **AND** save to the specified file path
- **AND** return Result<(), String> indicating success or failure

#### Scenario: Framebuffer readback
- **WHEN** reading pixels from a GL framebuffer for file save
- **THEN** it SHALL use glReadPixels or equivalent to retrieve RGBA pixel data
- **AND** convert from GL coordinate system (origin bottom-left) to image coordinate system (origin top-left)
- **AND** handle gamma correction if the framebuffer is in sRGB color space

### Requirement: Backward Compatibility

Existing code using the old module structure SHALL continue to work without modification.

#### Scenario: Existing imports
- **WHEN** existing code uses `use renderer::{CpuTracer, GlTracer, BcfTracer, ComputeTracer, MeshRenderer}`
- **THEN** these imports SHALL continue to work via re-exports from lib.rs
- **AND** no compilation errors SHALL occur

#### Scenario: Existing trait usage
- **WHEN** existing code uses `Renderer` trait methods
- **THEN** behavior SHALL remain unchanged
- **AND** all existing renderer implementations SHALL continue to work
