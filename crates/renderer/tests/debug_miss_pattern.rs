//! Test to identify the pattern of octree raycast misses

use renderer::gpu_tracer::GpuTracer;
use renderer::renderer::{CubeBounds, create_camera_ray};
use renderer::scenes::create_octa_cube;

#[test]
fn test_identify_miss_pattern() {
    println!("\n=== Identifying Octree Miss Pattern ===\n");

    let cube = create_octa_cube();
    let gpu_tracer = GpuTracer::new(cube.clone());

    // Setup camera
    let camera_pos = glam::Vec3::new(3.0, 2.0, 3.0);
    let target = glam::Vec3::ZERO;
    let up = glam::Vec3::Y;

    let width = 64u32;
    let height = 64u32;

    let mut bbox_hits = 0;
    let mut octree_hits = 0;
    let mut octree_misses = 0;

    let mut miss_samples = Vec::new();

    // Sample grid
    for y in 0..height {
        for x in 0..width {
            let uv = glam::Vec2::new(
                (x as f32 - 0.5 * width as f32) / height as f32,
                -((y as f32 - 0.5 * height as f32) / height as f32),
            );

            let ray = create_camera_ray(uv, camera_pos, target, up);
            let hit = gpu_tracer.raycast(ray.origin, ray.direction);

            if !hit.hit {
                continue;
            }

            bbox_hits += 1;

            // Transform to normalized space
            let bounds = CubeBounds::default();
            const SURFACE_EPSILON: f32 = 0.01;
            let advanced_hit_point = hit.point + ray.direction * SURFACE_EPSILON;
            let mut normalized_pos = (advanced_hit_point - bounds.min) / (bounds.max - bounds.min);

            const EPSILON: f32 = 0.001;
            normalized_pos =
                normalized_pos.clamp(glam::Vec3::splat(EPSILON), glam::Vec3::splat(1.0 - EPSILON));

            // Try raycast
            let is_empty = |v: &i32| *v == 0;
            let max_depth = 1;

            let cube_hit = cube.raycast(
                normalized_pos,
                ray.direction.normalize(),
                max_depth,
                &is_empty,
            );

            if cube_hit.is_some() {
                octree_hits += 1;
            } else {
                octree_misses += 1;

                // Collect first 10 miss samples
                if miss_samples.len() < 10 {
                    let octant_x = if normalized_pos.x < 0.5 { 0 } else { 1 };
                    let octant_y = if normalized_pos.y < 0.5 { 0 } else { 1 };
                    let octant_z = if normalized_pos.z < 0.5 { 0 } else { 1 };
                    let octant = (octant_x << 2) | (octant_y << 1) | octant_z;
                    let octant_is_empty = [3, 7].contains(&octant);

                    miss_samples.push((
                        x,
                        y,
                        normalized_pos,
                        ray.direction.normalize(),
                        octant,
                        octant_is_empty,
                    ));
                }
            }
        }
    }

    println!("=== Statistics ===");
    println!("Bounding box hits: {}", bbox_hits);
    println!(
        "Octree hits: {} ({:.1}%)",
        octree_hits,
        (octree_hits as f32 / bbox_hits as f32) * 100.0
    );
    println!(
        "Octree misses: {} ({:.1}%)",
        octree_misses,
        (octree_misses as f32 / bbox_hits as f32) * 100.0
    );

    println!("\n=== First 10 Miss Samples ===");
    for (x, y, pos, dir, octant, is_empty) in miss_samples {
        println!("Pixel ({}, {}): pos={:?}, dir={:?}", x, y, pos, dir);
        println!("  Octant: {} (empty={})", octant, is_empty);
        if !is_empty {
            println!("  ⚠ MISS in SOLID octant - this is the problem!");
        } else {
            println!("  (Correctly missed empty octant)");
        }
    }
}
