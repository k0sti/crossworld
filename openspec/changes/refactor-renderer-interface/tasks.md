## 1. Core Interface Design

- [ ] 1.1 Extend `Renderer` trait with GL lifecycle methods (`init_gl`, `destroy_gl`, `render_to_framebuffer`)
- [ ] 1.2 Add capability query methods (`supports_gl`, `supports_image_output`)
- [ ] 1.3 Split `renderer.rs` into focused modules: `renderer.rs` (trait), `camera.rs`, `lighting.rs`
- [ ] 1.4 Keep helper types in `renderer.rs` (Ray, CubeBounds, HitInfo, lighting calculation helpers)

## 2. Module Reorganization

- [ ] 2.1 Create `src/renderers/` directory
- [ ] 2.2 Move `cpu_tracer.rs` to `renderers/cpu_tracer.rs`
- [ ] 2.3 Move `bcf_cpu_tracer.rs` to `renderers/bcf_cpu_tracer.rs`
- [ ] 2.4 Move `gl_tracer.rs` to `renderers/gl_tracer.rs`
- [ ] 2.5 Move `gpu_tracer.rs` to `renderers/gpu_tracer.rs`
- [ ] 2.6 Move `mesh_renderer.rs` to `renderers/mesh_renderer.rs`
- [ ] 2.7 Create `renderers/mod.rs` with public exports
- [ ] 2.8 Update module paths in all moved files

## 3. Implement Extended Renderer Trait

- [ ] 3.1 Update CpuTracer to implement new Renderer methods
- [ ] 3.2 Update BcfTracer to implement new Renderer methods
- [ ] 3.3 Update GlTracer to implement new Renderer methods
- [ ] 3.4 Update ComputeTracer to implement new Renderer methods
- [ ] 3.5 Implement Renderer trait for MeshRenderer (currently missing)

## 4. GL Lifecycle Implementation

- [ ] 4.1 Ensure GlTracer `init_gl()` creates all GL resources properly
- [ ] 4.2 Ensure GlTracer `destroy_gl()` cleans up all resources
- [ ] 4.3 Add `render_to_framebuffer()` to GlTracer
- [ ] 4.4 Implement same for ComputeTracer
- [ ] 4.5 Implement same for MeshRenderer
- [ ] 4.6 Add capability queries to all renderers

## 5. File Output Implementation

- [ ] 5.1 Add `save_to_file()` method to Renderer trait with default implementation
- [ ] 5.2 Implement file saving for CpuTracer (use existing image buffer)
- [ ] 5.3 Implement file saving for BcfTracer (use existing image buffer)
- [ ] 5.4 Implement framebuffer readback helper function for GL renderers
- [ ] 5.5 Implement file saving for GlTracer using framebuffer readback
- [ ] 5.6 Implement file saving for ComputeTracer using framebuffer readback
- [ ] 5.7 Implement file saving for MeshRenderer using framebuffer readback
- [ ] 5.8 Add tests for file output from all renderer types

## 6. Library Exports and Backward Compatibility

- [ ] 6.1 Update `src/lib.rs` to export `renderer`, `camera`, `lighting`, and `renderers` modules
- [ ] 6.2 Add backward-compatible re-exports at crate root for all renderer types
- [ ] 6.3 Add backward-compatible re-exports for CameraConfig, Renderer, and lighting constants
- [ ] 6.4 Verify existing tests still compile and pass

## 7. Update proto-gl Integration

- [ ] 7.1 Update proto-gl to use `renderer::renderers::GlTracer`
- [ ] 7.2 Update proto-gl to use `renderer::renderers::MeshRenderer`
- [ ] 7.3 Use unified `init_gl()` / `destroy_gl()` lifecycle methods
- [ ] 7.4 Use `render_to_framebuffer()` for rendering operations
- [ ] 7.5 Remove any renderer-specific workarounds

## 8. Documentation and Examples

- [ ] 8.1 Add module-level documentation to `renderers/mod.rs`
- [ ] 8.2 Add module-level documentation to `renderer.rs`, `camera.rs`, `lighting.rs`
- [ ] 8.3 Update renderer struct documentation with trait implementation notes
- [ ] 8.4 Add example of using Renderer trait generically in documentation
- [ ] 8.5 Document file output capability and usage patterns
- [ ] 8.6 Document architecture (cube = data, physics = entities, renderer = rendering)

## 9. Testing and Validation

- [ ] 9.1 Verify all renderer tests still pass
- [ ] 9.2 Verify proto-gl builds and runs correctly
- [ ] 9.3 Verify backward compatibility with existing renderer usage
- [ ] 9.4 Add integration test using Renderer trait polymorphically
- [ ] 9.5 Test file output from all renderer types (software and GL)
