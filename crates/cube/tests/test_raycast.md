# Sparse Voxel Octree (SVO) Raycast Interface Tests

This document defines the test environment and test cases for a recursive SVO raycaster interface.

## Test Environment Configuration

### 1. Global Coordinate System
*   **World Space:** The Root Octree Node is an Axis-Aligned Bounding Box (AABB) encompassing the **Unit Cube**.
*   **Bounds:** Min $(0.0, 0.0, 0.0)$, Max $(1.0, 1.0, 1.0)$.
*   **Up Axis:** $+Y$
*   **Forward Axis:** $+Z$ (Right-handed or Left-handed depends on engine, assumed $+Z$ is depth for voxel indexing).

### 2. The "Test Cube" Structure
The root node is subdivided once (Depth 1), creating an $2 \times 2 \times 2$ grid. The input array provided is mapped to children indices $[0..7]$.

**Input Array:** `[1, 2, 0, 0, 3, 4, 5]`

**Child Node Mapping:**
Assuming standard linear indexing `index = x + (y * 2) + (z * 4)` or Morton code equivalent for $2 \times 2 \times 2$:

| Index | Local Grid (x, y, z) | World Bounds (Min - Max) | Value (`u8`) | State |
| :--- | :--- | :--- | :--- | :--- |
| **0** | $(0, 0, 0)$ | $(0.0, 0.0, 0.0) - (0.5, 0.5, 0.5)$ | **1** | **Solid** |
| **1** | $(1, 0, 0)$ | $(0.5, 0.0, 0.0) - (1.0, 0.5, 0.5)$ | **2** | **Solid** |
| **2** | $(0, 1, 0)$ | $(0.0, 0.5, 0.0) - (0.5, 1.0, 0.5)$ | **0** | **Empty** |
| **3** | $(1, 1, 0)$ | $(0.5, 0.5, 0.0) - (1.0, 1.0, 0.5)$ | **0** | **Empty** |
| **4** | $(0, 0, 1)$ | $(0.0, 0.0, 0.5) - (0.5, 0.5, 1.0)$ | **3** | **Solid** |
| **5** | $(1, 0, 1)$ | $(0.5, 0.0, 0.5) - (1.0, 0.5, 1.0)$ | **4** | **Solid** |
| **6** | $(0, 1, 1)$ | $(0.0, 0.5, 0.5) - (0.5, 1.0, 1.0)$ | **5** | **Solid** |
| **7** | $(1, 1, 1)$ | $(0.5, 0.5, 0.5) - (1.0, 1.0, 1.0)$ | **0** | **Empty*** |

*\*Implicitly 0 as input only provided 7 items.*

---

## Test Cases

### Table Legend
*   **Dir:** Normalized direction vector.
*   **Hit:** Boolean (True/False).
*   **CubeCoord:** Represented as `(x, y, z) @ Depth`.
*   **Normal:** The normal vector of the face hit.
*   **Visits:** Estimated entry count. Minimum 1 (Root). If Root is hit and subdivided, +1 for checking specific child.
*   **$\epsilon$:** Represents machine epsilon (rounding tolerance).

