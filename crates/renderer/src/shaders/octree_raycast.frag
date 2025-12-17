#version 300 es
precision highp float;
precision highp int;
precision highp sampler2D;
precision highp usampler2D;

out vec4 FragColor;

uniform vec2 u_resolution;
uniform float u_time;
uniform vec3 u_camera_pos;
uniform vec4 u_camera_rot;  // quaternion (x, y, z, w)
uniform bool u_use_camera;
uniform int u_max_depth;

// BCF octree data (stored as 2D texture for large data support)
uniform usampler2D u_octree_data;
uniform uint u_octree_data_size;  // Size of BCF data in bytes
uniform uint u_octree_texture_width;  // Width of texture (for 2D coordinate conversion)

uniform sampler2D u_material_palette; // Material palette (128 entries)
uniform bool u_disable_lighting; // If true, output pure material colors without lighting
uniform bool u_show_errors; // If true, show error colors; if false, background color

// ============================================================================
// Error Material Values (1-7 are reserved for error conditions)
// ============================================================================
// Material value 0 = empty space (no hit)
// Material values 1-7 = error conditions with animated checkered visualization
// Material values 8+ = normal voxel materials

// Get base color for error material values (1-7)
vec3 getErrorMaterialColor(int material_value) {
    if (material_value == 1) return vec3(1.0, 0.0, 0.3);      // Hot pink - Generic error
    if (material_value == 2) return vec3(1.0, 0.2, 0.0);      // Red-orange - Bounds/pointer errors
    if (material_value == 3) return vec3(1.0, 0.6, 0.0);      // Orange - Type validation errors
    if (material_value == 4) return vec3(0.0, 0.8, 1.0);      // Sky blue - Stack/iteration errors
    if (material_value == 5) return vec3(0.6, 0.0, 1.0);      // Purple - Octant errors
    if (material_value == 6) return vec3(0.0, 1.0, 0.5);      // Spring green - Data truncation
    if (material_value == 7) return vec3(1.0, 1.0, 0.0);      // Yellow - Unknown/other errors
    return vec3(1.0, 0.0, 1.0); // Bright magenta fallback
}

// Apply animated checkered pattern to error materials
vec3 applyErrorAnimation(vec3 base_color, vec2 pixel_coord, float time) {
    // Create checkered pattern (8x8 pixel grid)
    ivec2 grid_pos = ivec2(pixel_coord) / 8;

    // Animate: oscillate between light and dark with period of 2 seconds
    float anim = sin(time * 3.14159) * 0.5 + 0.5; // 0.0 to 1.0

    // Determine if this grid cell should be light or dark
    bool is_light_cell = ((grid_pos.x + grid_pos.y) % 2) == 0;

    // Animate which cells are light: flip pattern every half cycle
    bool flip_pattern = fract(time * 0.5) > 0.5;
    if (flip_pattern) {
        is_light_cell = !is_light_cell;
    }

    // Apply brightness: light cells get brightened, dark cells get darkened
    float brightness = is_light_cell ? mix(0.7, 1.0, anim) : mix(0.3, 0.7, anim);

    return base_color * brightness;
}

// Ray structure
struct Ray {
    vec3 origin;
    vec3 direction;
};

// Hit information
struct HitInfo {
    bool hit;
    float t;
    vec3 point;
    vec3 normal;
    int value;  // Material value (1-7 = error conditions, 8+ = normal materials)
};

// Rotate vector by quaternion
vec3 quat_rotate(vec4 q, vec3 v) {
    vec3 qv = q.xyz;
    vec3 uv = cross(qv, v);
    vec3 uuv = cross(qv, uv);
    return v + 2.0 * (uv * q.w + uuv);
}

// Ray-box intersection
HitInfo intersectBox(Ray ray, vec3 boxMin, vec3 boxMax) {
    HitInfo hitInfo;
    hitInfo.hit = false;
    hitInfo.t = 1e10;
    hitInfo.value = 0;

    vec3 invDir = 1.0 / ray.direction;
    vec3 tMin = (boxMin - ray.origin) * invDir;
    vec3 tMax = (boxMax - ray.origin) * invDir;

    vec3 t1 = min(tMin, tMax);
    vec3 t2 = max(tMin, tMax);

    float tNear = max(max(t1.x, t1.y), t1.z);
    float tFar = min(min(t2.x, t2.y), t2.z);

    if (tNear > tFar || tFar < 0.0) {
        return hitInfo;
    }

    hitInfo.hit = true;
    hitInfo.t = tNear > 0.0 ? tNear : tFar;
    hitInfo.point = ray.origin + ray.direction * hitInfo.t;

    // Calculate normal
    vec3 center = (boxMin + boxMax) * 0.5;
    vec3 localPoint = hitInfo.point - center;
    vec3 size = (boxMax - boxMin) * 0.5;
    vec3 d = abs(localPoint / size);

    float maxComponent = max(max(d.x, d.y), d.z);
    if (abs(maxComponent - d.x) < 0.0001) {
        hitInfo.normal = vec3(sign(localPoint.x), 0.0, 0.0);
    } else if (abs(maxComponent - d.y) < 0.0001) {
        hitInfo.normal = vec3(0.0, sign(localPoint.y), 0.0);
    } else {
        hitInfo.normal = vec3(0.0, 0.0, sign(localPoint.z));
    }

    return hitInfo;
}

