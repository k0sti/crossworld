//! Testbed-specific Steel configuration for scene setup
//!
//! Extends the base Steel configuration from `app` with physics and scene types.

use app::steel_config::{
    extract_f32, extract_u32, extract_u8, parse_quat, parse_vec3, steel_val_to_f64,
    steel_val_to_isize, QuatConfig, SteelConfig, Vec3Config,
};
// Re-export from app's steel re-exports
use app::steel_config::steel::{steel_vm::register_fn::RegisterFn, SteelVal};
use std::path::Path;

/// Ground type configuration
#[derive(Debug, Clone)]
pub enum GroundConfig {
    /// Solid cube with material and size_shift (edge = 2^size_shift)
    SolidCube { material: u8, size_shift: u32 },
    /// Simple cuboid with dimensions
    Cuboid { width: f32, height: f32, depth: f32 },
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

/// Testbed Steel configuration with scene-specific functions
pub struct TestbedConfig {
    config: SteelConfig,
}

impl TestbedConfig {
    /// Create a new testbed configuration engine
    ///
    /// Registers base types from `app::steel_config` plus:
    /// - `make-camera` - Camera configuration
    /// - `make-ground-cube` - Solid cube ground
    /// - `make-ground-cuboid` - Cuboid ground
    /// - `make-object` - Scene object
    /// - `make-scene` - Complete scene
    pub fn new() -> Self {
        let mut config = SteelConfig::new();

        // Register testbed-specific functions
        let engine = config.engine_mut();

        // Register make-camera
        engine.register_fn(
            "make-camera",
            |position: SteelVal, look_at: SteelVal| -> SteelVal {
                let list = vec![
                    SteelVal::SymbolV("camera".into()),
                    position,
                    look_at,
                ];
                SteelVal::ListV(list.into())
            },
        );

        // Register make-ground-cube (solid cube with material and size_shift)
        engine.register_fn(
            "make-ground-cube",
            |material: SteelVal, size_shift: SteelVal| -> SteelVal {
                let list = vec![
                    SteelVal::SymbolV("ground-cube".into()),
                    SteelVal::IntV(steel_val_to_isize(&material)),
                    SteelVal::IntV(steel_val_to_isize(&size_shift)),
                ];
                SteelVal::ListV(list.into())
            },
        );

        // Register make-ground-cuboid (cuboid with dimensions)
        engine.register_fn(
            "make-ground-cuboid",
            |width: SteelVal, height: SteelVal, depth: SteelVal| -> SteelVal {
                let list = vec![
                    SteelVal::SymbolV("ground-cuboid".into()),
                    SteelVal::NumV(steel_val_to_f64(&width)),
                    SteelVal::NumV(steel_val_to_f64(&height)),
                    SteelVal::NumV(steel_val_to_f64(&depth)),
                ];
                SteelVal::ListV(list.into())
            },
        );

        // Register make-object
        engine.register_fn(
            "make-object",
            |position: SteelVal, rotation: SteelVal| -> SteelVal {
                let list = vec![
                    SteelVal::SymbolV("object".into()),
                    position,
                    rotation,
                ];
                SteelVal::ListV(list.into())
            },
        );

        // Register make-object-with-material
        engine.register_fn(
            "make-object-with-material",
            |position: SteelVal, rotation: SteelVal, material: SteelVal| -> SteelVal {
                let list = vec![
                    SteelVal::SymbolV("object".into()),
                    position,
                    rotation,
                    SteelVal::IntV(steel_val_to_isize(&material)),
                ];
                SteelVal::ListV(list.into())
            },
        );

        // Register make-scene
        engine.register_fn(
            "make-scene",
            |ground: SteelVal, objects: SteelVal, camera: SteelVal| -> SteelVal {
                let list = vec![
                    SteelVal::SymbolV("scene".into()),
                    ground,
                    objects,
                    camera,
                ];
                SteelVal::ListV(list.into())
            },
        );

        Self { config }
    }

    /// Load and evaluate a Steel configuration file
    pub fn load_file(&mut self, path: &Path) -> Result<(), String> {
        self.config.load_file(path)
    }

    /// Load configuration from a string
    pub fn load_string(&mut self, content: &str) -> Result<(), String> {
        self.config.load_string(content)
    }

    /// Extract a scene configuration from the engine
    pub fn extract_scene(&self, name: &str) -> Result<SceneConfig, String> {
        let value = self.config.extract_value(name)?;
        parse_scene_config(&value)
    }

    /// Extract camera configuration
    pub fn extract_camera(&self, name: &str) -> Result<CameraConfig, String> {
        let value = self.config.extract_value(name)?;
        parse_camera_config(&value)
    }

    /// Extract ground configuration
    pub fn extract_ground(&self, name: &str) -> Result<GroundConfig, String> {
        let value = self.config.extract_value(name)?;
        parse_ground_config(&value)
    }

    /// Extract objects configuration
    pub fn extract_objects(&self, name: &str) -> Result<Vec<ObjectConfig>, String> {
        let value = self.config.extract_value(name)?;
        parse_objects_config(&value)
    }
}

impl Default for TestbedConfig {
    fn default() -> Self {
        Self::new()
    }
}

// Parsing functions

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
        SteelVal::ListV(list) => list.iter().map(|item| parse_object_config(item)).collect(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_config() {
        let mut config = TestbedConfig::new();
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
        let mut config = TestbedConfig::new();
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
        let mut config = TestbedConfig::new();
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
        let mut config = TestbedConfig::new();
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
