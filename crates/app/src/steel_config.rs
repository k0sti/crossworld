//! Steel configuration module for application scene setup
//!
//! Provides Steel (embedded Scheme) scripting support for configuring
//! application scenes declaratively. This allows scene configuration
//! to be defined in `.scm` files that can be edited without recompilation.
//!
//! # Example Steel Configuration
//!
//! ```scheme
//! ;; Define camera
//! (define camera
//!   (make-camera
//!     (vec3 0 6 -3)    ; position
//!     (vec3 0 0 4)))   ; look-at target
//!
//! ;; Define ground cube
//! (define ground
//!   (make-ground-cube 32 3))  ; material=32, size_shift=3 (8 units)
//!
//! ;; Define scene objects
//! (define objects
//!   (list
//!     (make-object
//!       (vec3 0 6 0)           ; position
//!       (quat-euler 0.1 0.2 0.3))))  ; rotation
//! ```

use glam::{Quat, Vec3};
use steel::steel_vm::engine::Engine;
use steel::steel_vm::register_fn::RegisterFn;
use steel::SteelVal;
use std::path::Path;

/// Configuration for a 3D vector
#[derive(Debug, Clone, Copy, Default)]
pub struct Vec3Config {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3Config {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn to_vec3(&self) -> Vec3 {
        Vec3::new(self.x, self.y, self.z)
    }
}

impl From<Vec3Config> for Vec3 {
    fn from(config: Vec3Config) -> Self {
        config.to_vec3()
    }
}

/// Configuration for a quaternion rotation
#[derive(Debug, Clone, Copy)]
pub struct QuatConfig {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Default for QuatConfig {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 1.0,
        }
    }
}

impl QuatConfig {
    pub fn from_euler(x: f32, y: f32, z: f32) -> Self {
        let q = Quat::from_euler(glam::EulerRot::XYZ, x, y, z);
        Self {
            x: q.x,
            y: q.y,
            z: q.z,
            w: q.w,
        }
    }

    pub fn to_quat(&self) -> Quat {
        Quat::from_xyzw(self.x, self.y, self.z, self.w)
    }
}

impl From<QuatConfig> for Quat {
    fn from(config: QuatConfig) -> Self {
        config.to_quat()
    }
}

/// Ground type configuration
#[derive(Debug, Clone)]
pub enum GroundConfig {
    /// Solid cube with material and size_shift (edge = 2^size_shift)
    SolidCube {
        material: u8,
        size_shift: u32,
    },
    /// Simple cuboid with dimensions
    Cuboid {
        width: f32,
        height: f32,
        depth: f32,
    },
}

impl Default for GroundConfig {
    fn default() -> Self {
        GroundConfig::SolidCube {
            material: 32,
            size_shift: 3,
        }
    }
}

/// Object configuration in the scene
#[derive(Debug, Clone)]
pub struct ObjectConfig {
    pub position: Vec3Config,
    pub rotation: QuatConfig,
    pub scale: f32,
    pub material: u8,
}

impl Default for ObjectConfig {
    fn default() -> Self {
        Self {
            position: Vec3Config::default(),
            rotation: QuatConfig::default(),
            scale: 1.0,
            material: 224,
        }
    }
}

/// Camera configuration
#[derive(Debug, Clone)]
pub struct CameraConfig {
    pub position: Vec3Config,
    pub look_at: Vec3Config,
    pub up: Vec3Config,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            position: Vec3Config::new(0.0, 6.0, -3.0),
            look_at: Vec3Config::new(0.0, 0.0, 4.0),
            up: Vec3Config::new(0.0, 1.0, 0.0),
        }
    }
}

/// Complete scene configuration
#[derive(Debug, Clone, Default)]
pub struct SceneConfig {
    pub ground: GroundConfig,
    pub objects: Vec<ObjectConfig>,
    pub camera: CameraConfig,
}

/// Steel configuration engine wrapper
pub struct SteelConfig {
    engine: Engine,
}

// Helper functions for converting SteelVal to numeric types
// These handle both IntV and NumV variants for flexibility

fn steel_val_to_f64(val: &SteelVal) -> f64 {
    match val {
        SteelVal::NumV(n) => *n,
        SteelVal::IntV(n) => *n as f64,
        _ => 0.0,
    }
}

