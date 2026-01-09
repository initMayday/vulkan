
use std::ffi::CString;
use std::time::{Duration, Instant};

use ash::{vk, Entry, Instance};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::EventLoop;
use winit::platform::pump_events::{EventLoopExtPumpEvents, PumpStatus};
use winit::raw_window_handle::HasDisplayHandle;
use winit::window::{Window, WindowAttributes, WindowId};

struct App {
    entry: Entry,
    instance: Option<Instance>,

    window: Option<Window>,
    running: bool,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.window.is_none() {
            let attrs = WindowAttributes::default()
                .with_title("We Vulkaning")
                .with_resizable(false);
            self.window = Some(event_loop.create_window(attrs).unwrap());
        }

        // Create Vulkan instance once we have a window (needed for required extensions).
        if self.instance.is_none() {
            let window = self.window.as_ref().unwrap();
            self.instance = Some(create_instance(&self.entry, window));
        }

        self.running = true;
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                self.running = false;
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                self.render_frame();
            }
            _ => {}
        }
    }
}

impl App {
    fn new() -> Self {
        Self {
            entry: Entry::linked(),
            instance: None,
            window: None,
            running: true, // must be true so the outer while-loop starts pumping
        }
    }

    fn tick(&mut self, _dt: Duration) {
        // update simulation here
    }

    fn render_frame(&mut self) {
        // render one frame here
    }

    fn cleanup(&mut self) {
        // cleanup here
        // Drop order: device -> surface -> instance -> entry, etc (later)
    }
}

fn create_instance(entry: &Entry, window: &Window) -> Instance {
    let app_name = CString::new("Vulkan Application").expect("CString");
    let engine_name = CString::new("Raw Dog").expect("CString");

    let app_info = vk::ApplicationInfo::default()
        .application_name(app_name.as_c_str())
        .application_version(vk::make_api_version(0, 0, 1, 0))
        .engine_name(engine_name.as_c_str())
        .engine_version(vk::make_api_version(0, 0, 1, 0))
        .api_version(vk::make_api_version(0, 1, 0, 0));

    // ask the platform what extensions are required for surfaces
    let extension_names = ash_window::enumerate_required_extensions(
        window.display_handle().unwrap().as_raw(),
    )
    .expect("required extensions")
    .to_vec();

    // If you later add validation + debug messenger, you'd push EXT_debug_utils here:
    // extension_names.push(vk::EXT_DEBUG_UTILS_NAME.as_ptr());

    let create_info = vk::InstanceCreateInfo::default()
        .application_info(&app_info)
        .enabled_extension_names(&extension_names);

    unsafe { entry.create_instance(&create_info, None).expect("create_instance") }
}

fn main() {
    let mut event_loop = EventLoop::new().unwrap();
    let mut app = App::new();

    let target_fps: u32 = 120;
    let frame_dt = Duration::from_secs_f64(1.0 / target_fps as f64);

    let mut next_frame = Instant::now();
    let mut last_tick = Instant::now();

    while app.running {
        let now = Instant::now();
        let timeout = next_frame.saturating_duration_since(now);

        let status = event_loop.pump_app_events(Some(timeout), &mut app);
        if matches!(status, PumpStatus::Exit(_)) {
            break;
        }

        let now = Instant::now();
        if now >= next_frame {
            let dt = now.duration_since(last_tick);
            last_tick = now;

            app.tick(dt);

            if let Some(w) = app.window.as_ref() {
                w.request_redraw();
            }

            next_frame += frame_dt;
            if now > next_frame + frame_dt {
                next_frame = now + frame_dt;
            }
        }
    }

    app.cleanup();
}