// Decode R2G3B2 color encoding to RGB
// Encoding: (r << 5) | (g << 2) | b
// where index = value - 128
vec3 decodeR2G3B2(int value) {
    int bits = value - 128;
    int r_bits = (bits >> 5) & 3;
    int g_bits = (bits >> 2) & 7;
    int b_bits = bits & 3;

    // Convert to normalized RGB values
    // Using same mapping as Rust implementation
    // 2-bit expansion for R and B: value * 255 / 3, then normalize
    float r = 0.0;
    if (r_bits == 1) r = 0.333;  // 0x55/255 = 85/255
    else if (r_bits == 2) r = 0.667;  // 0xAA/255 = 170/255
    else if (r_bits == 3) r = 1.0;    // 0xFF/255 = 255/255

    // 3-bit expansion for G: value * 255 / 7, then normalize
    float g = 0.0;
    if (g_bits == 1) g = 0.141;  // 0x24/255 ≈ 36/255
    else if (g_bits == 2) g = 0.286;  // 0x49/255 ≈ 73/255
    else if (g_bits == 3) g = 0.427;  // 0x6D/255 ≈ 109/255
    else if (g_bits == 4) g = 0.573;  // 0x92/255 ≈ 146/255
    else if (g_bits == 5) g = 0.714;  // 0xB6/255 ≈ 182/255
    else if (g_bits == 6) g = 0.859;  // 0xDB/255 ≈ 219/255
    else if (g_bits == 7) g = 1.0;    // 0xFF/255 = 255/255

    // 2-bit expansion for B (same as R)
    float b = 0.0;
    if (b_bits == 1) b = 0.333;  // 0x55/255 = 85/255
    else if (b_bits == 2) b = 0.667;  // 0xAA/255 = 170/255
    else if (b_bits == 3) b = 1.0;    // 0xFF/255 = 255/255

    return vec3(r, g, b);
}

// Get material color from value (time-dependent for error materials)
vec3 getMaterialColor(int value, vec2 pixel_coord, float time) {
    if (value < 0) {
        return vec3(0.0);
    }

    // Values 1-7: Error materials with animated checkered pattern
    if (value >= 1 && value <= 7) {
        vec3 base_color = getErrorMaterialColor(value);
        return applyErrorAnimation(base_color, pixel_coord, time);
    }

    // Values 0, 8-127: Use palette texture
    if (value < 128) {
        // Sample center of texel
        float u = (float(value) + 0.5) / 128.0;
        return texture(u_material_palette, vec2(u, 0.5)).rgb;
    }

    // Values 128-255: R2G3B2 encoded colors
    if (value <= 255) {
        return decodeR2G3B2(value);
    }

    return vec3(0.0);
}

// ============================================================================
// BCF (Binary Cube Format) Reading Functions
// ============================================================================

// Read a single byte from BCF data at given offset (bounds checked)
// Returns material error value (2) if out of bounds, otherwise 0
uint readU8(uint offset, inout uint error_material) {
    if (offset >= u_octree_data_size) {
        error_material = 2u; // Material 2 = bounds/pointer errors
        return 0u;
    }
    // Convert 1D offset to 2D texture coordinates
    // For large data, texture is stored as width x height 2D texture
    int x = int(offset % u_octree_texture_width);
    int y = int(offset / u_octree_texture_width);
    // Use texelFetch for direct integer access
    return texelFetch(u_octree_data, ivec2(x, y), 0).r;
}

// Read multi-byte pointer (little-endian)
// ssss: size exponent (0=1 byte, 1=2 bytes, 2=4 bytes, 3=8 bytes)
uint readPointer(uint offset, uint ssss, inout uint error_material) {
    uint ptr = 0u;
    if (ssss == 0u) {
        ptr = readU8(offset, error_material);
    } else if (ssss == 1u) {
        ptr = readU8(offset, error_material) | (readU8(offset + 1u, error_material) << 8);
        if (error_material == 2u) {
            error_material = 6u; // Material 6 = data truncation
        }
    } else if (ssss == 2u) {
        ptr = readU8(offset, error_material)
             | (readU8(offset + 1u, error_material) << 8)
             | (readU8(offset + 2u, error_material) << 16)
             | (readU8(offset + 3u, error_material) << 24);
        if (error_material == 2u) {
            error_material = 6u; // Material 6 = data truncation
        }
    } else {
        // 8-byte pointers not supported in WebGL
        return 0u;
    }

    // Validate pointer points within bounds
    if (error_material == 0u && ptr >= u_octree_data_size) {
        error_material = 2u; // Material 2 = invalid pointer
        return 0u;
    }

    return ptr;
}