fn steel_val_to_isize(val: &SteelVal) -> isize {
    match val {
        SteelVal::IntV(n) => *n,
        SteelVal::NumV(n) => *n as isize,
        _ => 0,
    }
}

impl SteelConfig {
    /// Create a new Steel configuration engine with registered functions
    pub fn new() -> Self {
        let mut engine = Engine::new();

        // Register vec3 constructor - accepts any numeric type
        engine.register_fn("vec3", |x: SteelVal, y: SteelVal, z: SteelVal| -> SteelVal {
            let x_num = steel_val_to_f64(&x);
            let y_num = steel_val_to_f64(&y);
            let z_num = steel_val_to_f64(&z);
            let list = vec![
                SteelVal::NumV(x_num),
                SteelVal::NumV(y_num),
                SteelVal::NumV(z_num),
            ];
            SteelVal::ListV(list.into())
        });

        // Register quat-euler constructor (from euler angles in radians)
        engine.register_fn("quat-euler", |x: SteelVal, y: SteelVal, z: SteelVal| -> SteelVal {
            let x_f = steel_val_to_f64(&x) as f32;
            let y_f = steel_val_to_f64(&y) as f32;
            let z_f = steel_val_to_f64(&z) as f32;
            let q = Quat::from_euler(glam::EulerRot::XYZ, x_f, y_f, z_f);
            let list = vec![
                SteelVal::NumV(q.x as f64),
                SteelVal::NumV(q.y as f64),
                SteelVal::NumV(q.z as f64),
                SteelVal::NumV(q.w as f64),
            ];
            SteelVal::ListV(list.into())
        });

        // Register quat-identity
        engine.register_fn("quat-identity", || -> SteelVal {
            let list = vec![
                SteelVal::NumV(0.0),
                SteelVal::NumV(0.0),
                SteelVal::NumV(0.0),
                SteelVal::NumV(1.0),
            ];
            SteelVal::ListV(list.into())
        });

        // Register make-camera
        engine.register_fn("make-camera", |position: SteelVal, look_at: SteelVal| -> SteelVal {
            let list = vec![
                SteelVal::SymbolV("camera".into()),
                position,
                look_at,
            ];
            SteelVal::ListV(list.into())
        });

        // Register make-ground-cube (solid cube with material and size_shift)
        engine.register_fn("make-ground-cube", |material: SteelVal, size_shift: SteelVal| -> SteelVal {
            let list = vec![
                SteelVal::SymbolV("ground-cube".into()),
                SteelVal::IntV(steel_val_to_isize(&material)),
                SteelVal::IntV(steel_val_to_isize(&size_shift)),
            ];
            SteelVal::ListV(list.into())
        });

        // Register make-ground-cuboid (cuboid with dimensions)
        engine.register_fn("make-ground-cuboid", |width: SteelVal, height: SteelVal, depth: SteelVal| -> SteelVal {
            let list = vec![
                SteelVal::SymbolV("ground-cuboid".into()),
                SteelVal::NumV(steel_val_to_f64(&width)),
                SteelVal::NumV(steel_val_to_f64(&height)),
                SteelVal::NumV(steel_val_to_f64(&depth)),
            ];
            SteelVal::ListV(list.into())
        });

        // Register make-object
        engine.register_fn("make-object", |position: SteelVal, rotation: SteelVal| -> SteelVal {
            let list = vec![
                SteelVal::SymbolV("object".into()),
                position,
                rotation,
            ];
            SteelVal::ListV(list.into())
        });

        // Register make-object-with-material
        engine.register_fn("make-object-with-material", |position: SteelVal, rotation: SteelVal, material: SteelVal| -> SteelVal {
            let list = vec![
                SteelVal::SymbolV("object".into()),
                position,
                rotation,
                SteelVal::IntV(steel_val_to_isize(&material)),
            ];
            SteelVal::ListV(list.into())
        });

        // Register make-scene
        engine.register_fn("make-scene", |ground: SteelVal, objects: SteelVal, camera: SteelVal| -> SteelVal {
            let list = vec![
                SteelVal::SymbolV("scene".into()),
                ground,
                objects,
                camera,
            ];
            SteelVal::ListV(list.into())
        });

        Self { engine }
    }