| ID | Description | Ray Origin $(x,y,z)$ | Ray Direction $(x,y,z)$ | Hit? | CubeCoord `Ivec3` @ `int` | Voxel `u8` | Normal $(x,y,z)$ | Hit Position $(x,y,z)$ | Visits (Est) |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **1** | **Direct Front Hit**<br>Hits node 0 (bottom-left-front). | $(0.25, 0.25, -1.0)$ | $(0, 0, 1)$ | ✅ | $(0, 0, 0)$ @ 1 | **1** | $(0, 0, -1)$ | $(0.25, 0.25, 0.0)$ | 2 |
| **2** | **Direct Front Hit (Right)**<br>Hits node 1 (bottom-right-front). | $(0.75, 0.25, -1.0)$ | $(0, 0, 1)$ | ✅ | $(1, 0, 0)$ @ 1 | **2** | $(0, 0, -1)$ | $(0.75, 0.25, 0.0)$ | 2 |
| **3** | **Pass-through to Back**<br>Enters empty node 2, hits node 6 behind it. | $(0.25, 0.75, -1.0)$ | $(0, 0, 1)$ | ✅ | $(0, 1, 1)$ @ 1 | **5** | $(0, 0, -1)$ | $(0.25, 0.75, 0.5)$ | 3 |
| **4** | **Double Pass-through**<br>Enters node 3 (empty), exits node 7 (empty). No hit. | $(0.75, 0.75, -1.0)$ | $(0, 0, 1)$ | ❌ | `null` | 0 | `null` | `null` | 3 |
| **5** | **Side Hit**<br>Hits node 1 from the right side ($+X$). | $(2.0, 0.25, 0.25)$ | $(-1, 0, 0)$ | ✅ | $(1, 0, 0)$ @ 1 | **2** | $(1, 0, 0)$ | $(1.0, 0.25, 0.25)$ | 2 |
| **6** | **Top Hit (Diagonal)**<br>Hits top of node 5. | $(0.75, 2.0, 0.75)$ | $(0, -1, 0)$ | ✅ | $(1, 0, 1)$ @ 1 | **4** | $(0, 1, 0)$ | $(0.75, 0.5, 0.75)$ | 2 |
| **7** | **Inside Hit**<br>Origin is inside solid node 4. | $(0.25, 0.25, 0.75)$ | $(0, 0, 1)$ | ✅ | $(0, 0, 1)$ @ 1 | **3** | *Ref/Impl Dependent*<br>*(Usually -Dir or 0)* | $(0.25, 0.25, 0.75)$ | 1-2 |
| **8** | **Total Miss (Bounds)**<br>Ray never intersects root cube. | $(-1.0, 0.5, 0.5)$ | $(0, 0, 1)$ | ❌ | `null` | 0 | `null` | `null` | 1 |
| **9** | **Angled Pass-through**<br>Enters via Node 2 (empty), hits internal face of Node 5. | $(0.1, 0.6, 0.1)$ | $(1, -1, 1)$<br>*(normalized)* | ✅ | $(1, 0, 1)$ @ 1 | **4** | $(-1, 0, 0)$ | $(0.5, 0.2, 0.5)$ | ~3 |

---

## Data Format Description

The data structures used in the interface are defined as follows:

### 1. `CubeCoord` Pair (Hit Location)
Defines the Integer Coordinate within the Octree grid at a specific depth level.
*   **Type:** `Struct` or `Pair`
*   **Format:** `{ Ivec3 position, int depth }`
*   **Details:**
    *   `depth`: The level of the octree where the hit occurred. For the "Test Cube", the root is depth 0, and the children (input values) are **depth 1**.
    *   `position`: Integer vector $(x, y, z)$. At depth 1, values are in range $[0, 1]$.
    *   *Example:* A hit at the top-back-left child returns `pos: (0, 1, 1), depth: 1`.

### 2. `Hit` Structure (Return Value)
*   **Format:**
    ```cpp
    struct Hit {
        CubeCoord voxel_id;   // The coordinate and depth identifier
        uint8_t   value;      // The voxel data (0-255)
        int       entry_count;// Debug counter for recursion steps
        Vec3      normal;     // Surface normal of the hit face
        Vec3      hit_pos;    // World space position of intersection
    };
    ```

### 3. Input Arguments
1.  **`ray_origin`**: `Vec3` (Floating point). The starting position of the ray in World Space.
2.  **`ray_direction`**: `Vec3` (Floating point). Must be a **Normalized** (Unit length) vector indicating direction.

### 4. Recursive Entry Count Logic
The `entry_count` helps profile optimization (DDA vs naive recursion).
*   **Logic:** Increments every time the ray checks a generic AABB node against the ray.
*   **Expectation:**
    *   **1:** Ray misses the Root Cube entirely.
    *   **2:** Ray hits Root Cube $\to$ checks specific Child $\to$ Hits Child immediately.
    *   **>2:** Ray hits Root $\to$ checks Child A (Empty) $\to$ steps to Child B $\to$ etc.

    