// Decode BCF type byte into its components
// Type byte format: [MSB][Type ID (3 bits)][Size/Value (4 bits)]
void decodeTypeByte(uint type_byte, out uint msb, out uint type_id, out uint size_val) {
    msb = (type_byte >> 7) & 1u;
    type_id = (type_byte >> 4) & 7u;
    size_val = type_byte & 15u;
}

// Calculate octant index from position relative to center
// Octant bits: bit 0 = x >= center.x, bit 1 = y >= center.y, bit 2 = z >= center.z
uint getOctant(vec3 pos, vec3 center) {
    uint octant = 0u;
    if (pos.x >= center.x) octant |= 1u;
    if (pos.y >= center.y) octant |= 2u;
    if (pos.z >= center.z) octant |= 4u;
    return octant;
}

// Parse BCF node at offset and return material value or child pointer
// If returns non-zero value and child_offset == 0: leaf node with material value
// If returns 0 and child_offset != 0: branch node, follow pointer
// If both return 0: error or empty node
// error_material: 0 = no error, 1-7 = error material value
uint parseBcfNode(uint offset, uint octant, out uint child_offset, inout uint error_material) {
    child_offset = 0u;

    uint type_byte = readU8(offset, error_material);
    if (error_material != 0u) {
        return error_material; // Return error material value
    }

    uint msb, type_id, size_val;
    decodeTypeByte(type_byte, msb, type_id, size_val);

    // Inline leaf (0x00-0x7F): MSB = 0
    if (msb == 0u) {
        return type_byte & 0x7Fu;
    }

    // Extended leaf (0x80-0x8F): type_id = 0
    if (type_id == 0u) {
        uint value = readU8(offset + 1u, error_material);
        if (error_material != 0u) return error_material;
        return value;
    }

    // Octa-with-leaves (0x90-0x9F): type_id = 1
    if (type_id == 1u) {
        // Validate octant
        if (octant > 7u) {
            error_material = 5u; // Material 5 = octant errors
            return error_material;
        }
        uint value = readU8(offset + 1u + octant, error_material);
        if (error_material != 0u) return error_material;
        return value;
    }

    // Octa-with-pointers (0xA0-0xAF): type_id = 2
    if (type_id == 2u) {
        // Validate octant
        if (octant > 7u) {
            error_material = 5u; // Material 5 = octant errors
            return error_material;
        }
        uint ssss = size_val;
        uint ptr_offset = offset + 1u + (octant * (1u << ssss));
        child_offset = readPointer(ptr_offset, ssss, error_material);
        if (error_material != 0u) return error_material;
        return 0u; // Not a leaf, follow pointer
    }

    // Invalid type ID (types 3-7 undefined)
    if (type_id >= 3u && type_id <= 7u) {
        error_material = 3u; // Material 3 = type validation errors
        return error_material;
    }

    // Unknown type or error
    return 0u;
}

// Calculate surface normal from entry point
// The normal points towards the direction the ray came from
vec3 calculateEntryNormal(vec3 pos, vec3 dir) {
    const float EPSILON = 1e-6;

    vec3 distToMin = pos;
    vec3 distToMax = vec3(1.0) - pos;

    float minDist = min(min(distToMin.x, distToMin.y), distToMin.z);
    float maxDist = min(min(distToMax.x, distToMax.y), distToMax.z);

    if (minDist < maxDist) {
        // Entered from min face (0, 0, 0)
        if (abs(distToMin.x - minDist) < EPSILON) {
            return vec3(-1.0, 0.0, 0.0);
        } else if (abs(distToMin.y - minDist) < EPSILON) {
            return vec3(0.0, -1.0, 0.0);
        } else {
            return vec3(0.0, 0.0, -1.0);
        }
    } else {
        // Entered from max face (1, 1, 1)
        if (abs(distToMax.x - maxDist) < EPSILON) {
            return vec3(1.0, 0.0, 0.0);
        } else if (abs(distToMax.y - maxDist) < EPSILON) {
            return vec3(0.0, 1.0, 0.0);
        } else {
            return vec3(0.0, 0.0, 1.0);
        }
    }
}

// ============================================================================
// BCF Octree Traversal (based on working CPU raycast algorithm)
// ============================================================================

// Helper: sign function (returns -1 or 1)
vec3 sign3(vec3 v) {
    return mix(vec3(-1.0), vec3(1.0), greaterThanEqual(v, vec3(0.0)));
}

