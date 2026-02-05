//! High-resolution timing utilities
//!
//! This module provides cross-platform timing utilities for measuring time,
//! computing delta times, and tracking elapsed time. Supports both native
//! (std::time) and web (performance.now()) environments.

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// High-resolution instant in time
///
/// This type provides a cross-platform way to measure time intervals.
/// On native platforms, it wraps `std::time::Instant`.
/// On web/WASM, it uses `performance.now()`.
#[derive(Debug, Clone, Copy)]
pub struct Instant {
    #[cfg(not(target_arch = "wasm32"))]
    inner: std::time::Instant,
    #[cfg(target_arch = "wasm32")]
    millis: f64,
}

impl Instant {
    /// Returns the current instant
    pub fn now() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            Self {
                inner: std::time::Instant::now(),
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            Self {
                millis: performance_now(),
            }
        }
    }

    /// Returns the elapsed time since this instant was created
    pub fn elapsed(&self) -> Duration {
        Self::now().duration_since(self)
    }

    /// Returns the duration since another instant
    pub fn duration_since(&self, earlier: &Instant) -> Duration {
        #[cfg(not(target_arch = "wasm32"))]
        {
            Duration::from_std(self.inner.duration_since(earlier.inner))
        }
        #[cfg(target_arch = "wasm32")]
        {
            let millis = self.millis - earlier.millis;
            Duration::from_millis_f64(millis)
        }
    }
}

impl Default for Instant {
    fn default() -> Self {
        Self::now()
    }
}

/// Duration of time
///
/// A cross-platform duration type that provides consistent behavior
/// on both native and web platforms.
#[derive(Debug, Clone, Copy, Default, PartialEq, PartialOrd)]
pub struct Duration {
    /// Duration stored as seconds (f64 for precision across platforms)
    secs: f64,
}

impl Duration {
    /// Create a zero-length duration
    pub const ZERO: Self = Self { secs: 0.0 };

    /// Create a duration from seconds
    #[inline]
    pub const fn from_secs_f64(secs: f64) -> Self {
        Self { secs }
    }

    /// Create a duration from seconds (f32)
    #[inline]
    pub fn from_secs_f32(secs: f32) -> Self {
        Self { secs: secs as f64 }
    }

    /// Create a duration from milliseconds
    #[inline]
    pub fn from_millis(millis: u64) -> Self {
        Self {
            secs: millis as f64 / 1000.0,
        }
    }

    /// Create a duration from milliseconds (f64, for web interop)
    #[inline]
    pub fn from_millis_f64(millis: f64) -> Self {
        Self {
            secs: millis / 1000.0,
        }
    }

    /// Convert from std::time::Duration
    #[cfg(not(target_arch = "wasm32"))]
    #[inline]
    pub fn from_std(d: std::time::Duration) -> Self {
        Self {
            secs: d.as_secs_f64(),
        }
    }

    /// Get the duration as seconds (f64)
    #[inline]
    pub const fn as_secs_f64(&self) -> f64 {
        self.secs
    }

    /// Get the duration as seconds (f32)
    #[inline]
    pub fn as_secs_f32(&self) -> f32 {
        self.secs as f32
    }

    /// Get the duration as milliseconds
    #[inline]
    pub fn as_millis(&self) -> f64 {
        self.secs * 1000.0
    }

    /// Get the duration as whole milliseconds (u128)
    #[inline]
    pub fn as_millis_u128(&self) -> u128 {
        (self.secs * 1000.0) as u128
    }

    /// Check if the duration is zero
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.secs == 0.0
    }
}

impl std::ops::Add for Duration {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            secs: self.secs + rhs.secs,
        }
    }
}

impl std::ops::Sub for Duration {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            secs: self.secs - rhs.secs,
        }
    }
}

impl std::ops::AddAssign for Duration {
    fn add_assign(&mut self, rhs: Self) {
        self.secs += rhs.secs;
    }
}

impl std::ops::SubAssign for Duration {
    fn sub_assign(&mut self, rhs: Self) {
        self.secs -= rhs.secs;
    }
}

