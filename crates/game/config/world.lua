-- Game World Configuration
-- This file configures the world generation parameters using Lua

-- =============================================================================
-- World Generation Configuration
-- =============================================================================

-- World cube configuration
-- Macro depth: procedurally generated terrain (0-3)
-- Micro depth: user edits (4-7)
-- Border depth: transition/blending layers
world_config = {
    macro_depth = 3,
    micro_depth = 5,
    border_depth = 1,
    seed = 12345,
}

-- =============================================================================
-- 2D Map Configuration
-- =============================================================================

-- Character mapping for 2D map
-- These define what each character in the map represents
map_chars = {
    [' '] = { mat = "empty" },      -- Empty space
    ['#'] = { mat = "bedrock" },    -- Bedrock walls
    ['^'] = { mat = "empty", spawn = true },  -- Spawn point
}

-- 2D map layout (will be inserted into ground cube centered on XZ plane, y=0)
map_layout = [[
#####
   ###
#   ^#
##  ###
######
]]

-- Material palette (match indices to cube materials)
materials = {
    empty = 0,      -- Air/empty
    bedrock = 1,    -- Solid bedrock
    grass = 32,     -- Green grass
    stone = 64,     -- Gray stone
}

-- =============================================================================
-- World Model Configuration
-- =============================================================================

-- Pseudorandom number generator (simple LCG)
local function prng(seed)
    local a = 1664525
    local c = 1013904223
    local m = 2^32
    return function()
        seed = (a * seed + c) % m
        return seed / m
    end
end

-- Generate models pseudorandomly in a specific area
local function generate_models(config)
    local models = {}
    local rng = prng(config.seed or 42)

    for i = 1, config.count do
        -- Generate pseudorandom position within bounds
        local x = (rng() * 2 - 1) * config.radius_x
        local z = (rng() * 2 - 1) * config.radius_z

        table.insert(models, {
            pattern = config.pattern,
            index = i - 1,  -- Use sequential index for model selection
            align = config.align or vec3(0.5, 0, 0.5),
            position = vec3(x, config.y or 0, z),
        })
    end

    return models
end

-- Model generation configuration
local model_config = {
    pattern = "scene_*",        -- Model name pattern
    count = 10,                 -- Number of models to generate
    radius_x = 50,              -- X radius of spawn area
    radius_z = 50,              -- Z radius of spawn area
    y = 0,                      -- Y position (ground level)
    align = vec3(0.5, 0, 0.5),  -- Bottom-center alignment
    seed = world_config.seed,   -- Use world seed for reproducibility
}

-- Generate models
world_models = generate_models(model_config)

