//! Debug test to examine coordinate transformation from bounding box hits to octree space

use renderer::cpu_tracer::CpuCubeTracer;
use renderer::gpu_tracer::GpuTracer;
use renderer::renderer::{CameraConfig, CubeBounds, HitInfo, Ray, create_camera_ray};
use renderer::scenes::create_octa_cube;

#[test]
fn test_coordinate_transformation_debug() {
    println!("\n=== Coordinate Transformation Debug ===\n");

    let cube = create_octa_cube();
    let gpu_tracer = GpuTracer::new(cube.clone());

    // Setup camera (same as render)
    let camera_pos = glam::Vec3::new(3.0, 2.0, 3.0);
    let target = glam::Vec3::ZERO;
    let up = glam::Vec3::Y;

    // Test a few specific pixels
    let test_pixels = vec![
        (128, 128, "center"),
        (64, 64, "upper-left"),
        (192, 192, "lower-right"),
        (128, 64, "top-center"),
        (128, 192, "bottom-center"),
    ];

    let width = 256u32;
    let height = 256u32;

    for (x, y, desc) in test_pixels {
        println!("--- Pixel ({}, {}) - {} ---", x, y, desc);

        // Create ray (same as cpu_tracer)
        let uv = glam::Vec2::new(
            (x as f32 - 0.5 * width as f32) / height as f32,
            -((y as f32 - 0.5 * height as f32) / height as f32),
        );

        let ray = create_camera_ray(uv, camera_pos, target, up);
        println!("Ray origin: {:?}", ray.origin);
        println!("Ray direction: {:?}", ray.direction);

        // Bounding box intersection
        let hit = gpu_tracer.raycast(ray.origin, ray.direction);

        if !hit.hit {
            println!("✗ No bounding box hit\n");
            continue;
        }

        println!("✓ Bounding box hit at: {:?}", hit.point);

        // Get cube bounds
        let bounds = CubeBounds::default();
        println!("Cube bounds: min={:?}, max={:?}", bounds.min, bounds.max);

        // Transform to normalized coordinates
        let normalized_pos = (hit.point - bounds.min) / (bounds.max - bounds.min);
        println!("Normalized position (before clamp): {:?}", normalized_pos);

        // Clamp
        const EPSILON: f32 = 0.001;
        let clamped_pos =
            normalized_pos.clamp(glam::Vec3::splat(EPSILON), glam::Vec3::splat(1.0 - EPSILON));
        println!("Clamped position: {:?}", clamped_pos);

        // Determine which octant this should be in
        let octant_x = if clamped_pos.x < 0.5 { 0 } else { 1 };
        let octant_y = if clamped_pos.y < 0.5 { 0 } else { 1 };
        let octant_z = if clamped_pos.z < 0.5 { 0 } else { 1 };
        let octant = (octant_x << 2) | (octant_y << 1) | octant_z;
        println!(
            "Expected octant: {} (x={}, y={}, z={})",
            octant, octant_x, octant_y, octant_z
        );

        // Check if octant is solid (0,1,2,4,5,6 are solid; 3,7 are empty)
        let octant_is_solid = ![3, 7].contains(&octant);
        println!("Octant is solid: {}", octant_is_solid);

        // Try raycast
        let is_empty = |v: &i32| *v == 0;
        let max_depth = 1;

        let cube_hit = cube.raycast(clamped_pos, ray.direction.normalize(), max_depth, &is_empty);

        match cube_hit {
            Some(hit) => {
                println!("✓ Octree raycast HIT");
                println!("  Hit position: {:?}", hit.position);
                println!("  Hit normal: {:?}", hit.normal);
                println!("  Hit value: {}", hit.value);
            }
            None => {
                println!("✗ Octree raycast MISS");
                if octant_is_solid {
                    println!("  ⚠ PROBLEM: Expected hit in solid octant!");
                } else {
                    println!("  (Correctly missed empty octant)");
                }
            }
        }

        println!();
    }
}

#[test]
fn test_boundary_surface_issue() {
    println!("\n=== Testing Boundary Surface Issue ===\n");
    println!("Hypothesis: Rays hitting bounding box surface need to be advanced slightly inside\n");

    let cube = create_octa_cube();
    let gpu_tracer = GpuTracer::new(cube.clone());

    // Create a ray that will hit the front face of the bounding box
    let ray_origin = glam::Vec3::new(0.0, 0.0, 5.0);
    let ray_dir = glam::Vec3::new(0.0, 0.0, -1.0).normalize();

    println!("Testing ray:");
    println!("  Origin: {:?}", ray_origin);
    println!("  Direction: {:?}", ray_dir);

    let hit = gpu_tracer.raycast(ray_origin, ray_dir);

    if !hit.hit {
        println!("✗ No bounding box hit (unexpected)\n");
        return;
    }

    println!("✓ Bounding box hit at: {:?}", hit.point);

    let bounds = CubeBounds::default();
    let normalized_pos = (hit.point - bounds.min) / (bounds.max - bounds.min);

    println!("\nNormalized position: {:?}", normalized_pos);
    println!("Expected: z should be ~1.0 (front face), x and y around 0.5");

    // Now try advancing the ray slightly into the cube
    const EPSILON: f32 = 0.001;
    let advanced_world_pos = hit.point + ray_dir * EPSILON;
    let advanced_norm_pos = (advanced_world_pos - bounds.min) / (bounds.max - bounds.min);

    println!("\nAdvanced position (world): {:?}", advanced_world_pos);
    println!("Advanced position (normalized): {:?}", advanced_norm_pos);

    // Clamp to valid range
    let clamped_pos =
        advanced_norm_pos.clamp(glam::Vec3::splat(EPSILON), glam::Vec3::splat(1.0 - EPSILON));

    println!("Clamped position: {:?}", clamped_pos);

    // Test raycast from both positions
    let is_empty = |v: &i32| *v == 0;
    let max_depth = 1;

    println!("\n--- Raycast from surface position ---");
    let hit1 = cube.raycast(
        normalized_pos.clamp(glam::Vec3::splat(EPSILON), glam::Vec3::splat(1.0 - EPSILON)),
        ray_dir,
        max_depth,
        &is_empty,
    );
    println!("Result: {}", if hit1.is_some() { "HIT" } else { "MISS" });

    println!("\n--- Raycast from advanced position ---");
    let hit2 = cube.raycast(clamped_pos, ray_dir, max_depth, &is_empty);
    println!("Result: {}", if hit2.is_some() { "HIT" } else { "MISS" });

    if hit1.is_none() && hit2.is_some() {
        println!("\n✓ SOLUTION FOUND: Advancing ray into cube fixes the miss!");
    } else if hit1.is_some() && hit2.is_some() {
        println!("\n? Both positions hit - need more investigation");
    } else {
        println!("\n✗ Neither position hits - different issue");
    }
}
