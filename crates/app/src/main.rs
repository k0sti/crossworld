use app::{App, CREATE_APP_SYMBOL};
use glow::*;
use glutin::config::ConfigTemplateBuilder;
use glutin::context::{ContextApi, ContextAttributesBuilder, Version};
use glutin::display::GetGlDisplay;
use glutin::prelude::*;
use glutin::surface::{SurfaceAttributesBuilder, WindowSurface};
use glutin_winit::DisplayBuilder;
use libloading::Library;
use notify_debouncer_mini::{new_debouncer, notify::*, DebouncedEvent};
use raw_window_handle::HasWindowHandle;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

#[cfg(target_os = "linux")]
use winit::platform::x11::EventLoopBuilderExtX11;

/// Get the platform-specific library filename
fn get_lib_name() -> &'static str {
    #[cfg(target_os = "linux")]
    return "libgame.so";

    #[cfg(target_os = "macos")]
    return "libgame.dylib";

    #[cfg(target_os = "windows")]
    return "game.dll";
}

/// Get the path to the game library
fn get_lib_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // Go from crates/app to crates
    path.pop(); // Go from crates to workspace root
    path.push("target");
    path.push("debug");
    path.push(get_lib_name());
    path
}

struct AppRuntime {
    window: Option<Window>,
    gl_context: Option<glutin::context::PossiblyCurrentContext>,
    gl_surface: Option<glutin::surface::Surface<WindowSurface>>,
    gl: Option<Arc<Context>>,
    game_lib: Option<Library>,
    game_app: Option<Box<dyn App>>,
    last_update: Instant,
    file_events: Arc<Mutex<Receiver<std::result::Result<Vec<DebouncedEvent>, notify_debouncer_mini::notify::Error>>>>,
    last_reload_time: Option<Duration>,
}

impl AppRuntime {
    fn new(file_events: Arc<Mutex<Receiver<std::result::Result<Vec<DebouncedEvent>, notify_debouncer_mini::notify::Error>>>>) -> Self {
        Self {
            window: None,
            gl_context: None,
            gl_surface: None,
            gl: None,
            game_lib: None,
            game_app: None,
            last_update: Instant::now(),
            file_events,
            last_reload_time: None,
        }
    }
}

impl AppRuntime {
    /// Load the game library and create the app instance
    unsafe fn load_game(&mut self) {
        let lib_path = get_lib_path();

        println!("[AppRuntime] Loading game library from: {}", lib_path.display());

        if !lib_path.exists() {
            eprintln!("[AppRuntime] ERROR: Game library not found at: {}", lib_path.display());
            return;
        }

        let lib = match Library::new(&lib_path) {
            Ok(lib) => lib,
            Err(e) => {
                eprintln!("[AppRuntime] ERROR: Failed to load library: {}", e);
                return;
            }
        };

        let create_app: libloading::Symbol<unsafe extern "C" fn() -> *mut dyn App> = match lib.get(CREATE_APP_SYMBOL) {
            Ok(func) => func,
            Err(e) => {
                eprintln!("[AppRuntime] ERROR: Failed to find create_app symbol: {}", e);
                return;
            }
        };

        let app_ptr = create_app();
        if app_ptr.is_null() {
            eprintln!("[AppRuntime] ERROR: create_app returned null");
            return;
        }

        let app = Box::from_raw(app_ptr);

        // Initialize the app if we have a GL context
        if let Some(gl) = &self.gl {
            println!("[AppRuntime] Calling app.init()");
            let mut app = app;
            app.init(Arc::clone(gl));
            self.game_app = Some(app);
        } else {
            self.game_app = Some(app);
        }

        self.game_lib = Some(lib);

        println!("[AppRuntime] Game loaded successfully");
    }

    /// Unload the current game library
    unsafe fn unload_game(&mut self) {
        println!("[AppRuntime] Unloading game library");

        if let (Some(mut app), Some(gl)) = (self.game_app.take(), &self.gl) {
            println!("[AppRuntime] Calling app.uninit()");
            app.uninit(Arc::clone(gl));
            drop(app);
        }

        if let Some(lib) = self.game_lib.take() {
            drop(lib);
            println!("[AppRuntime] Game library unloaded");
        }
    }

