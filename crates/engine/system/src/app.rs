//! Application trait - re-exports from app
//!
//! The canonical application framework lives in the [`app`] crate.

pub use app::{App, CreateAppFn, FrameContext, CREATE_APP_SYMBOL};

#[cfg(test)]
mod tests {
    #[test]
    fn reexports_app_types() {
        let width = 1920u32;
        let height = 1080u32;
        let aspect = width as f32 / height as f32;
        assert!((aspect - 1.778).abs() < 0.01);
    }
}
