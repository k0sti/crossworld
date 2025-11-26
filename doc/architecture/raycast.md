# Raycast System Design

### Types
```rust
pub enum Cube<T> {
    Solid(T),
    Cubes(Box<[Rc<Cube<T>>; 8]>),
}
pub struct CubeCoord {
    pub pos: IVec3,
    pub depth: u32,
}
pub struct Hit<T> {
    pub coord: CubeCoord,
    pub value: T,
    pub normal_axis: Axis,
    /// Exact hit position in world space
    pub pos: Vec3,
}
pub struct RaycastDebugState {
    pub entry_count: u32,
    /// limit to exit algorithm
    pub max_entries: u32,
    /// traversed cubes during raycst
    pub path: Vec<CubeCoord>,
}
```
### Coordinate Systems

#### World Space
- Ray origin and direction in world coordinates
- Root raycast cube in [-1, 1]³ in world space

#### Normalized Cube Space [-1, 1]³
- Working in this space during raycast algorithm

#### Octree Coordinate Space
- `CubeCoord { pos: IVec3, depth: u32 }`
- Position encoded as Morton code / octant path
- Depth = 0 is root, increasing depth = smaller voxels

## Raycast

Raycast into [-1, 1]³ Cube structure

### Input
- ray_origin: Vec3
- ray_dir: Vec3
- `Option<DebugState>`
### Output
- `Option<Hit>`

## Raycast Algorithm

```rust
function raycast(
    ray_origin: Vec3,
    ray_dir: Vec3,
    cube_coord: CubeCoord,
    debug: Option<DebugState>,
)
```
- if ray_origin is outside Cube AABB `[-1,1]`
	- Raycast to AAB. if miss return None
- now ray position should be inside or on face of a cube
- start with CubeCoord(0, 0)
- if ray_dir close to axis dir, call raycast_axis
- Call raycast_recursive

The octree raycast uses a recursive DDA (Digital Differential Analyzer) approach.

- sign(x) = signum with -1 if x<0, 1 otherwise

```rust
function raycast_recursive(
    ray_origin: Vec3,
    ray_dir: Vec3,
    normal: Axis,
    cube_coord: CubeCoord,
    debug: Option<DebugState>,
) {
    debug.entry_count += 1;

    match cube in cube_coord:
        Cube::Solid(value):
            debug.path += coord
            if value != 0:  return Hit(coord, pos, normal)
            else: return None  # Empty voxel
        Cube::Cubes:
            let dir_sign = sign(ray_dir)
            let octant = (1 + sign(ray_origin).to_vec_i) / 2
            let mut axis = axis
            let mut ray_origin = ray_origin
            loop {
                octant_idx = octant.dot(1,2,4)

                // convert ray_origin to child coord
                let offset = octant.as_vec3() * 2.0 - 1.0;
                let p2 = ray_origin * 2.0 - offset;
                
                let c2 = CubeCoord(cube_coord.pos * 2 +octant, cube_coord.depth+1)
                let child = 
                let hit = raycast_recursive(p2, ray_dir, axis, c2, debug)
                if (hit) return hit

                // calculate shortest time axis
                // unify tests to position 0 and 1 into single tests: moving positive position one cell back so testing towards 0 TODO: is this logic clear?
                let pos = if sign(ray_origin*dir_sign)>=0 {
                    // towards edge, flip to other side so should be towards 0
                    pos -= dir_sign
                } else ray_origin
                let dist = abs(pos)
                let time = dist / dir
                // step vector to next child
                axis = min_axis(time)
                let step: -axis.to_vec_i
                octant += step
                if (out-of-bound) return None
            }
}

function raycast_axis(
    ray_origin: Vec3,
    ray_dir: Axis,
    cube_coord: CubeCoord,
    debug: Option<DebugState>,
) {
    sign = ray_dir.signum()
    octant = (1 + sign) / 2

    ...
}

```