    /// Reload the game library
    unsafe fn reload_game(&mut self) {
        let start_time = Instant::now();
        println!("[AppRuntime] ⏱️  Hot-reload triggered!");

        // Capture current window size before unloading
        let current_size = self.window.as_ref().map(|w| w.inner_size());

        self.unload_game();

        // Small delay to ensure file is fully written
        std::thread::sleep(Duration::from_millis(50));

        self.load_game();

        let reload_time = start_time.elapsed();
        self.last_reload_time = Some(reload_time);

        // Send resize event to new app instance to sync window size
        if let (Some(app), Some(size)) = (self.game_app.as_mut(), current_size) {
            app.event(&WindowEvent::Resized(size));
        }

        if self.game_app.is_some() {
            println!("[AppRuntime] ✅ Hot-reload successful in {:.2}ms", reload_time.as_secs_f64() * 1000.0);
        } else {
            eprintln!("[AppRuntime] ❌ Hot-reload failed after {:.2}ms", reload_time.as_secs_f64() * 1000.0);
            eprintln!("[AppRuntime] Continuing with previous version (if any)");
        }
    }
}

impl ApplicationHandler for AppRuntime {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        println!("[AppRuntime] Initializing window and GL context");

        let window_attributes = Window::default_attributes()
            .with_title("Hot-Reload Demo - Rotating Cube")
            .with_inner_size(winit::dpi::LogicalSize::new(800, 600));

        let template = ConfigTemplateBuilder::new()
            .with_alpha_size(8)
            .with_transparency(false);

        let display_builder = DisplayBuilder::new().with_window_attributes(Some(window_attributes));

        let (window, gl_config) = display_builder
            .build(event_loop, template, |configs| {
                configs
                    .reduce(|accum, config| {
                        if config.num_samples() > accum.num_samples() {
                            config
                        } else {
                            accum
                        }
                    })
                    .unwrap()
            })
            .unwrap();

        let window = window.unwrap();
        let window_handle = window.window_handle().ok().map(|h| h.as_raw());
        let gl_display = gl_config.display();

