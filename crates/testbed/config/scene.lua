-- Testbed Scene Configuration
-- This file configures the physics testbed scene using Lua 5.4
--
-- The testbed now shows 4 viewports comparing different collider types:
--   1. Cuboid collider (top-left)
--   2. Mesh collider via VoxelColliderBuilder (top-right)
--   3. Terrain collider via VoxelTerrainCollider (bottom-left)
--   4. Empty/future implementation (bottom-right)

-- =============================================================================
-- Ground Configuration (shared by all 4 scenes)
-- =============================================================================

-- Solid cube ground with octree-style configuration
-- - material: 32 (green-ish color from palette)
-- - size_shift: 3 (2^3 = 8 units cube edge)
-- - center: (0, -4, 0) - positioned so top face is at Y=0
ground = ground_cube(32, 3, vec3(0, -4, 0))

-- Alternative: Load ground from CSM file (uncomment to use)
-- ground = ground_csm("terrain.csm", 3, vec3(0, -4, 0))

-- =============================================================================
-- Scene Objects
-- =============================================================================

-- Helper function to generate random cube parameters
function gen_random_cube()
    -- Generate position (-3 to 3 for X/Z, 2 to 10 for Y)
    local px = rand_range(-3.0, 3.0)
    local py = rand_range(2.0, 10.0)
    local pz = rand_range(-3.0, 3.0)

    -- Generate rotation quaternion components
    local rx = rand_range(-0.5, 0.5)
    local ry = rand_range(-0.5, 0.5)
    local rz = rand_range(-0.5, 0.5)

    -- Compute w to normalize quaternion (w = sqrt(1 - x^2 - y^2 - z^2))
    local rw_sq = 1.0 - (rx * rx + ry * ry + rz * rz)
    local rw = rw_sq > 0.0 and math.sqrt(rw_sq) or 0.0

    -- Generate size (0.2 to 0.6 for each component)
    local sx = rand_range(0.2, 0.6)
    local sy = rand_range(0.2, 0.6)
    local sz = rand_range(0.2, 0.6)

    -- Generate mass (0.5 to 2.0)
    local mass = rand_range(0.5, 2.0)

    -- Generate material (64 to 224)
    local material = math.floor(rand_range(64.0, 224.0))

    return object(
        vec3(px, py, pz),
        quat(rx, ry, rz, rw),
        vec3(sx, sy, sz),
        mass,
        material
    )
end

-- Generate N random cubes
function generate_cubes(n)
    local cubes = {}
    for i = 1, n do
        table.insert(cubes, gen_random_cube())
    end
    return cubes
end

-- Initialize random seed and generate 10 random cubes
rand_seed(42)
scene_objects = generate_cubes(10)

-- =============================================================================
-- Camera Configuration
-- =============================================================================

-- Camera setup for observing all 4 scenes
-- - Position: (15, 12, 15) - elevated view for wider viewports
-- - Look-at: (0, 0, 0) - looking at ground center
scene_camera = camera(
    vec3(15, 12, 15),
    vec3(0, 0, 0)
)
