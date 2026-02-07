//! Hot-reload runtime for dynamic library loading
//!
//! Provides hot-reload capability for App implementations by watching
//! library files for changes and automatically reloading them while
//! preserving GL context and window state.

use crate::{App, FrameContext, CREATE_APP_SYMBOL};
use glow::Context;
use libloading::Library;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};
use winit::window::Window;

use notify_debouncer_mini::{new_debouncer, notify::*, DebouncedEvent};
use std::sync::mpsc::{channel, Receiver};

/// Configuration for hot-reload system
#[derive(Debug, Clone)]
pub struct HotReloadConfig {
    /// Path to the library file to watch
    pub library_path: PathBuf,
    /// Debounce duration for file changes (default: 100ms)
    pub debounce_duration: Duration,
    /// Print debug messages
    pub verbose: bool,
}

impl HotReloadConfig {
    /// Create configuration for a library in target/debug
    pub fn for_library(lib_name: &str) -> Self {
        let lib_path = Self::get_target_lib_path(lib_name);
        Self {
            library_path: lib_path,
            debounce_duration: Duration::from_millis(100),
            verbose: true,
        }
    }

    /// Get platform-specific library path in target/debug
    fn get_target_lib_path(name: &str) -> PathBuf {
        let filename = Self::platform_lib_filename(name);
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.pop(); // crates/engine/app -> crates/engine
        path.pop(); // crates/engine -> crates
        path.pop(); // crates -> workspace root
        path.push("target");
        path.push("debug");
        path.push(filename);
        path
    }

    /// Get platform-specific library filename
    fn platform_lib_filename(name: &str) -> String {
        #[cfg(target_os = "linux")]
        return format!("lib{}.so", name);

        #[cfg(target_os = "macos")]
        return format!("lib{}.dylib", name);

        #[cfg(target_os = "windows")]
        return format!("{}.dll", name);
    }
}

/// Hot-reload library manager
pub struct HotReloadLibrary {
    config: HotReloadConfig,
    library: Option<Library>,
    app_instance: Option<Box<dyn App>>,
    last_modified: Option<SystemTime>,
    file_watcher: Arc<Mutex<Receiver<Result<Vec<DebouncedEvent>, notify_debouncer_mini::notify::Error>>>>,
    _debouncer: notify_debouncer_mini::Debouncer<RecommendedWatcher>,
}

impl HotReloadLibrary {
    /// Create a new hot-reload library manager
    pub fn new(config: HotReloadConfig) -> Result<Self, String> {
        // Set up file watcher
        let (tx, rx) = channel();
        let mut debouncer = new_debouncer(config.debounce_duration, tx)
            .map_err(|e| format!("Failed to create file watcher: {}", e))?;

        // Watch the library file
        debouncer
            .watcher()
            .watch(&config.library_path, RecursiveMode::NonRecursive)
            .map_err(|e| format!("Failed to watch library file: {}", e))?;

        if config.verbose {
            println!("[HotReload] Watching: {}", config.library_path.display());
        }

        let file_watcher = Arc::new(Mutex::new(rx));

        Ok(Self {
            config,
            library: None,
            app_instance: None,
            last_modified: None,
            file_watcher,
            _debouncer: debouncer,
        })
    }

    /// Load the library and create app instance
    pub unsafe fn load(&mut self, window: &Window, gl: &Arc<Context>) -> Result<(), String> {
        if self.config.verbose {
            println!("[HotReload] Loading library: {}", self.config.library_path.display());
        }

        // Track modification time
        if let Ok(metadata) = std::fs::metadata(&self.config.library_path) {
            if let Ok(modified) = metadata.modified() {
                self.last_modified = Some(modified);
            }
        }

        if !self.config.library_path.exists() {
            return Err(format!("Library not found: {}", self.config.library_path.display()));
        }

        // Load library
        let lib = Library::new(&self.config.library_path)
            .map_err(|e| format!("Failed to load library: {}", e))?;

        // Get create_app symbol
        let symbol_name = std::str::from_utf8(CREATE_APP_SYMBOL).unwrap();
        let create_app: libloading::Symbol<unsafe extern "C" fn() -> *mut dyn App> =
            lib.get(CREATE_APP_SYMBOL)
                .map_err(|e| format!("Failed to find {} symbol: {}", symbol_name, e))?;

        // Create app instance
        let app_ptr = create_app();
        if app_ptr.is_null() {
            return Err("create_app returned null".to_string());
        }

        let mut app: Box<dyn App> = Box::from_raw(app_ptr);

        // Initialize the app
        let size = window.inner_size();
        let ctx = FrameContext {
            gl,
            window,
            delta_time: 0.0,
            elapsed: 0.0,
            frame: 0,
            size: (size.width, size.height),
        };
        app.init(&ctx);

        self.app_instance = Some(app);
        self.library = Some(lib);

        if self.config.verbose {
            println!("[HotReload] Library loaded successfully");
        }

        Ok(())
    }

    /// Unload the current library
    pub unsafe fn unload(&mut self, window: &Window, gl: &Arc<Context>, frame_count: u64, elapsed: f32) {
        if self.config.verbose {
            println!("[HotReload] Unloading library");
        }

        if let Some(mut app) = self.app_instance.take() {
            let size = window.inner_size();
            let ctx = FrameContext {
                gl,
                window,
                delta_time: 0.0,
                elapsed,
                frame: frame_count,
                size: (size.width, size.height),
            };
            app.shutdown(&ctx);
            drop(app);
        }

        if let Some(lib) = self.library.take() {
            drop(lib);
            if self.config.verbose {
                println!("[HotReload] Library unloaded");
            }
        }
    }

    /// Reload the library
    pub unsafe fn reload(&mut self, window: &Window, gl: &Arc<Context>, frame_count: u64, elapsed: f32) -> Result<(), String> {
        let start = Instant::now();
        if self.config.verbose {
            println!("[HotReload] Reload triggered!");
        }

        self.unload(window, gl, frame_count, elapsed);

        // Small delay to ensure file is fully written
        std::thread::sleep(Duration::from_millis(50));

        self.load(window, gl)?;

        // Clear pending file events
        if let Ok(rx) = self.file_watcher.lock() {
            while rx.try_recv().is_ok() {
                // Drain the channel
            }
        }

        if self.config.verbose {
            println!("[HotReload] Reloaded in {:?}", start.elapsed());
        }

        Ok(())
    }

    /// Check if a reload is needed based on file changes
    pub fn check_reload(&self) -> bool {
        if let Ok(rx) = self.file_watcher.lock() {
            let result: Result<Result<Vec<DebouncedEvent>, notify_debouncer_mini::notify::Error>, _> = rx.try_recv();
            if let Ok(Ok(events)) = result {
                // Check if any modify events occurred
                for event in &events {
                    if matches!(event.kind, notify_debouncer_mini::notify::EventKind::Modify(_)) {
                        // Verify the modification time changed to avoid spurious reloads
                        if let Ok(metadata) = std::fs::metadata(&event.path) {
                            if let Ok(modified) = metadata.modified() {
                                if Some(modified) != self.last_modified {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }
        false
    }

    /// Get the app instance
    pub fn app_mut(&mut self) -> Option<&mut Box<dyn App>> {
        self.app_instance.as_mut()
    }

    /// Get the app instance (immutable)
    pub fn app(&self) -> Option<&Box<dyn App>> {
        self.app_instance.as_ref()
    }
}

impl Drop for HotReloadLibrary {
    fn drop(&mut self) {
        // Library will be unloaded when dropped
        // App instance must be dropped first (already handled by field order)
    }
}
