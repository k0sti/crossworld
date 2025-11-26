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

// BCF octree data (stored as 1D-like 2D texture)
uniform usampler2D u_octree_data;
uniform uint u_octree_data_size;  // Size of BCF data in bytes

uniform sampler2D u_material_palette; // Material palette (128 entries)
uniform bool u_disable_lighting; // If true, output pure material colors without lighting

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
    int value;
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
    float r = 0.0;
    if (r_bits == 1) r = 0.286;
    else if (r_bits == 2) r = 0.573;
    else if (r_bits == 3) r = 0.859;

    float g = 0.0;
    if (g_bits == 1) g = 0.141;
    else if (g_bits == 2) g = 0.286;
    else if (g_bits == 3) g = 0.427;
    else if (g_bits == 4) g = 0.573;
    else if (g_bits == 5) g = 0.714;
    else if (g_bits == 6) g = 0.859;
    else if (g_bits == 7) g = 1.0;

    float b = 0.0;
    if (b_bits == 1) b = 0.286;
    else if (b_bits == 2) b = 0.573;
    else if (b_bits == 3) b = 0.859;

    return vec3(r, g, b);
}

// Get material color from value
vec3 getMaterialColor(int value) {
    if (value < 0) {
        return vec3(0.0);
    }

    // Values 0-127: Use palette texture
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
uint readU8(uint offset) {
    if (offset >= u_octree_data_size) {
        return 0u;
    }
    // 1D-like 2D texture: width = data_size, height = 1
    // Use texelFetch for direct integer access
    return texelFetch(u_octree_data, ivec2(int(offset), 0), 0).r;
}

// Read multi-byte pointer (little-endian)
// ssss: size exponent (0=1 byte, 1=2 bytes, 2=4 bytes, 3=8 bytes)
uint readPointer(uint offset, uint ssss) {
    if (ssss == 0u) {
        return readU8(offset);
    } else if (ssss == 1u) {
        return readU8(offset) | (readU8(offset + 1u) << 8);
    } else if (ssss == 2u) {
        return readU8(offset)
             | (readU8(offset + 1u) << 8)
             | (readU8(offset + 2u) << 16)
             | (readU8(offset + 3u) << 24);
    }
    // 8-byte pointers not supported in WebGL
    return 0u;
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
uint parseBcfNode(uint offset, uint octant, out uint child_offset) {
    child_offset = 0u;

    if (offset >= u_octree_data_size) {
        return 0u;
    }

    uint type_byte = readU8(offset);
    uint msb, type_id, size_val;
    decodeTypeByte(type_byte, msb, type_id, size_val);

    // Inline leaf (0x00-0x7F): MSB = 0
    if (msb == 0u) {
        return type_byte & 0x7Fu;
    }

    // Extended leaf (0x80-0x8F): type_id = 0
    if (type_id == 0u) {
        return readU8(offset + 1u);
    }

    // Octa-with-leaves (0x90-0x9F): type_id = 1
    if (type_id == 1u) {
        return readU8(offset + 1u + octant);
    }

    // Octa-with-pointers (0xA0-0xAF): type_id = 2
    if (type_id == 2u) {
        uint ssss = size_val;
        uint ptr_offset = offset + 1u + (octant * (1u << ssss));
        child_offset = readPointer(ptr_offset, ssss);
        return 0u; // Not a leaf, follow pointer
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
// BCF Octree Traversal
// ============================================================================

// Raycast through BCF-encoded octree
// pos: ray position in cube-local space [-1, 1]³
// dir: normalized ray direction
HitInfo raycastBcfOctree(vec3 pos, vec3 dir) {
    HitInfo result;
    result.hit = false;
    result.t = 1e10;
    result.value = 0;

    // BCF header: magic (4 bytes) + version (4 bytes) + ssss (4 bytes)
    // Root node starts at offset 12
    const uint BCF_HEADER_SIZE = 12u;

    // Stack for traversal (depth, offset, bounds)
    // Max depth 8 for typical octrees
    const int MAX_DEPTH = 8;
    uint stack_offset[MAX_DEPTH];
    vec3 stack_min[MAX_DEPTH];
    vec3 stack_max[MAX_DEPTH];
    int stack_ptr = 0;

    // Initialize with root node
    stack_offset[0] = BCF_HEADER_SIZE;
    stack_min[0] = vec3(-1.0);
    stack_max[0] = vec3(1.0);

    // Max iterations to prevent infinite loops
    const int MAX_ITERATIONS = 256;
    int iter = 0;

    while (stack_ptr >= 0 && iter < MAX_ITERATIONS) {
        iter++;

        // Pop from stack
        uint node_offset = stack_offset[stack_ptr];
        vec3 box_min = stack_min[stack_ptr];
        vec3 box_max = stack_max[stack_ptr];
        stack_ptr--;

        // Check if ray intersects this box
        vec3 box_center = (box_min + box_max) * 0.5;
        vec3 box_size = (box_max - box_min) * 0.5;

        // Simple box intersection test
        vec3 inv_dir = 1.0 / dir;
        vec3 t_min = (box_min - pos) * inv_dir;
        vec3 t_max = (box_max - pos) * inv_dir;
        vec3 t1 = min(t_min, t_max);
        vec3 t2 = max(t_min, t_max);
        float t_near = max(max(t1.x, t1.y), t1.z);
        float t_far = min(min(t2.x, t2.y), t2.z);

        if (t_near > t_far || t_far < 0.0) {
            continue; // No intersection
        }

        // Calculate entry point and octant
        vec3 entry = pos + dir * max(t_near, 0.0);
        uint octant = getOctant(entry, box_center);

        // Parse BCF node
        uint child_offset;
        uint value = parseBcfNode(node_offset, octant, child_offset);

        if (value != 0u && child_offset == 0u) {
            // Hit a leaf with material value
            vec3 normal = calculateEntryNormal((entry - box_min) / (box_max - box_min), dir);
            result.hit = true;
            result.t = max(t_near, 0.0);
            result.point = entry;
            result.normal = normal;
            result.value = int(value);
            return result;
        } else if (child_offset != 0u) {
            // Branch node - push child octant bounds onto stack
            vec3 octant_min = box_min;
            vec3 octant_max = box_center;

            if ((octant & 1u) != 0u) { octant_min.x = box_center.x; octant_max.x = box_max.x; }
            if ((octant & 2u) != 0u) { octant_min.y = box_center.y; octant_max.y = box_max.y; }
            if ((octant & 4u) != 0u) { octant_min.z = box_center.z; octant_max.z = box_max.z; }

            // Push child onto stack
            if (stack_ptr + 1 < MAX_DEPTH) {
                stack_ptr++;
                stack_offset[stack_ptr] = child_offset;
                stack_min[stack_ptr] = octant_min;
                stack_max[stack_ptr] = octant_max;
            }
        }
        // If value == 0 and child_offset == 0, it's empty - continue to next stack item
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
        // Get material color from voxel value
        vec3 materialColor = getMaterialColor(octreeHit.value);

        // Apply lighting (or output pure color if disabled)
        if (u_disable_lighting) {
            color = materialColor;
        } else {
            // Lighting constants (match Rust constants)
            vec3 lightDir = normalize(vec3(0.431934, 0.863868, 0.259161));
            float ambient = 0.3;
            float diffuseStrength = 0.7;

            // Diffuse lighting
            float diffuse = max(dot(octreeHit.normal, lightDir), 0.0);

            // Combine lighting (simplified Lambert model)
            color = materialColor * (ambient + diffuse * diffuseStrength);
        }
    }

    // Gamma correction
    color = pow(color, vec3(1.0 / 2.2));

    FragColor = vec4(color, 1.0);
}
