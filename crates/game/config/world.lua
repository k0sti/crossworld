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
##  #####
######
]]

-- Material palette (match indices to cube materials)
materials = {
    empty = 0,      -- Air/empty
    bedrock = 1,    -- Solid bedrock
    grass = 32,     -- Green grass
    stone = 64,     -- Gray stone
}
