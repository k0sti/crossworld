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
    SolidCube {
        material: u8,
        size_shift: u32,
        center: Vec3Config,
    },
    /// Simple cuboid with dimensions
    Cuboid {
        width: f32,
        center: Vec3Config,
    },
}


/// Object configuration in the scene
#[derive(Debug, Clone)]
pub struct ObjectConfig {
    pub position: Vec3Config,
    pub rotation: QuatConfig,
    pub size: Vec3Config,
    pub mass: f32,
    pub material: u8,
}

/// Camera configuration
#[derive(Debug, Clone)]
pub struct CameraConfig {
    pub position: Vec3Config,
    pub look_at: Vec3Config,
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

        // Register quat (raw quaternion constructor)
        engine.register_fn(
            "quat",
            |x: SteelVal, y: SteelVal, z: SteelVal, w: SteelVal| -> SteelVal {
                let list = vec![
                    SteelVal::NumV(steel_val_to_f64(&x)),
                    SteelVal::NumV(steel_val_to_f64(&y)),
                    SteelVal::NumV(steel_val_to_f64(&z)),
                    SteelVal::NumV(steel_val_to_f64(&w)),
                ];
                SteelVal::ListV(list.into())
            },
        );

        // Register camera
        engine.register_fn(
            "camera",
            |position: SteelVal, look_at: SteelVal| -> SteelVal {
                let list = vec![
                    SteelVal::SymbolV("camera".into()),
                    position,
                    look_at,
                ];
                SteelVal::ListV(list.into())
            },
        );

        // Register ground-cube
        engine.register_fn(
            "ground-cube",
            |material: SteelVal, size_shift: SteelVal, center: SteelVal| -> SteelVal {
                let list = vec![
                    SteelVal::SymbolV("ground-cube".into()),
                    SteelVal::IntV(steel_val_to_isize(&material)),
                    SteelVal::IntV(steel_val_to_isize(&size_shift)),
                    center,
                ];
                SteelVal::ListV(list.into())
            },
        );

        // Register ground-cuboid
        engine.register_fn(
            "ground-cuboid",
            |width: SteelVal, center: SteelVal| -> SteelVal {
                let list = vec![
                    SteelVal::SymbolV("ground-cuboid".into()),
                    SteelVal::NumV(steel_val_to_f64(&width)),
                    center,
                ];
                SteelVal::ListV(list.into())
            },
        );

        // Register object
        engine.register_fn(
            "object",
            |position: SteelVal, rotation: SteelVal, size: SteelVal, mass: SteelVal, material: SteelVal| -> SteelVal {
                let list = vec![
                    SteelVal::SymbolV("object".into()),
                    position,
                    rotation,
                    size,
                    mass,
                    SteelVal::IntV(steel_val_to_isize(&material)),
                ];
                SteelVal::ListV(list.into())
            },
        );

        // Register scene
        engine.register_fn(
            "scene",
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
    #[cfg(test)]
    pub fn load_string(&mut self, content: &str) -> Result<(), String> {
        self.config.load_string(content)
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

            Ok(CameraConfig {
                position,
                look_at,
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
                    if items.len() < 4 {
                        return Err("ground-cube requires material, size_shift, and center".to_string());
                    }
                    let material = extract_u8(&items[1])?;
                    let size_shift = extract_u32(&items[2])?;
                    let center = parse_vec3(&items[3])?;
                    Ok(GroundConfig::SolidCube {
                        material,
                        size_shift,
                        center,
                    })
                }
                SteelVal::SymbolV(s) if s.as_str() == "ground-cuboid" => {
                    if items.len() < 3 {
                        return Err("ground-cuboid requires width and center".to_string());
                    }
                    let width = extract_f32(&items[1])?;
                    let center = parse_vec3(&items[2])?;
                    Ok(GroundConfig::Cuboid {
                        width,
                        center,
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
            if items.len() < 6 {
                return Err("object requires position, rotation, size, mass, and material".to_string());
            }

            // First element should be 'object symbol
            match &items[0] {
                SteelVal::SymbolV(s) if s.as_str() == "object" => {}
                _ => return Err("Expected 'object symbol".to_string()),
            }

            let position = parse_vec3(&items[1])?;
            let rotation = parse_quat(&items[2])?;
            let size = parse_vec3(&items[3])?;
            let mass = extract_f32(&items[4])?;
            let material = extract_u8(&items[5])?;

            Ok(ObjectConfig {
                position,
                rotation,
                size,
                mass,
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
                  (camera
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
            .load_string("(define test-ground (ground-cube 32 3 (vec3 0 -4 0)))")
            .unwrap();

        let ground = config.extract_ground("test-ground").unwrap();
        match ground {
            GroundConfig::SolidCube {
                material,
                size_shift,
                center,
            } => {
                assert_eq!(material, 32);
                assert_eq!(size_shift, 3);
                assert_eq!(center.y, -4.0);
            }
            _ => panic!("Expected SolidCube"),
        }
    }

    #[test]
    fn test_ground_cuboid_config() {
        let mut config = TestbedConfig::new();
        config
            .load_string("(define test-ground (ground-cuboid 8 (vec3 0 -4 0)))")
            .unwrap();

        let ground = config.extract_ground("test-ground").unwrap();
        match ground {
            GroundConfig::Cuboid {
                width,
                center,
            } => {
                assert_eq!(width, 8.0);
                assert_eq!(center.y, -4.0);
            }
            _ => panic!("Expected Cuboid"),
        }
    }

}
