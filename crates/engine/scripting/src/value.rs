//! Value types for the state tree

use crate::{Error, Result};
use glam::{Quat, Vec3};
use std::collections::HashMap;

/// A value in the state tree
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Value {
    /// Null/empty value
    #[default]
    Null,
    /// Boolean value
    Bool(bool),
    /// Integer value
    Int(i64),
    /// Floating point value
    Float(f64),
    /// String value
    String(String),
    /// 3D vector
    Vec3(Vec3),
    /// Quaternion rotation
    Quat(Quat),
    /// Array of values
    Array(Vec<Value>),
    /// Map of string keys to values
    Map(HashMap<String, Value>),
}

impl Value {
    /// Get the type name of this value
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "bool",
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::String(_) => "string",
            Value::Vec3(_) => "vec3",
            Value::Quat(_) => "quat",
            Value::Array(_) => "array",
            Value::Map(_) => "map",
        }
    }

    /// Try to get as bool
    pub fn as_bool(&self) -> Result<bool> {
        match self {
            Value::Bool(b) => Ok(*b),
            _ => Err(Error::TypeError {
                expected: "bool".to_string(),
                actual: self.type_name().to_string(),
            }),
        }
    }

    /// Try to get as i64
    pub fn as_i64(&self) -> Result<i64> {
        match self {
            Value::Int(i) => Ok(*i),
            Value::Float(f) => Ok(*f as i64),
            _ => Err(Error::TypeError {
                expected: "int".to_string(),
                actual: self.type_name().to_string(),
            }),
        }
    }

    /// Try to get as u32
    pub fn as_u32(&self) -> Result<u32> {
        match self {
            Value::Int(i) => {
                if *i >= 0 && *i <= u32::MAX as i64 {
                    Ok(*i as u32)
                } else {
                    Err(Error::InvalidValue(format!("value {} out of u32 range", i)))
                }
            }
            Value::Float(f) => {
                let i = *f as i64;
                if i >= 0 && i <= u32::MAX as i64 {
                    Ok(i as u32)
                } else {
                    Err(Error::InvalidValue(format!("value {} out of u32 range", f)))
                }
            }
            _ => Err(Error::TypeError {
                expected: "int".to_string(),
                actual: self.type_name().to_string(),
            }),
        }
    }

    /// Try to get as f64
    pub fn as_f64(&self) -> Result<f64> {
        match self {
            Value::Float(f) => Ok(*f),
            Value::Int(i) => Ok(*i as f64),
            _ => Err(Error::TypeError {
                expected: "float".to_string(),
                actual: self.type_name().to_string(),
            }),
        }
    }

    /// Try to get as f32
    pub fn as_f32(&self) -> Result<f32> {
        self.as_f64().map(|f| f as f32)
    }

    /// Try to get as string
    pub fn as_str(&self) -> Result<&str> {
        match self {
            Value::String(s) => Ok(s.as_str()),
            _ => Err(Error::TypeError {
                expected: "string".to_string(),
                actual: self.type_name().to_string(),
            }),
        }
    }

    /// Try to get as Vec3
    pub fn as_vec3(&self) -> Result<Vec3> {
        match self {
            Value::Vec3(v) => Ok(*v),
            Value::Array(arr) if arr.len() == 3 => {
                let x = arr[0].as_f32()?;
                let y = arr[1].as_f32()?;
                let z = arr[2].as_f32()?;
                Ok(Vec3::new(x, y, z))
            }
            _ => Err(Error::TypeError {
                expected: "vec3".to_string(),
                actual: self.type_name().to_string(),
            }),
        }
    }

    /// Try to get as Quat
    pub fn as_quat(&self) -> Result<Quat> {
        match self {
            Value::Quat(q) => Ok(*q),
            Value::Array(arr) if arr.len() == 4 => {
                let x = arr[0].as_f32()?;
                let y = arr[1].as_f32()?;
                let z = arr[2].as_f32()?;
                let w = arr[3].as_f32()?;
                Ok(Quat::from_xyzw(x, y, z, w))
            }
            _ => Err(Error::TypeError {
                expected: "quat".to_string(),
                actual: self.type_name().to_string(),
            }),
        }
    }

    /// Try to get as array
    pub fn as_array(&self) -> Result<&[Value]> {
        match self {
            Value::Array(arr) => Ok(arr.as_slice()),
            _ => Err(Error::TypeError {
                expected: "array".to_string(),
                actual: self.type_name().to_string(),
            }),
        }
    }

    /// Try to get as map
    pub fn as_map(&self) -> Result<&HashMap<String, Value>> {
        match self {
            Value::Map(map) => Ok(map),
            _ => Err(Error::TypeError {
                expected: "map".to_string(),
                actual: self.type_name().to_string(),
            }),
        }
    }

    /// Check if value is null
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }
}

// Conversion from common types

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<i32> for Value {
    fn from(i: i32) -> Self {
        Value::Int(i as i64)
    }
}

impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Value::Int(i)
    }
}

impl From<u32> for Value {
    fn from(i: u32) -> Self {
        Value::Int(i as i64)
    }
}

impl From<f32> for Value {
    fn from(f: f32) -> Self {
        Value::Float(f as f64)
    }
}

impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Value::Float(f)
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_string())
    }
}

impl From<Vec3> for Value {
    fn from(v: Vec3) -> Self {
        Value::Vec3(v)
    }
}

impl From<Quat> for Value {
    fn from(q: Quat) -> Self {
        Value::Quat(q)
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(v: Vec<T>) -> Self {
        Value::Array(v.into_iter().map(Into::into).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_conversions() {
        let v = Value::from(42i32);
        assert_eq!(v.as_i64().unwrap(), 42);
        assert_eq!(v.as_u32().unwrap(), 42);
        assert_eq!(v.as_f64().unwrap(), 42.0);

        let v = Value::from(3.14f64);
        assert!((v.as_f64().unwrap() - 3.14).abs() < 0.001);

        let v = Value::from("hello");
        assert_eq!(v.as_str().unwrap(), "hello");

        let v = Value::from(true);
        assert!(v.as_bool().unwrap());
    }

    #[test]
    fn test_vec3_from_array() {
        let v = Value::Array(vec![
            Value::from(1.0f32),
            Value::from(2.0f32),
            Value::from(3.0f32),
        ]);
        let vec = v.as_vec3().unwrap();
        assert_eq!(vec, Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_type_errors() {
        let v = Value::from("string");
        assert!(v.as_i64().is_err());
        assert!(v.as_bool().is_err());
    }
}
