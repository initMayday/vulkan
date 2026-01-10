use std::time::{Duration, Instant};

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::EventLoop;
use winit::platform::pump_events::{EventLoopExtPumpEvents, PumpStatus};
use winit::window::{Window, WindowAttributes, WindowId};

use crate::state::VulkanState;

mod state;

struct App {
    vk: Option<VulkanState>,

    window: Option<Window>,
    running: bool,
}

impl App {
    fn new() -> Self {
        Self {
            vk: None,
            window: None,
            running: true,
        }
    }

    fn tick(&mut self, _dt: Duration) {}

    fn render_frame(&mut self) {}

    fn cleanup(&mut self) {
        if let Some(vk) = self.vk.take() {
            vk.destroy();
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.window.is_none() {
            let attrs = WindowAttributes::default()
                .with_title("We Vulkaning")
                .with_resizable(false);
            self.window = Some(event_loop.create_window(attrs).unwrap());
        }

        // Initialize Vulkan once window exists
        if self.vk.is_none() {
            let window = self.window.as_ref().unwrap();
            self.vk = Some(VulkanState::new(window));
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
            WindowEvent::RedrawRequested => self.render_frame(),
            _ => {}
        }
    }
}

fn main() {
    let default_filter = if cfg!(debug_assertions) {
        tracing_subscriber::EnvFilter::new("info")
    } else {
        tracing_subscriber::EnvFilter::new("info")
    };

    let filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or(default_filter);

    tracing_subscriber::fmt().with_env_filter(filter).init();

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
