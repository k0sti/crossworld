//! Window handle abstraction
//!
//! This module provides a platform-agnostic window handle abstraction
//! that wraps the underlying windowing system (winit on native, canvas on web).

use glam::UVec2;

/// Window handle abstraction
///
/// Provides a unified interface for window operations across platforms.
/// On native, wraps a winit Window.
/// On web, wraps a canvas element.
#[derive(Debug)]
pub struct WindowHandle {
    /// Window size in pixels
    size: UVec2,
    /// Window scale factor (for HiDPI)
    scale_factor: f64,
    /// Window title
    title: String,
    /// Whether the window has focus
    focused: bool,
    /// Whether the window is visible
    visible: bool,
}

impl Default for WindowHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowHandle {
    /// Create a new window handle with default values
    pub fn new() -> Self {
        Self {
            size: UVec2::new(1280, 720),
            scale_factor: 1.0,
            title: String::from("Crossworld"),
            focused: true,
            visible: true,
        }
    }

    /// Create a window handle with specified size
    pub fn with_size(width: u32, height: u32) -> Self {
        Self {
            size: UVec2::new(width, height),
            ..Self::new()
        }
    }

    /// Update from a winit window reference
    #[cfg(not(target_arch = "wasm32"))]
    pub fn update_from_winit(&mut self, window: &winit::window::Window) {
        let size = window.inner_size();
        self.size = UVec2::new(size.width, size.height);
        self.scale_factor = window.scale_factor();
        self.focused = window.has_focus();
        self.visible = window.is_visible().unwrap_or(true);
    }

    /// Get the window width in pixels
    #[inline]
    pub fn width(&self) -> u32 {
        self.size.x
    }

    /// Get the window height in pixels
    #[inline]
    pub fn height(&self) -> u32 {
        self.size.y
    }

    /// Get the window size as a tuple
    #[inline]
    pub fn size(&self) -> (u32, u32) {
        (self.size.x, self.size.y)
    }

    /// Get the window size as a UVec2
    #[inline]
    pub fn size_uvec2(&self) -> UVec2 {
        self.size
    }

    /// Get the aspect ratio (width / height)
    #[inline]
    pub fn aspect_ratio(&self) -> f32 {
        if self.size.y > 0 {
            self.size.x as f32 / self.size.y as f32
        } else {
            1.0
        }
    }

    /// Get the scale factor (for HiDPI displays)
    #[inline]
    pub fn scale_factor(&self) -> f64 {
        self.scale_factor
    }

    /// Get the window title
    #[inline]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Set the window title
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
    }

    /// Check if the window has focus
    #[inline]
    pub fn has_focus(&self) -> bool {
        self.focused
    }

    /// Check if the window is visible
    #[inline]
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Set the focused state
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    /// Set the visible state
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    /// Set the window size
    pub fn set_size(&mut self, width: u32, height: u32) {
        self.size = UVec2::new(width, height);
    }

    /// Set the scale factor
    pub fn set_scale_factor(&mut self, factor: f64) {
        self.scale_factor = factor;
    }

    /// Get the physical size (size * scale_factor)
    pub fn physical_size(&self) -> (u32, u32) {
        let w = (self.size.x as f64 * self.scale_factor) as u32;
        let h = (self.size.y as f64 * self.scale_factor) as u32;
        (w, h)
    }
}

/// Window configuration for creating new windows
#[derive(Debug, Clone)]
pub struct WindowConfig {
    /// Window title
    pub title: String,
    /// Window width in pixels
    pub width: u32,
    /// Window height in pixels
    pub height: u32,
    /// Whether the window is resizable
    pub resizable: bool,
    /// Whether to start in fullscreen
    pub fullscreen: bool,
    /// Whether VSync is enabled
    pub vsync: bool,
    /// OpenGL major version
    pub gl_major: u8,
    /// OpenGL minor version
    pub gl_minor: u8,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: String::from("Crossworld"),
            width: 1280,
            height: 720,
            resizable: true,
            fullscreen: false,
            vsync: true,
            gl_major: 3,
            gl_minor: 3,
        }
    }
}

impl WindowConfig {
    /// Create a new window configuration with the given title
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Self::default()
        }
    }

    /// Set the window size
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Set whether the window is resizable
    pub fn with_resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Set whether to start in fullscreen
    pub fn with_fullscreen(mut self, fullscreen: bool) -> Self {
        self.fullscreen = fullscreen;
        self
    }

    /// Set VSync
    pub fn with_vsync(mut self, vsync: bool) -> Self {
        self.vsync = vsync;
        self
    }

    /// Set the OpenGL version
    pub fn with_gl_version(mut self, major: u8, minor: u8) -> Self {
        self.gl_major = major;
        self.gl_minor = minor;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_handle_default() {
        let handle = WindowHandle::new();
        assert_eq!(handle.width(), 1280);
        assert_eq!(handle.height(), 720);
        assert!((handle.aspect_ratio() - 1.7778).abs() < 0.01);
    }

    #[test]
    fn test_window_handle_with_size() {
        let handle = WindowHandle::with_size(1920, 1080);
        assert_eq!(handle.width(), 1920);
        assert_eq!(handle.height(), 1080);
        assert_eq!(handle.size(), (1920, 1080));
    }

    #[test]
    fn test_window_config_builder() {
        let config = WindowConfig::new("Test Window")
            .with_size(800, 600)
            .with_resizable(false)
            .with_gl_version(4, 5);

        assert_eq!(config.title, "Test Window");
        assert_eq!(config.width, 800);
        assert_eq!(config.height, 600);
        assert!(!config.resizable);
        assert_eq!(config.gl_major, 4);
        assert_eq!(config.gl_minor, 5);
    }

    #[test]
    fn test_physical_size() {
        let mut handle = WindowHandle::with_size(1920, 1080);
        handle.set_scale_factor(2.0);
        let (pw, ph) = handle.physical_size();
        assert_eq!(pw, 3840);
        assert_eq!(ph, 2160);
    }
}
