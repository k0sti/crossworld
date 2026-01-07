//! Base Steel configuration module for application scene setup
//!
//! Provides Steel (embedded Scheme) scripting support with base types
//! like vectors and quaternions. Application-specific types (scenes,
//! physics, etc.) should be registered by the application.
//!
//! # Example Steel Configuration
//!
//! ```scheme
//! ;; Create vectors and quaternions
//! (define pos (vec3 0 6 -3))
//! (define rot (quat-euler 0.1 0.2 0.3))
//! ```

use glam::{Quat, Vec3};
use std::path::Path;

// Re-export steel crate for use by dependent crates
pub use steel;
use steel::steel_vm::engine::Engine;
use steel::steel_vm::register_fn::RegisterFn;
use steel::SteelVal;

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

/// Steel configuration engine wrapper
///
/// Provides base functionality for Steel scripting with common types.
/// Applications can extend this by registering additional functions.
pub struct SteelConfig {
    engine: Engine,
}

// Helper functions for converting SteelVal to numeric types
// These handle both IntV and NumV variants for flexibility

/// Convert a SteelVal to f64, handling both IntV and NumV
pub fn steel_val_to_f64(val: &SteelVal) -> f64 {
    match val {
        SteelVal::NumV(n) => *n,
        SteelVal::IntV(n) => *n as f64,
        _ => 0.0,
    }
}

/// Convert a SteelVal to isize, handling both IntV and NumV
pub fn steel_val_to_isize(val: &SteelVal) -> isize {
    match val {
        SteelVal::IntV(n) => *n,
        SteelVal::NumV(n) => *n as isize,
        _ => 0,
    }
}

/// Parse a SteelVal list as Vec3Config
pub fn parse_vec3(val: &SteelVal) -> Result<Vec3Config, String> {
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

/// Parse a SteelVal list as QuatConfig
pub fn parse_quat(val: &SteelVal) -> Result<QuatConfig, String> {
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

/// Extract f32 from a SteelVal
pub fn extract_f32(val: &SteelVal) -> Result<f32, String> {
    match val {
        SteelVal::NumV(n) => Ok(*n as f32),
        SteelVal::IntV(n) => Ok(*n as f32),
        _ => Err(format!("Expected number, got {:?}", val)),
    }
}

/// Extract u8 from a SteelVal
pub fn extract_u8(val: &SteelVal) -> Result<u8, String> {
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

/// Extract u32 from a SteelVal
pub fn extract_u32(val: &SteelVal) -> Result<u32, String> {
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

impl SteelConfig {
    /// Create a new Steel configuration engine with base types registered
    ///
    /// Registers:
    /// - `vec3` - Create 3D vectors
    /// - `quat-euler` - Create quaternions from Euler angles
    /// - `quat-identity` - Identity quaternion
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

        Self { engine }
    }

    /// Get mutable access to the underlying Steel engine
    ///
    /// Use this to register additional functions for your application.
    pub fn engine_mut(&mut self) -> &mut Engine {
        &mut self.engine
    }

    /// Get immutable access to the underlying Steel engine
    pub fn engine(&self) -> &Engine {
        &self.engine
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

    /// Extract a value by name from the engine
    pub fn extract_value(&self, name: &str) -> Result<SteelVal, String> {
        self.engine
            .extract_value(name)
            .map_err(|e| format!("Failed to extract '{}': {}", name, e))
    }
}

impl Default for SteelConfig {
    fn default() -> Self {
        Self::new()
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

        let val = config.extract_value("test-vec").unwrap();
        let vec = parse_vec3(&val).unwrap();
        assert_eq!(vec.x, 1.0);
        assert_eq!(vec.y, 2.0);
        assert_eq!(vec.z, 3.0);
    }

    #[test]
    fn test_vec3_with_integers() {
        let mut config = SteelConfig::new();
        config
            .load_string("(define test-vec (vec3 1 2 3))")
            .unwrap();

        let val = config.extract_value("test-vec").unwrap();
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

        let val = config.extract_value("test-quat").unwrap();
        let quat = parse_quat(&val).unwrap();
        // Identity quaternion
        assert!((quat.w - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_quat_identity() {
        let mut config = SteelConfig::new();
        config
            .load_string("(define test-quat (quat-identity))")
            .unwrap();

        let val = config.extract_value("test-quat").unwrap();
        let quat = parse_quat(&val).unwrap();
        assert_eq!(quat.x, 0.0);
        assert_eq!(quat.y, 0.0);
        assert_eq!(quat.z, 0.0);
        assert_eq!(quat.w, 1.0);
    }
}