// Compute starting octant; at boundary (pos=0), use ray direction
ivec3 computeOctant(vec3 pos, vec3 dir_sign) {
    // Use component-wise logical operations for vectors
    bvec3 is_greater = greaterThan(pos, vec3(0.0));
    bvec3 is_equal = equal(pos, vec3(0.0));
    bvec3 dir_positive = greaterThan(dir_sign, vec3(0.0));

    // Combine: positive if pos > 0, or if pos == 0 and dir_sign > 0
    bvec3 positive = bvec3(
        is_greater.x || (is_equal.x && dir_positive.x),
        is_greater.y || (is_equal.y && dir_positive.y),
        is_greater.z || (is_equal.z && dir_positive.z)
    );
    return ivec3(positive);
}

// Convert octant ivec3 to index 0-7
uint octantToIndex(ivec3 o) {
    return uint(o.x + o.y * 2 + o.z * 4);
}

// Find axis with minimum time, return axis index (0=x, 1=y, 2=z)
int minTimeAxis(vec3 t) {
    if (t.x <= t.y && t.x <= t.z) return 0;
    if (t.y <= t.z) return 1;
    return 2;
}

// Simplified axis-aligned raycast (matches CPU raycast_axis)
// Only traverses along one axis, avoiding floating-point precision issues
HitInfo raycastBcfOctreeAxisAligned(vec3 pos, vec3 dir) {
    HitInfo result;
    result.hit = false;
    result.t = 1e10;
    result.value = 0;

    const uint BCF_HEADER_SIZE = 12u;

    // Find the dominant axis (non-zero component)
    vec3 abs_dir = abs(dir);
    int axis_idx;
    if (abs_dir.x >= abs_dir.y && abs_dir.x >= abs_dir.z) {
        axis_idx = 0;
    } else if (abs_dir.y >= abs_dir.z) {
        axis_idx = 1;
    } else {
        axis_idx = 2;
    }

    vec3 dir_sign = sign3(dir);
    int axis_sign = int(dir_sign[axis_idx]);

    // Find entry point if outside [-1,1]³
    vec3 ray_origin = pos;
    vec3 entry_normal = vec3(0.0);
    entry_normal[axis_idx] = -float(axis_sign); // Surface normal (outward from entry face)

    if (max(abs(pos.x), max(abs(pos.y), abs(pos.z))) > 1.0) {
        vec3 t_entry = (-dir_sign - pos) / dir;
        vec3 t_exit = (dir_sign - pos) / dir;
        float t_enter = max(max(t_entry.x, t_entry.y), t_entry.z);
        float t_leave = min(min(t_exit.x, t_exit.y), t_exit.z);

        if (t_enter > t_leave || t_leave < 0.0) {
            return result;
        }
        ray_origin = pos + dir * max(t_enter, 0.0);
    }

    // Stack for traversal
    const int MAX_STACK = 16;
    uint stack_offset[MAX_STACK];
    vec3 stack_min[MAX_STACK];
    vec3 stack_max[MAX_STACK];
    vec3 stack_ray_pos[MAX_STACK];
    int stack_ptr = 0;

    // Push root
    stack_offset[0] = BCF_HEADER_SIZE;
    stack_min[0] = vec3(-1.0);
    stack_max[0] = vec3(1.0);
    stack_ray_pos[0] = ray_origin;

    const int MAX_ITERATIONS = 512;
    int iter = 0;

    while (stack_ptr >= 0 && iter < MAX_ITERATIONS) {
        iter++;

        uint node_offset = stack_offset[stack_ptr];
        vec3 box_min = stack_min[stack_ptr];
        vec3 box_max = stack_max[stack_ptr];
        vec3 ray_pos = stack_ray_pos[stack_ptr];
        stack_ptr--;

        // Axis-aligned traversal: only step along one axis
        const int MAX_OCTANT_STEPS = 8;
        for (int octant_step = 0; octant_step < MAX_OCTANT_STEPS; octant_step++) {
            vec3 box_size = box_max - box_min;
            vec3 box_center = (box_min + box_max) * 0.5;
            vec3 local_pos = (ray_pos - box_center) / (box_size * 0.5);

            // Compute octant
            ivec3 octant = computeOctant(local_pos, dir_sign);
            uint octant_idx = octantToIndex(octant);

            // Parse node
            uint child_offset;
            uint error_material = 0u;
            uint value = parseBcfNode(node_offset, octant_idx, child_offset, error_material);

            // Check for error (materials 1-7)
            if (error_material != 0u) {
                result.hit = true;
                result.point = ray_pos;
                result.normal = entry_normal;
                result.value = int(error_material);
                return result;
            }

            if (value != 0u && child_offset == 0u) {
                result.hit = true;
                result.point = ray_pos;
                // For axis-aligned rays, normal is always opposite to ray direction
                // This matches CPU raycast_axis: normal = ray_axis.flip()
                result.normal = entry_normal;
                result.value = int(value);
                return result;
            }

            if (child_offset != 0u) {
                // Recurse into child
                vec3 offset_vec = vec3(octant) * 2.0 - 1.0;
                vec3 child_min = box_center + offset_vec * box_size * 0.25;
                vec3 child_max = child_min + box_size * 0.5;

                if (stack_ptr + 1 >= MAX_STACK) {
                    result.hit = true;
                    result.point = ray_pos;
                    result.normal = entry_normal;
                    result.value = 4; // Material 4 = stack/iteration errors
                    return result;
                }
                stack_ptr++;
                stack_offset[stack_ptr] = child_offset;
                stack_min[stack_ptr] = child_min;
                stack_max[stack_ptr] = child_max;
                stack_ray_pos[stack_ptr] = ray_pos;
                break;
            }

            // Step to next octant along axis
            octant[axis_idx] += axis_sign;

            // Check if exited
            if (octant[axis_idx] < 0 || octant[axis_idx] > 1) {
                break;
            }

            // Compute new ray position at octant boundary
            float boundary = float(octant[axis_idx]) - float(axis_sign + 1) * 0.5;
            ray_pos[axis_idx] = box_center[axis_idx] + boundary * box_size[axis_idx] * 0.5;
        }
    }

    // Iteration timeout - treat as error hit
    if (iter >= MAX_ITERATIONS) {
        result.hit = true;
        result.value = 4; // Material 4 = stack/iteration errors
    }

    return result;
}