    /// Load and evaluate a Steel configuration file
    pub fn load_file(&mut self, path: &Path) -> Result<(), String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file {:?}: {}", path, e))?;

        self.engine
            .run(content)
            .map_err(|e| format!("Failed to evaluate config: {}", e))?;

        Ok(())
    }

    /// Load configuration from a string
    pub fn load_string(&mut self, content: &str) -> Result<(), String> {
        self.engine
            .run(content.to_string())
            .map_err(|e| format!("Failed to evaluate config: {}", e))?;
        Ok(())
    }

    /// Extract a scene configuration from the engine
    pub fn extract_scene(&self, name: &str) -> Result<SceneConfig, String> {
        let value = self
            .engine
            .extract_value(name)
            .map_err(|e| format!("Failed to extract '{}': {}", name, e))?;

        parse_scene_config(&value)
    }

    /// Extract camera configuration
    pub fn extract_camera(&self, name: &str) -> Result<CameraConfig, String> {
        let value = self
            .engine
            .extract_value(name)
            .map_err(|e| format!("Failed to extract '{}': {}", name, e))?;

        parse_camera_config(&value)
    }

    /// Extract ground configuration
    pub fn extract_ground(&self, name: &str) -> Result<GroundConfig, String> {
        let value = self
            .engine
            .extract_value(name)
            .map_err(|e| format!("Failed to extract '{}': {}", name, e))?;

        parse_ground_config(&value)
    }

    /// Extract objects configuration
    pub fn extract_objects(&self, name: &str) -> Result<Vec<ObjectConfig>, String> {
        let value = self
            .engine
            .extract_value(name)
            .map_err(|e| format!("Failed to extract '{}': {}", name, e))?;

        parse_objects_config(&value)
    }
}

impl Default for SteelConfig {
    fn default() -> Self {
        Self::new()
    }
}

// Helper functions to parse Steel values into config structs

fn parse_vec3(val: &SteelVal) -> Result<Vec3Config, String> {
    match val {
        SteelVal::ListV(list) => {
            let items: Vec<_> = list.iter().collect();
            if items.len() != 3 {
                return Err(format!("vec3 requires 3 elements, got {}", items.len()));
            }
            let x = extract_f32(&items[0])?;
            let y = extract_f32(&items[1])?;
            let z = extract_f32(&items[2])?;
            Ok(Vec3Config::new(x, y, z))
        }
        _ => Err(format!("Expected list for vec3, got {:?}", val)),
    }
}

fn parse_quat(val: &SteelVal) -> Result<QuatConfig, String> {
    match val {
        SteelVal::ListV(list) => {
            let items: Vec<_> = list.iter().collect();
            if items.len() != 4 {
                return Err(format!("quat requires 4 elements, got {}", items.len()));
            }
            let x = extract_f32(&items[0])?;
            let y = extract_f32(&items[1])?;
            let z = extract_f32(&items[2])?;
            let w = extract_f32(&items[3])?;
            Ok(QuatConfig { x, y, z, w })
        }
        _ => Err(format!("Expected list for quat, got {:?}", val)),
    }
}

fn parse_camera_config(val: &SteelVal) -> Result<CameraConfig, String> {
    match val {
        SteelVal::ListV(list) => {
            let items: Vec<_> = list.iter().collect();
            if items.len() < 3 {
                return Err("camera requires at least 3 elements".to_string());
            }

            // First element should be 'camera symbol
            match &items[0] {
                SteelVal::SymbolV(s) if s.as_str() == "camera" => {}
                _ => return Err("Expected 'camera symbol".to_string()),
            }

            let position = parse_vec3(&items[1])?;
            let look_at = parse_vec3(&items[2])?;
            let up = if items.len() > 3 {
                parse_vec3(&items[3])?
            } else {
                Vec3Config::new(0.0, 1.0, 0.0)
            };

            Ok(CameraConfig {
                position,
                look_at,
                up,
            })
        }
        _ => Err(format!("Expected list for camera, got {:?}", val)),
    }
}