        // Request OpenGL 4.3 for compute shader support
        let context_attributes = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::OpenGl(Some(Version::new(4, 3))))
            .build(window_handle);

        let gl_context = unsafe {
            gl_display
                .create_context(&gl_config, &context_attributes)
                .unwrap()
        };

        let size = window.inner_size();
        let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            window_handle.unwrap(),
            std::num::NonZeroU32::new(size.width).unwrap(),
            std::num::NonZeroU32::new(size.height).unwrap(),
        );

        let gl_surface = unsafe {
            gl_display
                .create_window_surface(&gl_config, &attrs)
                .unwrap()
        };

        let gl_context = gl_context.make_current(&gl_surface).unwrap();

        let gl = Arc::new(unsafe {
            Context::from_loader_function_cstr(|s| gl_display.get_proc_address(s))
        });

        println!("[AppRuntime] OpenGL context created successfully");

        self.window = Some(window);
        self.gl_context = Some(gl_context);
        self.gl_surface = Some(gl_surface);
        self.gl = Some(gl);

        // Load the game library
        unsafe {
            self.load_game();
            if self.game_app.is_none() {
                eprintln!("[AppRuntime] Failed to load game, exiting...");
                event_loop.exit();
                return;
            }
        }

        self.last_update = Instant::now();
    }

    fn device_event(&mut self, _event_loop: &ActiveEventLoop, _device_id: winit::event::DeviceId, event: winit::event::DeviceEvent) {
        use winit::event::DeviceEvent;

        // Forward raw mouse motion to game for infinite mouse movement
        if let DeviceEvent::MouseMotion { delta } = event {
            if let Some(app) = &mut self.game_app {
                app.mouse_motion(delta);
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        // Forward events to game
        if let Some(app) = &mut self.game_app {
            app.event(&event);
        }

        match event {
            WindowEvent::CloseRequested => {
                println!("[AppRuntime] Close requested");
                unsafe {
                    self.unload_game();
                }
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let (Some(gl_surface), Some(gl_context)) =
                    (self.gl_surface.as_ref(), self.gl_context.as_ref())
                {
                    gl_surface.resize(
                        gl_context,
                        std::num::NonZeroU32::new(size.width).unwrap(),
                        std::num::NonZeroU32::new(size.height).unwrap(),
                    );
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                // Check for file changes
                let should_reload = {
                    let lib_path = get_lib_path();
                    let mut needs_reload = false;
                    if let Ok(rx) = self.file_events.lock() {
                        while let Ok(events_result) = rx.try_recv() {
                            if let Ok(events) = events_result {
                                for event in events {
                                    if event.path == lib_path {
                                        println!("[FileWatcher] Detected change: {}", event.path.display());
                                        needs_reload = true;
                                    }
                                }
                            }
                        }
                    }
                    needs_reload
                };

                if should_reload {
                    unsafe {
                        self.reload_game();
                    }
                }

                if let (Some(window), Some(gl), Some(app), Some(gl_context), Some(gl_surface)) = (
                    self.window.as_ref(),
                    self.gl.as_ref(),
                    self.game_app.as_mut(),
                    self.gl_context.as_ref(),
                    self.gl_surface.as_ref(),
                ) {
                    let now = Instant::now();
                    let delta_time = (now - self.last_update).as_secs_f32();
                    self.last_update = now;

                    // Update game logic
                    app.update(delta_time);

                    // Apply cursor state changes
                    if let Some((grab_mode, visible)) = app.cursor_state() {
                        // Hide or show cursor
                        window.set_cursor_visible(visible);

                        // Try to grab cursor, fall back to Confined if Locked isn't supported
                        if let Err(_) = window.set_cursor_grab(grab_mode) {
                            use winit::window::CursorGrabMode;
                            // On some platforms, Locked isn't supported, try Confined instead
                            if matches!(grab_mode, CursorGrabMode::Locked) {
                                let _ = window.set_cursor_grab(CursorGrabMode::Confined);
                            }
                        }
                    } else {
                        // Default: cursor visible and not grabbed
                        window.set_cursor_visible(true);
                        let _ = window.set_cursor_grab(winit::window::CursorGrabMode::None);
                    }

                    // Render
                    unsafe {
                        app.render(Arc::clone(gl));
                    }

                    gl_surface.swap_buffers(gl_context).unwrap();
                    window.request_redraw();
                }
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}

impl Drop for AppRuntime {
    fn drop(&mut self) {
        unsafe {
            self.unload_game();
        }
    }
}

fn main() {
    println!("[AppRuntime] Starting hot-reload runtime");

    let lib_path = get_lib_path();
    println!("[AppRuntime] Watching for changes: {}", lib_path.display());

    // Setup file watcher for hot-reload
    let (tx, rx) = channel();
    let mut _debouncer = new_debouncer(Duration::from_millis(100), tx)
        .expect("Failed to create file watcher");

    // Watch the parent directory (watching the file directly doesn't work well with atomic writes)
    let watch_dir = lib_path.parent().unwrap();
    _debouncer.watcher().watch(watch_dir, RecursiveMode::NonRecursive)
        .expect("Failed to watch directory");

    println!("[AppRuntime] File watcher initialized");

    let file_events = Arc::new(Mutex::new(rx));

    #[cfg(target_os = "linux")]
    let event_loop = {
        let mut builder = EventLoop::builder();
        builder.with_x11();
        builder.build().expect("Failed to create event loop")
    };

    #[cfg(not(target_os = "linux"))]
    let event_loop = EventLoop::new().expect("Failed to create event loop");

    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app_runtime = AppRuntime::new(file_events);

    event_loop.run_app(&mut app_runtime).expect("Event loop error");
}
