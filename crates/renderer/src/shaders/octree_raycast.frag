#version 300 es
precision highp float;
precision highp int;
precision highp sampler2D;
precision highp sampler3D;

out vec4 FragColor;

uniform vec2 u_resolution;
uniform float u_time;
uniform vec3 u_camera_pos;
uniform vec4 u_camera_rot;  // quaternion (x, y, z, w)
uniform bool u_use_camera;
uniform int u_max_depth;

// Octree data texture
// For now, we'll pass octree data as a 3D texture or buffer
// The implementation will use a simple encoding scheme
uniform sampler3D u_octree_texture;
uniform int u_octree_size;  // Size of octree at max depth (e.g., 8 for depth 3)
uniform sampler2D u_material_palette; // Material palette (128 entries)

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

// Get voxel value from octree texture
// For simplicity, use 3D texture lookup
int getVoxelValue(vec3 pos) {
    // Sample the 3D texture at the given position
    vec4 texel = texture(u_octree_texture, pos);
    // Assume value is encoded in red channel as normalized float
    return int(texel.r * 255.0);
}

// Calculate next integer boundary in direction of sign
vec3 nextIntegerBoundary(vec3 v, vec3 sign) {
    vec3 scaled = v * sign + vec3(1.0);
    return floor(scaled) * sign;
}

// Calculate the next position after stepping to next octant boundary
vec3 calculateNextPosition(vec3 pos2, vec3 dir, vec3 sign) {
    const float EPSILON = 1e-8;

    vec3 nextInteger = nextIntegerBoundary(pos2, sign);
    vec3 diff = nextInteger - pos2;

    // Avoid division by zero
    if (abs(diff.x) < EPSILON && abs(diff.y) < EPSILON && abs(diff.z) < EPSILON) {
        return pos2 / 2.0;
    }

    vec3 invTime = dir / diff;
    float maxInv = max(max(invTime.x, invTime.y), invTime.z);

    if (abs(maxInv) < EPSILON) {
        return pos2 / 2.0;
    }

    vec3 step = diff * (invTime / maxInv);
    vec3 nextPos = (pos2 + step) / 2.0;

    // Clamp to valid range
    return clamp(nextPos, vec3(0.0), vec3(1.0));
}

// Recursive octree raycast
// Returns HitInfo with voxel intersection
HitInfo raycastOctree(vec3 pos, vec3 dir, int currentDepth) {
    HitInfo result;
    result.hit = false;
    result.t = 1e10;
    result.value = 0;

    // Max iterations to prevent infinite loops
    const int MAX_ITERATIONS = 256;

    for (int iter = 0; iter < MAX_ITERATIONS; iter++) {
        // Validate position is in [0, 1]³
        if (any(lessThan(pos, vec3(0.0))) || any(greaterThan(pos, vec3(1.0)))) {
            break;
        }

        // At max depth, check voxel value
        if (currentDepth == 0) {
            int value = getVoxelValue(pos);
            if (value != 0) {
                // Hit non-empty voxel
                vec3 normal = calculateEntryNormal(pos, dir);
                result.hit = true;
                result.point = pos;
                result.normal = normal;
                result.value = value;
                return result;
            }
            // Empty voxel, step forward
            // For depth 0, we're at leaf level, so step to next boundary
            vec3 sign = sign(dir);
            vec3 pos2 = pos * 2.0;
            pos = calculateNextPosition(pos2, dir, sign);
            continue;
        }

        // Calculate which octant we're in
        vec3 pos2 = pos * 2.0;
        vec3 sign = sign(dir);

        // Calculate octant bit using floor and sign adjustment
        ivec3 signInt = ivec3(
            dir.x >= 0.0 ? 1 : -1,
            dir.y >= 0.0 ? 1 : -1,
            dir.z >= 0.0 ? 1 : -1
        );
        ivec3 sign10 = ivec3(
            dir.x >= 0.0 ? 0 : 1,
            dir.y >= 0.0 ? 0 : 1,
            dir.z >= 0.0 ? 0 : 1
        );

        ivec3 bit = ivec3(floor(pos2 * vec3(sign)));
        bit = bit * signInt + sign10;

        // Check octant validity
        if (any(lessThan(bit, ivec3(0))) || any(greaterThan(bit, ivec3(1)))) {
            break;
        }

        // Transform to child coordinate space
        vec3 childPos = (pos2 - vec3(bit)) / 2.0;

        // Sample octree at child level
        // This is a simplified version - actual implementation would need
        // hierarchical octree structure
        int value = getVoxelValue(childPos);

        if (value != 0) {
            // Hit solid voxel
            vec3 normal = calculateEntryNormal(childPos, dir);
            result.hit = true;
            result.point = pos;
            result.normal = normal;
            result.value = value;
            return result;
        }

        // Miss in this octant - step to next boundary
        pos = calculateNextPosition(pos2, dir, sign);
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

    // Cube bounds (world space)
    vec3 boxMin = vec3(-1.0, -1.0, -1.0);
    vec3 boxMax = vec3(1.0, 1.0, 1.0);

    // Intersect with bounding box first
    HitInfo boxHit = intersectBox(ray, boxMin, boxMax);

    // Background color (matches BACKGROUND_COLOR in Rust)
    vec3 color = vec3(0.4, 0.5, 0.6);

    if (boxHit.hit) {
        // Transform hit point to normalized [0,1]³ cube space
        vec3 normalizedPos = (boxHit.point - boxMin) / (boxMax - boxMin);

        // Move slightly inside the cube to avoid boundary issues
        const float EPSILON = 1e-6;
        normalizedPos = normalizedPos + ray.direction * EPSILON;
        normalizedPos = clamp(normalizedPos, vec3(0.0), vec3(1.0));

        // Raycast through octree
        HitInfo octreeHit = raycastOctree(normalizedPos, ray.direction, u_max_depth);

        if (octreeHit.hit) {
            // Transform hit position back to world space
            vec3 worldHitPoint = octreeHit.point * (boxMax - boxMin) + boxMin;

            // Get material color from voxel value
            vec3 materialColor = getMaterialColor(octreeHit.value);

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