fn parse_ground_config(val: &SteelVal) -> Result<GroundConfig, String> {
    match val {
        SteelVal::ListV(list) => {
            let items: Vec<_> = list.iter().collect();
            if items.is_empty() {
                return Err("ground config is empty".to_string());
            }

            match &items[0] {
                SteelVal::SymbolV(s) if s.as_str() == "ground-cube" => {
                    if items.len() < 3 {
                        return Err("ground-cube requires material and size_shift".to_string());
                    }
                    let material = extract_u8(&items[1])?;
                    let size_shift = extract_u32(&items[2])?;
                    Ok(GroundConfig::SolidCube {
                        material,
                        size_shift,
                    })
                }
                SteelVal::SymbolV(s) if s.as_str() == "ground-cuboid" => {
                    if items.len() < 4 {
                        return Err("ground-cuboid requires width, height, depth".to_string());
                    }
                    let width = extract_f32(&items[1])?;
                    let height = extract_f32(&items[2])?;
                    let depth = extract_f32(&items[3])?;
                    Ok(GroundConfig::Cuboid {
                        width,
                        height,
                        depth,
                    })
                }
                _ => Err(format!("Unknown ground type: {:?}", items[0])),
            }
        }
        _ => Err(format!("Expected list for ground, got {:?}", val)),
    }
}

fn parse_object_config(val: &SteelVal) -> Result<ObjectConfig, String> {
    match val {
        SteelVal::ListV(list) => {
            let items: Vec<_> = list.iter().collect();
            if items.len() < 3 {
                return Err("object requires at least 3 elements".to_string());
            }

            // First element should be 'object symbol
            match &items[0] {
                SteelVal::SymbolV(s) if s.as_str() == "object" => {}
                _ => return Err("Expected 'object symbol".to_string()),
            }

            let position = parse_vec3(&items[1])?;
            let rotation = parse_quat(&items[2])?;
            let material = if items.len() > 3 {
                extract_u8(&items[3])?
            } else {
                224 // default material
            };

            Ok(ObjectConfig {
                position,
                rotation,
                scale: 1.0,
                material,
            })
        }
        _ => Err(format!("Expected list for object, got {:?}", val)),
    }
}

fn parse_objects_config(val: &SteelVal) -> Result<Vec<ObjectConfig>, String> {
    match val {
        SteelVal::ListV(list) => {
            list.iter()
                .map(|item| parse_object_config(item))
                .collect()
        }
        _ => Err(format!("Expected list for objects, got {:?}", val)),
    }
}

fn parse_scene_config(val: &SteelVal) -> Result<SceneConfig, String> {
    match val {
        SteelVal::ListV(list) => {
            let items: Vec<_> = list.iter().collect();
            if items.len() < 4 {
                return Err("scene requires 4 elements".to_string());
            }

            // First element should be 'scene symbol
            match &items[0] {
                SteelVal::SymbolV(s) if s.as_str() == "scene" => {}
                _ => return Err("Expected 'scene symbol".to_string()),
            }

            let ground = parse_ground_config(&items[1])?;
            let objects = parse_objects_config(&items[2])?;
            let camera = parse_camera_config(&items[3])?;

            Ok(SceneConfig {
                ground,
                objects,
                camera,
            })
        }
        _ => Err(format!("Expected list for scene, got {:?}", val)),
    }
}

fn extract_f32(val: &SteelVal) -> Result<f32, String> {
    match val {
        SteelVal::NumV(n) => Ok(*n as f32),
        SteelVal::IntV(n) => Ok(*n as f32),
        _ => Err(format!("Expected number, got {:?}", val)),
    }
}

fn extract_u8(val: &SteelVal) -> Result<u8, String> {
    match val {
        SteelVal::IntV(n) => {
            if *n >= 0 && *n <= 255 {
                Ok(*n as u8)
            } else {
                Err(format!("Value {} out of u8 range", n))
            }
        }
        SteelVal::NumV(n) => {
            let i = *n as i64;
            if i >= 0 && i <= 255 {
                Ok(i as u8)
            } else {
                Err(format!("Value {} out of u8 range", n))
            }
        }
        _ => Err(format!("Expected integer, got {:?}", val)),
    }
}