// BCF Node Type Constants (matching BCF specification)
const uint NODE_TYPE_INLINE_LEAF = 0u;
const uint NODE_TYPE_EXTENDED_LEAF = 1u;
const uint NODE_TYPE_OCTA_LEAVES = 2u;
const uint NODE_TYPE_OCTA_POINTERS = 3u;

// Stack depth and iteration limits (matching CPU implementation)
const int MAX_STACK_DEPTH = 16;
const int MAX_ITERATIONS = 256;

// Read BCF node type and extract values/pointers
// Returns node type and fills out value or children arrays
void readBcfNode(uint offset, out uint node_type, out uint value, out uint values[8], out uint pointers[8], out uint ssss, inout uint error_material) {
    uint type_byte = readU8(offset, error_material);
    if (error_material != 0u) {
        node_type = NODE_TYPE_INLINE_LEAF;
        value = error_material;
        return;
    }

    // Inline leaf (0x00-0x7F)
    if (type_byte <= 0x7Fu) {
        node_type = NODE_TYPE_INLINE_LEAF;
        value = type_byte & 0x7Fu;
        return;
    }

    uint msb_type = (type_byte >> 4u) & 0x3u;

    // Extended leaf (0x80-0x8F)
    if (msb_type == 0u) {
        node_type = NODE_TYPE_EXTENDED_LEAF;
        value = readU8(offset + 1u, error_material);
        if (error_material != 0u) value = error_material;
        return;
    }

    // Octa-leaves (0x90-0x9F)
    if (msb_type == 1u) {
        node_type = NODE_TYPE_OCTA_LEAVES;
        for (int i = 0; i < 8; i++) {
            values[i] = readU8(offset + 1u + uint(i), error_material);
            if (error_material != 0u) {
                values[i] = error_material;
                return;
            }
        }
        return;
    }

    // Octa-pointers (0xA0-0xAF)
    if (msb_type == 2u) {
        node_type = NODE_TYPE_OCTA_POINTERS;
        ssss = type_byte & 0x0Fu;
        uint ptr_offset = offset + 1u;
        uint ptr_size = 1u << ssss;
        for (int i = 0; i < 8; i++) {
            pointers[i] = readPointer(ptr_offset, ssss, error_material);
            if (error_material != 0u) {
                pointers[i] = 0u;
                return;
            }
            ptr_offset += ptr_size;
        }
        return;
    }

    // Invalid type
    node_type = NODE_TYPE_INLINE_LEAF;
    value = 3u; // Error material 3 (type validation error)
    error_material = 3u;
}

// Encode axis as integer (for normal tracking)
// 0 = no axis, 1-3 = +X,+Y,+Z, 4-6 = -X,-Y,-Z
int encodeAxis(vec3 normal) {
    if (length(normal) < 0.1) return 0;
    if (normal.x > 0.9) return 1;   // +X
    if (normal.y > 0.9) return 2;   // +Y
    if (normal.z > 0.9) return 3;   // +Z
    if (normal.x < -0.9) return 4;  // -X
    if (normal.y < -0.9) return 5;  // -Y
    if (normal.z < -0.9) return 6;  // -Z
    return 0;
}

