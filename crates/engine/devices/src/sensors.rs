//! Sensor abstractions
//!
//! This module provides types for handling device sensors like accelerometers
//! and compasses. This is primarily for mobile/XR device support and is
//! currently a placeholder for future implementation.

use glam::Vec3;

/// Accelerometer sensor state
///
/// Provides acceleration data in device coordinates (m/s^2).
/// On mobile devices, this typically includes gravity.
#[derive(Debug, Clone, Copy, Default)]
pub struct Accelerometer {
    /// Acceleration along X axis (right positive)
    pub x: f32,
    /// Acceleration along Y axis (up positive)
    pub y: f32,
    /// Acceleration along Z axis (forward positive)
    pub z: f32,
    /// Whether the sensor is available and active
    pub available: bool,
}

impl Accelerometer {
    /// Create a new accelerometer state
    pub fn new() -> Self {
        Self::default()
    }

    /// Get acceleration as a vector
    pub fn acceleration(&self) -> Vec3 {
        Vec3::new(self.x, self.y, self.z)
    }

    /// Get the magnitude of acceleration
    pub fn magnitude(&self) -> f32 {
        self.acceleration().length()
    }

    /// Check if the device is roughly level (gravity pointing down)
    ///
    /// Returns true if the device appears to be laying flat,
    /// with a tolerance for small deviations.
    pub fn is_level(&self, tolerance: f32) -> bool {
        if !self.available {
            return false;
        }

        // Gravity should be ~9.8 on Y axis when device is flat
        const GRAVITY: f32 = 9.81;
        let expected = Vec3::new(0.0, -GRAVITY, 0.0);
        self.acceleration().distance(expected) < tolerance
    }

    /// Update the accelerometer values
    pub fn update(&mut self, x: f32, y: f32, z: f32) {
        self.x = x;
        self.y = y;
        self.z = z;
    }
}

/// Compass/magnetometer sensor state
///
/// Provides heading information relative to magnetic north.
#[derive(Debug, Clone, Copy, Default)]
pub struct Compass {
    /// Heading in degrees (0-360, 0 = North, 90 = East)
    pub heading: f32,
    /// Magnetic field strength in microtesla
    pub field_strength: f32,
    /// Accuracy indicator (0.0 = poor, 1.0 = excellent)
    pub accuracy: f32,
    /// Whether the sensor is available and active
    pub available: bool,
}

impl Compass {
    /// Create a new compass state
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the heading as a normalized direction vector (in XZ plane)
    ///
    /// Returns a vector pointing in the heading direction where:
    /// - (0, 0, -1) is North
    /// - (1, 0, 0) is East
    /// - (0, 0, 1) is South
    /// - (-1, 0, 0) is West
    pub fn direction(&self) -> Vec3 {
        let radians = self.heading.to_radians();
        Vec3::new(radians.sin(), 0.0, -radians.cos())
    }

    /// Get cardinal direction as a string
    pub fn cardinal(&self) -> &'static str {
        match self.heading as i32 {
            338..=360 | 0..=22 => "N",
            23..=67 => "NE",
            68..=112 => "E",
            113..=157 => "SE",
            158..=202 => "S",
            203..=247 => "SW",
            248..=292 => "W",
            293..=337 => "NW",
            _ => "?",
        }
    }

    /// Update the compass values
    pub fn update(&mut self, heading: f32, field_strength: f32, accuracy: f32) {
        self.heading = heading % 360.0;
        if self.heading < 0.0 {
            self.heading += 360.0;
        }
        self.field_strength = field_strength;
        self.accuracy = accuracy.clamp(0.0, 1.0);
    }
}

/// Gyroscope sensor state
///
/// Provides angular velocity data in radians per second.
#[derive(Debug, Clone, Copy, Default)]
pub struct Gyroscope {
    /// Angular velocity around X axis (pitch rate)
    pub pitch: f32,
    /// Angular velocity around Y axis (yaw rate)
    pub yaw: f32,
    /// Angular velocity around Z axis (roll rate)
    pub roll: f32,
    /// Whether the sensor is available and active
    pub available: bool,
}

impl Gyroscope {
    /// Create a new gyroscope state
    pub fn new() -> Self {
        Self::default()
    }

    /// Get angular velocity as a vector
    pub fn angular_velocity(&self) -> Vec3 {
        Vec3::new(self.pitch, self.yaw, self.roll)
    }

    /// Check if the device is stationary (below threshold)
    pub fn is_stationary(&self, threshold: f32) -> bool {
        self.angular_velocity().length() < threshold
    }

    /// Update the gyroscope values
    pub fn update(&mut self, pitch: f32, yaw: f32, roll: f32) {
        self.pitch = pitch;
        self.yaw = yaw;
        self.roll = roll;
    }
}

/// Combined sensor state for all device sensors
#[derive(Debug, Clone, Default)]
pub struct SensorState {
    /// Accelerometer sensor
    pub accelerometer: Accelerometer,
    /// Compass/magnetometer sensor
    pub compass: Compass,
    /// Gyroscope sensor
    pub gyroscope: Gyroscope,
}

impl SensorState {
    /// Create a new sensor state with all sensors unavailable
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if any sensors are available
    pub fn any_available(&self) -> bool {
        self.accelerometer.available || self.compass.available || self.gyroscope.available
    }

    /// Check if all sensors are available
    pub fn all_available(&self) -> bool {
        self.accelerometer.available && self.compass.available && self.gyroscope.available
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accelerometer() {
        let mut accel = Accelerometer::new();
        assert!(!accel.available);
        assert_eq!(accel.magnitude(), 0.0);

        accel.update(0.0, -9.81, 0.0);
        accel.available = true;
        assert!(accel.is_level(1.0));

        accel.update(5.0, -9.81, 0.0);
        assert!(!accel.is_level(1.0));
    }

    #[test]
    fn test_compass_cardinal() {
        let mut compass = Compass::new();

        compass.update(0.0, 50.0, 0.8);
        assert_eq!(compass.cardinal(), "N");

        compass.update(90.0, 50.0, 0.8);
        assert_eq!(compass.cardinal(), "E");

        compass.update(180.0, 50.0, 0.8);
        assert_eq!(compass.cardinal(), "S");

        compass.update(270.0, 50.0, 0.8);
        assert_eq!(compass.cardinal(), "W");

        compass.update(45.0, 50.0, 0.8);
        assert_eq!(compass.cardinal(), "NE");
    }

    #[test]
    fn test_compass_direction() {
        let mut compass = Compass::new();

        compass.update(0.0, 50.0, 0.8);
        let dir = compass.direction();
        assert!((dir.z - (-1.0)).abs() < 0.001); // North = -Z

        compass.update(90.0, 50.0, 0.8);
        let dir = compass.direction();
        assert!((dir.x - 1.0).abs() < 0.001); // East = +X
    }

    #[test]
    fn test_gyroscope_stationary() {
        let mut gyro = Gyroscope::new();
        assert!(gyro.is_stationary(0.1));

        gyro.update(0.5, 0.5, 0.5);
        assert!(!gyro.is_stationary(0.1));
    }

    #[test]
    fn test_sensor_state() {
        let mut state = SensorState::new();
        assert!(!state.any_available());

        state.accelerometer.available = true;
        assert!(state.any_available());
        assert!(!state.all_available());

        state.compass.available = true;
        state.gyroscope.available = true;
        assert!(state.all_available());
    }
}