fn extract_u32(val: &SteelVal) -> Result<u32, String> {
    match val {
        SteelVal::IntV(n) => {
            if *n >= 0 {
                Ok(*n as u32)
            } else {
                Err(format!("Value {} cannot be negative for u32", n))
            }
        }
        SteelVal::NumV(n) => {
            let i = *n as i64;
            if i >= 0 {
                Ok(i as u32)
            } else {
                Err(format!("Value {} cannot be negative for u32", n))
            }
        }
        _ => Err(format!("Expected integer, got {:?}", val)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec3_creation() {
        let mut config = SteelConfig::new();
        config
            .load_string("(define test-vec (vec3 1.0 2.0 3.0))")
            .unwrap();

        let val = config.engine.extract_value("test-vec").unwrap();
        let vec = parse_vec3(&val).unwrap();
        assert_eq!(vec.x, 1.0);
        assert_eq!(vec.y, 2.0);
        assert_eq!(vec.z, 3.0);
    }

    #[test]
    fn test_quat_euler() {
        let mut config = SteelConfig::new();
        config
            .load_string("(define test-quat (quat-euler 0.0 0.0 0.0))")
            .unwrap();

        let val = config.engine.extract_value("test-quat").unwrap();
        let quat = parse_quat(&val).unwrap();
        // Identity quaternion
        assert!((quat.w - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_camera_config() {
        let mut config = SteelConfig::new();
        config
            .load_string(
                r#"
                (define test-camera
                  (make-camera
                    (vec3 0 6 -3)
                    (vec3 0 0 4)))
            "#,
            )
            .unwrap();

        let camera = config.extract_camera("test-camera").unwrap();
        assert_eq!(camera.position.x, 0.0);
        assert_eq!(camera.position.y, 6.0);
        assert_eq!(camera.position.z, -3.0);
        assert_eq!(camera.look_at.x, 0.0);
        assert_eq!(camera.look_at.y, 0.0);
        assert_eq!(camera.look_at.z, 4.0);
    }

    #[test]
    fn test_ground_cube_config() {
        let mut config = SteelConfig::new();
        config
            .load_string("(define test-ground (make-ground-cube 32 3))")
            .unwrap();

        let ground = config.extract_ground("test-ground").unwrap();
        match ground {
            GroundConfig::SolidCube {
                material,
                size_shift,
            } => {
                assert_eq!(material, 32);
                assert_eq!(size_shift, 3);
            }
            _ => panic!("Expected SolidCube"),
        }
    }

    #[test]
    fn test_ground_cuboid_config() {
        let mut config = SteelConfig::new();
        config
            .load_string("(define test-ground (make-ground-cuboid 8 8 8))")
            .unwrap();

        let ground = config.extract_ground("test-ground").unwrap();
        match ground {
            GroundConfig::Cuboid {
                width,
                height,
                depth,
            } => {
                assert_eq!(width, 8.0);
                assert_eq!(height, 8.0);
                assert_eq!(depth, 8.0);
            }
            _ => panic!("Expected Cuboid"),
        }
    }

    #[test]
    fn test_full_scene_config() {
        let mut config = SteelConfig::new();
        config
            .load_string(
                r#"
                (define test-scene
                  (make-scene
                    (make-ground-cube 32 3)
                    (list
                      (make-object
                        (vec3 0 6 0)
                        (quat-euler 0.1 0.2 0.3)))
                    (make-camera
                      (vec3 0 6 -3)
                      (vec3 0 0 4))))
            "#,
            )
            .unwrap();

        let scene = config.extract_scene("test-scene").unwrap();

        // Check ground
        match scene.ground {
            GroundConfig::SolidCube {
                material,
                size_shift,
            } => {
                assert_eq!(material, 32);
                assert_eq!(size_shift, 3);
            }
            _ => panic!("Expected SolidCube"),
        }

        // Check objects
        assert_eq!(scene.objects.len(), 1);
        assert_eq!(scene.objects[0].position.x, 0.0);
        assert_eq!(scene.objects[0].position.y, 6.0);
        assert_eq!(scene.objects[0].position.z, 0.0);

        // Check camera
        assert_eq!(scene.camera.position.y, 6.0);
        assert_eq!(scene.camera.look_at.z, 4.0);
    }
}