// Decode axis integer to vec3 normal
vec3 decodeAxis(int axis) {
    if (axis == 1) return vec3(1.0, 0.0, 0.0);   // +X
    if (axis == 2) return vec3(0.0, 1.0, 0.0);   // +Y
    if (axis == 3) return vec3(0.0, 0.0, 1.0);   // +Z
    if (axis == 4) return vec3(-1.0, 0.0, 0.0);  // -X
    if (axis == 5) return vec3(0.0, -1.0, 0.0);  // -Y
    if (axis == 6) return vec3(0.0, 0.0, -1.0);  // -Z
    return vec3(0.0, 1.0, 0.0); // Default
}

// Flip axis (for converting exit to entry normal)
int flipAxis(int axis) {
    if (axis >= 1 && axis <= 3) return axis + 3; // +XYZ -> -XYZ
    if (axis >= 4 && axis <= 6) return axis - 3; // -XYZ -> +XYZ
    return axis;
}

// Raycast through BCF-encoded octree using correct [-1,1]³ normalized space algorithm
// This implementation matches bcf_raycast.rs lines 184-379
// pos: ray position in cube-local space [-1, 1]³
// dir: normalized ray direction
HitInfo raycastBcfOctree(vec3 pos, vec3 dir) {
    HitInfo result;
    result.hit = false;
    result.t = 1e10;
    result.value = 0;

    const uint BCF_HEADER_SIZE = 12u;

    // Check for axis-aligned ray (use specialized traversal)
    vec3 abs_dir = abs(dir);
    float max_comp = max(abs_dir.x, max(abs_dir.y, abs_dir.z));
    vec3 near_zero = step(abs_dir, vec3(max_comp * 1e-6));
    float near_zero_count = near_zero.x + near_zero.y + near_zero.z;
    if (near_zero_count >= 2.0) {
        return raycastBcfOctreeAxisAligned(pos, dir);
    }

    // Compute direction signs
    vec3 dir_sign = sign3(dir);

    // Find entry point if outside [-1,1]³
    vec3 ray_origin = pos;
    int entry_normal_axis = 0;

    if (max(abs(pos.x), max(abs(pos.y), abs(pos.z))) > 1.0) {
        vec3 t_entry = (-dir_sign - pos) / dir;
        vec3 t_exit = (dir_sign - pos) / dir;
        float t_enter = max(max(t_entry.x, t_entry.y), t_entry.z);
        float t_leave = min(min(t_exit.x, t_exit.y), t_exit.z);

        if (t_enter > t_leave || t_leave < 0.0) {
            return result; // Ray misses cube
        }

        ray_origin = pos + dir * max(t_enter, 0.0);

        // Calculate entry normal (surface normal pointing outward from hit face)
        vec3 entry_point = ray_origin;
        vec3 abs_entry = abs(entry_point);
        float max_entry = max(abs_entry.x, max(abs_entry.y, abs_entry.z));
        const float EPSILON = 0.001;
        if (abs(abs_entry.x - max_entry) < EPSILON) {
            entry_normal_axis = entry_point.x > 0.0 ? 1 : 4; // +X or -X (outward)
        } else if (abs(abs_entry.y - max_entry) < EPSILON) {
            entry_normal_axis = entry_point.y > 0.0 ? 2 : 5; // +Y or -Y (outward)
        } else {
            entry_normal_axis = entry_point.z > 0.0 ? 3 : 6; // +Z or -Z (outward)
        }
    }

    // Stack arrays for traversal state (NO bounds arrays!)
    // Each node lives in its own [-1,1]³ normalized space
    uint stack_offset[MAX_STACK_DEPTH];
    vec3 stack_local_origin[MAX_STACK_DEPTH];
    vec3 stack_ray_dir[MAX_STACK_DEPTH];
    int stack_normal[MAX_STACK_DEPTH];
    // Note: coord not needed for rendering, only for debugging
    int stack_ptr = 0;

    // Initialize stack with root node (bcf_raycast_impl lines 202-212)
    stack_offset[0] = BCF_HEADER_SIZE;
    stack_local_origin[0] = ray_origin;
    stack_ray_dir[0] = dir;
    stack_normal[0] = entry_normal_axis;
    stack_ptr = 1;

    // Main traversal loop (bcf_raycast_impl lines 215-376)
    int iter = 0;
    while (stack_ptr > 0 && iter < MAX_ITERATIONS) {
        iter++;

        // Pop state from stack (lines 217-218)
        stack_ptr--;
        uint node_offset = stack_offset[stack_ptr];
        vec3 local_origin = stack_local_origin[stack_ptr];
        vec3 ray_dir = stack_ray_dir[stack_ptr];
        int normal_axis = stack_normal[stack_ptr];

        // Read BCF node at current offset (lines 220-224)
        uint node_type;
        uint leaf_value = 0u;
        uint leaf_values[8];
        uint child_pointers[8];
        uint ssss = 0u;
        uint error_material = 0u;

        readBcfNode(node_offset, node_type, leaf_value, leaf_values, child_pointers, ssss, error_material);

        if (error_material != 0u) {
            result.hit = true;
            result.point = local_origin;
            result.normal = decodeAxis(normal_axis);
            result.value = int(error_material);
            return result;
        }

        // Handle inline/extended leaf (lines 227-238)
        if (node_type == NODE_TYPE_INLINE_LEAF || node_type == NODE_TYPE_EXTENDED_LEAF) {
            if (leaf_value != 0u) {
                // Hit non-empty voxel
                result.hit = true;
                result.point = local_origin;
                result.normal = decodeAxis(normal_axis);
                result.value = int(leaf_value);
                return result;
            }
            // Empty voxel, continue to next stack item
            continue;
        }

        // Handle octa-leaves (lines 240-290)
        if (node_type == NODE_TYPE_OCTA_LEAVES) {
            ivec3 octant = computeOctant(local_origin, dir_sign);
            vec3 current_origin = local_origin;
            int current_normal = normal_axis;

            // DDA loop through octants
            for (int step = 0; step < 8; step++) {
                // Check bounds
                if (octant.x < 0 || octant.x > 1 || octant.y < 0 || octant.y > 1 || octant.z < 0 || octant.z > 1) {
                    break;
                }

                uint oct_idx = octantToIndex(octant);
                uint value = leaf_values[oct_idx];

                if (value != 0u) {
                    // Hit non-empty voxel
                    result.hit = true;
                    result.point = current_origin;
                    result.normal = decodeAxis(current_normal);
                    result.value = int(value);
                    return result;
                }

                // DDA step to next octant (lines 264-289)
                bvec3 far_side = greaterThanEqual(current_origin * dir_sign, vec3(0.0));
                vec3 adjusted = mix(current_origin, current_origin - dir_sign, far_side);
                vec3 dist = abs(adjusted);
                vec3 time = dist / abs(ray_dir);

                int exit_axis_idx = minTimeAxis(time);
                float exit_time = time[exit_axis_idx];

                // Advance ray
                current_origin += ray_dir * exit_time;

                // Step octant
                int exit_sign = int(dir_sign[exit_axis_idx]);
                octant[exit_axis_idx] += exit_sign;

                // Snap to boundary
                float boundary = float(octant[exit_axis_idx]) - float(exit_sign + 1) * 0.5;
                current_origin[exit_axis_idx] = boundary;

                // Update entry normal (opposite of exit direction)
                if (exit_axis_idx == 0) {
                    current_normal = exit_sign > 0 ? 4 : 1; // -X or +X
                } else if (exit_axis_idx == 1) {
                    current_normal = exit_sign > 0 ? 5 : 2; // -Y or +Y
                } else {
                    current_normal = exit_sign > 0 ? 6 : 3; // -Z or +Z
                }
            }
            // Exited all octants, continue to next stack item
            continue;
        }

        // Handle octa-pointers (lines 293-373)
        if (node_type == NODE_TYPE_OCTA_POINTERS) {
            ivec3 octant = computeOctant(local_origin, dir_sign);
            vec3 current_origin = local_origin;
            int current_normal = normal_axis;

            // Collect children to visit in DDA order
            struct ChildState {
                uint offset;
                vec3 origin;
                int normal;
            };
            ChildState children_to_visit[8];
            int children_count = 0;

            // DDA loop to collect children (lines 304-349)
            for (int step = 0; step < 8; step++) {
                // Check bounds
                if (octant.x < 0 || octant.x > 1 || octant.y < 0 || octant.y > 1 || octant.z < 0 || octant.z > 1) {
                    break;
                }

                uint oct_idx = octantToIndex(octant);
                uint child_offset = child_pointers[oct_idx];

                if (child_offset > 0u) {
                    // Non-empty child - transform ray to child's [-1,1]³ space (line 311)
                    vec3 offset_vec = vec3(octant) * 2.0 - 1.0;
                    vec3 child_origin = current_origin * 2.0 - offset_vec;

                    // Record child for later processing
                    children_to_visit[children_count].offset = child_offset;
                    children_to_visit[children_count].origin = child_origin;
                    children_to_visit[children_count].normal = current_normal;
                    children_count++;
                }

                // DDA step to next octant (lines 323-343)
                bvec3 far_side = greaterThanEqual(current_origin * dir_sign, vec3(0.0));
                vec3 adjusted = mix(current_origin, current_origin - dir_sign, far_side);
                vec3 dist = abs(adjusted);
                vec3 time = dist / abs(ray_dir);

                int exit_axis_idx = minTimeAxis(time);
                float exit_time = time[exit_axis_idx];

                // Advance ray
                current_origin += ray_dir * exit_time;

                // Step octant
                int exit_sign = int(dir_sign[exit_axis_idx]);
                octant[exit_axis_idx] += exit_sign;

                // Snap to boundary
                float boundary = float(octant[exit_axis_idx]) - float(exit_sign + 1) * 0.5;
                current_origin[exit_axis_idx] = boundary;

                // Update entry normal (opposite of exit direction)
                if (exit_axis_idx == 0) {
                    current_normal = exit_sign > 0 ? 4 : 1; // -X or +X
                } else if (exit_axis_idx == 1) {
                    current_normal = exit_sign > 0 ? 5 : 2; // -Y or +Y
                } else {
                    current_normal = exit_sign > 0 ? 6 : 3; // -Z or +Z
                }
            }

            // Push all collected children to stack in REVERSE order (lines 351-373)
            // This ensures they pop in DDA order (front-to-back)
            for (int i = children_count - 1; i >= 0; i--) {
                if (stack_ptr >= MAX_STACK_DEPTH) {
                    // Stack overflow
                    result.hit = true;
                    result.point = local_origin;
                    result.normal = decodeAxis(normal_axis);
                    result.value = 4; // Error material 4 (stack errors)
                    return result;
                }

                stack_offset[stack_ptr] = children_to_visit[i].offset;
                stack_local_origin[stack_ptr] = children_to_visit[i].origin;
                stack_ray_dir[stack_ptr] = ray_dir; // Same direction
                stack_normal[stack_ptr] = children_to_visit[i].normal;
                stack_ptr++;
            }
            continue;
        }
    }

    // Iteration timeout
    if (iter >= MAX_ITERATIONS) {
        result.hit = true;
        result.value = 4; // Error material 4 (iteration timeout)
    }

    return result;
}