/// Timer for tracking frame timing
///
/// Provides convenient frame timing tracking with delta time,
/// elapsed time, and frame counting.
#[derive(Debug, Clone)]
pub struct FrameTimer {
    /// When the timer was started
    start_time: Instant,
    /// When the last frame was recorded
    last_frame_time: Instant,
    /// Delta time for the current frame
    delta_time: Duration,
    /// Total elapsed time since start
    elapsed: Duration,
    /// Current frame number
    frame_count: u64,
}

impl Default for FrameTimer {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameTimer {
    /// Create a new frame timer, starting now
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            start_time: now,
            last_frame_time: now,
            delta_time: Duration::ZERO,
            elapsed: Duration::ZERO,
            frame_count: 0,
        }
    }

    /// Call at the start of each frame to update timing
    ///
    /// Returns the delta time for this frame.
    pub fn tick(&mut self) -> f32 {
        let now = Instant::now();
        self.delta_time = now.duration_since(&self.last_frame_time);
        self.elapsed = now.duration_since(&self.start_time);
        self.last_frame_time = now;
        self.frame_count += 1;
        self.delta_time.as_secs_f32()
    }

    /// Get the delta time for the current frame in seconds
    #[inline]
    pub fn delta_time(&self) -> f32 {
        self.delta_time.as_secs_f32()
    }

    /// Get the total elapsed time since the timer was created
    #[inline]
    pub fn elapsed(&self) -> f32 {
        self.elapsed.as_secs_f32()
    }

    /// Get the current frame number (1-indexed, incremented by tick())
    #[inline]
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Get the average frames per second based on elapsed time
    #[inline]
    pub fn average_fps(&self) -> f32 {
        if self.elapsed.is_zero() {
            0.0
        } else {
            self.frame_count as f32 / self.elapsed.as_secs_f32()
        }
    }

    /// Get the instantaneous frames per second based on delta time
    #[inline]
    pub fn fps(&self) -> f32 {
        let dt = self.delta_time.as_secs_f32();
        if dt > 0.0 {
            1.0 / dt
        } else {
            0.0
        }
    }

    /// Reset the timer to its initial state
    pub fn reset(&mut self) {
        let now = Instant::now();
        self.start_time = now;
        self.last_frame_time = now;
        self.delta_time = Duration::ZERO;
        self.elapsed = Duration::ZERO;
        self.frame_count = 0;
    }
}

/// Get performance.now() on web platforms
#[cfg(target_arch = "wasm32")]
fn performance_now() -> f64 {
    // Use wasm_bindgen to access performance.now()
    js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("performance"))
        .ok()
        .and_then(|perf| js_sys::Reflect::get(&perf, &JsValue::from_str("now")).ok())
        .and_then(|now| {
            if now.is_function() {
                let func: js_sys::Function = now.into();
                func.call0(&js_sys::global()).ok()
            } else {
                None
            }
        })
        .and_then(|val| val.as_f64())
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instant_now() {
        let _instant = Instant::now();
        // Should not panic
    }

    #[test]
    fn test_instant_elapsed() {
        let instant = Instant::now();
        let elapsed = instant.elapsed();
        // Elapsed should be non-negative
        assert!(elapsed.as_secs_f64() >= 0.0);
    }

    #[test]
    fn test_duration_from_secs() {
        let d = Duration::from_secs_f64(1.5);
        assert!((d.as_secs_f64() - 1.5).abs() < 0.001);
        assert!((d.as_millis() - 1500.0).abs() < 1.0);
    }

    #[test]
    fn test_duration_arithmetic() {
        let d1 = Duration::from_secs_f64(1.0);
        let d2 = Duration::from_secs_f64(0.5);

        let sum = d1 + d2;
        assert!((sum.as_secs_f64() - 1.5).abs() < 0.001);

        let diff = d1 - d2;
        assert!((diff.as_secs_f64() - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_frame_timer() {
        let mut timer = FrameTimer::new();
        assert_eq!(timer.frame_count(), 0);

        // Tick a few times
        for _ in 0..3 {
            let _dt = timer.tick();
        }

        assert_eq!(timer.frame_count(), 3);
        assert!(timer.elapsed() >= 0.0);
    }

    #[test]
    fn test_frame_timer_reset() {
        let mut timer = FrameTimer::new();
        timer.tick();
        timer.tick();
        assert_eq!(timer.frame_count(), 2);

        timer.reset();
        assert_eq!(timer.frame_count(), 0);
    }
}