void main() {
    // Normalized pixel coordinates
    vec2 uv = (gl_FragCoord.xy - 0.5 * u_resolution) / u_resolution.y;

    // Camera setup
    vec3 cameraPos;
    vec3 forward, right, camUp;

    if (u_use_camera) {
        // Use explicit camera configuration
        cameraPos = u_camera_pos;

        // Get camera basis vectors from quaternion
        forward = quat_rotate(u_camera_rot, vec3(0.0, 0.0, -1.0));
        right = quat_rotate(u_camera_rot, vec3(1.0, 0.0, 0.0));
        camUp = quat_rotate(u_camera_rot, vec3(0.0, 1.0, 0.0));
    } else {
        // Use time-based orbiting camera
        cameraPos = vec3(3.0 * cos(u_time * 0.3), 2.0, 3.0 * sin(u_time * 0.3));
        vec3 target = vec3(0.0, 0.0, 0.0);
        vec3 up = vec3(0.0, 1.0, 0.0);

        forward = normalize(target - cameraPos);
        right = normalize(cross(forward, up));
        camUp = cross(right, forward);
    }

    // Create ray
    Ray ray;
    ray.origin = cameraPos;
    ray.direction = normalize(forward + uv.x * right + uv.y * camUp);

    // Background color (matches BACKGROUND_COLOR in Rust)
    vec3 color = vec3(0.4, 0.5, 0.6);

    // Raycast through BCF-encoded octree
    // The octree is in world space [-1, 1]³
    HitInfo octreeHit = raycastBcfOctree(ray.origin, ray.direction);

    if (octreeHit.hit) {
        // Get material color from voxel value (materials 1-7 are animated error materials)
        vec3 materialColor = getMaterialColor(octreeHit.value, gl_FragCoord.xy, u_time);

        // Apply lighting (or output pure color if disabled)
        // Skip lighting for error materials (1-7) to show them clearly
        bool is_error_material = (octreeHit.value >= 1 && octreeHit.value <= 7);
        if (u_disable_lighting || is_error_material) {
            color = materialColor;
        } else {
            // Lighting constants (match Rust constants exactly)
            // LIGHT_DIR is already normalized in CPU version
            vec3 lightDir = vec3(0.431934, 0.863868, 0.259161);
            float ambient = 0.3;
            float diffuseStrength = 0.7;

            // Diffuse lighting using Lambert's cosine law
            // Matches CPU: hit.normal.dot(LIGHT_DIR).max(0.0)
            float diffuse = max(dot(octreeHit.normal, lightDir), 0.0);

            // Combine lighting (matches CPU formula)
            // CPU: material_color * (AMBIENT + diffuse * DIFFUSE_STRENGTH)
            color = materialColor * (ambient + diffuse * diffuseStrength);
        }
    }

    // Gamma correction
    color = pow(color, vec3(1.0 / 2.2));

    FragColor = vec4(color, 1.0);
}
